from __future__ import annotations

import json

from llm_wiki.core.jobs import Job, TaskDescriptor
from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.completion_archive import CompletionArchivePublisher
from llm_wiki.services.patches import digest
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.settings import ProviderSettings


class JobSubmissionService:
    """Create durable jobs without leaking HTTP response concerns into use cases."""

    def __init__(
        self,
        repository: JobRepository,
        settings: ProviderSettings,
        retrieval: RetrievalEngine,
        completion_archive: CompletionArchivePublisher,
    ):
        self.repository = repository
        self.settings = settings
        self.retrieval = retrieval
        self.completion_archive = completion_archive

    async def enqueue(
        self,
        descriptor: TaskDescriptor,
        payload: dict[str, object],
        *,
        idempotency_key: str = "",
        source_hash: str = "",
        model_task: str | None = None,
    ) -> Job:
        return await self.repository.create(
            descriptor,
            {**payload, "entity_type": descriptor.entity_type, "entity_id": descriptor.entity_id},
            idempotency_key=idempotency_key,
            source_hash=source_hash,
            model=self.settings.model_for(model_task or descriptor.task_kind),
        )

    async def enqueue_derived_translation(
        self,
        entity_type: str,
        entity_id: str,
        field: str,
        source: str,
        source_locale: str,
    ) -> None:
        if not source.strip():
            return
        source_hash = digest(source)
        await self.repository.create(
            TaskDescriptor("derived_translation", entity_type, entity_id, "owning_content"),
            {
                "entity_type": entity_type,
                "entity_id": entity_id,
                "field": field,
                "source": source,
                "source_locale": source_locale,
            },
            idempotency_key=f"derived-translation:{entity_type}:{entity_id}:{field}:{source_hash}",
            source_hash=source_hash,
            model=self.settings.model_for("knowledge_translation"),
        )

    async def enqueue_embeddings(self) -> None:
        manifest = self.retrieval.manifest_hash()
        await self.repository.create(
            TaskDescriptor("embedding_refresh", "vault", "documents", "embedding_coverage"),
            {"entity_type": "vault", "entity_id": "documents", "manifest_hash": manifest},
            idempotency_key=f"embedding-refresh:{manifest}",
            source_hash=manifest,
            model="local-semantic-embedder",
        )

    async def enqueue_completion_report(self, problem_id: str, *, refresh_lineage: bool) -> str:
        lineages = self.completion_archive.lineages(problem_id, refresh=refresh_lineage)
        source_hash = digest(json.dumps(lineages, sort_keys=True, ensure_ascii=False))
        job = await self.repository.create(
            TaskDescriptor("completion_report", "problems", problem_id, "completed_knowledge"),
            {"entity_type": "problems", "entity_id": problem_id, "refresh_lineage": False},
            idempotency_key=f"completion-report:{problem_id}:{source_hash}",
            source_hash=source_hash,
            model=self.settings.model_for("completion_report"),
        )
        return job.id
