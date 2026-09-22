# Research: Task Codex Execution

## Persistent execution boundary

**Decision**: Run one supervised `codex app-server` process per desktop application. Bind each Task work session to an exact Codex thread and each local Run to one turn.

**Rationale**: Codex retains conversation and harness state; LLM Wiki owns Task association, audit, UI recovery, and Work Log projection.

**Alternatives considered**: Fresh `codex exec` calls rebuild context. Replaying local history while resuming duplicates context. A generic provider engine adds unused scope.

## Authority and effective configuration

**Decision**: Omit approval-policy and sandbox overrides. Route approvals to `user` for Ask; allow `auto_review` only after a new explicit opt-in. Treat legacy `auto` as inert and show the safe effective thread configuration returned by start/resume.

**Rationale**: Historical intent is not current authorization. Guessed privilege controls can broaden a real provider request.

**Alternatives considered**: Mapping legacy `auto` to `never` or unrestricted execution violates human authority.

## Transport, dispatch, and interruption

**Decision**: Use one serialized stdin writer, one stdout reader, and connection generations. Persist dispatch before `turn/start`; an unprovable crash gap becomes `needs_attention` and is never resent automatically. Interrupt acknowledgement records only the request; the actual terminal event resolves completion/cancellation races.

**Rationale**: This preserves ordering and is honest about the gap between a durable record and an external side effect.

**Alternatives considered**: Multiple readers misroute responses. Provider exactly-once dispatch is unsupported.

## Events and formal requests

**Decision**: Stream safe item/status progress metadata, buffer text deltas per item without exposing partial text, and upsert/display redacted completed items by Run/item ID. Preserve identical deltas. Bind formal requests to generation, RPC ID, Task/session/thread/turn, actual choices, and blocking behavior. Capability-check experimental user input; do not collect secret input.

**Rationale**: Completed items are bounded and replay-safe. Formal answers must return through the original RPC, never prose.

**Alternatives considered**: Token-row storage is unbounded; hash dedup corrupts repeated output; invented buttons change permissions.

## Context continuity

**Decision**: Bootstrap with bounded Task definition, exact linked references, constraints, and instruction. Later turns use Codex history plus a Task-context delta only when its hash changes. Vault evidence remains capped at eight passages and 6,000 tokens.

**Rationale**: The Task stays current without duplicating the conversation Codex already owns.

**Alternatives considered**: Re-sending all Knowledge or history is costly and risks cross-Task context.

## Work Log and report

**Decision**: Create one existing Work Log row per Run with an immutable initial body. Join current Run status plus bounded report/evidence excerpts into its normal projection, with full detail in the Run. No summary turn is added. Missing report is explicit; repair touches only Run-owned metadata and never reruns Codex.

**Rationale**: Manual body/comments/attachments remain untouched and model claims remain distinct from observed evidence.

**Alternatives considered**: Terminal prose overwrite risks user content. Separate summarization adds cost and another failure. Fragment logs are unreadable.

## Platform and validation

**Decision**: Spawn program plus fixed argv without a shell; support an explicit test/config executable and bounded platform-aware discovery. Validate an absolute existing cwd. Use deterministic protocol tests, one opt-in real two-turn integration in an isolated repository, one controlled targeted packaged scenario, and an Astra High design review before substantive UI implementation, separate rendered verification, and Luna E2E.

**Rationale**: GUI PATH and termination differ across macOS/Windows. Mock and live evidence establish different claims.

**Alternatives considered**: Interactive shell discovery expands injection risk. Mock-only completion cannot prove actual Codex integration. Full E2E lacks a concrete broader risk.
