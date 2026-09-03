# React and native Tauri migration report

See the [native completeness audit](native-completeness-audit.md) for the clean-code, parity,
performance, stability, Python-runtime, platform, and UX review.

This document records the current implementation. The earlier packaged Python sidecar design was
removed after the native-boundary audit found that it still opened an internal loopback socket.

## Architecture

Before the native cutover:

```text
React -> generic Tauri HTTP request -> loopback FastAPI sidecar -> Python services -> SQLite/Vault
```

After the native cutover:

```text
React feature -> ApplicationClient -> domain operation mapper
              -> Tauri workflow/settings/vault/jobs/system command
              -> Rust application module -> SQLite/Vault
              -> bundled multilingual ONNX model for local embeddings
              -> external AI provider only when an AI capability is requested
```

The desktop executable no longer allocates a localhost port, starts Python, embeds a Python binary,
or forwards HTTP-shaped requests. `ApplicationGateway`, sidecar build scripts, the
`desktop-backend` CLI, and PyInstaller are removed. After parity verification, the separately
launched Python web product was also retired in a distinct follow-up commit. Its final source remains
in Git history at `caef236`.

## Module map

| Boundary | Location | Responsibility |
| --- | --- | --- |
| React screens | `frontend/src/features/` | User-visible screens and interactions |
| Theme | `frontend/src/theme/` | Tokens, fonts, global and reusable component styles |
| Application adapter | `frontend/src/services/tauriApplicationClient.ts` | Maps compatibility paths to typed domain operations |
| Tauri commands | `src-tauri/src/lib.rs` | Domain allowlists, streaming channel, cancellation, E2E hook |
| Native workflow | `src-tauri/src/native/workflow.rs` | Workflow state, progress, Compass scoring, and handoff |
| Native job lifecycle | `src-tauri/src/native/jobs.rs` | Durable submission, execution, retry, cancellation, and notifications |
| Native job results | `src-tauri/src/native/job_results.rs` | Validate and persist task-specific results |
| Completion and Lineage | `src-tauri/src/native/completion.rs`, `lineage.rs` | Completion records and auditable evidence snapshots |
| Refinement and projection | `src-tauri/src/native/refinement.rs`, `projection.rs` | Bounded context and protected Markdown writes |
| Native vault | `src-tauri/src/native/vault.rs` | Markdown indexing, search, safe reads |
| Native embeddings | `src-tauri/src/native/semantic.rs` | Offline ONNX inference and semantic reranking |
| Model manifest | `src-tauri/resources/embedding-model/manifest.json` | Pinned revision, size, and SHA-256 verification |
| Native settings | `src-tauri/src/native/settings.rs` | Locale, resources, provider configuration |
| Native schema | `src-tauri/src/native/schema.sql` | Desktop persistence schema compatible with existing core tables |

## API-to-command map

| Previous HTTP family | Native application capability | Tauri command |
| --- | --- | --- |
| health | system state | `system_command` |
| browser-only index events | removed transport wrapper | NOT_APPLICABLE |
| index/search/knowledge | vault index and read model | `vault_command` |
| locale/i18n/provider | persisted settings | `settings_command` |
| captures/problems/features/progress/items/goals | workflow application service | `workflow_command` |
| jobs/notifications | durable job repository | `jobs_command` |
| draft/refine/reviews | native provider job submission | `enqueue_ai_job` |
| chat | provider stream plus AI-run persistence | `conversation_stream` / `cancel_conversation` |

Compatibility status values remain inside the in-memory response returned to the UI runtime; no
HTTP method, header set, URL, or localhost origin crosses the Tauri IPC boundary.

## Test migration map

| Existing behavioral source | Native replacement | Status |
| --- | --- | --- |
| Capture/create/board API tests | direct Rust workflow command test | PASS |
| Approval and conflict gates | direct Rust workflow transition test | PASS |
| Work Log/comment/checklist tests | direct Rust persistence test | PASS |
| Delete/restore API behavior | direct Rust visibility test | PASS |
| Vault indexing/search/read tests | direct Rust filesystem test | PASS |
| Locale/provider secrecy | direct Rust settings test | PASS |
| Tauri transport tests | domain-routing React tests and cross-domain Rust rejection | PASS |
| Retired browser HTTP-only behavior | removed with browser product; final suite at `caef236` | NOT_APPLICABLE |
| Release desktop launch/relaunch | real `.app` E2E | PASS |
| Bundled model and native semantic search | real ONNX unit/command and `.app` E2E | PASS |

## Coverage matrix

| Behavior | BDD scenario | Existing API | React UI | Rust command | Desktop E2E | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Capture and board persistence | Given a thought, when saved, then it appears after reload | PASS | PASS | PASS | PASS | PASS |
| Workflow review gates | Given an unreviewed item, when advanced, then validation prevents it | PASS | PASS | PASS | PASS | PASS |
| Work Log and checklist | Given active work, when evidence is added, then it persists | PASS | PASS | PASS | PASS | PASS |
| Vault lexical and semantic search | Given Markdown, when indexed, then bundled local inference reranks it | PASS | PASS | PASS | PASS | PASS |
| Chat streaming/cancellation | Given a configured provider, when asked, then chunks stream and can cancel | PASS | PASS | PASS | PASS | PASS |
| Durable AI jobs | Given an AI task, when queued, then result/error is durable | PASS | PASS | PASS | PASS | PASS |
| Relaunch persistence | Given saved state, when the app relaunches, then it is restored | PASS | PASS | PASS | PASS | PASS |

## Verification record

| Check | Result |
| --- | --- |
| Retired Python unit/integration/API tests | PASS — 196 at `caef236`; current product has none |
| React unit/component/adapter tests | PASS — 10 plus runtime boundary gate |
| TypeScript typecheck | PASS |
| Frontend production build | PASS |
| Rust command/unit tests | PASS — 23 |
| Tauri release build | PASS — native `.app`, verified embedding model and fonts, no bundled sidecar |
| Desktop E2E | PASS — launch, full workflow, bundled semantic search, relaunch, restoration |

## Retained and removed boundaries

- Retained: no HTTP application endpoint or Python runtime.
- Removed: packaged Python runtime, generic HTTP-shaped Tauri commands, loopback port allocation,
  sidecar process lifecycle, PyInstaller packaging, browser delivery, HTTP adapter, and HTTP-only
  tests.
- External network: AI provider requests still use the configured HTTPS endpoint (or loopback HTTP
  only for a user-selected local model/test double). This is product integration traffic, not
  desktop internal IPC.

## Known limitations and decisions

- The compatibility mapper still accepts the historic path strings because the remaining bounded
  imperative UI controllers use them. These strings terminate in the React adapter and are mapped
  to domain operations; they are not sent as IPC payloads. Replacing the remaining controllers with
  typed hooks is a presentation refactor and does not require another backend migration.
- Static UI sources now live under `frontend/`, build into ignored `dist/`, and have no HTTP
  application fallback. The deleted browser source is retained only by Git history.
- Model binaries remain outside Git because the verified ONNX and tokenizer total about 246 MB.
  `npm run prepare:embedding` deterministically restores them from the immutable model revision;
  `tauri:build` always runs that verification before packaging.
- macOS packages only the `.app` in local builds to avoid Finder-dependent DMG layout automation.
  The base Tauri configuration remains `all` for Windows/Linux packaging, and CI performs native
  build checks on macOS, Windows, and Linux.
