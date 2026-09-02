from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.jobs import TaskDescriptor
from llm_wiki.services.jobs import StaleJobError
from llm_wiki.services.lineage import report_context, validate_inference_payload
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine


class LineageInferenceHandler:
    def __init__(self, workflow: WorkflowEngine, settings: ProviderSettings, provider_factory: Callable[[], AsyncOpenAICompatibleProvider] | None = None):
        self.workflow = workflow
        self.settings = settings
        self.provider_factory = provider_factory

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("lineage_inference", result_interface="solution_lineage"), self.__call__)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        feature_id = str(context.payload.get("entity_id", ""))
        if context.source_hash != lineage_source_hash(self.workflow, feature_id):
            raise StaleJobError("Lineage source changed before inference")
        lineage = self.workflow.create_lineage_snapshot(feature_id, force=bool(context.payload.get("force", True)))
        if self.provider_factory:
            provider = self.provider_factory()
        else:
            base_url, api_key, model = self.settings.credentials("lineage_inference")
            provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)
        try:
            result = await provider.complete_json([
                {"role": "system", "content": "Return JSON only: {\"claims\":[{\"claim_key\":string,\"text\":string,\"confidence\":\"high|medium|low\",\"evidence_ids\":[string]}]}. Add only useful inferred rationale with supplied evidence IDs. Never invent facts or claim conflict resolution."},
                {"role": "user", "content": report_context([lineage])},
            ], "lineage inference")
        finally:
            await provider.aclose()
        if context.source_hash != lineage_source_hash(self.workflow, feature_id):
            raise StaleJobError("Lineage source changed during inference")
        valid = validate_inference_payload(result, set(lineage["evidence"]))
        if valid:
            return self.workflow.add_lineage_inferences(feature_id, str(lineage["snapshot_id"]), valid)
        return self.workflow.mark_lineage_inference_complete(feature_id, str(lineage["snapshot_id"]))


def lineage_source_hash(workflow: WorkflowEngine, feature_id: str) -> str:
    row = workflow.db.execute(
        """SELECT f.problem_id,p.state AS problem_state,c.state AS completion_state
           FROM features f JOIN problems p ON p.id=f.problem_id
           LEFT JOIN completions c ON c.feature_id=f.id WHERE f.id=?""",
        (feature_id,),
    ).fetchone()
    if not row:
        raise ValueError("Solution not found")
    return digest(json.dumps(dict(row), sort_keys=True, ensure_ascii=False))
