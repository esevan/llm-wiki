# Data Model: Background AI Queue

## AI Job

One durable asynchronous request. Fast requests are excluded.

| Field | Meaning | Rule |
| --- | --- | --- |
| id | Stable identity | UUID, immutable |
| task_kind | Handler key | Required |
| entity_type / entity_id | Owning item | Nullable only for global maintenance |
| status | Lifecycle | `queued`, `running`, `awaiting_review`, `completed`, `cancelling`, `cancelled`, `failed`, `retryable`, `stale` |
| input_json / result_json | Versioned snapshot and validated result | No secrets or oversized binary payloads |
| source_hash / model | Exact source and generator identity | Required when task-bound |
| execution_mode | Migration provenance | `synchronous` or `asynchronous` |
| idempotency_key | Duplicate boundary | Unique for equivalent active work |
| progress_completed / progress_total | Countable progress | 0 ≤ completed ≤ total |
| result_interface / notification_policy | Task presentation rules | Registry-controlled |
| available_at | Earliest claim | Supports backoff |
| created_at / started_at / finished_at | Lifecycle timing | Ordered when present |
| error_code / error_message | Safe diagnostic | No credentials/private payload |

## Job Attempt

One at-least-once execution: `(job_id, attempt)`, worker identity, opaque lease token, lease expiry, heartbeat, timing, outcome, and safe error. Only the current token can update or publish.

## Job Checkpoint

One validated resumable unit: `(job_id, unit_key)`, source hash, model, ordinal, validated result metadata, and completion time. Knowledge uses paragraph keys; embeddings use normalized paths. Changed source/model invalidates reuse.

## Job Publication

Idempotency record `(job_id, publication_kind)` with target revision, destination, and publication time. SQLite domain changes commit publication with mutation. Vault output uses staging, hash recheck, atomic replace, and reconciliation.

## Notification

Decision-required notice with stable id, unique `(job_id, kind)`, localizable content, target, read/dismiss times, and creation time. Uniqueness prevents retry or replay from increasing unread count.

## Fast Request

Ephemeral payload, response stream, and cancellation scope only. It has no durable identity, row, global status, retry, recovery, history, or notification.

## State transitions

```text
queued ─claim→ running ─proposal→ awaiting_review ─user action→ completed
                    └─safe automatic publication────────────→ completed
queued/running ─cancel→ cancelling → cancelled
running ─transient failure→ queued/backoff or retryable
running ─permanent failure→ failed
queued/running/awaiting_review ─source mismatch→ stale
```

Expired ownership is recovered only after lease expiry. Old tokens cannot publish. Knowledge working rows are deleted only after the durable Vault file is reconciled.
