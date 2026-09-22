# Tasks: Task-Centered Workbench

**Input**: Design documents from `/specs/012-task-centered-workbench/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are required by FR-038–FR-040. Each story starts with a failing contract, unit,
interaction, or packaged scenario before implementation.

**Organization**: Tasks are grouped by independently testable user story. Schema and shared typed
contracts are foundational; migration follows immediately because no user data may be opened under
the new writers before preservation succeeds.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup and Executable Contract

**Purpose**: Freeze names, invariants, and test fixtures before parallel implementation.

- [x] T001 Encode new Task aggregate DTOs and request-bridge route mappings from contracts/application-api.md in frontend/src/types/application.ts, frontend/src/services/tauriApplicationClient.ts, and src-tauri/src/native/mod.rs
- [x] T002 [P] Add reusable schema-7 migration fixture builders and invariant comparators in src-tauri/src/native/migrations.rs tests
- [x] T003 [P] Add fake-provider scheduling, cancellation, malformed-result, and evidence fixtures in tests/fakes/openai_server.mjs
- [x] T004 [P] Add Task-centered acceptance rows and requirement IDs to tests/CHARACTERIZATION.md
- [x] T005 Add contract compile tests for shared Workbench, Task, refinement, review, lineage, and error shapes in frontend/src/types/application.test.ts and src-tauri/src/native/mod.rs tests

**Checkpoint**: Backend, frontend, async-job, and MCP slices share one executable vocabulary.

---

## Phase 2: Foundational Domain and Storage

**Purpose**: Shared primitives required by every story.

- [x] T006 Write failing domain tests for Task states, expected revisions, idempotent operations, completion separation, relationship cycles, and user-activity allowlist in src-tauri/src/domain/task.rs
- [x] T007 Implement Task/revision/link/relationship/readiness/completion value objects and validation in src-tauri/src/domain/task.rs
- [x] T008 [P] Write failing repository contract tests for atomic Task aggregate reads/writes and append-only decisions in src-tauri/src/adapters/sqlite/task_repository.rs
- [x] T009 Implement SQLite Task repository transactions and activity-event allowlist in src-tauri/src/adapters/sqlite/task_repository.rs
- [x] T010 Implement one application Task service consumed by native routes and MCP in src-tauri/src/application/task_service.rs
- [x] T011 Wire thin Task aggregate routes and uniform operation/head conflict errors in src-tauri/src/native/mod.rs
- [x] T012 [P] Add category override migration/type handling for canonical `tasks` and legacy refinement items in src-tauri/src/native/workbench.rs
- [x] T013 Run focused Rust and TypeScript contract tests and reconcile data-model.md only if executable constraints require a documented change

**Checkpoint**: A provider-free Task aggregate exists behind typed routes; user stories can proceed.

---

## Phase 3: User Story 6 - Preserve Existing Work Through Migration (Priority: P1)

**Goal**: Upgrade schema 7 records only after a verified backup, or roll back and restore safely.

**Independent Test**: Migrate and restore every migration-contract fixture and compare IDs, hashes,
bytes, links, foreign keys, and evidence counts.

- [x] T014 [P] [US6] Write failing backup-manifest, integrity, fsync/reopen, interruption, low-disk, and hash-mismatch tests in src-tauri/src/native/database.rs
- [x] T015 [P] [US6] Write failing v8 mapping tests for all valid/invalid fixtures in src-tauri/src/native/migrations.rs
- [x] T016 [US6] Implement writer quiescence and consistent timestamped SQLite backup/manifest verification in src-tauri/src/native/database.rs
- [x] T017 [US6] Implement schema v8 tables, constrained-table rebuilds, indexes, and legacy-to-Task mappings in new src-tauri/src/native/task_schema.sql included only by src-tauri/src/native/migrations.rs
- [x] T018 [US6] Implement row/ID/hash/byte/relationship/foreign-key/evidence invariant validation before `user_version=8` commit in src-tauri/src/native/migrations.rs
- [x] T019 [US6] Implement rollback state, recovery copy, explicit verified restore and safe startup error routes in src-tauri/src/native/database.rs and src-tauri/src/native/mod.rs
- [x] T020 [US6] Add migration startup/retry/restore UI states without exposing paths unsafely in frontend/src/features/vault-setup/MigrationRecoveryView.tsx
- [x] T021 [US6] Run the complete fixture matrix twice to prove rerun idempotency and source recoverability in src-tauri/src/native/migrations.rs tests

**Checkpoint**: Existing records open truthfully under schema 8 or remain verified and restorable.

---

## Phase 4: User Story 1 - Capture or Register a Task (Priority: P1) 🎯 MVP

**Goal**: Explicit input modes and correctly ordered canonical Workbench projections.

**Independent Test**: Save both modes, fail/retry, restart, and verify shortcut/category identity and
ordering without category redesign.

- [x] T022 [P] [US1] Write failing native tests for atomic Capture/direct-Task creation, hidden provenance, projection deduplication, and background recency in src-tauri/src/native/workbench.rs
- [x] T023 [P] [US1] Write failing React interaction tests for two input modes, retained failure input, ordered shortcut areas, categories, keyboard and narrow layout in frontend/src/features/workbench/WorkbenchView.test.tsx
- [x] T024 [US1] Implement `/captures`, `/tasks`, and ordered `/workbench` projection using the Task service in src-tauri/src/native/workbench.rs and src-tauri/src/native/mod.rs
- [x] T025 [US1] Implement React Workbench input, active shortcuts, refining shortcuts, and canonical category cards in frontend/src/features/workbench/WorkbenchView.tsx and frontend/src/features/workbench/task-workbench.css
- [x] T026 [US1] Disconnect legacy `#board`, `loadBoard`, initialization, next-stage, and board click ownership while preserving Search/Settings/queue runtime in frontend/public/runtime/foundation.js, frontend/public/runtime/workbench.js, frontend/public/runtime/explore.js, and frontend/public/runtime/conflicts.js
- [x] T027 [US1] Add English/Korean Workbench labels, accessible names, empty/loading/error copy and status text in frontend/public/i18n/en.json and frontend/public/i18n/ko.json
- [x] T028 [US1] Add restart, save-failure, shortcut-order, canonical-identity, pointer and keyboard packaged scenarios in frontend/src/test/desktopScenario.ts

