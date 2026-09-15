# LLM Wiki — Continuation handoff

**Updated:** 2026-09-15
**Current status:** The React/Tauri/Rust desktop now uses the Task as the canonical unit of work.
Capture remains a lightweight input; Work Log, refinement, exact Problem revisions, Task
relationships, advisory review, completion, and Knowledge publication are independent decisions.
The Python/FastAPI browser delivery remains retired in Git history at `caef236`.

## Latest local installation verification

- Installed the application built from `7eac07c` (Capture/refinement images) at
  `/Applications/LLM Wiki.app` on 2026-09-15. The guarded installer retained the
  previous bundle at `/Applications/LLM Wiki.app.previous-1789513091476-58810`.
- Strict/deep signature checks passed with the existing local signing identity and
  unchanged designated requirement. A normal Launch Services launch remained alive
  and created a 1280 × 821 window. The installed executable matched the tested
  candidate (SHA-256 `af987f8c3722fc21e20261886f42a306be875d218687c0a9c4e8996aa3f59d50`).
- A read-only check confirmed the existing database reached schema 11 and contains
  `input_images`. No screen interaction or feature E2E was performed in this run.
- **Packaging follow-up:** the default Intel macOS signed build still references an
  unsigned Homebrew ONNX runtime and aborts before database initialization. For this
  installation, 86 runtime libraries were copied into the candidate's Frameworks
  directory, their dependencies changed to bundle-relative paths, and all libraries
  and the app signed. Local signing has no Team ID, so the candidate also required
  `com.apple.security.cs.disable-library-validation`; system security settings were
  unchanged. These are artifact repairs, not changes to the build scripts. Automate
  dependency bundling and the local-signing policy before relying on a fresh build.
- Local diagnostic scripts and results remain under
  `.worktrees/capture-refinement-images/.tmp/` (`repair-package.py`,
  `package-smoke.py`, and `installed-launch-verification.json`).

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

## Refinement dialog follow-up

The focused Refinement dialog keeps the proposed result and the conversation visible together, and
preserves Task proposal `body` values as Task detail when accepted. The signed release build,
202 frontend tests, typecheck, lint, and whitespace checks passed. Rendered review covered Korean
at 1280×800 and 800×600, English at 600×650, and keyboard focus containment.
The final packaged E2E passed 29 of 33 scenarios, including refinement and modal geometry at
1200px/900px. `task-controls` (waiting for a Knowledge draft), `task-legacy-chat-controls`,
`task-mcp-continuation`, and `global-queue-notifications` timed out. Three also failed before the
dialog change; `task-controls` is newly observed and its cause is not established. The earlier
`task-chat-controls` failure passed in the final run. The complete release gate remains unmet.
Results are retained in the task worktree at `.tmp/desktop-e2e-artifacts-PUCV85/results.json`.

Integration with the database-lock recovery change passed 206 npm tests, typecheck, lint, a signed
release build, and six focused packaged scenarios: refinement, localization, provider recovery,
close retry, close pending, and relaunch. Results: `.tmp/desktop-e2e-artifacts-YF1Gtd/results.json`.
This focused verification does not replace the full release gate noted above.

## Knowledge draft Queue handoff — 2026-09-14

- Branch `fix/knowledge-draft-feedback` queues private Knowledge generation and opens the exact
  completed result in Workbench Review, focused and scrolled to its Markdown preview. The Queue
  result survives closing the Task panel and restarting the app; publication stays explicit.
- Cancelled jobs cannot save a draft. Draft storage, source validation, and Queue completion are
  atomic. Older, corrected, or published result snapshots are read-only.
- Previous feature validation: 213 frontend tests, full Cargo tests, typecheck, lint, whitespace
  check, signed release build, and packaged `task-controls`, `task-publication`, and `task-review`
  passed. English and Korean wide/narrow windows and opening a persisted result from Search were
  visually reviewed.
- The additional `global-queue-notifications` packaged scenario still times out waiting for the
  visible click target of an existing completed-job result. It did not pass; do not report the entire
  desktop suite green. Diagnostics: `.tmp/desktop-e2e-artifacts-BcWWpu/results.json` in this worktree.
- The primary checkout's existing user edits are excluded from this feature commit.
- Post-rebase integration validation passed 217 frontend tests, full Cargo tests, typecheck, lint,
  whitespace checks, and the signed release build. Packaged `task-publication` and `task-review`
  passed on that build. `task-controls` timed out after lineage loading, so the full desktop gate
  remains unmet. Diagnostics: `.tmp/desktop-e2e-artifacts-919VQg/results.json` in this worktree.
