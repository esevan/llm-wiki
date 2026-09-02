from __future__ import annotations

import asyncio
import logging
import sqlite3
import uuid

import httpx

from llm_wiki.core.jobs import Job, JobLease, RetryableJobError, StaleJobError
from llm_wiki.repositories.jobs import JobRepository
from llm_wiki.services.handlers.registry import HandlerContext, HandlerRegistry

logger = logging.getLogger(__name__)


class AsyncJobWorker:
    def __init__(
        self,
        repository: JobRepository,
        registry: HandlerRegistry,
        *,
        lease_seconds: int = 30,
        worker_id: str | None = None,
    ):
        self.repository = repository
        self.registry = registry
        self.lease_seconds = lease_seconds
        self.worker_id = worker_id or f"worker-{uuid.uuid4()}"

    async def run_once(self) -> bool:
        lease = await self.repository.claim_next(self.worker_id, lease_seconds=self.lease_seconds)
        if lease is None:
            return False
        await self._execute(lease)
        return True

    async def run_job(self, job_id: str) -> bool:
        lease = await self.repository.claim(job_id, self.worker_id, lease_seconds=self.lease_seconds)
        if lease is None:
            return False
        await self._execute(lease)
        return True

    async def _execute(self, lease: JobLease) -> None:
        job = await self.repository.get(lease.job_id)
        if job is None:
            return
        try:
            result = await self._run_handler(job, lease)
            await self._complete(job, lease, result)
        except Exception as error:
            await self._handle_error(job, lease, error)

    def _context(self, job: Job, lease: JobLease) -> HandlerContext:
        return HandlerContext(
            job.id,
            job.input,
            job.source_hash,
            job.model,
            lambda: self.repository.cancellation_requested(job.id),
            lambda completed, total: self.repository.update_progress(job.id, lease.lease_token, completed, total),
            lambda unit_key, source_hash, model, ordinal, result: self.repository.save_checkpoint(
                job.id, lease.lease_token, unit_key, source_hash, model, ordinal, result
            ),
            lambda source_hash, model: self.repository.checkpoints(job.id, source_hash=source_hash, model=model),
        )

    async def _run_handler(self, job: Job, lease: JobLease) -> dict[str, object]:
        handler = self.registry.handler(job.descriptor.task_kind)
        task = asyncio.create_task(handler(self._context(job, lease)))
        while not task.done():
            await asyncio.wait({task}, timeout=max(0.05, self.lease_seconds / 3))
            if task.done():
                break
            if await self.repository.cancellation_requested(job.id):
                await self._cancel_task(task)
                raise InterruptedError("Job execution was cancelled")
            if not task.done() and not await self.repository.heartbeat(lease, lease_seconds=self.lease_seconds):
                await self._cancel_task(task)
                raise PermissionError("Job lease expired")
        return await task

    @staticmethod
    async def _cancel_task(task: asyncio.Task) -> None:
        task.cancel()
        try:
            await task
        except asyncio.CancelledError:
            pass

    async def _complete(self, job: Job, lease: JobLease, result: dict[str, object]) -> None:
        awaiting_review = job.descriptor.notification_policy != "none"
        staged_only = {"workflow_draft", "workflow_refinement", "completion_review"}
        await self.repository.complete(
            job.id,
            lease.lease_token,
            result,
            awaiting_review=awaiting_review,
            notification_kind=job.descriptor.notification_policy if awaiting_review else "",
            notification_title="Completion review is ready" if awaiting_review else "",
            notification_target={
                "job_id": job.id,
                "entity_type": job.descriptor.entity_type,
                "entity_id": job.descriptor.entity_id,
            },
            clear_checkpoints=job.descriptor.task_kind == "knowledge_translation",
            publication_kind="" if job.descriptor.task_kind in staged_only else job.descriptor.result_interface,
            publication_destination=f"{job.descriptor.entity_type}:{job.descriptor.entity_id}",
            publication_revision=job.source_hash,
        )

    async def _handle_error(self, job: Job, lease: JobLease, error: Exception) -> None:
        if isinstance(error, sqlite3.OperationalError) and "locked" in str(error).lower():
            await self.repository.fail(job.id, lease.lease_token, "database_locked", str(error), retryable=True)
            return
        if isinstance(error, (RetryableJobError, httpx.TimeoutException, httpx.NetworkError)):
            await self.repository.fail(job.id, lease.lease_token, "transient", str(error), retryable=True)
            return
        if isinstance(error, httpx.HTTPStatusError):
            status = error.response.status_code
            await self.repository.fail(
                job.id,
                lease.lease_token,
                f"http_{status}",
                f"Provider request failed with HTTP {status}",
                retryable=status == 429 or status >= 500,
            )
            return
        if isinstance(error, InterruptedError):
            await self._handle_interruption(job, lease)
            return
        if isinstance(error, StaleJobError):
            await self.repository.stale(job.id, lease.lease_token, str(error))
            return
        logger.exception("Durable job failed job_id=%s attempt=%s", job.id, lease.attempt, exc_info=error)
        await self.repository.fail(job.id, lease.lease_token, type(error).__name__, str(error))

    async def _handle_interruption(self, job: Job, lease: JobLease) -> None:
        if await self.repository.cancellation_requested(job.id):
            await self.repository.finish_cancel(job.id, lease.lease_token)
            return
        await self.repository.fail(
            job.id,
            lease.lease_token,
            "interrupted",
            "Job execution was interrupted",
            retryable=True,
        )

    async def run(self, stop: asyncio.Event, *, idle_seconds: float = 0.2) -> None:
        while not stop.is_set():
            if not await self.run_once():
                try:
                    await asyncio.wait_for(stop.wait(), timeout=idle_seconds)
                except TimeoutError:
                    pass
