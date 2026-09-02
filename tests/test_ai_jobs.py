from __future__ import annotations

import asyncio
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from llm_wiki.core.jobs import JobStatus, TaskDescriptor
from llm_wiki.repositories.jobs import JobRepository


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
