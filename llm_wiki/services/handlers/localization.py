from __future__ import annotations

from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    VaultKnowledgeTranslationCache,
    knowledge_translation_blocks,
)
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine, WorkflowError


class LocalizationJobHandlers:
    def __init__(self, workflow: WorkflowEngine, settings: ProviderSettings, vault: MarkdownVaultAdapter):
        self.workflow = workflow
        self.settings = settings
        self.vault = vault
        self.cache = VaultKnowledgeTranslationCache(vault, KnowledgeTranslationCache(workflow.db))

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("knowledge_translation", result_interface="knowledge_document"), self.knowledge)
        registry.register(TaskDescriptor("derived_translation", result_interface="owning_content"), self.derived)

    def _provider(self) -> AsyncOpenAICompatibleProvider:
        base_url, api_key, model = self.settings.credentials("knowledge_translation")
        return AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)

    async def knowledge(self, context: HandlerContext) -> dict[str, Any]:
        path = str(context.payload.get("path", ""))
        canonical = self.vault.read_text(path)
        if digest(canonical) != context.source_hash:
            raise StaleJobError("Canonical Knowledge changed before translation")
        blocks = knowledge_translation_blocks(canonical)
        indices = [index for index, block in enumerate(blocks) if block["translatable"]]
        provider = self._provider()
        model = context.model or provider.model
        saved = {item.unit_key: item.result for item in await context.checkpoints(context.source_hash, model)}
        try:
            completed = 0
            for ordinal, index in enumerate(indices):
                unit_key = f"paragraph:{index}"
                original = str(blocks[index]["markdown"])
                unit_hash = digest(original)
                checkpoint = saved.get(unit_key)
                if checkpoint and checkpoint.get("unit_hash") == unit_hash:
                    translated = str(checkpoint["markdown"])
                else:
                    if await context.cancelled():
                        raise InterruptedError("Knowledge translation cancelled")
                    result = await provider.complete_json([
                        {"role": "system", "content": "Return JSON only: {\"markdown\":string}. Translate this complete English Markdown paragraph into natural Korean. Preserve headings, code, identifiers, citations, quoted evidence, URLs, paths, and link targets exactly. Never add facts."},
                        {"role": "user", "content": original.strip()},
                    ], "knowledge Korean paragraph translation")
                    translated = str(result.get("markdown", "")).strip()
                    if not translated:
                        raise ValueError("Knowledge paragraph translation was empty")
                    await context.save_checkpoint(unit_key, context.source_hash, model, ordinal, {"unit_hash": unit_hash, "markdown": translated})
                trailing = original[len(original.rstrip()):]
                blocks[index]["markdown"] = translated + trailing
                completed += 1
                await context.progress(completed, len(indices))
        finally:
            await provider.aclose()
        translated_markdown = "".join(str(block["prefix"]) + str(block["markdown"]) for block in blocks)
        current = digest(self.vault.read_text(path))
        if current != context.source_hash:
            raise StaleJobError("Canonical Knowledge changed during translation")
        if not self.cache.put(path, "ko", context.source_hash, translated_markdown, model, current_source_hash=current):
            raise StaleJobError("Canonical Knowledge changed before publication")
        return {"path": path, "locale": "ko", "source_hash": context.source_hash, "derived_path": self.vault.korean_translation_path(path)}

    async def derived(self, context: HandlerContext) -> dict[str, Any]:
        entity_type = str(context.payload.get("entity_type", ""))
        entity_id = str(context.payload.get("entity_id", ""))
        field = str(context.payload.get("field", ""))
        source = str(context.payload.get("source", ""))
        source_locale = str(context.payload.get("source_locale", "en"))
        if digest(source) != context.source_hash:
            raise StaleJobError("Derived translation source did not match its queued snapshot")
        if entity_type not in {"captures", "solution_progress_entries", "solution_progress_comments", "solution_checklist_items"}:
            raise WorkflowError("Unsupported derived translation target")
        table_field = {"captures": "text", "solution_progress_entries": "body", "solution_progress_comments": "body", "solution_checklist_items": "body"}[entity_type]
        row = self.workflow.db.execute(f"SELECT {table_field} FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
        if not row or digest(str(row[0])) != context.source_hash:
            raise StaleJobError("Authored source changed before derived translation")
        provider = self._provider()
        try:
            result = await provider.complete_json([
                {"role": "system", "content": "Return JSON only: {\"ko\":string,\"en\":string}. Produce natural Korean and English versions of the supplied user-authored text. Preserve code, URLs, paths, identifiers, and quoted text exactly. Never add facts."},
                {"role": "user", "content": source},
            ], "derived bilingual translation")
        finally:
            await provider.aclose()
        versions = {locale: {field: str(result.get(locale, "")).strip()} for locale in ("ko", "en")}
        if any(not values[field] for values in versions.values()):
            raise ValueError("Derived translation requires Korean and English versions")
        current = self.workflow.db.execute(f"SELECT {table_field} FROM {entity_type} WHERE id=?", (entity_id,)).fetchone()
        if not current or digest(str(current[0])) != context.source_hash:
            raise StaleJobError("Authored source changed during derived translation")
        versions[source_locale] = {field: source}
        self.workflow.localized.save_versions(entity_type, entity_id, versions, source_hash=context.source_hash, complete=False)
        self.workflow.db.commit()
        return {"entity_type": entity_type, "entity_id": entity_id, "field": field, "available_locales": ["ko", "en"]}