**Checkpoint**: The new Workbench entry/resume surface is independently useful.

---

## Phase 5: User Story 2 - Work and Resume Independently (Priority: P1)

**Goal**: Full Problem-free Task lifecycle and preserved Work Log.

**Independent Test**: Add every evidence type before start, transition, complete, restart and reopen.

- [x] T029 [P] [US2] Write failing service/repository tests for pre-start Work Log, comments, checklist, attachment bytes, decisions, transitions, completion and reopen in src-tauri/src/application/task_service.rs
- [x] T030 [P] [US2] Write failing Task detail interaction tests for overview, Work Log, state action, completed evidence and follow-up behavior in frontend/src/features/workbench/TaskDetail.test.tsx
- [x] T031 [US2] Implement Task Work Log/comment/checklist/attachment/decision/completion repository operations and native routes in src-tauri/src/adapters/sqlite/task_repository.rs and src-tauri/src/native/mod.rs
- [x] T032 [US2] Implement Task detail overview, Work Log, checklist, decisions and explicit transition/reopen/follow-up actions in frontend/src/features/workbench/TaskDetail.tsx
- [x] T033 [US2] Add byte/hash/restart/completed-read-only Problem-free lifecycle packaged scenarios in frontend/src/test/desktopScenario.ts

**Checkpoint**: Task is a complete independent execution and evidence unit.

---

## Phase 6: User Story 3 - Refine Without Blocking Work (Priority: P2)

**Goal**: Resumable Capture/Task conversation and exact, selective proposal application.

**Independent Test**: Interrupt both session types, resume exact UI state, and apply mixed decisions.

