# Backend architecture

**English** | [한국어](architecture.ko.md)

LLM Wiki uses a layered backend with a single web composition boundary. Dependencies point from
delivery toward application logic and persistence; lower layers never reach back into controllers
or the web application.

The desktop application uses React presentation and a native Rust application layer. React feature
modules depend on `ApplicationClient`; its desktop adapter converts historic compatibility paths to
named domain operations. Tauri exposes separate workflow, vault, settings, jobs, and system
commands plus a provider-stream command with cancellation. Rust opens SQLite and the configured
Vault directly. It does not allocate an internal port, forward HTTP-shaped IPC, launch Python, or
package a Python executable.

```text
React feature -> ApplicationClient -> HTTP adapter -> FastAPI (separate browser product)
                                  -> Tauri adapter -> Rust domain commands -> SQLite/Vault
                                                                        -> bundled ONNX embeddings
                                                                        -> configured AI provider
```

Frontend design tokens, global/component styles, and locally bundled fonts live under
`frontend/src/theme/`. Primary screens are domain modules under `frontend/src/features/`. The
remaining imperative behavior is split into eleven bounded, domain-named controllers under
`llm_wiki/static/runtime/`; React owns navigation, dialogs, and dock surfaces. Controllers contain
no injected styles or raw visual constants, and the HTML entry point contains no inline bootstrap
code. Their incremental replacement with typed React hooks remains tracked in the migration audit.

| Layer | Responsibility | Main location |
| --- | --- | --- |
| Web | Build the application runtime and expose the public app factory | `llm_wiki/web/` |
| Controller | Validate HTTP input, map responses, and bind routes to use cases | `llm_wiki/controllers/` |
| Service | Execute workflow use cases, submit jobs, and run task handlers | `llm_wiki/services/` |
| Repository | Persist durable job state and checkpoints in SQLite | `llm_wiki/repositories/` |
| Core | Define dependency-free Queue domain values, states, and errors | `llm_wiki/core/` |
| Adapter | Integrate with AI providers and other external mechanisms | `llm_wiki/adapters/` |
| Native desktop application | Execute desktop workflow, jobs, settings, and Vault operations | `src-tauri/src/native/` |

The native Vault module starts Markdown indexing on a blocking-worker thread after application
state is ready, so first-window creation does not wait for a Vault scan or ONNX model load.
`native/semantic.rs` lazily loads
the checksum-pinned multilingual MiniLM model from Tauri resources, writes 384-dimensional vectors
to SQLite, and reranks selected search results locally. The release app never fetches this model at
runtime; only the reproducible desktop build preparation downloads the verified assets. Nunito,
DM Mono, and Noto Sans KR are likewise bundled and verified during the frontend build.

`web.app.create_app` is the public composition root. It builds one `ApplicationRuntime`, then
passes that runtime to `controllers.application.create_http_app`. Controllers do not construct
repositories or provider adapters. Queue submission belongs to `services/job_submission.py`, while
SQLite lifecycle and transition rules remain in `repositories/jobs.py`.

That Python composition root is retained only for the browser delivery mode. Tauri does not import,
spawn, or contact it. The desktop composition root is `src-tauri/src/lib.rs`; thin Tauri handlers
allowlist a domain operation and delegate to `src-tauri/src/native/`. HTTP is used from the desktop
only for an explicitly configured external AI provider.

## AI task module map

Each durable task has one `TaskDescriptor` and one searchable task module:

| Task | Handler module |
| --- | --- |
| Draft | `services/handlers/drafting.py` |
| Refine | `services/handlers/refinement.py` |
| Image Summary | `services/handlers/image_summary.py` |
| Completion Review | `services/handlers/completion_review.py` |
| Knowledge translation | `services/handlers/knowledge_translation.py` |
| Capture, Work Log, comment, and checklist translation | `services/handlers/derived_translation.py` |
| Embedding refresh | `services/handlers/embeddings.py` |
| Conflict Review | `services/handlers/conflict_review.py` |
| Workbench organization | `services/handlers/organization.py` |
| Lineage inference | `services/handlers/lineage.py` |
| Completion report | `services/handlers/completion_report.py` |

Shared provider setup, target validation, handler registration, and worker orchestration live in
`provider.py`, `targets.py`, `catalog.py`, `registry.py`, and `worker.py`. Task behavior does not go
into those shared modules.

Conflict Review normalizes provider output in its handler and stores review-scoped conflicts through
`WorkflowEngine`. The browser submits a complete set of human resolutions to the application
controller; the workflow service validates the set and commits resolutions plus the existing
conflict report/address gate in one SQLite transaction. Source-query comparison rejects stale
reviews. Provider output never contains or persists the human action.

## Enforced design rules

Architecture tests reject reverse layer imports, internal import cycles, duplicate or misplaced
task descriptors, restored superseded modules, and excessive branching in Queue-facing backend
functions. API-level compatibility tests run against the public web factory so refactors preserve
the observable contract. These checks live in `tests/test_architecture.py` and
`tests/test_ai_task_inventory.py`.

See [Background AI Queue](features/background-ai-queue.md) for user-visible Queue behavior and
[Conflict Resolution Workflow](features/conflict-resolution-workflow.md) for the review decision flow.
