from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.conversation import bilingual_draft_prompt, bilingual_refinement_prompt
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.handlers.validation import (
    validate_bilingual_draft,
    validate_bilingual_image_summary,
    validate_draft,
    validate_refinement,
)
from llm_wiki.services.jobs import TaskDescriptor
from llm_wiki.services.jobs import StaleJobError
from llm_wiki.services.localization import SUPPORTED_LOCALES, response_language_instruction
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


ProviderFactory = Callable[[str], AsyncOpenAICompatibleProvider]


TASK_DESCRIPTORS = (
    TaskDescriptor("workflow_draft", result_interface="inline_preview"),
    TaskDescriptor("workflow_refinement", result_interface="inline_preview"),
    TaskDescriptor("image_summary", result_interface="solution_work_summary"),
    TaskDescriptor("completion_review", result_interface="completion_review", notification_policy="review_ready"),
)


class WorkflowJobHandlers:
    def __init__(self, workflow: WorkflowEngine, settings: ProviderSettings, provider_factory: ProviderFactory | None = None):
        self.workflow = workflow
        self.settings = settings
        self.provider_factory = provider_factory or self._provider

    def register(self, registry: HandlerRegistry) -> None:
        handlers = {
            "workflow_draft": self.draft,
            "workflow_refinement": self.refine,
            "image_summary": self.image_summary,
            "completion_review": self.completion_review,
        }
        for descriptor in TASK_DESCRIPTORS:
            registry.register(descriptor, handlers[descriptor.task_kind])

    def _provider(self, task: str) -> AsyncOpenAICompatibleProvider:
        base_url, api_key, model = self.settings.credentials(task)
        return AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)

    async def draft(self, context: HandlerContext) -> dict[str, Any]:
        entity_type, entity_id = _target(context)
        locale = str(context.payload.get("locale", "en"))
        item = self.workflow.context_for(entity_type, entity_id, locale)
        _require_hash(context, digest(f"{item['title']}\n{item['detail']}"))
        next_stage = {"captures": "problems", "problems": "features"}.get(entity_type)
        if not next_stage:
            raise WorkflowError("This workflow item has no next stage")
        task = "problem_drafting" if next_stage == "problems" else "solution_drafting"
        provider = self.provider_factory(task)
        try:
            result = await provider.complete_json(
                [{"role": "system", "content": bilingual_draft_prompt(entity_type, item["title"], item["detail"])}, *self.workflow.chat_history(entity_type, entity_id)],
                f"{entity_type} draft",
            )
        finally:
            await provider.aclose()
        current = self.workflow.context_for(entity_type, entity_id, locale)
        _require_hash(context, digest(f"{current['title']}\n{current['detail']}"))
        if set(result) == set(SUPPORTED_LOCALES):
            versions = validate_bilingual_draft(entity_type, result)
            reviewed = versions[locale]
        else:
            reviewed = validate_draft(entity_type, result)
            versions = {locale: reviewed}
        payload = {**reviewed, "source_locale": locale, "localized_versions": versions, "missing_locales": [value for value in SUPPORTED_LOCALES if value not in versions]}
        self.workflow.record_ai_run(entity_type, entity_id, "workflow_draft", "Create a reviewed bilingual draft", str(payload))
        return payload

    async def refine(self, context: HandlerContext) -> dict[str, Any]:
        entity_type, entity_id = _target(context)
        locale = str(context.payload.get("locale", "en"))
        item = self.workflow.context_for(entity_type, entity_id, locale)
        _require_hash(context, digest(f"{item['title']}\n{item['detail']}"))
        task = {"captures": "capture_assistance", "problems": "problem_assistance", "features": "solution_assistance"}.get(entity_type)
        provider = self.provider_factory(str(task or ""))
        messages: list[dict[str, object]] = [{"role": "system", "content": bilingual_refinement_prompt(entity_type, item["title"], item["detail"])}]
        if entity_type == "captures":
            messages.append({"role": "system", "content": response_language_instruction(locale)})
        messages.extend(self.workflow.chat_history(entity_type, entity_id))
        try:
            result = await provider.complete_json(messages, f"{entity_type} refinement")
        finally:
            await provider.aclose()
        current = self.workflow.context_for(entity_type, entity_id, locale)
        _require_hash(context, digest(f"{current['title']}\n{current['detail']}"))
        if entity_type in {"problems", "features"} and set(result) == set(SUPPORTED_LOCALES):
            versions = {value: validate_refinement(entity_type, dict(result[value])) for value in SUPPORTED_LOCALES}
            reviewed = versions[locale]
        else:
            reviewed = validate_refinement(entity_type, result)
            versions = {locale: reviewed} if entity_type in {"problems", "features"} else {}
        payload = {**reviewed, "source_note": item["detail"] or item["title"], "localized_versions": versions, "missing_locales": [value for value in SUPPORTED_LOCALES if value not in versions] if versions else []}
        self.workflow.record_ai_run(entity_type, entity_id, "workflow_refinement", "Refine current item", str(payload))
        return payload

    async def image_summary(self, context: HandlerContext) -> dict[str, Any]:
        _entity_type, entry_id = _target(context)
        locale = str(context.payload.get("locale", "en"))
        row = self.workflow.db.execute("SELECT image_data,image_media_type,feature_id FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone()
        if not row or not row[0]:
            raise WorkflowError("Progress image is no longer available")
        _require_hash(context, digest(str(row[0])))
        provider = self.provider_factory("image_summary")
        content: list[object] = [
            {"type": "text", "text": "Return JSON only with exactly this shape: {\"ko\":{\"summary\":string},\"en\":{\"summary\":string}}. Summarize this work-progress image accurately and concisely in natural Korean and English in this one response. Both versions must describe the same visible evidence. Do not infer invisible details."},
            {"type": "image_url", "image_url": {"url": f"data:{row[1] or 'image/png'};base64,{row[0]}"}},
        ]
        try:
            result = await provider.complete_json([{"role": "user", "content": content}], "progress image summary")
        finally:
            await provider.aclose()
        current = self.workflow.db.execute("SELECT image_data FROM solution_progress_entries WHERE id=?", (entry_id,)).fetchone()
        if not current:
            raise StaleJobError("Progress entry was removed while its summary was running")
        _require_hash(context, digest(str(current[0])))
        versions = validate_bilingual_image_summary(result)
        self.workflow.set_solution_progress_summaries(entry_id, versions, locale)
        return {"summary": versions[locale]["image_summary"], "localized_versions": versions, "missing_locales": [], "entry_id": entry_id, "feature_id": str(row[2])}

    async def completion_review(self, context: HandlerContext) -> dict[str, Any]:
        _entity_type, feature_id = _target(context)
        locale = str(context.payload.get("locale", "en"))
        board = self.workflow.board(locale)
        feature = next((item for item in board["features"] if item["id"] == feature_id), None)
        if not feature:
            raise WorkflowError("Solution not found")
        problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
        progress = self.workflow.solution_progress(feature_id, locale)
        _require_hash(context, digest(json.dumps({"feature": feature, "progress": progress}, sort_keys=True, ensure_ascii=False)))
        entries = [{key: value for key, value in entry.items() if key != "image_data"} for entry in progress["entries"]]
        provider = self.provider_factory("completion_review")
        try:
            report = await provider.complete_json([
                {"role": "system", "content": "Return JSON only: {\"resolution\":\"complete|partial|insufficient_evidence\",\"executive_summary\":string,\"what_changed\":[string],\"criteria_review\":[{\"criterion\":string,\"status\":\"met|partial|not_evidenced\",\"evidence\":string}],\"remaining_checklist\":[string],\"decision_rationale\":string,\"problem_recommendation\":\"complete|keep_open\",\"capture_recommendation\":\"complete|keep_open\"}. Create a concise factual completion report from supplied evidence only. Assess every Validation Criteria bullet separately. Cite exact recorded evidence; never infer invisible implementation details or change state."},
                {"role": "system", "content": response_language_instruction(locale)},
                {"role": "user", "content": json.dumps({"problem": problem, "solution": feature, "validation_criteria": feature.get("validation_criteria", ""), "progress_records": entries, "checklist": progress["checklist"]}, ensure_ascii=False)},
            ], "completion review")
        finally:
            await provider.aclose()
        current_board = self.workflow.board(locale)
        current_feature = next((item for item in current_board["features"] if item["id"] == feature_id), None)
        if not current_feature:
            raise StaleJobError("Solution was removed while its review was running")
        current_progress = self.workflow.solution_progress(feature_id, locale)
        _require_hash(context, digest(json.dumps({"feature": current_feature, "progress": current_progress}, sort_keys=True, ensure_ascii=False)))
        review_id = self.workflow.save_completion_review(feature_id, report)
        return {"review_id": review_id, "problem_id": feature["problem_id"], "feature_id": feature_id, "report": report}


def _target(context: HandlerContext) -> tuple[str, str]:
    entity_type = str(context.payload.get("entity_type", ""))
    entity_id = str(context.payload.get("entity_id", ""))
    if not entity_type or not entity_id:
        raise ValueError("Job target is required")
    return entity_type, entity_id


def _require_hash(context: HandlerContext, current: str) -> None:
    if context.source_hash and context.source_hash != current:
        raise StaleJobError("Source changed while this AI job was queued or running")
