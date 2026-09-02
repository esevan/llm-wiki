from __future__ import annotations

from typing import Any

from llm_wiki.core.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError

TRANSLATED_FIELDS = {
    "captures": "text",
    "solution_progress_entries": "body",
    "solution_progress_comments": "body",
    "solution_checklist_items": "body",
}


class DerivedTranslationHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor("derived_translation", result_interface="owning_content")

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
        entity_type = str(context.payload.get("entity_type", ""))
        entity_id = str(context.payload.get("entity_id", ""))
        field = str(context.payload.get("field", ""))
        source = str(context.payload.get("source", ""))
        source_locale = str(context.payload.get("source_locale", "en"))
        if digest(source) != context.source_hash:
            raise StaleJobError("Derived translation source did not match its queued snapshot")
        table_field = TRANSLATED_FIELDS.get(entity_type)
        if not table_field:
            raise WorkflowError("Unsupported derived translation target")
        self._require_unchanged(entity_type, entity_id, table_field, context.source_hash, "before")
        provider = self.provider_factory("knowledge_translation")
        try:
            result = await provider.complete_json(self._messages(source), "derived bilingual translation")
        finally:
            await provider.aclose()
        versions = {locale: {field: str(result.get(locale, "")).strip()} for locale in ("ko", "en")}
        if any(not values[field] for values in versions.values()):
            raise ValueError("Derived translation requires Korean and English versions")
        self._require_unchanged(entity_type, entity_id, table_field, context.source_hash, "during")
        versions[source_locale] = {field: source}
        self.workflow.localized.save_versions(
            entity_type,
            entity_id,
            versions,
            source_hash=context.source_hash,
            complete=False,
        )
        self.workflow.db.commit()
        return {
            "entity_type": entity_type,
            "entity_id": entity_id,
            "field": field,
            "available_locales": ["ko", "en"],
        }

    def _require_unchanged(
        self,
        entity_type: str,
        entity_id: str,
        table_field: str,
        source_hash: str,
        phase: str,
    ) -> None:
        row = self.workflow.db.execute(
            f"SELECT {table_field} FROM {entity_type} WHERE id=?",
            (entity_id,),
        ).fetchone()
        if not row or digest(str(row[0])) != source_hash:
            raise StaleJobError(f"Authored source changed {phase} derived translation")

    @staticmethod
    def _messages(source: str) -> list[dict[str, str]]:
        return [
            {
                "role": "system",
                "content": (
                    'Return JSON only: {"ko":string,"en":string}. Produce natural Korean and English versions of '
                    "the supplied user-authored text. Preserve code, URLs, paths, identifiers, and quoted text "
                    "exactly. "
                    "Never add facts."
                ),
            },
            {"role": "user", "content": source},
        ]
