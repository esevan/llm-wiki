# Tasks: Task Codex Execution

**Input**: Design documents from `specs/014-task-codex-execution/`

**Tests**: Required by the feature brief. Deterministic and actual-provider evidence remain separate.

## Phase 1: Setup

- [X] T001 Record the installed app-server methods, capabilities, event/request fixtures, and safe error mapping used by the adapter in `src-tauri/tests/fixtures/codex_app_server/`
- [X] T002 [P] Add shared Run, completed-item, formal-request, effective-config, and Work Log execution projection types in `frontend/src/types/taskWorkbench.ts`
- [X] T003 [P] Add bilingual execution, status, evidence, approval, question, recovery, and error copy in `frontend/src/features/workbench/taskWorkbenchText.ts`

---

## Phase 2: Foundational

- [X] T004 Add additive schema 15 for thread bindings, Runs, completed items, formal requests, and Work Log links in `src-tauri/src/native/task_execution_schema.sql` and register it after the released v13 journey cache and v14 sessions in `src-tauri/src/native/migrations.rs`
- [X] T005 [P] Add populated main-v13→v15 journey/manual-data preservation plus v14→v15 session, attachment, manual Work Log, identity, rollback, and foreign-key assertions in `src-tauri/src/native/migrations.rs`, `src-tauri/tests/work_tracking_migrations.rs`, and `src-tauri/tests/task_tracking_v9_migration.rs`
- [X] T006 Implement exact Task-session-Run ownership, submission replay/conflict, active-Run uniqueness, atomic instruction/Run/Work Log/link creation, reads, and projection repair in `src-tauri/src/application/task_execution_service.rs`
- [X] T007 Add Task-scoped Run/formal-response/interrupt/sync routing with URL identity precedence in `frontend/src/services/taskOperation.ts`, `frontend/src/services/taskOperation.test.ts`, and `frontend/src/services/taskClient.ts`
- [X] T008 Implement the supervised single-process JSON-RPC transport, initialization/capability negotiation, serialized writes, single reader, connection generation, executable discovery, redaction, and shutdown in `src-tauri/src/codex_app_server.rs`
- [X] T009 Manage the execution service/runtime and expose start, subscribe, formal-response, and interrupt native commands in `src-tauri/src/native/mod.rs`, `src-tauri/src/lib.rs`, and `frontend/src/services/taskExecutionClient.ts`

**Checkpoint**: Migration, durable operations, and the isolated app-server boundary are ready; no user flow is claimed yet.

---

## Phase 3: User Story 1 - Run Codex and create canonical Work Log evidence (Priority: P1) 🎯 MVP

**Goal**: One explicit instruction performs one real Codex turn, saves its result, and appears once in the existing Task Work Log without completing the Task.

**Independent Test**: Execute one harmless instruction in an isolated folder and observe one Run, one turn, one saved report/evidence set, and one linked Work Log item.

### Tests for User Story 1

- [X] T010 [P] [US1] Add failing application tests for atomic start, same-key replay/different-payload conflict, invalid cwd, missing executable/auth/start failures, exactly one Work Log link, manual-log preservation, and unchanged Task state in `src-tauri/tests/task_execution.rs`
- [X] T011 [P] [US1] Add failing protocol tests for thread start, turn start, completed-item accumulation including identical deltas, redaction, final report capture, and terminal error mapping in `src-tauri/src/codex_app_server.rs`
- [X] T012 [P] [US1] Add failing component tests for separate Save note/Run actions, retained failed draft, real progress states, effective config, report/evidence attribution, unsupported attachments, and Work Log link-back in `frontend/src/features/workbench/TaskWorkSessions.test.tsx` and `frontend/src/features/workbench/TaskDetail.test.tsx`

### Implementation for User Story 1

- [X] T013 [US1] Build the bounded first-turn Task context and same-turn final-report instruction without full Knowledge/history replay in `src-tauri/src/application/task_execution_service.rs`
- [X] T014 [US1] Dispatch thread/start and turn/start only after durable dispatch recording; bind early events and persist redacted completed items/final status in `src-tauri/src/codex_app_server.rs`
- [X] T015 [US1] Join Run-owned status, attributed report, observed evidence, artifacts, limitations, and sync state into existing Work Log reads without mutating manual fields in `src-tauri/src/application/task_service.rs`
- [X] T016 [US1] Implement Run composer, effective settings, safe live metadata, completed output, final report/evidence, retry traceability, and supported-input boundary in `frontend/src/features/workbench/TaskWorkSessions.tsx`
- [X] T017 [US1] Add Work Log execution block and exact session/Run link-back focus handoff in `frontend/src/features/workbench/TaskDetail.tsx`
- [X] T018 [US1] Add execution layout, long-output containment, status/evidence attribution, visible focus, and 640 px behavior in `frontend/src/features/workbench/task-workbench.css`

