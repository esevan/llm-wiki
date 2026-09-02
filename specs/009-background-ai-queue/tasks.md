# Tasks: Background AI Queue

**Input**: Design documents from `specs/009-background-ai-queue/`
**Tests**: Required test-first migration and fail-safe coverage.

## Phase 1: Setup

- [x] T001 Add runtime HTTPX and aiosqlite dependencies in pyproject.toml
- [x] T002 Create layered package boundaries in llm_wiki/web/__init__.py, llm_wiki/controllers/__init__.py, llm_wiki/services/handlers/__init__.py, llm_wiki/repositories/__init__.py, and llm_wiki/adapters/__init__.py
- [x] T003 [P] Create deterministic async JSON/stream provider fakes in tests/fakes/ai_provider.py
- [x] T004 [P] Inventory and lock current AI endpoint contracts in tests/test_ai_job_compatibility.py and tests/test_api.py
- [x] T005 [P] Lock current Chat streaming, disconnect, and locale contracts in tests/test_fast_queue.py
- [x] T006 [P] Lock current embedding startup, refresh, and lexical-fallback contracts in tests/test_embedding_jobs.py

---

## Phase 2: Foundational

- [x] T007 Define job states, task descriptors, results, leases, checkpoints, and safe errors in llm_wiki/services/jobs.py
- [x] T008 Write schema, transition, claim, lease, checkpoint, publication, and notification contract tests in tests/test_ai_jobs.py
- [x] T009 Implement SQLite job schema and async repository with per-process connections in llm_wiki/repositories/jobs.py
- [x] T010 Implement atomic claim, heartbeat, expiry recovery, bounded retry, and live-token guards in llm_wiki/repositories/jobs.py
- [x] T011 Implement handler registry, task context, cancellation, progress, and result validation contracts in llm_wiki/services/handlers/registry.py
- [x] T012 Characterize synchronous compatibility behavior, then remove the temporary executor after every endpoint migrates
- [x] T013 Add compatibility parity tests for success, failure, source protection, and transaction boundaries in tests/test_ai_job_compatibility.py
- [x] T014 Characterize Draft, Refine, Image Summary, Completion Review, enrichment, Lineage, and report calls before changing API responses
- [x] T015 Implement durable async dispatcher, configurable concurrency, lease heartbeat, shutdown, and recovery in llm_wiki/services/jobs.py
- [x] T016 Add concurrent claim, lease recovery, duplicate-delivery, and poison-job tests in tests/test_ai_jobs.py and tests/test_async_workers.py
- [x] T017 Convert provider adapter to shared-process HTTPX AsyncClient JSON and stream methods while preserving fake injection in llm_wiki/adapters/provider.py
- [x] T018 Preserve synchronous provider compatibility during migration, then delete it after all call sites use Fast or durable queues
- [x] T019 Add independent `web`, `fast-worker`, and `async-worker` CLI roles plus default supervisor wiring in llm_wiki/cli.py
- [x] T020 Add cross-platform spawned-process lifecycle tests in tests/test_worker_processes.py

**Checkpoint**: Existing API suite passes unchanged and eligible synchronous calls produce terminal compatibility Jobs.

---

## Phase 3: User Story 1 — Observable non-blocking work (P1)

**Goal**: Durable work runs outside web processes and is visible in the bottom-right Queue; Fast requests remain hidden and globally throttled.

**Independent Test**: Run two web processes, one Fast worker, and two durable workers; verify one Fast provider request, independent durable saturation, and live Queue states.

