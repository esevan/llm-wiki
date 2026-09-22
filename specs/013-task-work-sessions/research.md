# Research: Task Work Sessions

## Session boundary

- **Decision**: Use a Task-owned aggregate separate from AI refinement and MCP work tracking.
- **Rationale**: Existing sessions carry proposal/job or external-client semantics that conflict with plain private records.
- **Alternatives considered**: Refinement sessions and Work Log both introduce unrelated lifecycle meaning.

## Persistence and ownership

- **Decision**: Store locally and require both Task ID and session ID on every session read/write.
- **Rationale**: Restart persistence and cross-Task rejection belong below the UI.
- **Alternatives considered**: Browser storage lacks native ownership enforcement; files add a new adapter.

## Attachments

- **Decision**: One optional app-managed attachment per record with name, media type, bytes, and a 10 MB input cap.
- **Rationale**: Supports paste/preview/reopen/download without arbitrary project-file access.
- **Alternatives considered**: External paths become stale; indexing and multiple files expand scope.

## Provider, model, approval, and workspace

- **Decision**: Keep core lifecycle provider-independent. Expose a catalog seam with Codex only, five fixed model IDs, Sol default, saved approval intent, and opaque workspace path.
- **Rationale**: Selects prevent invalid values while keeping execution absent and extensibility explicit.
- **Alternatives considered**: Freeform values, runtime discovery, folder traversal, and actual CLI integration exceed scope.

## Async and failure behavior

- **Decision**: Guard responses by Task/session generation; retain one operation identity until confirmed; separate committed append from reload failure.
- **Rationale**: Rapid navigation cannot expose stale data or clear newer drafts, and retries cannot duplicate records.
- **Alternatives considered**: Unguarded promises and a new identity per click are unsafe.

## Verification

- **Decision**: Component/native tests plus one targeted packaged restart scenario; add a 100 ms p95 local projection profiling assertion.
- **Rationale**: Only real process relaunch remains materially uncovered after cheaper checks. Full E2E is disproportionate.
- **Alternatives considered**: Unit-only cannot prove packaged restart; full-suite E2E covers unrelated workflows.
