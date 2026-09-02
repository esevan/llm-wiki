from __future__ import annotations

from typing import Any

from llm_wiki.core.jobs import TaskDescriptor
from llm_wiki.services.conversation import bilingual_draft_prompt
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.handlers.targets import job_target, require_source_hash
from llm_wiki.services.handlers.validation import (
    validate_bilingual_draft,
    validate_draft,
)
from llm_wiki.services.localization import SUPPORTED_LOCALES
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


class WorkflowDraftHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor("workflow_draft", result_interface="inline_preview")

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
        next_stage = {"captures": "problems", "problems": "features"}.get(entity_type)
        if not next_stage:
            raise WorkflowError("This workflow item has no next stage")
        task = "problem_drafting" if next_stage == "problems" else "solution_drafting"
        provider = self.provider_factory(task)
        try:
            result = await provider.complete_json(
                [
                    {"role": "system", "content": bilingual_draft_prompt(entity_type, item["title"], item["detail"])},
                    *self.workflow.chat_history(entity_type, entity_id),
                ],
                f"{entity_type} draft",
            )
        finally:
            await provider.aclose()
        current = self.workflow.context_for(entity_type, entity_id, locale)
        require_source_hash(context, digest(f"{current['title']}\n{current['detail']}"))
        versions = self._versions(entity_type, locale, result)
        reviewed = versions[locale]
        payload = {
            **reviewed,
            "source_locale": locale,
            "localized_versions": versions,
            "missing_locales": [value for value in SUPPORTED_LOCALES if value not in versions],
        }
        self.workflow.record_ai_run(
            entity_type,
            entity_id,
            "workflow_draft",
            "Create a reviewed bilingual draft",
            str(payload),
        )
        return payload

    @staticmethod
    def _versions(entity_type: str, locale: str, result: dict[str, Any]) -> dict[str, dict[str, str]]:
        if set(result) == set(SUPPORTED_LOCALES):
            return validate_bilingual_draft(entity_type, result)
        return {locale: validate_draft(entity_type, result)}
