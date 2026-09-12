# LLM Wiki — Continuation handoff

**Updated:** 2026-09-12
**Current status:** The React/Tauri/Rust desktop now uses the Task as the canonical unit of work.
Capture remains a lightweight input; Work Log, refinement, exact Problem revisions, Task
relationships, advisory review, completion, and Knowledge publication are independent decisions.
The Python/FastAPI browser delivery remains retired in Git history at `caef236`.

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

## Task-centered verification record

**UI/UX follow-up final package:** the signed `9b2b81082a43de0637bedd33a8cc670709ff2b21` bundle passed 32/32 scenarios. The generated inventory recorded 188 scanned source controls and 175 rendered/exercised/asserted controls; all six coverage-gap arrays were empty. Manual Korean IME, VoiceOver, OS reduced motion, Windows, zoom, and native quit/crash draft durability remain unverified or out of scope.

Nine final native 1198×768 Korean-light captures from isolated `Dlnhjg` state were visually reviewed, including the Workbench, Task detail, Refinement, Task review, completion/Knowledge, Queue recovery, Vault search, Compass, and AI setup. Task Escape returned focus to its original trigger. This does not verify real Korean IME, VoiceOver, or OS reduced motion.

- React/adapter/runtime: 34 Vitest files and 176 tests passed, with six desktop-helper, two fake-provider,
  and four signing/runtime/native-only/application-boundary checks. Typecheck and scoped changed-code
  ESLint pass. Full lint retains 23 pre-existing `no-explicit-any` findings in untouched completed- and remaining-runtime tests.
- Rust: `cargo test --manifest-path src-tauri/Cargo.toml` passed 106 tests.
- macOS: the signed release bundle built 2026-09-12 11:44:31 AM by `LLM Wiki Local Signing` has CDHash
  `9b2b81082a43de0637bedd33a8cc670709ff2b21`. It was built from dirty `fix/ui-ux-improvements` worktree
  state based on `de01ff47398871902a765d43b5a4060161316f92`; its full packaged E2E passed 32/32.
  Manual visual/accessibility claims remain limited as stated above.
- Windows/Linux/macOS lint, typecheck, Rust tests, and unbundled Tauri builds remain configured in
  CI. Windows MSI/NSIS packaging is automated by `scripts/package_windows.ps1`; installed Windows
  named-pipe and Unicode-path acceptance remains an external gate on this macOS host.
- Exact Task-centered evidence is retained in
  [acceptance-verification.md](../specs/012-task-centered-workbench/acceptance-verification.md).
- Current interactive coverage recorded 175 rendered/exercised/asserted controls across 188 scanned source controls; all six evidence-gap arrays were empty. OS reduced-motion review, real-provider quality/latency, and Windows/Linux installed testing remain unverified.

## MCP Workbench Bridge convergence

- Capture preview is non-persistent until exact acceptance. In-app cards and MCP Elicitation share
  the same application service for Problem, Task, checkpoints, conflict resolution, completion,
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

## Deferred Task-centered architecture work

The behavior release intentionally defers the internal follow-up items recorded in
[Task-centered architecture backlog](backlog/task-centered-architecture.md): duplicated Task
mutation/service/refinement SQL, handwritten DTO and operation registries, legacy DOM retirement,
Task assistance responsibilities, and migration recovery scope. A plugin backlog capture was
attempted but requires connector elicitation; no remote record was saved.

## Future work rules

Run `npm test`, lint, typecheck, production build, Rust format/clippy/tests, Tauri build, and packaged
desktop E2E for native changes. Preserve the application-client boundary and keep command handlers
thin. Do not restore the browser server as a compatibility shim; its retirement record is
[here](migrations/python-browser-retirement.md).