- [x] T034 [P] [US3] Write failing refinement persistence tests for subject XOR, append-only messages/drafts, 500 ms workspace save, flush, mixed decisions and stale draft in src-tauri/src/native/refinement.rs
- [x] T035 [P] [US3] Write failing React tests for autosave/retry/resume and independent apply/edit/reject proposal controls in frontend/src/features/workbench/RefinementPanel.test.tsx
- [x] T036 [US3] Implement Capture/Task refinement session, workspace, messages, drafts and exact proposal decision routes in src-tauri/src/native/refinement.rs and src-tauri/src/native/mod.rs
- [x] T037 [US3] Implement atomic proposal application through Task service for task_patch/new_task/problem_snapshot/task_problem_link in src-tauri/src/application/task_service.rs
- [x] T038 [US3] Implement resumable Refinement panel and local failed-save retention in frontend/src/features/workbench/RefinementPanel.tsx
- [x] T039 [US3] Add packaged A→B→A close/reopen workspace restoration in frontend/src/test/desktopScenario.ts; cover proposal controls in RefinementPanel.test.tsx and atomic proposal decisions in src-tauri/src/native/task_assistance.rs tests

**Checkpoint**: Refinement improves either record without controlling Task lifecycle.

---

## Phase 7: User Story 4 - Connect Tasks and Problems (Priority: P2)

**Goal**: Exact Problem links, Task graph relationships, and field-level readiness.

**Independent Test**: Exercise many-to-many revisions, all relationship rules, navigation and readiness.

- [x] T040 [P] [US4] Write failing Task service tests for exact Problem revision links, revision advance, inverse relationships, duplicate/self/cycle rejection and readiness evidence in src-tauri/src/application/task_service.rs
- [x] T041 [P] [US4] Write failing Task detail tests for Problems, Relationships, readiness status/reasons and no-score behavior in frontend/src/features/workbench/TaskDetail.test.tsx
- [x] T042 [US4] Implement Problem revision/link, relationship graph and deterministic readiness repository queries in src-tauri/src/adapters/sqlite/task_repository.rs
- [x] T043 [US4] Implement link/relationship/readiness routes and revision-bound decisions in src-tauri/src/native/mod.rs
- [x] T044 [US4] Implement Problems, Relationships and readiness panels with bidirectional navigation in frontend/src/features/workbench/TaskDetail.tsx
- [x] T045 [US4] Add exact Problem revision, prerequisite, and unresolved-readiness continue packaged coverage in frontend/src/test/desktopScenario.ts; cover stale revisions and relationship cycles in application_commands.rs

**Checkpoint**: Optional context and decomposition remain useful without a parent gate.

---

## Phase 8: User Story 5 - Review Conflicts Asynchronously (Priority: P2)

**Goal**: Nonblocking exact review for Capture draft or Task revision with truthful failure/currentness.

**Independent Test**: Mutate both subject kinds during delayed/faulted jobs and verify identity,
citations, last-current preservation, latency and continued Task actions.

- [x] T046 [P] [US5] Write failing identity/currentness tests for material/non-material changes, exact Vault/grant revisions and eight statuses in src-tauri/src/native/jobs.rs
- [x] T047 [P] [US5] Write failing orchestration tests for 800 ms debounce, per-subject cancellation, global concurrency, late results, previous-current preservation and false-clear prevention in src-tauri/src/native/jobs.rs
- [x] T048 [P] [US5] Write failing review UI tests for Capture draft, Task revision, attempt/current separation, citation navigation and nonblocking warnings in frontend/src/features/workbench/ConflictReviewPanel.test.tsx
- [x] T049 [US5] Implement tagged review identity, run/finding/decision persistence and currentness calculation in src-tauri/src/native/jobs.rs and src-tauri/src/native/job_results.rs
- [x] T050 [US5] Implement debounce, cancellation, concurrency and exact result application through existing provider/vault adapters in src-tauri/src/native/jobs.rs
- [x] T051 [US5] Implement Capture/Task review routes and immutable finding decision routes in src-tauri/src/native/mod.rs
- [x] T052 [US5] Implement review status, attempts, citations, warnings and continue decision UI in frontend/src/features/workbench/ConflictReviewPanel.tsx
- [x] T053 [US5] Add delayed/cancelled/stale/cited-findings packaged coverage and latency capture in frontend/src/test/desktopScenario.ts; cover failed display in ConflictReviewPanel.test.tsx and insufficient/malformed/clear validation in task_assistance.rs tests

**Checkpoint**: Review is accurate, useful before Task persistence, and never a gate.

---

