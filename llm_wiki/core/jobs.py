from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from enum import StrEnum
from typing import Any


class JobStatus(StrEnum):
    QUEUED = "queued"
    RUNNING = "running"
    AWAITING_REVIEW = "awaiting_review"
    COMPLETED = "completed"
    CANCELLING = "cancelling"
    CANCELLED = "cancelled"
    FAILED = "failed"
    RETRYABLE = "retryable"
    STALE = "stale"


ACTIVE_JOB_STATUSES = {
    JobStatus.QUEUED,
    JobStatus.RUNNING,
    JobStatus.AWAITING_REVIEW,
    JobStatus.CANCELLING,
    JobStatus.RETRYABLE,
}


@dataclass(frozen=True)
class TaskDescriptor:
    task_kind: str
    entity_type: str = ""
    entity_id: str = ""
    result_interface: str = "none"
    notification_policy: str = "none"


@dataclass(frozen=True)
class Job:
    id: str
    descriptor: TaskDescriptor
    status: JobStatus
    input: dict[str, Any] = field(default_factory=dict)
    result: dict[str, Any] = field(default_factory=dict)
    source_hash: str = ""
    model: str = ""
    execution_mode: str = "asynchronous"
    idempotency_key: str = ""
    progress_completed: int = 0
    progress_total: int = 0
    attempt: int = 0
    error_code: str = ""
    error_message: str = ""
    created_at: datetime | None = None
    started_at: datetime | None = None
    finished_at: datetime | None = None


@dataclass(frozen=True)
class JobLease:
    job_id: str
    worker_id: str
    lease_token: str
    attempt: int
    expires_at: datetime


@dataclass(frozen=True)
class JobCheckpoint:
    job_id: str
    unit_key: str
    source_hash: str
    model: str
    ordinal: int
    result: dict[str, Any]


@dataclass(frozen=True)
class JobNotification:
    id: str
    job_id: str
    kind: str
    title: str
    target: dict[str, Any]
    read_at: datetime | None = None
    dismissed_at: datetime | None = None


class RetryableJobError(RuntimeError):
    """A transient failure that may be attempted again."""


class StaleJobError(RuntimeError):
    """The captured source no longer matches the target."""
