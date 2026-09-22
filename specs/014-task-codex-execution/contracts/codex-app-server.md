# Codex App-Server Adapter Contract

## Process and initialization

- Spawn resolved Codex with fixed `app-server` argv, piped stdin/stdout, no shell, supervised shutdown.
- One serialized writer and one reader own JSON-RPC I/O.
- Complete initialize/initialized and capability-check experimental user input before use.
- Increment connection generation on replacement; older outstanding requests become stale.

## Thread and turn mapping

- First Run uses `thread/start` with canonical cwd, model, bounded Task bootstrap, and reviewer `user` unless current explicit auto-review opt-in exists.
- Later Runs use `thread/resume` with the exact stored ID, never global latest.
- Omit approval-policy and sandbox overrides; store/show safe effective response fields.
- `turn/start` sends the new instruction, bounded Task delta if needed, and requires the same turn to finish with a report of work, changes, checks, artifacts, and unresolved issues.
- `turn/interrupt` targets exact thread/turn; terminal notification decides outcome.

## Messages and server requests

- Bind thread/turn/item lifecycle, deltas, completed items, and safe errors to the exact Run.
- Buffer early events between dispatch marker and returned turn ID, then bind by exact thread.
- Coalesce deltas internally; expose live status/item metadata and persist/display text only after the item completes; never hash-deduplicate fragments.
- Redact before UI/storage; retain no raw RPC/config/auth/secret/stderr.
- Approval responses use only actual supported decisions. Denial does not itself fail the Run.
- User-input requests render every actual question. Options select per question; one explicit Submit answers the request. Null/empty options allow free-text-only; with options, extra text follows `isOther`. No selection auto-submits.
- Honor `isBlocking`; nonblocking requests do not freeze the turn. Reject stale generation, duplicate, wrong-owner, terminal-Run, and secret responses. Retain nonsecret proposed answers after transport error.

## Failure mapping

Missing executable, authentication, cwd, spawn/init, turn failure, abnormal exit, and explicit interruption remain distinct. Dispatch uncertainty becomes `needs_attention` without resend. On process loss, preserve durable items and mark unresolved Runs interrupted or needing confirmation unless exact recovery proves terminal state.