**Checkpoint**: One real first turn can be executed, persisted, reopened, and read through the canonical Work Log.

---

## Phase 4: User Story 2 - Continue and control the exact Codex thread (Priority: P1)

**Goal**: Follow-up turns reuse the exact bound thread, while formal approvals/questions and Stop remain correctly scoped to the active Run.

**Independent Test**: Execute two turns in one session, respond to actual multi-question and approval requests, detach/reconnect, and prove Task B cannot control the Run.

### Tests for User Story 2

- [X] T019 [P] [US2] Add failing protocol tests for exact thread resume, no full-history replay, Task context delta, command/file/permission decisions, denial continuation, Stop/completion race, and stale generation rejection in `src-tauri/src/codex_app_server.rs`
- [X] T020 [P] [US2] Add failing service tests for one active Run, wrong Task/session/thread/turn ownership, duplicate formal responses, nonsecret answer retention, and retry-as-new-Run in `src-tauri/tests/task_execution.rs`
- [X] T021 [P] [US2] Add failing UI tests for actual approval choices, per-question option selection plus one Submit, free-text-only and optional-other input, blocking/nonblocking behavior, submitting/answered/stale/error states, failed-answer retention, keyboard/focus, and secret-request fallback in `frontend/src/features/workbench/TaskWorkSessions.test.tsx`

### Implementation for User Story 2

- [X] T022 [US2] Resume only the session's exact thread and send new instruction plus bounded changed-Task delta in `src-tauri/src/codex_app_server.rs` and `src-tauri/src/application/task_execution_service.rs`
- [X] T023 [US2] Persist and route generation-bound formal approvals and experimental user-input requests with request-specific responses and secret exclusion in `src-tauri/src/application/task_execution_service.rs` and `src-tauri/src/codex_app_server.rs`
- [X] T024 [US2] Implement exact-turn interrupt acknowledgement and terminal-event race handling in `src-tauri/src/codex_app_server.rs`
- [X] T025 [US2] Render provider-derived approval buttons and multi-question/free-text structured response forms with one explicit Submit and retained failure state in `frontend/src/features/workbench/TaskWorkSessions.tsx`
- [X] T026 [US2] Preserve active Run subscription across Task/session UI detachment and restore exact selection without cancellation in `frontend/src/features/workbench/TaskDetail.tsx` and `frontend/src/features/workbench/TaskWorkSessions.tsx`

**Checkpoint**: Same-session continuity, human authority, Stop, and formal request interaction are independently demonstrable.

---

## Phase 5: User Story 3 - Recover and audit safely (Priority: P2)

**Goal**: Reconnect and replay never fabricate success or duplicate work, and uncertain states remain actionable without model rerun.

**Independent Test**: Break transport at dispatch and during a turn, replay completed events, restart, and repair Work Log projection while preserving all earlier records.

### Tests for User Story 3

- [X] T027 [P] [US3] Add failing recovery tests for dispatch gap, process loss, early events, completed-item replay, late prior-Run events, stale formal requests, and no automatic resend in `src-tauri/tests/task_execution.rs`
- [X] T028 [P] [US3] Add failing Work Log tests for projection failure/status, log-only retry, missing final report on failure/cancel, preserved comments/manual body, and reopened link identity in `src-tauri/tests/task_execution.rs`
- [X] T029 [P] [US3] Add failing UI tests for interrupted/needs-attention, missing report, sync failure/retry, revisioned snapshot ordering, and reconnect within one second of local data in `frontend/src/features/workbench/TaskWorkSessions.test.tsx`

### Implementation for User Story 3

- [X] T030 [US3] Recover nonterminal Runs on startup, resume only exact threads, and classify unprovable outcomes without resend in `src-tauri/src/application/task_execution_service.rs` and `src-tauri/src/codex_app_server.rs`
- [X] T031 [US3] Implement idempotent completed-item upserts, delayed-event rejection, revisioned snapshots, and projection-only Work Log repair in `src-tauri/src/application/task_execution_service.rs`
- [X] T032 [US3] Render recovery, missing-report, stale-request, and Work Log sync actions without losing saved output or drafts in `frontend/src/features/workbench/TaskWorkSessions.tsx`

**Checkpoint**: Restart and partial-failure states are truthful, durable, isolated, and repairable without duplicate execution.

---

## Phase 6: Documentation and Verification

