# Tasks: Automatic Capture Distillation

**Input**: Design documents from `specs/016-capture-auto-distillation/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`

**Tests**: Focused tests are required by FR-025. Write each listed test before its implementation and verify it fails for the intended missing behavior.

## Phase 1: Setup and Foundation Integration

**Purpose**: Land and map the shared workflow foundation before Capture domain wiring.

- [x] T001 Merge the completed workflow-foundation commit into this branch and reconcile `specs/016-capture-auto-distillation/contracts/application-api.md` with actual symbols in `src-tauri/src/workflow_foundation.rs` and `src-tauri/src/native/workflow_foundation.rs`
- [x] T002 Register/verify the versioned `capture_distillation` prompt definition and operation policy in the merged workflow-foundation registry files without changing its public contract
- [x] T003 [P] Add localized Capture distillation/status/retry/proposal strings in `frontend/src/features/workbench/taskWorkbenchText.ts`

---

## Phase 2: Foundational Capture Contracts

**Purpose**: Establish exact Capture currentness, source preservation, projection, and reusable motion before story integration.

**⚠️ CRITICAL**: No Capture job/result wiring begins before T001.

- [x] T004 Add failing migration tests for old-row baseline snapshots, no backfill eligibility, initial raw text and owned image-byte provenance, logical-operation/currentness defaults, derived/proposal foreign keys, and rollback in `src-tauri/src/native/migrations.rs`
- [x] T005 Implement immutable Capture source snapshots with owned image bytes, logical-operation/currentness, initial-distillation eligibility, derived distillation, and proposal schema migration in `src-tauri/src/native/migrations.rs` (and a focused schema file under `src-tauri/src/native/` if consistent with merged foundation)
- [ ] T006 [P] Add failing type/projection tests for `CaptureDistillationSummary`, foundation lifecycle fields, source/display separation, and superseded-job proposal in `frontend/src/types/taskWorkbench.ts` and `frontend/src/services/taskClient.test.ts`
- [x] T007 Extend Capture types and application-client mappings for one shared source/display/distillation/proposal projection in `frontend/src/types/taskWorkbench.ts`, `frontend/src/services/taskClient.ts`, and `frontend/src/services/tauriApplicationClient.ts`
- [x] T008 [P] Add failing shared transition tests for eligible revision replacement, stationary title layers, semantic causes, no mount/rerender/navigation/historical/locale replay, focus/order preservation, and reduced motion in `frontend/src/components/ContentRevisionTransition.test.tsx`
- [x] T009 Implement reusable revision-aware title/body transition semantics in `frontend/src/components/ContentRevisionTransition.tsx` and shared styles in `frontend/src/theme/refresh.css`

**Checkpoint**: Foundation contract is merged; Capture can represent exact source/currentness and the shared transition is independently usable.

---

## Phase 3: User Story 1 - Capture First, Organize Automatically (Priority: P1) 🎯 MVP

**Goal**: Persist text/image source immediately, submit one automatic operation for new Captures, and apply one grounded exact-head result without moving the Capture from Inbox.

**Independent Test**: Delay provider work; save text-only, mixed, and image-only Captures; restart; then complete grounded results and verify exact source, one logical operation, same Capture identity/Inbox position, and no old-Capture backfill.

### Tests for User Story 1

- [ ] T010 [P] [US1] Add failing Rust tests for immediate source commit, image-only placeholder/source, new-only eligibility, stable activity/order, operation-id replay, one exact-source submission, and no new-Capture legacy synthetic image refinement in `src-tauri/src/application/task_service.rs` and `src-tauri/src/native/task_assistance.rs`
- [x] T011 [P] [US1] Add failing handler/result tests for structured limits, source references, empty optional fields, malformed/invented result rejection, and exact-head atomic apply in `src-tauri/src/native/capture_distillation.rs`
- [x] T012 [P] [US1] Add failing Workbench component tests for immediate placeholder/source metadata and same-key/same-lane applied display in `frontend/src/features/workbench/WorkbenchView.test.tsx`

### Implementation for User Story 1

