from __future__ import annotations

import asyncio
import sqlite3
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from llm_wiki.core.jobs import JobStatus, TaskDescriptor
from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.handlers.conflict_review import ConflictReviewJobHandler
from llm_wiki.services.handlers.embeddings import EmbeddingJobHandler
from llm_wiki.services.handlers.registry import HandlerRegistry
from llm_wiki.services.handlers.worker import AsyncJobWorker
from llm_wiki.services.semantic import SemanticUnavailable
from llm_wiki.services.workflow import WorkflowEngine


def run(coroutine):
    return asyncio.run(coroutine)


def test_job_repository_records_terminal_synchronous_work(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "jobs.sqlite")
    run(repository.initialize())
    job = run(
        repository.create(
            TaskDescriptor("workflow_draft", "captures", "capture-1"), {"locale": "ko"}, execution_mode="synchronous"
        )
    )
    lease = run(repository.claim(job.id, "request-1", lease_seconds=30))
    assert lease is not None
    run(repository.complete(job.id, lease.lease_token, {"title": "Draft"}))

    stored = run(repository.get(job.id))
    assert stored is not None
    assert stored.status is JobStatus.COMPLETED
    assert stored.execution_mode == "synchronous"
    assert stored.result == {"title": "Draft"}


def test_only_live_lease_can_publish_and_expired_work_recovers(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "leases.sqlite")
    run(repository.initialize())
    job = run(repository.create(TaskDescriptor("completion_review", "features", "feature-1"), {}))
    first = run(repository.claim(job.id, "worker-1", lease_seconds=1))
    assert first is not None
    run(repository.expire_lease_for_test(job.id, datetime.now(timezone.utc) - timedelta(seconds=1)))
    assert run(repository.recover_expired()) == 1
    second = run(repository.claim(job.id, "worker-2", lease_seconds=30))
    assert second is not None and second.lease_token != first.lease_token

    with pytest.raises(PermissionError):
        run(repository.complete(job.id, first.lease_token, {"review": "late"}))
    run(repository.complete(job.id, second.lease_token, {"review": "current"}))
    assert run(repository.get(job.id)).result == {"review": "current"}  # type: ignore[union-attr]


def test_idempotency_reuses_equivalent_active_job(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "idempotency.sqlite")
    run(repository.initialize())
    descriptor = TaskDescriptor("image_summary", "progress", "entry-1")
    first = run(repository.create(descriptor, {}, idempotency_key="summary:entry-1:hash"))
    second = run(repository.create(descriptor, {}, idempotency_key="summary:entry-1:hash"))
    assert second.id == first.id


def test_checkpoint_is_reused_only_for_same_source_and_model(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "checkpoints.sqlite")
    run(repository.initialize())
    job = run(repository.create(TaskDescriptor("knowledge_translation", "knowledge", "note.md"), {}))
    lease = run(repository.claim(job.id, "worker", lease_seconds=30))
    assert lease is not None
    run(
        repository.save_checkpoint(
            job.id, lease.lease_token, "paragraph-1", "hash-a", "model-a", 0, {"markdown": "번역"}
        )
    )
    assert run(repository.checkpoints(job.id, source_hash="hash-a", model="model-a"))[0].result["markdown"] == "번역"
    assert run(repository.checkpoints(job.id, source_hash="hash-b", model="model-a")) == []


def test_notification_publication_is_idempotent(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "notifications.sqlite")
    run(repository.initialize())
    job = run(repository.create(TaskDescriptor("completion_review", "features", "feature-1"), {}))
    first = run(repository.publish_notification(job.id, "review_ready", "Review ready", {"feature_id": "feature-1"}))
    second = run(repository.publish_notification(job.id, "review_ready", "Review ready", {"feature_id": "feature-1"}))
    assert first.id == second.id
    assert len(run(repository.notifications(unread_only=True))) == 1


def test_review_completion_and_notification_commit_atomically(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "atomic-notification.sqlite")
    run(repository.initialize())
    descriptor = TaskDescriptor("completion_review", "features", "feature-1", notification_policy="review_ready")
    job = run(repository.create(descriptor, {}))
    lease = run(repository.claim(job.id, "worker", lease_seconds=30))
    assert lease is not None
    run(
        repository.complete(
            job.id,
            lease.lease_token,
            {"report": "ready"},
            awaiting_review=True,
            notification_kind="review_ready",
            notification_title="Completion review is ready",
            notification_target={"feature_id": "feature-1"},
        )
    )
    assert run(repository.get(job.id)).status is JobStatus.AWAITING_REVIEW  # type: ignore[union-attr]
    notices = run(repository.notifications(unread_only=True))
    assert len(notices) == 1 and notices[0].job_id == job.id


def test_automatic_result_and_publication_record_commit_together(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "atomic-publication.sqlite")
    run(repository.initialize())
    job = run(repository.create(TaskDescriptor("image_summary", "progress", "entry-1"), {}, source_hash="source-a"))
    lease = run(repository.claim(job.id, "worker", lease_seconds=30))
    assert lease is not None
    run(
        repository.complete(
            job.id,
            lease.lease_token,
            {"summary": "done"},
            publication_kind="solution_work_summary",
            publication_destination="progress:entry-1",
            publication_revision="source-a",
        )
    )
    publications = run(repository.publications(job.id))
    assert publications == [
        {
            "publication_kind": "solution_work_summary",
            "target_revision": "source-a",
            "destination": "progress:entry-1",
            "published_at": publications[0]["published_at"],
        }
    ]


