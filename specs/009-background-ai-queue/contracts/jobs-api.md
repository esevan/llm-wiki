# Durable Jobs API Contract

## Migration compatibility

Before conversion, each API preserves its status/body and records one `execution_mode: synchronous` Job ending `completed` or `failed`. No Job id is required in the legacy response.

## Submit

Converted endpoints return `202 Accepted` with `job_id`, `status`, `task_kind`, `entity_type`, and `entity_id`. Equivalent active submissions reuse the Job. Invalid requests remain immediate `4xx` errors.

## Query and events

- `GET /api/jobs` lists visible durable work; Fast requests never appear.
- `GET /api/jobs/{job_id}` returns identity, related item, localized state, progress, timing, safe error, actions, and destination.
- `GET /api/jobs/events` streams resumable invalidation hints; clients reconcile snapshots after connect, reconnect, or a sequence gap.

Raw prompts, credentials, binary images, and private provider payloads are never returned.

## Cancel, retry, and results

- `POST /api/jobs/{job_id}/cancel` requests cancellation.
- `POST /api/jobs/{job_id}/retry` schedules an allowed attempt.
- `GET /api/jobs/{job_id}/result` returns safe task-specific result metadata and destination.

Draft/Refine remain unapplied, Completion Review returns review content, Image Summary returns a Solution Work anchor, translations return owning-content destinations, and embeddings return coverage. Application uses existing reviewed workflow operations; no generic apply bypass exists.

## Notifications

- `GET /api/notifications` returns unread and recent decision-required notices.
- `POST /api/notifications/{id}/read` marks read.
- `POST /api/notifications/{id}/dismiss` dismisses.

Unread count derives from persisted state and is process/restart safe.