- [x] T013 [US1] Extend the Capture create transaction to assign canonical source/current identity and submit/coalesce one `capture_distillation` operation after raw text/images are safe in `src-tauri/src/application/task_service.rs` and `src-tauri/src/native/mod.rs`
- [x] T014 [US1] Implement exact source snapshot construction, bounded multimodal provider request, structured result validation, and grounding attribution in `src-tauri/src/native/capture_distillation.rs`
- [x] T015 [US1] Publish exact-head output and provenance atomically with foundation disposition `applied`, leaving raw source/activity/Inbox state unchanged in `src-tauri/src/native/capture_distillation.rs` and `src-tauri/src/native/job_results.rs`
- [x] T016 [US1] Project source/display/current job state efficiently from `TaskService::workbench` and Capture/refinement context reads in `src-tauri/src/application/task_service.rs` and the existing refinement context module
- [x] T017 [US1] Render localized image placeholder, derived title/body, quiet processing/success state, and stable Capture key/Inbox placement through `ContentRevisionTransition` in `frontend/src/features/workbench/WorkbenchView.tsx` and `frontend/src/features/workbench/task-workbench.css`

**Checkpoint**: New Captures persist immediately and receive one safe automatic current result; old Captures do not backfill.

---

## Phase 4: User Story 2 - Continue While Capture Improves (Priority: P2)

**Goal**: Show the same live status/source in card and modal while preserving active input, focus, selection/composition, tab, and scroll.

**Independent Test**: Open a running Capture modal, type/select/scroll, then publish success and failure; verify both surfaces reconcile without navigation or interaction loss, and motion occurs only for eligible live replacement.

### Tests for User Story 2

- [ ] T018 [P] [US2] Add failing modal tests for immediate raw source, shared processing/success/failure projection, no enqueue on open, enabled input, focus/selection/input/tab/scroll preservation, and one-time announcements in `frontend/src/features/workbench/RefinementPanel.test.tsx`
- [ ] T019 [P] [US2] Add failing Workbench reconciliation tests for out-of-order/reconnect hints, card/modal agreement, eligible `automatic_apply` cause, and no hydration/navigation/same-revision animation in `frontend/src/features/workbench/WorkbenchView.test.tsx`
- [x] T020 [P] [US2] Add failing application-command read tests proving modal/context reads never create distillation or synthetic initial-image jobs for new eligible Captures and expose the same revision/status/result as Workbench in `src-tauri/tests/application_commands.rs`

### Implementation for User Story 2

- [x] T021 [US2] Extend Capture refinement context with raw source and the shared distillation projection, and bypass `prepare_image_capture` for new distillation-eligible image-only Captures without affecting old-row compatibility in `src-tauri/src/native/task_assistance.rs`
- [x] T022 [US2] Reconcile Capture job invalidation hints into one durable Workbench/modal snapshot without directly applying event payload content in `frontend/src/features/workbench/WorkbenchView.tsx`
- [x] T023 [US2] Render source, processing/success/failure/superseded-proposal details while preserving the existing editor and modal state machinery in `frontend/src/features/workbench/RefinementPanel.tsx`
- [x] T024 [US2] Add accessible status/retry/proposal styling, mask/body motion, long-content wrapping, narrow layout, and reduced-motion rules in `frontend/src/features/workbench/task-workbench.css`

**Checkpoint**: Card and modal agree, source is immediately inspectable, and background updates never interrupt conversation.

---

## Phase 5: User Story 3 - Keep User Changes and Recover Clearly (Priority: P3)

**Goal**: Reject late overwrite, retain useful conflicting output for review, expose durable failure, and recover/retry without duplication.

**Independent Test**: Delay work, advance Capture currentness with edit/refinement input, then finish the old result; verify superseded disposition plus optional proposal, visible restart-safe failure, explicit source-bound retry, and one active equivalent attempt.

### Tests for User Story 3

- [ ] T025 [P] [US3] Add failing transactional race tests for direct-edit proposal/no-overwrite, explanatory-chat latest-context successor coalescing within one logical operation, delete-before-complete supersession, no partial apply, activity stability, and retry coalescing in `src-tauri/src/native/capture_distillation.rs`
- [ ] T026 [P] [US3] Add failing restart/retry application-command tests for durable queued/running recovery, permanent/exhausted failure visibility, source-bound retry, and duplicate gesture coalescing in `src-tauri/tests/application_commands.rs`
- [x] T027 [P] [US3] Add failing UI tests for persistent failure, localized safe error, retry eligibility/action, superseded state, and optional proposal without current overwrite in `frontend/src/features/workbench/WorkbenchView.test.tsx` and `frontend/src/features/workbench/RefinementPanel.test.tsx`