## Phase 9: User Story 7 - Complete, Resolve, and Publish Deliberately (Priority: P3)

**Goal**: Separate decisions and graph lineage through exact Knowledge publication.

**Independent Test**: Complete a Task, leave Problem open, resolve separately, publish an exact draft,
and exercise external-change protection and graph navigation.

- [x] T054 [P] [US7] Write failing domain tests for completion/Problem-resolution/publication independence and exact draft/hash approval in src-tauri/src/domain/knowledge_publication.rs
- [x] T055 [P] [US7] Write failing lineage graph tests for all node/edge types, regeneration, corrections and legacy provenance in src-tauri/src/native/lineage.rs
- [x] T056 [US7] Replace feature/problem completion coupling with Task completion and separate Problem resolution service/routes in src-tauri/src/native/completion.rs and src-tauri/src/native/mod.rs
- [x] T057 [US7] Implement Task graph lineage snapshots and evidence navigation in src-tauri/src/native/lineage.rs
- [x] T058 [US7] Implement exact Task Knowledge draft/publish/regenerate/withdraw with reversible external-hash guards in src-tauri/src/domain/knowledge_publication.rs and src-tauri/src/native/patches.rs
- [x] T059 [US7] Implement completion, Problem contribution/resolution, lineage and publication panels in frontend/src/features/workbench/TaskDetail.tsx
- [x] T060 [US7] Add three-decision, external-change, regenerate/withdraw and complete-lineage packaged scenarios in frontend/src/test/desktopScenario.ts
- [x] T078 [US7] Benchmark capture/direct-Task persistence and warm Workbench p95 against a representative 1,000-item Task, Problem and Work Log fixture in src-tauri/tests/application_commands.rs

**Checkpoint**: Private completion becomes portable Knowledge only through deliberate publication.

---

## Phase 10: User Story 8 - Continue the Same Work Through MCP (Priority: P3)

**Goal**: Desktop and chat share Task identity, revisions, scope and human approval.

**Independent Test**: Cross-create/revise, reject stale approval, replay projections, and compare views.

- [x] T061 [P] [US8] Write failing MCP schema/resource/elicitation tests for Task events, snapshots, stale approval, scope and publication in src-tauri/src/mcp.rs
- [x] T062 [P] [US8] Write failing projection replay/idempotency/recency tests for new Task events in src-tauri/src/application/work_tracking_service.rs and src-tauri/src/native/work_tracking_projector.rs
- [x] T063 [US8] Replace Problem/Solution event and projection schemas with Task event contract in src-tauri/src/domain/work_tracking_state.rs and src-tauri/src/native/work_tracking_schema.sql
- [x] T064 [US8] Route MCP Task proposals and exact reviews through Task service in src-tauri/src/mcp.rs and src-tauri/src/mcp_ipc.rs
- [x] T065 [US8] Update workbench resources, connection descriptions, scopes and workflow skill contract without revoking existing grants in src-tauri/src/native/work_tracking.rs and frontend/src/features/chat/WorkflowSkillContract.test.ts
- [x] T066 [US8] Update connected-chat cards and tracking UI from Solution gates to Task decisions in frontend/src/features/chat/WorkTrackingCards.tsx and frontend/src/features/chat/ChatTrackingSurface.tsx
- [x] T067 [US8] Add desktop/chat identity, stale approval, scope denial, replay and background-recency packaged scenarios in frontend/src/test/desktopScenario.ts

**Checkpoint**: Both surfaces observe and mutate one exact Task truth.

---

## Phase 11: Polish, Documentation, and Final Validation

**Purpose**: Finish current documentation, cross-platform quality and one-artifact acceptance.

