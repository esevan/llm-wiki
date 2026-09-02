from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.completion_archive import CompletionArchivePublisher
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.jobs import TaskDescriptor
from llm_wiki.services.jobs import StaleJobError
from llm_wiki.services.lineage import readable_report_context
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings


class CompletionReportHandler:
    def __init__(self, publisher: CompletionArchivePublisher, settings: ProviderSettings, provider_factory: Callable[[], AsyncOpenAICompatibleProvider] | None = None):
        self.publisher = publisher
        self.settings = settings
        self.provider_factory = provider_factory

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("completion_report", result_interface="completed_knowledge"), self.__call__)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        problem_id = str(context.payload.get("entity_id", ""))
        current_lineages = self.publisher.lineages(problem_id)
        queued_hash = digest(json.dumps(current_lineages, sort_keys=True, ensure_ascii=False))
        if context.source_hash and context.source_hash != queued_hash:
            raise StaleJobError("Completion evidence changed before report generation")
        lineages = self.publisher.lineages(problem_id, refresh=True) if context.payload.get("refresh_lineage") else current_lineages
        report_input = readable_report_context(lineages)
        if self.provider_factory:
            provider = self.provider_factory()
        else:
            base_url, api_key, model = self.settings.credentials("completion_report")
            provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)
        try:
            report = await provider.complete_json([
                {"role": "system", "content": "Return JSON only: {\"executive_summary_markdown\":string,\"report_body_markdown\":string}. Prepare concise English-canonical Knowledge from supplied validated Lineage only. Preserve Observed, Decided, and Inferred distinctions; cite human-readable evidence labels; never expose internal IDs or invent facts."},
                {"role": "user", "content": report_input},
            ], "completion executive summary")
        finally:
            await provider.aclose()
        executive = str(report.get("executive_summary_markdown", "")).strip()
        body = str(report.get("report_body_markdown", "")).strip()
        if not context.payload.get("refresh_lineage") and digest(json.dumps(self.publisher.lineages(problem_id), sort_keys=True, ensure_ascii=False)) != context.source_hash:
            raise StaleJobError("Completion evidence changed during report generation")
        self.publisher.workflow.record_ai_run("problems", problem_id, "completion_executive_summary", "Validated Lineage", json.dumps(report, ensure_ascii=False))
        return self.publisher.publish(problem_id, lineages=lineages, executive_summary=executive, report_body=body, status="generated")