### Implementation for User Story 3

- [x] T028 [US3] Append Capture source provenance and advance source/currentness through existing `item.update`, retain useful direct-edit mismatches as optional proposals, and advance explanatory-chat context by superseding/coalescing one latest-context successor under the same logical operation in `src-tauri/src/native/workflow.rs`, `src-tauri/src/native/task_assistance.rs`, and `src-tauri/src/native/capture_distillation.rs`
- [x] T029 [US3] Connect explicit retry to the existing jobs retry operation while preserving prompt/source revision and active-equivalent coalescing in `frontend/src/services/taskClient.ts`, `frontend/src/features/workbench/WorkbenchView.tsx`, and `frontend/src/features/workbench/RefinementPanel.tsx`
- [x] T030 [US3] Ensure recovery and result routing recognize `capture_distillation` and retain safe terminal state/destination across restart in `src-tauri/src/native/jobs.rs`, `src-tauri/src/native/job_results.rs`, and `src-tauri/src/native/mod.rs`

**Checkpoint**: User changes always win; useful conflicts remain optional; failure/recovery/retry are honest and deduplicated.

---

## Phase 6: Polish, Documentation, and Verification

- [x] T031 [P] Add English and Korean user guides in `docs/features/capture-auto-distillation.md` and `docs/features/capture-auto-distillation.ko.md`, then link them in `docs/features/README.md` and `docs/features/README.ko.md`
- [x] T032 [P] Update current observable Capture behavior in `docs/product-spirit.md` and `docs/product-spirit.ko.md` without claiming unsupported OCR, workflow transitions, or publication
- [ ] T033 Run focused Capture/foundation/migration/Workbench/modal/transition tests, then run `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check` as separate commands
- [x] T034 Measure representative Capture persistence and 1,000-item Workbench projection against 50 ms/100 ms p95 and 15% regression gates; record evidence in `specs/016-capture-auto-distillation/quickstart.md`
- [ ] T035 Perform rendered visual review of wide/narrow Korean/English, long/empty/error/proposal states, keyboard focus, active input/scroll, normal motion, and reduced motion; record artifacts/limitations in `specs/016-capture-auto-distillation/quickstart.md`
- [x] T036 Decide packaged E2E from remaining concrete risk; skip with rationale or build once and run only `npm run test:desktop -- --scenario capture-distillation-restart`, then record result in `specs/016-capture-auto-distillation/quickstart.md`
- [ ] T037 Review `docs/DOCUMENTATION_GUIDE.md`, align all paired documentation, and verify the final task branch has one compliant implementation commit relative to its post-foundation base

---

## Dependencies & Execution Order

### Phase dependencies

- Setup requires the foundation implementation commit and blocks all schema/queue wiring.
- Foundational Capture contracts follow setup and block all user stories.
- US1 supplies saved source, job, result, and card projection required by US2 and US3.
- US2 and US3 may proceed in parallel after US1 if they do not edit the same Workbench/modal files concurrently; otherwise execute US2 then US3.
- Documentation and final verification follow all selected stories.

### Parallel opportunities

- T003 can proceed independently after contract names settle.
- Migration tests/schema work and transition tests/component work use separate files (T004–T009).
- Each story's Rust, UI, and command test tasks marked `[P]` touch separate primary files or can be sequenced before overlapping implementation.
- English/Korean guides and product-spirit updates are separate from final test execution.

## Implementation Strategy

### MVP first

1. Complete foundation integration and Capture contracts.
2. Complete US1 and independently prove immediate persistence, one operation, exact apply, and no backfill.
3. Validate before adding modal synchronization and conflict recovery UI.

### Incremental delivery

1. US1: safe automatic organization for new Captures.
2. US2: uninterrupted card/modal experience and shared motion.
3. US3: conflict proposals, durable failure, recovery, and explicit retry.
4. Documentation, performance evidence, rendered review, and risk-based E2E decision.

## Format Validation

All 37 tasks use the required checkbox, sequential task ID, optional `[P]`, required story label inside user-story phases, concrete action, and explicit file path.
