# Native application architecture

**English** | [한국어](architecture.ko.md)

LLM Wiki is a native-only React and Tauri application. React depends on one typed
`ApplicationClient`; the Tauri adapter maps presentation compatibility operations to a small set
of allowlisted domain commands. Rust owns application behavior, SQLite workflow persistence,
home-directory application settings, Vault file access, background jobs, provider integration,
localization, and offline semantic retrieval.

```text
React feature -> ApplicationClient -> Tauri domain command -> Rust application module
                                                       -> SQLite / ~/.llm-workbench settings
                                                       -> Markdown Vault
                                                       -> bundled ONNX embeddings
                                                       -> configured AI provider
```

There is no FastAPI server, internal TCP listener, Python process, sidecar, HTTP application
adapter, or web delivery mode. `http://ipc.localhost` in the content-security policy is Tauri's
virtual WebView IPC origin and does not represent a listening socket. HTTP is used only when a user
configures an external AI provider.

## Frontend boundaries

| Boundary | Location | Responsibility |
| --- | --- | --- |
| App shell | `frontend/src/app/` | Window navigation and top-level composition |
| Features | `frontend/src/features/` | Domain screens and visible interaction surfaces |
| Shared UI | `frontend/src/components/` | Reusable primitives only |
| Application client | `frontend/src/services/` | Typed Tauri application boundary |
| Theme | `frontend/src/theme/` | Semantic tokens, global rules, and component styles |
| Static controllers | `frontend/public/runtime/` | Eleven bounded compatibility controllers |
| Localization | `frontend/public/i18n/` | Bundled English and Korean resources |
| Build output | `dist/` | Ignored, reproducible frontend artifact consumed by Tauri |

The compatibility controllers contain no raw `fetch` or direct IPC. They call the centralized
application client; only `tauriApplicationClient.ts` imports Tauri `invoke` and `Channel`.

## Native boundaries

| Boundary | Location | Responsibility |
| --- | --- | --- |
| Tauri entry | `src-tauri/src/lib.rs` | Validate domain allowlists and delegate commands |
| Workflow | `src-tauri/src/native/workflow.rs` | Capture, Problem, Solution, transitions, evidence, and Compass |
| Jobs | `src-tauri/src/native/jobs.rs` | Durable lifecycle, retry, timeout, and cancellation |
| Result handlers | `src-tauri/src/native/job_results.rs` | Validate and persist task-specific results |
| Completion/Lineage | `completion.rs`, `lineage.rs` | Completion records and auditable lineage |
| Vault | `vault.rs`, `patches.rs`, `projection.rs` | Safe Markdown indexing, search, and atomic writes |
| Semantic engine | `semantic.rs` | Bundled multilingual ONNX inference |
| Settings | `settings.rs`, `localization.rs` | Atomic home settings, locale/provider routing, and legacy import |
| Database | `database.rs`, `migrations.rs`, `schema.sql` | Connections, ordered schema versions, and legacy normalization |
| Provider | `src-tauri/src/provider.rs` | The only external AI protocol adapter |

Tauri commands remain thin: they select a domain, validate the operation boundary, invoke
`NativeApplication`, and translate the result. Persistent writes use transactions or temporary-file
replacement, source hashes block stale publication, and Windows overwrite uses `ReplaceFileW`.

## Startup and resources

On a new installation, the React onboarding layer blocks application interaction while a thin Tauri
command opens the native folder picker. Rust validates and persists the chosen path in
`~/.llm-workbench/settings.json`, then restarts the application against that Vault. Existing
databases adopt their former Documents Vault without a prompt and import legacy locale/provider
values once. Existing version-zero databases are upgraded through ordered, transactional SQLite
migrations before that import. A database created by a newer app is rejected instead of being
silently downgraded. The window becomes available before Vault indexing; indexing does not start while setup
is pending. A blocking worker performs the initial scan and loads the checksum-pinned multilingual
MiniLM model only when semantic work needs it. The release application never downloads a model or
font at runtime. Build preparation verifies five model files and copies 148 WOFF2 subsets for
Nunito, DM Mono, and Noto Sans KR.

## Verification boundaries

- Vitest protects React composition and the Tauri application adapter.
- `verify_runtime.mjs` parses all eleven compatibility controllers and rejects an HTTP fallback.
- Rust unit and command tests exercise real SQLite, filesystem, workflow, cancellation, and ONNX
  behavior.
- The packaged desktop E2E uses React → Tauri → Rust → SQLite/Vault and verifies full relaunch.
- macOS produces an `.app`; the Windows workflow produces MSI and NSIS installers using
  [the Windows agent guide](windows-packaging.md).

The retired FastAPI implementation and its characterization tests remain inspectable at Git commit
`caef236`; see [the retirement record](migrations/python-browser-retirement.md).