- [x] T021 [P] [US1] Write durable Jobs list/detail/events/cancel/retry contract tests in tests/test_jobs_api.py
- [x] T022 [P] [US1] Write global single-consumer Fast throttling and disconnect tests in tests/test_fast_queue.py
- [x] T023 [US1] Implement loopback-only single-consumer Fast service and stream protocol in llm_wiki/services/fast_queue.py
- [x] T024 [US1] Proxy Chat, next-stage Chat, and completed Chat through Fast service in llm_wiki/controllers/application.py
- [x] T025 [US1] Implement Jobs snapshots, SSE invalidation hints, cancel, retry, and safe-result endpoints in llm_wiki/controllers/jobs.py
- [x] T026 [US1] Convert approved durable endpoints from compatibility execution to `202 + job_id` one task family at a time in llm_wiki/controllers/application.py
- [x] T027 [US1] Add Queue panel layout, active/recent sections, progress, cancellation, retry, and result actions in llm_wiki/static/index.html
- [x] T028 [P] [US1] Add Queue and status resources with `사용자` terminology in llm_wiki/static/i18n/en.json and llm_wiki/static/i18n/ko.json
- [x] T029 [US1] Add browser coverage for Queue state, keyboard operation, focus, reconnect, and hidden Fast work in tests/test_browser_menu.py
- [x] T030 [US1] Move semantic embedding generation/refresh/cleanup to resumable durable document Jobs in llm_wiki/services/retrieval.py
- [x] T031 [US1] Add embedding progress, checkpoint, stale-model/source, restart, and lexical-fallback tests in tests/test_embedding_jobs.py

---

## Phase 4: User Story 2 — Task-specific result context (P1)

**Goal**: Each completed Job returns to the correct inline or deep-linked context without unexpected navigation.

**Independent Test**: Complete Draft, Refine, Image Summary, Completion Review, Conflict Review, Lineage, and report Jobs and verify their distinct result policies.

- [x] T032 [P] [US2] Write task-result policy and source-race tests in tests/test_ai_job_compatibility.py and handler suites
- [x] T033 [P] [US2] Write Draft/Refine close, supersede, completion-race, and no-detached-result browser tests in tests/test_browser_menu.py
- [x] T034 [P] [US2] Write Image Summary scroll-preservation and Work-tab deep-link browser tests in tests/test_browser_menu.py
- [x] T035 [US2] Implement staged proposal and result-publication policies in llm_wiki/services/handlers/registry.py
- [x] T036 [US2] Implement Draft/Refine originating-surface cancellation and inline result binding in llm_wiki/static/index.html
- [x] T037 [US2] Implement Image Summary automatic exact-entry attachment and result destination in llm_wiki/services/handlers/image_summary.py
- [x] T038 [US2] Implement Solution Work-tab summary anchors and scroll-preserving live update in llm_wiki/static/index.html
- [x] T039 [US2] Migrate Conflict Review, Lineage Inference, Workbench Organization, and Completion Report to distinct modules in llm_wiki/services/handlers/
- [x] T040 [US2] Remove superseded in-process Conflict Review and semantic background thread state from llm_wiki/api/app.py and llm_wiki/services/conflict_review.py

---

## Phase 5: User Story 3 — Recoverable translations (P1)

**Goal**: Knowledge, Capture, and Work Log translations run durably with exact-source checkpoints and preserve authored content.

**Independent Test**: Interrupt paragraph and derived-content translations, change selected source units, restart workers, and verify valid reuse, stale rejection, Vault handoff, and source preservation.

- [x] T041 [P] [US3] Write Knowledge paragraph checkpoint, reconciliation, and cleanup tests in tests/test_localization.py
- [x] T042 [P] [US3] Write Capture and Work Log immediate-enqueue/source-preservation tests in tests/test_localization.py
- [x] T043 [US3] Implement paragraph-level Knowledge translation handler and checkpoints in llm_wiki/services/handlers/knowledge_translation.py
- [x] T044 [US3] Implement atomic Vault translation staging, exact-hash reconciliation, and post-handoff working-row cleanup in llm_wiki/services/localization.py
- [x] T045 [US3] Replace reader-bound server translation execution with durable Job progress and Knowledge result binding in llm_wiki/controllers/application.py
- [x] T046 [US3] Implement Capture and Work Log body/comment/checklist derived-translation enqueue triggers and handlers in llm_wiki/services/handlers/derived_translation.py
- [x] T047 [US3] Update Knowledge, Capture, and Work Log UI to consume background status/results without changing source in llm_wiki/static/index.html
- [x] T048 [US3] Add browser coverage for navigation-independent translation, retry, stale source, and source fallback in tests/test_browser_menu.py

---

## Phase 6: User Story 4 — Decision-required notifications (P2)

**Goal**: Completion Review and other decision-required results produce one temporary toast and one persisted unread alert.

**Independent Test**: Complete a Review away from its Solution, lose/replay events, open/dismiss the alert, and verify one notification and accurate unread count.