- [X] T033 [P] Update English/Korean feature documentation and indexes, replacing inert MVP execution copy while preserving feature 013 history, in `docs/features/task-work-sessions.md`, `docs/features/task-work-sessions.ko.md`, `docs/features/README.md`, and `docs/features/README.ko.md`
- [X] T034 [P] Extend interaction coverage identities and controlled packaged fixture/scenario for only `task-codex-execution` in `frontend/src/test/interactionCoverageManifest.ts`, `frontend/src/test/productionInteractiveSources.ts`, `frontend/src/test/workSessionScenarios.ts`, `frontend/src/test/desktopScenario.ts`, `src-tauri/src/desktop_e2e.rs`, and `scripts/desktop_e2e_helpers.mjs`
- [X] T035 Run focused `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, the 100 ms p95 projection check, and `git diff --check`; record results in `specs/014-task-codex-execution/quickstart.md`
- [X] T036 Run the isolated opt-in real Codex test through two turns on one exact thread and canonical Work Log projection; record actual-provider evidence separately in `specs/014-task-codex-execution/quickstart.md`
- [X] T037 Perform EN/KO normal/640 px CUA review of all execution and formal-request states and resolve or record findings in `specs/014-task-codex-execution/quickstart.md`
- [X] T038 Build the final release application once and use Luna to run only `npm run test:desktop -- --scenario task-codex-execution`; record fixture-vs-live evidence and residual macOS/Windows limits in `specs/014-task-codex-execution/quickstart.md`
- [X] T039 Review `docs/DOCUMENTATION_GUIDE.md`, update `docs/CONTINUATION.md`, and verify final implementation/spec/plan/task consistency in `docs/CONTINUATION.md` and `specs/014-task-codex-execution/`

---

## Dependencies & Execution Order

- Setup → Foundational → US1 → US2 → US3 → final verification.
- T004 blocks persistence work; T006 and T008 block Run dispatch.
- Test tasks in each story precede that story's implementation.
- US1 is the minimum valuable slice but is not release-complete until US2 authority controls and US3 recovery pass.
- T040 is the Astra High design gate before substantive UI work (T016–T018, T025–T026, T032). T037 is separate rendered verification using the stable release artifact; T038 builds that artifact once and runs the selected packaged scenario; the same artifact is used for T037 without concurrent UI control.

## Parallel Opportunities

- T002 and T003 can proceed while schema fixtures are prepared.
- Migration tests T005 can be written alongside transport T008.
- Within each story, native protocol, application, and UI failing tests marked `[P]` touch separate primary files.
- Documentation T033 and packaged-fixture preparation T034 can proceed after behavior contracts stabilize.

## Implementation Strategy

1. Establish additive durable state and the single supervised app-server boundary.
2. Complete the one-turn execution → result → existing Work Log slice.
3. Add exact-thread follow-up, formal responses, and interrupt races.
4. Add interruption/replay/recovery and projection repair.
5. Run deterministic checks, one real two-turn isolated Codex acceptance, rendered UI verification, then one Luna packaged scenario. The Astra High design gate T040 precedes substantive UI implementation.

## Format Validation

All implementation checklist lines use `- [ ] T### [P?] [US?] Description with file path`; setup/foundational/final tasks omit story labels and story phases include them.

## Clarification: UI/UX Design Gate

- [x] T040 Review the UI/UX design with Astra High before substantive UI implementation; apply findings to `specs/014-task-codex-execution/contracts/interaction.md`, related requirements, and implementation tasks. This is a design review, not a final rendered-review model requirement.

Design findings from T040 are mandatory details of T016/T018/T021/T025/T026/T029/T032: explicit preparation and settings revision, historical Run access, full-identity answer drafts, request-first hierarchy, safe retry draft handling, and bidirectional tab/focus links. T037 rendered evidence and unavailable cases are recorded separately in quickstart.md; this does not imply all platforms or transient states were visually inspected.

## Implementation evidence mapping

- Protocol fixtures are inline Rust tests and `tests/fakes/codex_app_server.mjs`; application/recovery tests live beside their services and in `src-tauri/src/native/task_execution_runtime.rs`, rather than the initially proposed `src-tauri/tests/task_execution.rs`. These are organizational path changes, not omitted feature surfaces.
- Completed implementation checkboxes do not imply T037 rendered verification or T038 packaged acceptance. The named native and component boundary-case matrices were completed in the final focused test pass.
- T018 includes responsive CSS; native 640 px rendering is unavailable under the existing 900 px minimum window width and is recorded as a T037 limitation.

## Final verification qualification

T037 records representative real EN/KO macOS CUA review and explicit unavailable cases: the existing 900 px native minimum prevents a 640 px window; not every transient state or Windows screen was visually inspected. T038 passed only the selected packaged scenario on the final signed artifact. Actual Codex two-turn evidence is separate from its controlled fixture. See `quickstart.md` for final results and residual limits. No commit, merge, or installed-app replacement was performed.
