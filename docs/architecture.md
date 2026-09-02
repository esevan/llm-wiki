# Backend architecture

**English** | [한국어](architecture.ko.md)

LLM Wiki uses a layered backend with a single web composition boundary. Dependencies point from
delivery toward application logic and persistence; lower layers never reach back into controllers
or the web application.

| Layer | Responsibility | Main location |
| --- | --- | --- |
| Web | Build the application runtime and expose the public app factory | `llm_wiki/web/` |
| Controller | Validate HTTP input, map responses, and bind routes to use cases | `llm_wiki/controllers/` |
| Service | Execute workflow use cases, submit jobs, and run task handlers | `llm_wiki/services/` |
| Repository | Persist durable job state and checkpoints in SQLite | `llm_wiki/repositories/` |
| Core | Define dependency-free Queue domain values, states, and errors | `llm_wiki/core/` |
| Adapter | Integrate with AI providers and other external mechanisms | `llm_wiki/adapters/` |

`web.app.create_app` is the public composition root. It builds one `ApplicationRuntime`, then
passes that runtime to `controllers.application.create_http_app`. Controllers do not construct
repositories or provider adapters. Queue submission belongs to `services/job_submission.py`, while
SQLite lifecycle and transition rules remain in `repositories/jobs.py`.

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

## Enforced design rules

Architecture tests reject reverse layer imports, internal import cycles, duplicate or misplaced
task descriptors, restored superseded modules, and excessive branching in Queue-facing backend
functions. API-level compatibility tests run against the public web factory so refactors preserve
the observable contract. These checks live in `tests/test_architecture.py` and
`tests/test_ai_task_inventory.py`.

See [Background AI Queue](features/background-ai-queue.md) for user-visible Queue behavior.
