from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.completion_archive import CompletionArchivePublisher
from llm_wiki.services.fast_queue import FastQueueClient
from llm_wiki.services.job_submission import JobSubmissionService
from llm_wiki.services.localization import (
    KnowledgeTranslationCache,
    LocaleSettings,
    VaultKnowledgeTranslationCache,
)
from llm_wiki.services.retrieval import RetrievalEngine
from llm_wiki.services.settings import ProviderSettings
from llm_wiki.services.vault import MarkdownVaultAdapter
from llm_wiki.services.workflow import WorkflowEngine


@dataclass(frozen=True)
class ApplicationRuntime:
    """Application services assembled once at the web composition boundary."""

    vault: MarkdownVaultAdapter
    retrieval: RetrievalEngine
    workflow: WorkflowEngine
    provider_settings: ProviderSettings
    locale_settings: LocaleSettings
    knowledge_cache: VaultKnowledgeTranslationCache
    completion_archive: CompletionArchivePublisher
    job_repository: JobRepository
    fast_queue: FastQueueClient
    job_submission: JobSubmissionService


def build_runtime(
    vault_path: Path,
    db_path: Path,
    *,
    fast_queue_client: FastQueueClient | None = None,
) -> ApplicationRuntime:
    vault = MarkdownVaultAdapter(vault_path)
    retrieval = RetrievalEngine(db_path, vault)
    workflow = WorkflowEngine(retrieval.db)
    provider_settings = ProviderSettings(retrieval.db)
    locale_settings = LocaleSettings(retrieval.db)
    legacy_knowledge_cache = KnowledgeTranslationCache(retrieval.db)
    knowledge_cache = VaultKnowledgeTranslationCache(vault, legacy_knowledge_cache)
    completion_archive = CompletionArchivePublisher(workflow, retrieval, vault, knowledge_cache)
    job_repository = JobRepository(db_path)
    fast_queue = fast_queue_client or FastQueueClient()
    job_submission = JobSubmissionService(job_repository, provider_settings, retrieval, completion_archive)
    return ApplicationRuntime(
        vault=vault,
        retrieval=retrieval,
        workflow=workflow,
        provider_settings=provider_settings,
        locale_settings=locale_settings,
        knowledge_cache=knowledge_cache,
        completion_archive=completion_archive,
        job_repository=job_repository,
        fast_queue=fast_queue,
        job_submission=job_submission,
    )
