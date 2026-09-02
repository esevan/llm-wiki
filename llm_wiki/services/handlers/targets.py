from __future__ import annotations

from llm_wiki.core.jobs import StaleJobError
from llm_wiki.services.handlers.registry import HandlerContext


def job_target(context: HandlerContext) -> tuple[str, str]:
    entity_type = str(context.payload.get("entity_type", ""))
    entity_id = str(context.payload.get("entity_id", ""))
    if not entity_type or not entity_id:
        raise ValueError("Job target is required")
    return entity_type, entity_id


def require_source_hash(context: HandlerContext, current: str) -> None:
    if context.source_hash and context.source_hash != current:
        raise StaleJobError("Source changed while this AI job was queued or running")
