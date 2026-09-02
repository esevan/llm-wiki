from __future__ import annotations

import json
from typing import Any

from llm_wiki.adapters.provider import AsyncOpenAICompatibleProvider
from llm_wiki.core.jobs import StaleJobError, TaskDescriptor
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry
from llm_wiki.services.localization import response_language_instruction
from llm_wiki.services.patches import digest
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.workflow import WorkflowEngine


def organization_items(workflow: WorkflowEngine, locale: str) -> list[dict[str, object]]:
    board = workflow.board(locale)
    return [
        {
            "entity_type": entity_type,
            "entity_id": item["id"],
            "title": item.get("statement") or item.get("title") or item.get("outcome") or item.get("text"),
            "state": item.get("state", "inbox"),
            "current_category": item.get("category", ""),
        }
        for entity_type in ("captures", "problems", "features")
        for item in board[entity_type]
    ]


class WorkbenchOrganizationHandler:
    def __init__(self, workflow: WorkflowEngine, settings: ProviderSettings):
        self.workflow = workflow
        self.settings = settings

    def register(self, registry: HandlerRegistry) -> None:
        registry.register(TaskDescriptor("workbench_organization", result_interface="workbench"), self.__call__)

    async def __call__(self, context: HandlerContext) -> dict[str, Any]:
        locale = str(context.payload.get("locale", "en"))
        items = organization_items(self.workflow, locale)
        if digest(json.dumps(items, sort_keys=True, ensure_ascii=False)) != context.source_hash:
            raise StaleJobError("Workbench changed before organization")
        base_url, api_key, model = self.settings.credentials("workbench_organization")
        provider = AsyncOpenAICompatibleProvider.with_client(base_url, api_key, model)
        try:
            response = await provider.complete_json(
                [
                    {
                        "role": "system",
                        "content": 'Return JSON only: {"entries":[{"entity_type":string,"entity_id":string,"category":string,"attention_rank":integer,"rationale":string}]}. Organize current work by attention without changing workflow states.',
                    },
                    {"role": "system", "content": response_language_instruction(locale)},
                    {"role": "user", "content": json.dumps({"items": items}, ensure_ascii=False)},
                ],
                "workbench organization",
            )
        finally:
            await provider.aclose()
        if (
            digest(json.dumps(organization_items(self.workflow, locale), sort_keys=True, ensure_ascii=False))
            != context.source_hash
        ):
            raise StaleJobError("Workbench changed during organization")
        return {"organized": self.workflow.apply_ai_organization(response.get("entries"))}
