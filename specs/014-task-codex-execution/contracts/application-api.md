# Application Contract: Task Codex Execution

URL identities override body identities. Every operation validates Task → session → Run ownership.

## Durable operations

- `GET /tasks/{taskId}/work-sessions/{sessionId}/runs`: bounded newest-first summaries plus active Run.
- `GET .../runs/{runId}`: Run, completed items, formal requests, Work Log link.
- `POST .../runs`: accepts operation ID, instruction, optional explicitly supported input; atomically creates instruction entry, Run, existing Work Log row, and link before dispatch. Same operation/payload replays; changed payload conflicts.
- `POST .../runs/{runId}/interrupt`: records an exact-turn interruption request without claiming cancellation.
- `POST .../runs/{runId}/formal-requests/{requestId}/response`: accepts request-specific decisions or nonsecret answers; rejects stale, wrong-owner, duplicate, unsupported, or secret responses.
- `POST .../runs/{runId}/work-log-sync`: repairs only Run-owned projection/link metadata and never dispatches Codex.

Errors distinguish missing executable, authentication required, invalid folder, start failure, uncertain dispatch, ownership failure, active-Run conflict, operation conflict, and unsupported input.

## Live subscription

A native subscription accepts Task/session/Run IDs and a Tauri Channel. It sends revisioned safe snapshots with Run state, live item/status metadata, completed-item text, and formal requests. Partial text is withheld unless a future stateful redactor is explicitly implemented and tested. Channel closure only detaches; reconnect begins from durable state. Clients ignore older revisions.

No raw JSON-RPC, auth value, raw provider config, secret, or raw stderr crosses this boundary.

## Settings and Work Log

Session update adds `approvalsReviewer`. `user` routes approvals to the user; `auto_review` needs current explicit opt-in. Legacy `approvalMode` does not enable it. Session read exposes safe effective model/cwd/approval/reviewer/sandbox.

Existing Task reads keep `workLog`. A linked row adds `execution` with Run/session IDs, status, tool, model, times, bounded attributed-report/evidence/artifact/limitation excerpts, and sync state; full content is fetched from Run detail. Manual rows omit it and retain all current fields.

## Explicit preparation

`task_session_prepare` accepts Task/session identity and returns a safe snapshot with authoritative effective settings, readiness, capabilities, and settingsRevision. It can create/resume the exact thread but cannot dispatch a turn or create a Run/Work Log. Execute requires that prepared revision and rejects stale/mismatched configuration before dispatch.
