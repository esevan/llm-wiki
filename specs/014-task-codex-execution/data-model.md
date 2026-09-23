# Data Model: Task Codex Execution

## Existing Task Work Session additions

- Nullable unique exact Codex thread ID.
- `approvals_reviewer`: `user` or `auto_review`, default `user`; legacy `approval_mode` remains readable but grants no automatic review.
- Safe effective model, canonical cwd, approval policy, reviewer, and sandbox returned by thread start/resume. Raw config is not stored.
- Hash of the last Task definition/link context sent to the thread.

Existing session IDs, entries, attachment bytes, timestamps, and Task ownership do not change.

## Task Work Session Run

- Stable ID; required Task and session; submission key and payload hash unique by session.
- Nullable `retry_of_run_id` references the prior attempt without replacing it.
- Immutable instruction plus user-entry and Work Log entry references.
- Provider/model/canonical-cwd/context snapshots, thread ID, and nullable provider turn ID.
- Lifecycle: `queued`, `running`, `awaiting_response`, `succeeded`, `failed`, `cancelled`, `interrupted`, `needs_attention`.
- Dispatch: `not_dispatched`, `dispatch_recorded`, `accepted`, `uncertain`.
- Final report, safe error, observed evidence, timestamps, last provider order, UI revision.
- Work Log projection state `pending`, `synced`, or `failed`; repair never starts a turn.

One session has at most one Run in `queued`, `running`, or `awaiting_response`. Same submission key/payload replays; changed payload conflicts. Retry creates a new Run.

## Completed Run Item

- Unique Run plus provider item ID, provider order, kind, terminal status, times.
- Redacted normalized assistant message, command result, file change, artifact, plan, or check evidence.

Deltas are buffered in bounded runtime memory and committed/displayed only with completion. Identical fragments stay ordered; completed-item replay upserts. Live UI progress uses safe status/item metadata, not partial text.

## Formal Request

- Local ID plus exact Run, Task, session, thread, turn, connection generation, and provider RPC ID.
- Command/file/permission approval or user input; redacted scope and actual supported decisions/questions.
- Per question: ID, header, prompt, nullable/empty options, `isOther`, `isSecret`; request-level `isBlocking`.
- State: `pending`, `submitting`, `answered`, `stale`, `error`.
- Nonsecret proposed answers persist on failure; accepted response and times persist on success.

Requests from an old generation, wrong owner, terminal Run, or answered request reject response. A free-text-only question uses text when options are null/empty. With options, extra text follows `isOther`. Secret questions store no answer.

## Work Log Execution Link

- Unique Run and unique existing Work Log entry, projection revision/state/error, timestamps.
- Initial Work Log body is immutable. Current Run status and bounded final-report/evidence/artifact/limitation excerpts join at normal read time; full content remains in Run detail.
- Manual Work Log rows have no link and are never changed by execution sync.

## Runtime-only state

- One child, stdin writer, stdout reader, capability state, connection generation.
- Pending RPC responses, exact thread/turn Run routes, bounded item buffers, early-event buffer, UI subscribers.
- UI subscriber loss does not cancel. Runtime state is not authoritative after process exit.

## Transitions

```text
explicit start -> queued/dispatch_recorded -> running
running -> awaiting_response (blocking request) -> running
running -> succeeded | failed | cancelled (actual terminal event)
dispatch gap -> needs_attention
process loss without exact outcome -> interrupted | needs_attention
retry -> new Run
```

An interrupt acknowledgement sets only “stop requested.”

## Migration 15 invariants

- Additive only; no rebuild of v14 session or Work Log tables, and no replacement of the released v13 Task journey cache.
- Existing sessions receive reviewer `user` regardless of legacy value; binding fields remain null.
- New tables start empty with foreign keys to existing identities.
- Tests compare pre/post session IDs, entries/attachments, Work Log bodies/attachments/comments, counts, and foreign keys.
- Existing verified backup and restore remains active.