- [x] T068 [P] Update README.md, README.ko.md, docs/product-spirit.md, docs/product-spirit.ko.md and docs/features indexes for the Task-centered flow
- [x] T069 [P] Update affected bilingual Workbench, conflict, completion, lineage and MCP guides and link specs/012-task-centered-workbench/spec.md under docs/features/
- [x] T070 Reconcile tests/CHARACTERIZATION.md against FR-001–FR-040 and every acceptance scenario, recording test or fixed-rubric evidence
- [x] T071 Run `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run typecheck`, and `git diff --check` as separate final source checks
- [x] T072 Build the release Tauri application once and record the artifact identity using package scripts in package.json
- [x] T073 Run packaged desktop E2E against that exact artifact using scripts/run_desktop_e2e.mjs
- [ ] T074 Complete wide/narrow English/Korean pointer/keyboard/reduced-motion visual QA against the same artifact using scripts/run_ui_review.mjs (final EN/KO wide/narrow manual review passed; actual OS reduced-motion execution remains external)
- [x] T075 Inspect configured real-provider availability without exposing secrets, then run the fixed 12-document/24-conflict corpus against the same app artifact or record the report as blocked in specs/012-task-centered-workbench/acceptance-verification.md (provider readiness false; blocked report recorded)
- [x] T076 Update specs/012-task-centered-workbench/quickstart.md and all current contracts if final observed behavior differs, then rerun only affected checks
- [x] T077 Review docs/DOCUMENTATION_GUIDE.md and record final documentation coverage and remaining operational facts in specs/012-task-centered-workbench/acceptance-verification.md

---

## Phase 11: Shipping Interactive Coverage Expansion

**Purpose**: Prove every shipping interactive control family through observable behavior in one final packaged artifact. Keep the architecture refactor in `docs/backlog/task-centered-architecture.md` deferred.

- [x] T079 Define the executable F01–F68 interaction manifest with stable selectors, native-operation expectations, effect assertions, and explicit reachability in `frontend/src/test/interactionCoverageManifest.ts` using `docs/testing/interactive-coverage.md`
- [x] T080 Add deterministic packaged fixtures for native failure, retry, stale revision, busy/disabled, migration recovery, first-run setup, notifications, queue states, and provider terminal states in `src-tauri/src/desktop_e2e.rs` and `tests/fakes/openai_server.mjs`
- [x] T081 Add shell, locale, Search, Compass, provider-settings, and MCP-settings pointer/keyboard scenarios for F01–F02 and F36–F43 in `frontend/src/test/desktopScenario.ts`
- [x] T082 Add Task Workbench happy/failure/retry/disabled/stale/keyboard scenarios for F03–F30 in `frontend/src/test/desktopScenario.ts`
- [x] T083 Add refinement, chat tracking, proposal decision, provider-absence, polling terminal, close-during-pending isolation, close-save-failure, and relaunch scenarios for F31–F35 and F44–F49 in `frontend/src/test/desktopScenario.ts`
  - Final `FcAbm6` packaged run passed all 32 scenarios, including provider recovery, close-save retry, late-result isolation, and a true two-process relaunch. No native refinement-job cancellation endpoint is claimed.
- [x] T084 Add notice, transition, queue, notification, onboarding, Vault setup, and migration recovery scenarios for F50–F59 in `frontend/src/test/desktopScenario.ts`
- [x] T085 Audit runtime reachability and add packaged behavior or explicit unreachable-trigger assertions for legacy dynamic F60–F68 in `frontend/src/test/desktopScenario.ts` and `frontend/src/test/remainingButtonsRuntime.test.ts`
- [x] T086 Implement only the minimum behavioral fixes revealed by T081–T085, including the agreed Task-UI ownership for Problem refinement, in the owning files under `frontend/src/`, `frontend/public/runtime/`, and `src-tauri/src/` without starting the deferred architecture refactor
- [x] T087 Add manifest completeness validation that fails when a shipping button, form, input, select, disclosure, dialog action, delegated `data-*` action, or keyboard handler lacks an F01–F68 scenario/effect mapping in `frontend/src/test/interactionCoverage.test.ts` and `frontend/src/test/productionInteractiveSources.test.ts`
- [ ] T088 Build once, record the executable path/code hash/signing time, run the complete interaction manifest plus wide/narrow English/Korean reduced-motion review against that exact artifact, and record per-family pass/fail/block evidence in `specs/012-task-centered-workbench/acceptance-verification.md`

