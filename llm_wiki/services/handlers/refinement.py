from __future__ import annotations

from typing import Any

from llm_wiki.core.jobs import TaskDescriptor
from llm_wiki.services.conversation import bilingual_refinement_prompt
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.handlers.targets import job_target, require_source_hash
from llm_wiki.services.handlers.validation import validate_refinement
from llm_wiki.services.localization import (
    SUPPORTED_LOCALES,
    response_language_instruction,
)
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine


class WorkflowRefinementHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor("workflow_refinement", result_interface="inline_preview")

    def __init__(
        self,
        workflow: WorkflowEngine,
        settings: ProviderSettings,
        provider_factory: ProviderFactory | None = None,
    ):
        super().__init__(settings, provider_factory)
        self.workflow = workflow

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(self.descriptor, self)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        entity_type, entity_id = job_target(context)
        locale = str(context.payload.get("locale", "en"))
        item = self.workflow.context_for(entity_type, entity_id, locale)
        require_source_hash(context, digest(f"{item['title']}\n{item['detail']}"))
        provider = self.provider_factory(self._task(entity_type))
        try:
            result = await provider.complete_json(
                self._messages(entity_type, entity_id, locale, item),
                f"{entity_type} refinement",
            )
        finally:
            await provider.aclose()
        current = self.workflow.context_for(entity_type, entity_id, locale)
        require_source_hash(context, digest(f"{current['title']}\n{current['detail']}"))
        versions = self._versions(entity_type, locale, result)
        reviewed = versions.get(locale) or validate_refinement(entity_type, result)
        payload = {
            **reviewed,
            "source_note": item["detail"] or item["title"],
            "localized_versions": versions,
            "missing_locales": [value for value in SUPPORTED_LOCALES if value not in versions] if versions else [],
        }
        self.workflow.record_ai_run(entity_type, entity_id, "workflow_refinement", "Refine current item", str(payload))
        return payload

    def _messages(
        self,
        entity_type: str,
        entity_id: str,
        locale: str,
        item: dict[str, Any],
    ) -> list[dict[str, object]]:
        messages: list[dict[str, object]] = [
            {"role": "system", "content": bilingual_refinement_prompt(entity_type, item["title"], item["detail"])}
        ]
        if entity_type == "captures":
            messages.append({"role": "system", "content": response_language_instruction(locale)})
        messages.extend(self.workflow.chat_history(entity_type, entity_id))
        return messages

    @staticmethod
    def _task(entity_type: str) -> str:
        return {
            "captures": "capture_assistance",
            "problems": "problem_assistance",
            "features": "solution_assistance",
        }.get(entity_type, "")

    @staticmethod
    def _versions(entity_type: str, locale: str, result: dict[str, Any]) -> dict[str, dict[str, str]]:
        if entity_type in {"problems", "features"} and set(result) == set(SUPPORTED_LOCALES):
            return {value: validate_refinement(entity_type, dict(result[value])) for value in SUPPORTED_LOCALES}
        if entity_type in {"problems", "features"}:
            return {locale: validate_refinement(entity_type, result)}
        return {}
