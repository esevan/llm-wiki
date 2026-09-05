# LLM Wiki — Continuation handoff

**Updated:** 2026-09-05
**Current status:** React/Tauri/Rust native desktop only. The Python/FastAPI browser delivery was
retired after parity verification and is available only in Git history at `caef236`.

## Runtime

- React owns the shell, primary screens, dialogs, navigation, and shared UI.
- Twelve bounded controllers in `frontend/public/runtime/` preserve complex presentation behavior
  through the centralized Tauri application client; none contains an HTTP fallback.
- Thin domain commands delegate to Rust workflow, jobs, completion, Lineage, localization, Vault,
  semantic, settings, and provider modules.
- SQLite stores workflow, indexes, and jobs. Application settings, including the API key, live in
  the atomic `~/.llm-workbench/settings.json`; the key is omitted from UI and MCP responses.
  Markdown Vault writes use source hashes and atomic replace.
- SQLite schema changes are ordered in `native/migrations.rs`, recorded with `PRAGMA user_version`,
  and committed atomically per version. Version-zero Python/native databases migrate on startup;
  newer unsupported versions fail closed.
- New installations require an explicit native folder choice; existing installations retain their
  former Documents Vault without a migration prompt.
- The multilingual embedding model and all required fonts are bundled; runtime downloads are not
  required.
- No internal port, Python process, sidecar, web server, or browser product remains.
- Local stdio MCP gives Codex and ChatGPT desktop a second Chat input over the same Workbench state.
  The stdio child is persistence-free and forwards over a protected Unix domain socket or Windows
  named pipe to the running GUI process, which is the sole owner of the application service,
  SQLite/Vault adapters, and projector. Connection scopes, immutable work events, a leased
  projector, current/topic/overview resources, Vault evidence tools, and separate Knowledge
  draft/publication gates are implemented. ChatGPT web and remote MCP remain intentionally outside
  the first release.

## Verification record

- React/adapter/runtime: 42 Vitest tests, including 4/4 release surface probes and actual runtime
  integration, plus runtime, native-only, and application-boundary checks.
- Rust: all ordinary unit, command, stdio MCP, review, concurrency, search, parity, acceptance, and
  projector tests pass. Release application-boundary acceptance is 9/9 and includes 10,000
  reordered events and projection latency budgets.
- macOS: the final release `.app` E2E passed through a real stdio MCP child, GUI-owned IPC, external
  Capture Elicitation, in-app Problem approval, connection revocation, workflow/Work Log/search,
  and full process relaunch. Installation and the live Codex plugin connection are the final local
  operational steps.
- Windows/Linux/macOS lint, typecheck, Rust tests, and unbundled Tauri builds remain configured in
  CI. Windows MSI/NSIS packaging is automated by `scripts/package_windows.ps1`; installed Windows
  named-pipe and Unicode-path acceptance remains an external gate on this macOS host.
- Exact commands and historical failures are retained in
  [acceptance-verification.md](../specs/011-mcp-workbench-bridge/acceptance-verification.md).

## MCP Workbench Bridge convergence

- Capture preview is non-persistent until exact acceptance. In-app cards and MCP Elicitation share
  the same application service for Problem, Solution, checkpoints, conflict resolution, completion,
  private Knowledge draft, exact publication, and recoverable withdrawal decisions.
- The GUI is the sole SQLite/Vault/application-service owner. A scoped stdio child forwards over a
  user-only Unix socket or Windows named pipe and fails closed when the GUI, connection, or grant is
  unavailable. Session handles are not credentials.
- Workbench and Chat read and update the same session head. Refresh preserves in-progress input;
  non-overlapping stale saves retry once, while governed stale actions require a renewed preview.
- Conflict review uses lexical and semantic evidence in the current Chat AI. The server rejects a
  hidden conflict-model job, fabricated/stale citations, and out-of-scope evidence.
- Topic membership is explicit, whole-Workbench access is a separate scope, and stable overview
  paging never silently mixes revisions. The representative Skill asks one decision at a time and
  keeps completion separate from publication.
- Remaining external evidence: installed Windows packaged E2E and a new Codex task loading the
  freshly installed plugin. ChatGPT web and remote MCP remain intentionally outside Phase 1.

## Post-migration product backlog

These are product enhancements rather than migration work:

1. Add a visible three-way merge for non-overlapping external Markdown edits.
2. Ratify and automate multilingual search, startup, command-latency, and memory budgets.
3. Add richer Conflict Review progress/deduplication and Lineage inference-failure presentation.
4. Decide the durable Queue history TTL and accessibility acceptance scope.

## Future work rules

Run `npm test`, lint, typecheck, production build, Rust format/clippy/tests, Tauri build, and packaged
desktop E2E for native changes. Preserve the application-client boundary and keep command handlers
thin. Do not restore the browser server as a compatibility shim; its retirement record is
[here](migrations/python-browser-retirement.md).
