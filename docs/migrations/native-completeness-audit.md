# Native completeness audit

This audit covers the native-only Tauri desktop application. The former FastAPI browser delivery
was retired after this parity audit and remains available only at Git snapshot `caef236`.

## 1. Clean code

PASS. Tauri handlers validate a bounded domain operation and delegate to `NativeApplication`.
Conversation streaming, provider requests, desktop E2E support, completion, Lineage, localization,
patches, refinement context, projection/archive, workbench organization, job lifecycle, and
task-result persistence are separate modules. The latest split reduced `jobs.rs` from 837 to 564
lines and `workflow.rs` from 1,391 to 1,195 lines. Clippy runs with warnings denied.

The remaining browser-compatible UI controllers are feature-named and isolated under
`frontend/public/runtime/`. They are a presentation migration seam, not a desktop transport seam;
they call the centralized application client and contain no direct Tauri IPC.

## 2. Bad smells

PASS with a documented presentation seam. Provider execution and result persistence no longer
diverge in the same module, protected filesystem writes share one cross-platform implementation,
and completion/Lineage/refinement behavior no longer requires changes to the workflow core.
Transport changes terminate in one adapter rather than requiring shotgun edits across features.

## 3. Existing Python behavior

PASS for the desktop capability surface. Every retained HTTP endpoint family maps to a named native
operation or an intentional native stream. Command tests cover workflow gates, localization schema
compatibility, Compass scoring, Work Log/comments/checklists, completion documents, Lineage,
conflicts, patches, Vault search, semantic embeddings, settings secrecy, durable jobs, retries and
cancellation. The release desktop E2E exercises the real React → Tauri → Rust → SQLite/Vault path.

HTTP-only routing, CORS, and server lifecycle assertions were removed with the browser product.
They are not application behavior and are therefore not duplicated in Tauri commands.

## 4. Performance

PASS. Database schema changes now use ordered, transactional versions during startup instead of
running on every command. Existing databases are normalized before legacy settings import, and
reopening at the current version is a no-op. SQLite uses a busy timeout. The window no longer waits
for a Vault walk or 224 MB ONNX model initialization: lexical and semantic indexing run on a
blocking worker after application state is managed. The embedding model is loaded lazily once and
reused, while unchanged documents
reuse source-hash-matched vectors.

## 5. Stability

PASS. Durable jobs have idempotency keys, persisted states, retry, timeouts, and cancellation tokens.
Streaming cancellation drops the active provider response. Managed Markdown and image writes use
temporary files plus atomic replacement; Windows uses `ReplaceFileW` when overwriting. Source hashes
block stale translations, patches, workbench organization, image summaries, and external completion
document edits. Desktop E2E verifies persistence across a full process relaunch.

## 6. Python replacement

PASS for the packaged application. The `.app` contains no Python executable, FastAPI server,
sidecar, port allocation, or loopback request path. Embedding preparation and desktop E2E tooling
were also migrated from Python to Node. The subsequent retirement commit removed the browser
product, Python tests and packaging metadata, and HTTP frontend adapter. The current source tree has
no tracked Python file.

## 7. Platform support

PASS at the source and build-policy level. macOS produces a verified `.app`. The base Tauri bundle
configuration enables all platform targets, with a macOS override that avoids Finder-dependent DMG
layout automation. CI installs and checks React, Rust, command tests, and an unbundled Tauri build on
macOS, Windows, and Linux. Paths use platform directory APIs; secrets use native keyring support;
overwrite semantics have an explicit Windows implementation.

## 8. UX flow

PASS. Capture remains one field and one action. AI work is queued without blocking the Workbench;
approval, conflict resolution, destructive actions, and completion remain explicit user decisions.
New installations require an explicit native Vault folder choice before interaction, while existing
installations keep their former path without a migration interruption. Startup indexing is
background work. Korean empty states and Compass summaries are translated at render time, avoiding
mixed-language dead ends. Four current packaged-app screenshots are organized by capability in the
[visual feature tour](../features/visual-guide.md).

## Verification evidence

| Check | Result |
| --- | --- |
| Retired Python/API characterization | PASS — 196 at `caef236`; removed with product |
| React component/adapter | PASS — 14 tests plus runtime boundary gate |
| Rust unit/command | PASS — 26 tests |
| TypeScript typecheck and ESLint | PASS |
| Rustfmt and Clippy `-D warnings` | PASS |
| Frontend production build | PASS; 148 local WOFF2 subsets verified |
| Embedding bundle | PASS; five pinned assets verified |
| macOS Tauri bundle | PASS; `LLM Wiki.app` |
| Real desktop E2E | PASS; launch, workflow, AI, completion, search, relaunch |