- [x] T049 [P] [US4] Write notification uniqueness, replay, read, dismiss, and missing-target API tests in tests/test_ai_jobs.py and tests/test_jobs_api.py
- [x] T050 [P] [US4] Write toast timing, bell badge, keyboard, and result-navigation browser tests in tests/test_browser_menu.py
- [x] T051 [US4] Implement idempotent notification publication and query/update repository methods in llm_wiki/repositories/jobs.py
- [x] T052 [US4] Implement notification Controller and Completion Review policy in llm_wiki/controllers/jobs.py and llm_wiki/services/handlers/completion_review.py
- [x] T053 [US4] Add bottom-right toast and top-right bell/unread panel in llm_wiki/static/index.html
- [x] T054 [P] [US4] Add localized notification resources in llm_wiki/static/i18n/en.json and llm_wiki/static/i18n/ko.json

---

## Phase 7: Polish and Cross-Cutting Concerns

- [x] T055 Reconcile every provider/embedding call site against the task registry and prohibit unregistered durable direct calls in tests/test_ai_task_inventory.py
- [x] T056 Add enqueue latency, Queue update, pool isolation, and 1,000-note embedding regression checks in tests/test_performance.py
- [x] T057 Add safe logging for job/attempt/lease/publication identities without prompts, credentials, or binary payloads in llm_wiki/services/jobs.py
- [x] T058 Update feature documentation and indexes in docs/features/background-ai-queue.md, docs/features/background-ai-queue.ko.md, docs/features/README.md, docs/features/README.ko.md, README.md, README.ko.md, and docs/CONTINUATION.md
- [x] T059 Run quickstart recovery/process/browser scenarios from specs/009-background-ai-queue/quickstart.md
- [x] T060 Run `uv run pytest -q`
- [x] T061 Run the exact browser-script syntax validation command from AGENTS.md
- [x] T062 Run `git diff --check`
- [x] T063 Add configurable durable worker count to llm_wiki/controllers/application.py, llm_wiki/services/settings.py, llm_wiki/static/index.html, and tests/test_api.py without exposing Fast worker concurrency
- [x] T064 Add web-process loss, SSE reconciliation, and browser reconnect coverage in tests/test_worker_processes.py and tests/test_browser_menu.py
- [x] T065 Cover Draft, Refine, Image Summary, Completion Review, enrichment, Lineage, and report characterization/parity by task family in tests/test_ai_job_compatibility.py and existing API suites
- [x] T066 Split each durable endpoint conversion into a named task registry entry and per-family `202` contract assertion in llm_wiki/services/handlers/registry.py and tests/test_jobs_api.py
- [x] T067 Move HTTP routes from llm_wiki/api/app.py into llm_wiki/controllers/ and expose app composition through llm_wiki/web/app.py
- [x] T068 Move durable Queue SQLite access into llm_wiki/repositories/ and preserve domain use cases in llm_wiki/services/
- [x] T069 Remove compatibility imports, superseded direct provider calls, unmanaged background threads, duplicate handlers, and dead modules after all callers migrate
- [x] T070 Add architecture dependency and dead-code inventory tests in tests/test_architecture.py and tests/test_ai_task_inventory.py

## Dependencies

- Setup → Foundational → all user stories.
- US1 establishes processes, APIs, and Queue UI required by US2–US4.
- US2 and US3 can proceed in parallel after US1; US4 depends on US1 notification-independent SSE/snapshot foundations but not US2/US3 completion.
- Polish requires all selected stories.

## Parallel Examples

- Setup characterization tests T003–T006 can be written independently before shared implementation.
- US1 API contract tests T021, Fast tests T022, and locale resources T028 can run in parallel.
- US2 task-result, Draft/Refine, and Image Summary tests T032–T034 can run in parallel.
- US3 Knowledge and derived-translation tests T041–T042 can run in parallel.
- US4 API and browser tests T049–T050 can run in parallel.

## Implementation Strategy

1. Treat T004–T020 as a mandatory migration gate: no endpoint becomes `202` until its current contract and synchronous Job parity pass.
2. Deliver US1 as the infrastructure MVP, including embeddings and global Fast throttling.
3. Add task-specific results and translations independently, then decision-required notifications.
4. Remove a compatibility path only after both legacy invariants and the new async contract pass.
