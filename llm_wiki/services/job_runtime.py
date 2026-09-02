from __future__ import annotations

import asyncio
import os
from pathlib import Path

from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.completion_archive import CompletionArchivePublisher
from llm_wiki.services.handlers.catalog import (
    register_translation_handlers,
    register_workflow_handlers,
)
from llm_wiki.services.handlers.completion_report import CompletionReportHandler
from llm_wiki.services.handlers.conflict_review import ConflictReviewJobHandler
from llm_wiki.services.handlers.embeddings import EmbeddingJobHandler
from llm_wiki.services.handlers.lineage import LineageInferenceHandler
from llm_wiki.services.handlers.organization import WorkbenchOrganizationHandler
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker
from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    VaultKnowledgeTranslationCache,
)
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine


def build_job_registry(vault_path: Path, db_path: Path) -> tuple[HandlerRegistry, RetrievalEngine]:
    vault = MarkdownVaultAdapter(vault_path)
    retrieval = RetrievalEngine(db_path, vault)
    registry = HandlerRegistry()
    workflow = WorkflowEngine(retrieval.db)
    settings = ProviderSettings(retrieval.db)
    register_workflow_handlers(registry, workflow, settings)
    register_translation_handlers(registry, workflow, settings, vault)
    EmbeddingJobHandler(retrieval).register(registry)
    ConflictReviewJobHandler(retrieval, workflow, settings).register(registry)
    WorkbenchOrganizationHandler(workflow, settings).register(registry)
    LineageInferenceHandler(workflow, settings).register(registry)
    translations = VaultKnowledgeTranslationCache(vault, KnowledgeTranslationCache(retrieval.db))
    CompletionReportHandler(CompletionArchivePublisher(workflow, retrieval, vault, translations), settings).register(
        registry
    )
    return registry, retrieval


async def run_async_workers(vault_path: Path, db_path: Path, worker_count: int, stop: asyncio.Event) -> None:
    if not 1 <= worker_count <= 32:
        raise ValueError("Async worker count must be between one and 32")
    repository = JobRepository(db_path)
    await repository.initialize()
    registry, retrieval = build_job_registry(vault_path, db_path)
    workers = [
        AsyncJobWorker(repository, registry, worker_id=f"async-{os.getpid()}-{index + 1}")
        for index in range(worker_count)
    ]
    try:
        async with asyncio.TaskGroup() as group:
            for worker in workers:
                group.create_task(worker.run(stop))
    finally:
        retrieval.db.close()