- [x] T089 Implement six-field Task draft merge, expected-revision conflict comparison, and guarded detail close/Task switch in `frontend/src/features/workbench/TaskDetail.tsx` with focused tests.
- [x] T090 Implement responsive shortcut/title-input fixes, selected-locale completion/privacy/readiness copy, nonmodal focus/Escape/IME behavior, and Queue Open AI setup recovery in their owning frontend/runtime modules with focused tests.
- [x] T091 Record final signed-package geometry and complete packaged E2E results against one artifact. Native quit/crash draft durability remains out of scope.
- [ ] T092 Record manual real Korean IME, VoiceOver, and OS reduced-motion results; automated package success does not replace these checks.

- T088 status: signed artifact and full functional/coverage gates passed; final EN/KO wide/narrow manual review passed. Only actual OS reduced-motion execution remains unverified. See the final acceptance record.

**Independent validation**: Every F01–F68 family has a manifest entry and an observable-effect result. Passing component/native tests or dispatching a click without state/readback assertions does not satisfy packaged coverage. The launched executable identity must equal the recorded final artifact.

---

## Dependencies & Execution Order

- Phase 1 freezes the executable contract. Phase 2 provides the shared aggregate.
- US6 migration must finish before new writers open an existing database.
- US1 and US2 may proceed after Phase 2 against fresh schema-8 fixtures while US6 finishes, but
  integration with upgraded data waits for US6.
- US3 and US4 depend on the Task service; they may run in parallel on different modules.
- US5 depends on refinement draft identity and Task revisions; US7 depends on Task completion.
- US8 depends on stable Task, decision and Knowledge contracts.
- Final validation depends on all selected stories and current documentation.

## Parallel Execution Examples

- Backend schema/migration: T014–T021; frontend Workbench: T023/T025–T028; provider fixtures: T003.
- After Task detail shell exists, refinement T034–T039 and relationships T040–T045 use separate files.
- MCP T061–T067 starts after aggregate semantics stabilize while frontend review/publication UI finishes.
- Harness tasks T073–T075 use the final artifact only and must not trigger a rebuild.

## Implementation Strategy

The minimum viable release includes Setup/Foundation, US6, US1 and US2: preserved upgrade, explicit
Capture/Task input, canonical Workbench, and independent Task lifecycle with Work Log. The full
authorized scope continues through US3–US5, US7–US8 and final validation. Do not release a partial
migration or retain legacy write gates as an interim compatibility path.

## Format Validation

Tasks T001–T092 use checkbox, sequential ID, appropriate `[P]` and `[US#]` markers, and concrete
file paths. Story tasks are independently testable and required tests precede implementation.

## Follow-up: Task journey visual language and exact Knowledge snapshot

- [x] T093 [US7] Add focused component tests for gap text, relationship details, Korean labels,
  invalid dates, and unique marker IDs; add native tests for bounded labels and grounded
  later-to-earlier semantic links. Record wide/narrow routing and key-node hierarchy through manual
  browser review rather than claiming component geometry assertions in
  `frontend/src/features/workbench/TaskJourneyGraph.test.tsx` and `src-tauri/src/native/jobs.rs`.
- [x] T094 [US7] Implement the journey graph visual hierarchy and inspectable relationship evidence
  in `frontend/src/features/workbench/TaskJourneyGraph.tsx` and `task-journey-graph.css`.
- [x] T095 [US7] Persist and reuse locale-bound journey projections with fallback plus deleted-Task,
  stale-source, and concurrent cache finalization guards in `src-tauri/src/native/jobs.rs`.
- [x] T096 [US7] Ensure app-generated Knowledge preparation embeds the exact current journey under
  `lineage.journey`, rechecks source/completion/journey currentness, keeps supplied-body drafts
  provider-free, and renders draft Review separately from current Details in `task_assistance.rs`
  and `TaskDetail.tsx`.
- [x] T097 [P] [US7] Align the dedicated bilingual lineage guide, spec 010 historical scope, and
  current 012 spec/design/validation records with the implemented behavior and limitations in
  `docs/features/lineage-knowledge-layer.md`, `docs/features/lineage-knowledge-layer.ko.md`, and
  `specs/010-lineage-knowledge-layer/spec.md`.
- [ ] T098 [US7] Validate real-provider locale-specific English and Korean titles and relationship
  rationale in the UI and record it in
  `specs/012-task-centered-workbench/acceptance-verification.md`; existing captures cover localized
  chrome/dates and mixed long fixture titles.
