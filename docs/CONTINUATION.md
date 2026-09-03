# LLM Wiki — Continuation handoff

**Updated:** 2026-09-03
**Current status:** React/Tauri/Rust native desktop only. The Python/FastAPI browser delivery was
retired after parity verification and is available only in Git history at `caef236`.

## Runtime

- React owns the shell, primary screens, dialogs, navigation, and shared UI.
- Eleven bounded controllers in `frontend/public/runtime/` preserve complex presentation behavior
  through the centralized Tauri application client; none contains an HTTP fallback.
- Thin domain commands delegate to Rust workflow, jobs, completion, Lineage, localization, Vault,
  semantic, settings, and provider modules.
- SQLite stores workflow and settings. Markdown Vault writes use source hashes and atomic replace.
- The multilingual embedding model and all required fonts are bundled; runtime downloads are not
  required.
- No internal port, Python process, sidecar, web server, or browser product remains.

## Verification record

- React/adapter/runtime: 10 Vitest tests plus the eleven-module runtime parse and HTTP-fallback gate.
- Rust: 23 unit and command tests.
- macOS: release `.app` build and real launch/workflow/search/relaunch E2E pass.
- Windows/Linux/macOS: lint, typecheck, Rust tests, and unbundled Tauri build are configured in CI.
- Windows MSI/NSIS packaging and optional installation are automated by
  `scripts/package_windows.ps1`; see [the Windows guide](windows-packaging.md).

## Post-migration product backlog

These are product enhancements rather than migration work:

1. Add corpus-wide semantic search in addition to lexical-candidate reranking.
2. Add a visible three-way merge for non-overlapping external Markdown edits.
3. Ratify and automate multilingual search, startup, command-latency, and memory budgets.
4. Add richer Conflict Review progress/deduplication and Lineage inference-failure presentation.
5. Decide the durable Queue history TTL and accessibility acceptance scope.

## Future work rules

Run `npm test`, lint, typecheck, production build, Rust format/clippy/tests, Tauri build, and packaged
desktop E2E for native changes. Preserve the application-client boundary and keep command handlers
thin. Do not restore the browser server as a compatibility shim; its retirement record is
[here](migrations/python-browser-retirement.md).