def test_concurrent_workers_cannot_claim_the_same_job(tmp_path: Path) -> None:
    async def scenario() -> None:
        repository = JobRepository(tmp_path / "atomic-claim.sqlite")
        await repository.initialize()
        await repository.create(TaskDescriptor("embedding_refresh"), {})
        claims = await asyncio.gather(
            repository.claim_next("worker-a", lease_seconds=30),
            repository.claim_next("worker-b", lease_seconds=30),
        )
        assert sum(claim is not None for claim in claims) == 1

    asyncio.run(scenario())


def test_database_lock_contention_uses_bounded_job_retry(tmp_path: Path) -> None:
    repository = JobRepository(tmp_path / "locked-retry.sqlite")
    run(repository.initialize())
    job = run(repository.create(TaskDescriptor("derived_translation"), {}))
    lease = run(repository.claim(job.id, "worker", lease_seconds=30))
    assert lease is not None
    worker = AsyncJobWorker(repository, HandlerRegistry(), worker_id="worker")

    run(worker._handle_error(job, lease, sqlite3.OperationalError("database is locked")))

    stored = run(repository.get(job.id))
    assert stored is not None
    assert stored.status is JobStatus.RETRYABLE
    assert stored.error_code == "database_locked"


def test_embedding_job_completes_with_lexical_fallback_when_semantic_runtime_is_optional() -> None:
    class Retrieval:
        def __init__(self) -> None:
            self.db = sqlite3.connect(":memory:")
            self.db.row_factory = sqlite3.Row
            self.db.execute("CREATE TABLE documents(path TEXT, source_hash TEXT, title TEXT, headings TEXT, body TEXT)")
            self.db.execute(
                "CREATE TABLE document_embeddings(path TEXT, source_hash TEXT, dimensions INTEGER, vector BLOB)"
            )
            self.db.execute("INSERT INTO documents VALUES ('note.md','hash','Title','','Body')")

        @staticmethod
        def embed_texts(_texts: list[str]) -> list[list[float]]:
            raise SemanticUnavailable("optional runtime is not installed")

        @staticmethod
        def status() -> dict[str, int]:
            return {"documents": 1, "semantic_ready": 0}

    class Context:
        model = "local-semantic-embedder"
        source_hash = "manifest"

        @staticmethod
        async def checkpoints(_source_hash: str, _model: str):
            return []

        @staticmethod
        async def cancelled() -> bool:
            return False

    result = run(EmbeddingJobHandler(Retrieval())(Context()))  # type: ignore[arg-type]

    assert result == {
        "updated": 0,
        "coverage": {"documents": 1, "semantic_ready": 0},
        "semantic_available": False,
    }


def test_conflict_finding_is_normalized_into_the_structured_card_contract() -> None:
    evidence = {
        "id": "evidence-1",
        "claim": "Client state is authoritative.",
        "path": "decisions/adr-008.md",
        "source_hash": "source-hash",
        "start_line": 12,
        "end_line": 18,
        "text": "Server state remains authoritative.",
    }
    response = {
        "conflict": True,
        "evidence_id": "evidence-1",
        "severity": "URGENT",
        "category": "Storage ownership",
        "summary": "The sources assign authority differently.",
        "current_claim": "Client state is authoritative.",
        "existing_claim": "Server state remains authoritative.",
        "impact": "Concurrent clients can diverge.",
        "recommendation": "Keep server authority.",
        "explanation": "Authority differs.",
    }

    conflict = ConflictReviewJobHandler._structured_conflict(response, evidence, 1)

    assert conflict == {
        "id": "conflict-1",
        "target_id": "decisions/adr-008.md",
        "target_title": "adr-008",
        "severity": "medium",
        "category": "Storage ownership",
        "summary": "The sources assign authority differently.",
        "current_claim": "Client state is authoritative.",
        "existing_claim": "Server state remains authoritative.",
        "impact": "Concurrent clients can diverge.",
        "recommendation": "Keep server authority.",
        "evidence": [
            {
                "evidence_id": "evidence-1",
                "citation": "decisions/adr-008.md:12-18",
                "excerpt": "Server state remains authoritative.",
                "source_hash": "source-hash",
                "start_line": 12,
                "end_line": 18,
            }
        ],
    }


def test_legacy_findings_only_conflict_report_remains_available_from_cache() -> None:
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row
    workflow = WorkflowEngine(db)
    problem = workflow.promote_capture(workflow.capture("Legacy conflict"))
    workflow.approve_problem(problem["id"])
    feature = workflow.create_feature(
        problem["id"], "Legacy Solution", "Keep history readable", validation_criteria="- [ ] Review history"
    )
    run_id = workflow.start_conflict_review(feature["id"], "legacy-query")
    finding = {"claim": "Old claim", "explanation": "Old explanation", "path": "old.md"}
    workflow.finish_conflict_review(run_id, [], {"run_id": run_id, "feature_id": feature["id"], "findings": [finding]})

    cached = workflow.cached_conflict_review("legacy-query")

    assert cached is not None
    assert cached["findings"] == [finding]
    assert cached.get("conflicts", []) == []
