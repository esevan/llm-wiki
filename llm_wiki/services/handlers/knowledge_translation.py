from __future__ import annotations

from typing import Any

from llm_wiki.core.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.handlers.provider import ProviderBackedHandler, ProviderFactory
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    VaultKnowledgeTranslationCache,
    knowledge_translation_blocks,
)
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine


class KnowledgeTranslationHandler(ProviderBackedHandler):
    descriptor = TaskDescriptor("knowledge_translation", result_interface="knowledge_document")

    def __init__(
        self,
        workflow: WorkflowEngine,
        settings: ProviderSettings,
        vault: MarkdownVaultAdapter,
        provider_factory: ProviderFactory | None = None,
    ):
        super().__init__(settings, provider_factory)
        self.vault = vault
        self.cache = VaultKnowledgeTranslationCache(vault, KnowledgeTranslationCache(workflow.db))

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(self.descriptor, self)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        path = str(context.payload.get("path", ""))
        canonical = self.vault.read_text(path)
        if digest(canonical) != context.source_hash:
            raise StaleJobError("Canonical Knowledge changed before translation")
        blocks = knowledge_translation_blocks(canonical)
        indices = [index for index, block in enumerate(blocks) if block["translatable"]]
        provider = self.provider_factory("knowledge_translation")
        model = context.model or provider.model
        saved = {item.unit_key: item.result for item in await context.checkpoints(context.source_hash, model)}
        try:
            await self._translate_blocks(context, provider, blocks, indices, saved, model)
        finally:
            await provider.aclose()
        translated_markdown = "".join(str(block["prefix"]) + str(block["markdown"]) for block in blocks)
        current = digest(self.vault.read_text(path))
        if current != context.source_hash:
            raise StaleJobError("Canonical Knowledge changed during translation")
        if not self.cache.put(path, "ko", context.source_hash, translated_markdown, model, current_source_hash=current):
            raise StaleJobError("Canonical Knowledge changed before publication")
        return {
            "path": path,
            "locale": "ko",
            "source_hash": context.source_hash,
            "derived_path": self.vault.korean_translation_path(path),
        }

    async def _translate_blocks(
        self,
        context: HandlerContext,
        provider: Any,
        blocks: list[dict[str, object]],
        indices: list[int],
        saved: dict[str, dict[str, Any]],
        model: str,
    ) -> None:
        for completed, index in enumerate(indices, start=1):
            unit_key = f"paragraph:{index}"
            original = str(blocks[index]["markdown"])
            unit_hash = digest(original)
            checkpoint = saved.get(unit_key)
            translated = await self._translated_block(
                context,
                provider,
                checkpoint,
                unit_key,
                unit_hash,
                original,
                model,
                completed - 1,
            )
            trailing = original[len(original.rstrip()) :]
            blocks[index]["markdown"] = translated + trailing
            await context.progress(completed, len(indices))

    @staticmethod
    async def _translated_block(
        context: HandlerContext,
        provider: Any,
        checkpoint: dict[str, Any] | None,
        unit_key: str,
        unit_hash: str,
        original: str,
        model: str,
        ordinal: int,
    ) -> str:
        if checkpoint and checkpoint.get("unit_hash") == unit_hash:
            return str(checkpoint["markdown"])
        if await context.cancelled():
            raise InterruptedError("Knowledge translation cancelled")
        result = await provider.complete_json(
            [
                {
                    "role": "system",
                    "content": (
                        'Return JSON only: {"markdown":string}. Translate this complete English Markdown paragraph '
                        "into natural Korean. Preserve headings, code, identifiers, citations, quoted evidence, URLs, "
                        "paths, and link targets exactly. Never add facts."
                    ),
                },
                {"role": "user", "content": original.strip()},
            ],
            "knowledge Korean paragraph translation",
        )
        translated = str(result.get("markdown", "")).strip()
        if not translated:
            raise ValueError("Knowledge paragraph translation was empty")
        await context.save_checkpoint(
            unit_key,
            context.source_hash,
            model,
            ordinal,
            {"unit_hash": unit_hash, "markdown": translated},
        )
        return translated
