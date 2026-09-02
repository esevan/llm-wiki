from __future__ import annotations

import json
import sqlite3
import uuid
from contextlib import asynccontextmanager
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

import aiosqlite

from llm_wiki.core.jobs import (
    ACTIVE_JOB_STATUSES,
    Job,
    JobCheckpoint,
    JobLease,
    JobNotification,
    JobStatus,
    TaskDescriptor,
)


def _now() -> datetime:
    return datetime.now(timezone.utc)


def _iso(value: datetime | None = None) -> str:
    return (value or _now()).isoformat()


def _datetime(value: object) -> datetime | None:
    return datetime.fromisoformat(str(value)) if value else None


class JobRepository:
    """Short-lived async SQLite operations safe for independent worker processes."""

    def __init__(self, db_path: Path):
        self.db_path = Path(db_path)

    @asynccontextmanager
    async def _connect(self):
        connection = await aiosqlite.connect(self.db_path, timeout=5)
        connection.row_factory = aiosqlite.Row
        await connection.execute("PRAGMA journal_mode=WAL")
        await connection.execute("PRAGMA busy_timeout=5000")
        try:
            yield connection
        finally:
            await connection.close()

    async def initialize(self) -> None:
        async with self._connect() as db:
            await db.executescript(
                """
                CREATE TABLE IF NOT EXISTS ai_jobs_v2 (
                  id TEXT PRIMARY KEY,
                  task_kind TEXT NOT NULL,
                  entity_type TEXT NOT NULL DEFAULT '',
                  entity_id TEXT NOT NULL DEFAULT '',
                  status TEXT NOT NULL,
                  input_json TEXT NOT NULL DEFAULT '{}',
                  result_json TEXT NOT NULL DEFAULT '{}',
                  source_hash TEXT NOT NULL DEFAULT '',
                  model TEXT NOT NULL DEFAULT '',
                  execution_mode TEXT NOT NULL,
                  idempotency_key TEXT NOT NULL DEFAULT '',
                  result_interface TEXT NOT NULL DEFAULT 'none',
                  notification_policy TEXT NOT NULL DEFAULT 'none',
                  progress_completed INTEGER NOT NULL DEFAULT 0,
                  progress_total INTEGER NOT NULL DEFAULT 0,
                  attempt INTEGER NOT NULL DEFAULT 0,
                  worker_id TEXT NOT NULL DEFAULT '',
                  lease_token TEXT NOT NULL DEFAULT '',
                  lease_expires_at TEXT,
                  heartbeat_at TEXT,
                  available_at TEXT NOT NULL,
                  error_code TEXT NOT NULL DEFAULT '',
                  error_message TEXT NOT NULL DEFAULT '',
                  created_at TEXT NOT NULL,
                  started_at TEXT,
                  finished_at TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_ai_jobs_v2_claim
                  ON ai_jobs_v2(status, available_at, created_at);
                CREATE INDEX IF NOT EXISTS idx_ai_jobs_v2_target
                  ON ai_jobs_v2(entity_type, entity_id, created_at);
                CREATE TABLE IF NOT EXISTS ai_job_checkpoints (
                  job_id TEXT NOT NULL,
                  unit_key TEXT NOT NULL,
                  source_hash TEXT NOT NULL,
                  model TEXT NOT NULL,
                  ordinal INTEGER NOT NULL,
                  result_json TEXT NOT NULL,
                  completed_at TEXT NOT NULL,
                  PRIMARY KEY(job_id, unit_key)
                );
                CREATE TABLE IF NOT EXISTS ai_job_publications (
                  job_id TEXT NOT NULL,
                  publication_kind TEXT NOT NULL,
                  target_revision TEXT NOT NULL DEFAULT '',
                  destination TEXT NOT NULL DEFAULT '',
                  published_at TEXT NOT NULL,
                  PRIMARY KEY(job_id, publication_kind)
                );
                CREATE TABLE IF NOT EXISTS notifications (
                  id TEXT PRIMARY KEY,
                  job_id TEXT NOT NULL,
                  kind TEXT NOT NULL,
                  title TEXT NOT NULL,
                  target_json TEXT NOT NULL,
                  read_at TEXT,
                  dismissed_at TEXT,
                  created_at TEXT NOT NULL,
                  UNIQUE(job_id, kind)
                );
                """
            )
            await db.commit()

    async def create(
        self,
        descriptor: TaskDescriptor,
        payload: dict[str, Any],
        *,
        execution_mode: str = "asynchronous",
        idempotency_key: str = "",
        source_hash: str = "",
        model: str = "",
    ) -> Job:
        await self.initialize()
        active = tuple(status.value for status in ACTIVE_JOB_STATUSES)
        async with self._connect() as db:
            if idempotency_key:
                placeholders = ",".join("?" for _ in active)
                row = await (
                    await db.execute(
                        f"SELECT * FROM ai_jobs_v2 WHERE idempotency_key=? AND status IN ({placeholders}) ORDER BY created_at DESC LIMIT 1",
                        (idempotency_key, *active),
                    )
                ).fetchone()
                if row:
                    return self._job(row)
            job_id = str(uuid.uuid4())
            created = _iso()
            await db.execute(
                """INSERT INTO ai_jobs_v2(
                     id,task_kind,entity_type,entity_id,status,input_json,source_hash,model,
                     execution_mode,idempotency_key,result_interface,notification_policy,available_at,created_at
                   ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
                (
                    job_id,
                    descriptor.task_kind,
                    descriptor.entity_type,
                    descriptor.entity_id,
                    JobStatus.QUEUED.value,
                    json.dumps(payload, ensure_ascii=False),
                    source_hash,
                    model,
                    execution_mode,
                    idempotency_key,
                    descriptor.result_interface,
                    descriptor.notification_policy,
                    created,
                    created,
                ),
            )
            await db.commit()
        value = await self.get(job_id)
        assert value is not None
        return value

    async def get(self, job_id: str) -> Job | None:
        async with self._connect() as db:
            row = await (await db.execute("SELECT * FROM ai_jobs_v2 WHERE id=?", (job_id,))).fetchone()
        return self._job(row) if row else None

    async def list(self, *, limit: int = 100) -> list[Job]:
        async with self._connect() as db:
            rows = await (
                await db.execute("SELECT * FROM ai_jobs_v2 ORDER BY created_at DESC LIMIT ?", (limit,))
            ).fetchall()
        return [self._job(row) for row in rows]

    async def claim(self, job_id: str, worker_id: str, *, lease_seconds: int) -> JobLease | None:
        async with self._connect() as db:
            await db.execute("BEGIN IMMEDIATE")
            row = await (await db.execute("SELECT status,attempt FROM ai_jobs_v2 WHERE id=?", (job_id,))).fetchone()
            if not row or row["status"] not in {JobStatus.QUEUED.value, JobStatus.RETRYABLE.value}:
                await db.rollback()
                return None
            token = str(uuid.uuid4())
            now, expiry = _now(), _now() + timedelta(seconds=lease_seconds)
            attempt = int(row["attempt"]) + 1
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,attempt=?,worker_id=?,lease_token=?,lease_expires_at=?,heartbeat_at=?,started_at=COALESCE(started_at,?)
                   WHERE id=? AND status IN (?,?)""",
                (
                    JobStatus.RUNNING.value,
                    attempt,
                    worker_id,
                    token,
                    _iso(expiry),
                    _iso(now),
                    _iso(now),
                    job_id,
                    JobStatus.QUEUED.value,
                    JobStatus.RETRYABLE.value,
                ),
            )
            if changed.rowcount != 1:
                await db.rollback()
                return None
            await db.commit()
            return JobLease(job_id, worker_id, token, attempt, expiry)

    async def claim_next(self, worker_id: str, *, lease_seconds: int) -> JobLease | None:
        await self.recover_expired()
        async with self._connect() as db:
            await db.execute("BEGIN IMMEDIATE")
            row = await (
                await db.execute(
                    "SELECT id,attempt FROM ai_jobs_v2 WHERE status IN (?,?) AND available_at<=? ORDER BY created_at LIMIT 1",
                    (JobStatus.QUEUED.value, JobStatus.RETRYABLE.value, _iso()),
                )
            ).fetchone()
            if not row:
                await db.rollback()
                return None
            token = str(uuid.uuid4())
            now, expiry = _now(), _now() + timedelta(seconds=lease_seconds)
            attempt = int(row["attempt"]) + 1
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,attempt=?,worker_id=?,lease_token=?,lease_expires_at=?,heartbeat_at=?,started_at=COALESCE(started_at,?)
                   WHERE id=? AND status IN (?,?)""",
                (
                    JobStatus.RUNNING.value,
                    attempt,
                    worker_id,
                    token,
                    _iso(expiry),
                    _iso(now),
                    _iso(now),
                    row["id"],
                    JobStatus.QUEUED.value,
                    JobStatus.RETRYABLE.value,
                ),
            )
            if changed.rowcount != 1:
                await db.rollback()
                return None
            await db.commit()
            return JobLease(str(row["id"]), worker_id, token, attempt, expiry)

    async def heartbeat(self, lease: JobLease, *, lease_seconds: int) -> bool:
        expiry = _now() + timedelta(seconds=lease_seconds)
        async with self._connect() as db:
            changed = await db.execute(
                "UPDATE ai_jobs_v2 SET heartbeat_at=?,lease_expires_at=? WHERE id=? AND status=? AND lease_token=?",
                (_iso(), _iso(expiry), lease.job_id, JobStatus.RUNNING.value, lease.lease_token),
            )
            await db.commit()
            return changed.rowcount == 1

    async def complete(
        self,
        job_id: str,
        lease_token: str,
        result: dict[str, Any],
        *,
        awaiting_review: bool = False,
        notification_kind: str = "",
        notification_title: str = "",
        notification_target: dict[str, Any] | None = None,
        clear_checkpoints: bool = False,
        publication_kind: str = "",
        publication_destination: str = "",
        publication_revision: str = "",
    ) -> None:
        status = JobStatus.AWAITING_REVIEW if awaiting_review else JobStatus.COMPLETED
        await self._terminal(
            job_id,
            lease_token,
            status,
            result=result,
            notification_kind=notification_kind,
            notification_title=notification_title,
            notification_target=notification_target,
            clear_checkpoints=clear_checkpoints,
            publication_kind=publication_kind,
            publication_destination=publication_destination,
            publication_revision=publication_revision,
        )

    async def fail(
        self, job_id: str, lease_token: str, code: str, message: str, *, retryable: bool = False, max_attempts: int = 3
    ) -> None:
        job = await self.get(job_id)
        may_retry = bool(retryable and job and job.attempt < max_attempts)
        if not may_retry:
            await self._terminal(job_id, lease_token, JobStatus.FAILED, error_code=code, error_message=message)
            return
        delay = min(60, 2 ** max(0, job.attempt - 1))
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,error_code=?,error_message=?,available_at=?,worker_id='',lease_token='',lease_expires_at=NULL
                   WHERE id=? AND status=? AND lease_token=?""",
                (
                    JobStatus.RETRYABLE.value,
                    code,
                    message[:2000],
                    _iso(_now() + timedelta(seconds=delay)),
                    job_id,
                    JobStatus.RUNNING.value,
                    lease_token,
                ),
            )
            if changed.rowcount != 1:
                await db.rollback()
                raise PermissionError("Job lease is no longer current")
            await db.commit()

    async def stale(self, job_id: str, lease_token: str, message: str) -> None:
        await self._terminal(job_id, lease_token, JobStatus.STALE, error_code="stale", error_message=message)

    async def _terminal(
        self,
        job_id: str,
        lease_token: str,
        status: JobStatus,
        *,
        result: dict[str, Any] | None = None,
        error_code: str = "",
        error_message: str = "",
        notification_kind: str = "",
        notification_title: str = "",
        notification_target: dict[str, Any] | None = None,
        clear_checkpoints: bool = False,
        publication_kind: str = "",
        publication_destination: str = "",
        publication_revision: str = "",
    ) -> None:
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,result_json=?,error_code=?,error_message=?,finished_at=?,worker_id='',lease_token='',lease_expires_at=NULL
                   WHERE id=? AND status=? AND lease_token=?""",
                (
                    status.value,
                    json.dumps(result or {}, ensure_ascii=False),
                    error_code,
                    error_message[:2000],
                    _iso(),
                    job_id,
                    JobStatus.RUNNING.value,
                    lease_token,
                ),
            )
            if changed.rowcount != 1:
                await db.rollback()
                raise PermissionError("Job lease is no longer current")
            if notification_kind:
                await db.execute(
                    """INSERT OR IGNORE INTO notifications(id,job_id,kind,title,target_json,created_at)
                       VALUES (?,?,?,?,?,?)""",
                    (
                        str(uuid.uuid4()),
                        job_id,
                        notification_kind,
                        notification_title,
                        json.dumps(notification_target or {}, ensure_ascii=False),
                        _iso(),
                    ),
                )
            if clear_checkpoints:
                await db.execute("DELETE FROM ai_job_checkpoints WHERE job_id=?", (job_id,))
            if publication_kind:
                await db.execute(
                    """INSERT OR IGNORE INTO ai_job_publications(
                         job_id,publication_kind,target_revision,destination,published_at
                       ) VALUES (?,?,?,?,?)""",
                    (job_id, publication_kind, publication_revision, publication_destination, _iso()),
                )
            await db.commit()

    async def recover_expired(self) -> int:
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,worker_id='',lease_token='',lease_expires_at=NULL,available_at=?
                   WHERE status=? AND lease_expires_at IS NOT NULL AND lease_expires_at<?""",
                (JobStatus.RETRYABLE.value, _iso(), JobStatus.RUNNING.value, _iso()),
            )
            await db.commit()
            return changed.rowcount

    async def request_cancel(self, job_id: str) -> Job | None:
        terminal: Job | None = None
        async with self._connect() as db:
            row = await (await db.execute("SELECT status FROM ai_jobs_v2 WHERE id=?", (job_id,))).fetchone()
            if not row:
                return None
            current = JobStatus(row["status"])
            if current is JobStatus.QUEUED or current is JobStatus.RETRYABLE:
                status, finished = JobStatus.CANCELLED, _iso()
            elif current is JobStatus.RUNNING:
                status, finished = JobStatus.CANCELLING, None
            else:
                terminal = self._job(
                    await (await db.execute("SELECT * FROM ai_jobs_v2 WHERE id=?", (job_id,))).fetchone()
                )
                status = current
                finished = None
            if terminal is not None:
                return terminal
            await db.execute(
                "UPDATE ai_jobs_v2 SET status=?,finished_at=? WHERE id=?", (status.value, finished, job_id)
            )
            await db.commit()
        return await self.get(job_id)

    async def cancellation_requested(self, job_id: str) -> bool:
        async with self._connect() as db:
            row = await (await db.execute("SELECT status FROM ai_jobs_v2 WHERE id=?", (job_id,))).fetchone()
        return bool(row and row["status"] in {JobStatus.CANCELLING.value, JobStatus.CANCELLED.value})

    async def finish_cancel(self, job_id: str, lease_token: str) -> None:
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,finished_at=?,worker_id='',lease_token='',lease_expires_at=NULL
                   WHERE id=? AND status=? AND lease_token=?""",
                (JobStatus.CANCELLED.value, _iso(), job_id, JobStatus.CANCELLING.value, lease_token),
            )
            if changed.rowcount != 1:
                await db.rollback()
                raise PermissionError("Job lease is no longer current")
            await db.commit()

    async def retry(self, job_id: str) -> Job | None:
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET status=?,available_at=?,finished_at=NULL,error_code='',error_message=''
                   WHERE id=? AND status IN (?,?)""",
                (JobStatus.QUEUED.value, _iso(), job_id, JobStatus.FAILED.value, JobStatus.RETRYABLE.value),
            )
            await db.commit()
        return await self.get(job_id) if changed.rowcount else None

    async def update_progress(self, job_id: str, lease_token: str, completed: int, total: int) -> None:
        if completed < 0 or total < 0 or completed > total:
            raise ValueError("Invalid job progress")
        async with self._connect() as db:
            changed = await db.execute(
                """UPDATE ai_jobs_v2 SET progress_completed=?,progress_total=?
                   WHERE id=? AND status=? AND lease_token=?""",
                (completed, total, job_id, JobStatus.RUNNING.value, lease_token),
            )
            if changed.rowcount != 1:
                raise PermissionError("Job lease is no longer current")
            await db.commit()

    async def expire_lease_for_test(self, job_id: str, value: datetime) -> None:
        async with self._connect() as db:
            await db.execute("UPDATE ai_jobs_v2 SET lease_expires_at=? WHERE id=?", (_iso(value), job_id))
            await db.commit()

    async def save_checkpoint(
        self,
        job_id: str,
        lease_token: str,
        unit_key: str,
        source_hash: str,
        model: str,
        ordinal: int,
        result: dict[str, Any],
    ) -> None:
        async with self._connect() as db:
            owner = await (
                await db.execute(
                    "SELECT 1 FROM ai_jobs_v2 WHERE id=? AND status=? AND lease_token=?",
                    (job_id, JobStatus.RUNNING.value, lease_token),
                )
            ).fetchone()
            if not owner:
                raise PermissionError("Job lease is no longer current")
            await db.execute(
                """INSERT INTO ai_job_checkpoints(job_id,unit_key,source_hash,model,ordinal,result_json,completed_at)
                   VALUES (?,?,?,?,?,?,?) ON CONFLICT(job_id,unit_key) DO UPDATE SET
                   source_hash=excluded.source_hash,model=excluded.model,ordinal=excluded.ordinal,
                   result_json=excluded.result_json,completed_at=excluded.completed_at""",
                (job_id, unit_key, source_hash, model, ordinal, json.dumps(result, ensure_ascii=False), _iso()),
            )
            await db.commit()

    async def checkpoints(self, job_id: str, *, source_hash: str, model: str) -> list[JobCheckpoint]:
        async with self._connect() as db:
            rows = await (
                await db.execute(
                    "SELECT * FROM ai_job_checkpoints WHERE job_id=? AND source_hash=? AND model=? ORDER BY ordinal",
                    (job_id, source_hash, model),
                )
            ).fetchall()
        return [
            JobCheckpoint(
                row["job_id"],
                row["unit_key"],
                row["source_hash"],
                row["model"],
                row["ordinal"],
                json.loads(row["result_json"]),
            )
            for row in rows
        ]

    async def publications(self, job_id: str) -> list[dict[str, str]]:
        async with self._connect() as db:
            rows = await (
                await db.execute(
                    "SELECT publication_kind,target_revision,destination,published_at FROM ai_job_publications WHERE job_id=?",
                    (job_id,),
                )
            ).fetchall()
        return [dict(row) for row in rows]

    async def publish_notification(self, job_id: str, kind: str, title: str, target: dict[str, Any]) -> JobNotification:
        async with self._connect() as db:
            notification_id = str(uuid.uuid4())
            await db.execute(
                "INSERT OR IGNORE INTO notifications(id,job_id,kind,title,target_json,created_at) VALUES (?,?,?,?,?,?)",
                (notification_id, job_id, kind, title, json.dumps(target, ensure_ascii=False), _iso()),
            )
            await db.commit()
            row = await (
                await db.execute("SELECT * FROM notifications WHERE job_id=? AND kind=?", (job_id, kind))
            ).fetchone()
        assert row
        return self._notification(row)

    async def notifications(self, *, unread_only: bool = False) -> list[JobNotification]:
        condition = "WHERE read_at IS NULL AND dismissed_at IS NULL" if unread_only else ""
        async with self._connect() as db:
            rows = await (
                await db.execute(f"SELECT * FROM notifications {condition} ORDER BY created_at DESC")
            ).fetchall()
        return [self._notification(row) for row in rows]

    async def update_notification(self, notification_id: str, *, dismiss: bool = False) -> JobNotification | None:
        column = "dismissed_at" if dismiss else "read_at"
        async with self._connect() as db:
            await db.execute(f"UPDATE notifications SET {column}=? WHERE id=?", (_iso(), notification_id))
            await db.commit()
            row = await (await db.execute("SELECT * FROM notifications WHERE id=?", (notification_id,))).fetchone()
        return self._notification(row) if row else None

    @staticmethod
    def _job(row: aiosqlite.Row | sqlite3.Row) -> Job:
        descriptor = TaskDescriptor(
            row["task_kind"], row["entity_type"], row["entity_id"], row["result_interface"], row["notification_policy"]
        )
        return Job(
            id=row["id"],
            descriptor=descriptor,
            status=JobStatus(row["status"]),
            input=json.loads(row["input_json"]),
            result=json.loads(row["result_json"]),
            source_hash=row["source_hash"],
            model=row["model"],
            execution_mode=row["execution_mode"],
            idempotency_key=row["idempotency_key"],
            progress_completed=row["progress_completed"],
            progress_total=row["progress_total"],
            attempt=row["attempt"],
            error_code=row["error_code"],
            error_message=row["error_message"],
            created_at=_datetime(row["created_at"]),
            started_at=_datetime(row["started_at"]),
            finished_at=_datetime(row["finished_at"]),
        )

    @staticmethod
    def _notification(row: aiosqlite.Row | sqlite3.Row) -> JobNotification:
        return JobNotification(
            row["id"],
            row["job_id"],
            row["kind"],
            row["title"],
            json.loads(row["target_json"]),
            _datetime(row["read_at"]),
            _datetime(row["dismissed_at"]),
        )
