from __future__ import annotations

import json
from typing import Any

from llm_wiki.core.jobs import TaskDescriptor
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.handlers.targets import job_target, require_source_hash
from llm_wiki.services.localization import response_language_instruction
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


class CompletionReviewHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor(
        "completion_review",
        result_interface="completion_review",
        notification_policy="review_ready",
    )

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
        _entity_type, feature_id = job_target(context)
        locale = str(context.payload.get("locale", "en"))
        feature, problem, progress = self._review_context(feature_id, locale)
        require_source_hash(context, self._source_hash(feature, progress))
        provider = self.provider_factory("completion_review")
        try:
            report = await provider.complete_json(
                self._messages(feature, problem, progress, locale),
                "completion review",
            )
        finally:
            await provider.aclose()
        current_feature, _problem, current_progress = self._review_context(feature_id, locale)
        require_source_hash(context, self._source_hash(current_feature, current_progress))
        review_id = self.workflow.save_completion_review(feature_id, report)
        return {
            "review_id": review_id,
            "problem_id": feature["problem_id"],
            "feature_id": feature_id,
            "report": report,
        }

    def _review_context(
        self,
        feature_id: str,
        locale: str,
    ) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
        board = self.workflow.board(locale)
        feature = next((item for item in board["features"] if item["id"] == feature_id), None)
        if not feature:
            raise WorkflowError("Solution not found")
        problem = next((item for item in board["problems"] if item["id"] == feature["problem_id"]), {})
        return feature, problem, self.workflow.solution_progress(feature_id, locale)

    @staticmethod
    def _source_hash(feature: dict[str, Any], progress: dict[str, Any]) -> str:
        return digest(json.dumps({"feature": feature, "progress": progress}, sort_keys=True, ensure_ascii=False))

    @staticmethod
    def _messages(
        feature: dict[str, Any],
        problem: dict[str, Any],
        progress: dict[str, Any],
        locale: str,
    ) -> list[dict[str, object]]:
        entries = [{key: value for key, value in entry.items() if key != "image_data"} for entry in progress["entries"]]
        return [
            {
                "role": "system",
                "content": (
                    'Return JSON only: {"resolution":"complete|partial|insufficient_evidence",'
                    '"executive_summary":string,"what_changed":[string],"criteria_review":'
                    '[{"criterion":string,"status":"met|partial|not_evidenced","evidence":string}],'
                    '"remaining_checklist":[string],"decision_rationale":string,'
                    '"problem_recommendation":"complete|keep_open",'
                    '"capture_recommendation":"complete|keep_open"}. Create a concise factual completion report '
                    "from supplied evidence only. Assess every Validation Criteria bullet separately. Cite exact "
                    "recorded evidence; never infer invisible implementation details or change state."
                ),
            },
            {"role": "system", "content": response_language_instruction(locale)},
            {
                "role": "user",
                "content": json.dumps(
                    {
                        "problem": problem,
                        "solution": feature,
                        "validation_criteria": feature.get("validation_criteria", ""),
                        "progress_records": entries,
                        "checklist": progress["checklist"],
                    },
                    ensure_ascii=False,
                ),
            },
        ]
