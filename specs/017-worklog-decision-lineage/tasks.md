# Tasks: Evidence-Grounded Work Distillation

**Input**: Design documents from `/specs/017-worklog-decision-lineage/`

**Prerequisites**: Feature 015 merged; `spec.md`, `plan.md`, `research.md`, `data-model.md`, and contracts complete

**Tests**: Focused automated tests are required by FR-034. Heavy E2E remains risk-gated.

## Phase 1: Foundation Integration Gate

**Purpose**: Resolve shared ownership before any schema or implementation work.

- [x] T001 Merge feature 015 and map `src-tauri/src/workflow_foundation.rs`, `src-tauri/src/native/workflow_foundation.rs`, prompt registry, `sourceRevision`, source attribution, version provenance, application disposition, and retry helpers into `specs/017-worklog-decision-lineage/plan.md` and `specs/017-worklog-decision-lineage/contracts/distillation-result.md`
- [x] T002 Confirm `run_report_distillation` is the sole Run-linked Work Log operation and `task_journey_increment` consumes its validated structured result without enqueuing `work_log_distillation` in `specs/017-worklog-decision-lineage/contracts/distillation-result.md`
- [x] T003 Allocate the feature-specific migration after the foundation migration and record preservation/restore invariants in `src-tauri/src/native/migrations.rs` and `specs/017-worklog-decision-lineage/data-model.md`
- [x] T004 Re-run cross-artifact analysis and resolve all critical/high contract or governance findings in `specs/017-worklog-decision-lineage/`

---

## Phase 2: Foundational Semantic Contracts

**Purpose**: Establish strict types, validation, persistence, and test fixtures shared by every story.

- [x] T005 Add feature-owned Distillation claims, semantic actions, topic states, prepared detail, projection revisions, and dependency records in `src-tauri/src/application/task_distillation_service.rs` and `src-tauri/src/native/task_distillation.rs`
- [x] T006 Add the allocated feature schema and migration preservation checks for existing Run, Work Log, Task journey, and legacy lineage records in `src-tauri/src/native/task_distillation_schema.sql` and `src-tauri/src/native/migrations.rs`
- [ ] T007 [P] Add exact-source success, contradiction, missing-report, retry, decision-change, completion, edit, and deletion fixtures in `src-tauri/tests/fixtures/task_distillation/`
- [x] T008 Implement deterministic source selection that makes `final_report` primary and admits only material completed evidence in `src-tauri/src/application/task_distillation_service.rs`
- [x] T009 Implement strict result validation for citations, actor/epistemic state, verified evidence, semantic actions, and expected revisions in `src-tauri/src/application/task_distillation_service.rs`
- [x] T010 Implement stable semantic ID, source dependency, redirect, stale-publication, and last-good projection persistence in `src-tauri/src/native/task_distillation.rs`
- [x] T011 Add foundational unit tests for source selection, unsupported-claim rejection, stable IDs, and original-record preservation in `src-tauri/src/application/task_distillation_service.rs` and `src-tauri/src/native/task_distillation.rs`

**Checkpoint**: No user-story work starts until foundation contract mapping and strict semantic validation pass.

---

## Phase 3: User Story 1 — Read the Meaningful Run Result (P1)

**Goal**: Make the current Run-linked Work Log readable, cited, and faithful while keeping the original inspectable.

**Independent test**: Distill successful, contradictory, failed, and interrupted Run fixtures; verify exact claim sources, epistemic distinctions, original access, and no invented facts.

- [x] T012 [US1] Schedule `run_report_distillation` from terminal Run handling with exact final-report hash, Run revision, completed-evidence references, prompt identity, and rules version in `src-tauri/src/application/task_execution_service.rs`
- [x] T013 [US1] Build and validate the Run-centered structured result and readable section order without a second Work Log Distillation pass in `src-tauri/src/application/task_distillation_service.rs`
- [x] T014 [US1] Publish current/last-good Run projection atomically and expose freshness plus exact source links in `src-tauri/src/native/task_distillation.rs` and `src-tauri/src/lib.rs`
- [x] T015 [P] [US1] Add Work Log Distillation, freshness, warning, source-link, and original-detail types/client routes in `frontend/src/types/taskWorkbench.ts`, `frontend/src/services/taskClient.ts`, and `frontend/src/services/taskOperation.ts`
- [x] T016 [US1] Build the readable default and secondary original-report/evidence disclosure in `frontend/src/features/workbench/WorkLogDistillation.tsx` and integrate it in `frontend/src/features/workbench/TaskDetail.tsx`
- [x] T017 [US1] Add native tests for report-primary selection, contradiction, no-report failure, routine-control omission, retry idempotency, and Work Log preservation in `src-tauri/src/application/task_distillation_service.rs`
- [ ] T018 [US1] Add component tests for readable sections, citations, original access, pending/stale/failure fallback, long content, and no generation on read in `frontend/src/features/workbench/WorkLogDistillation.test.tsx`

---

## Phase 4: User Story 2 — Follow Decision Evolution (P1)

**Goal**: Incrementally project the same validated claims into stable active Task journey nodes and explicit topic states.

**Independent test**: Project suggestion → user adoption → performed → verified → superseded, plus withdrawn/unresolved topics and repeated completion, without overwriting history or using chronology as authority.

- [x] T019 [US2] Submit the validated Run result and exact affected Task source bundle to `task_journey_increment` without redistilling the Run in `src-tauri/src/application/task_distillation_service.rs`
- [x] T020 [US2] Implement add/enrich/merge/omit/supersede application, evidence-backed relationship validation, and topic-state resolution in `src-tauri/src/native/task_distillation.rs`
- [x] T021 [US2] Add immutable completion snapshots and reopen/later-completion status projection in `src-tauri/src/native/task_distillation.rs`
- [x] T022 [US2] Integrate semantic nodes with the active recorded Task journey while retaining original `followed_by` events separately in `src-tauri/src/native/task_journey.rs`
- [x] T023 [P] [US2] Extend Task journey contracts for provenance, epistemic state, topic state, prepared detail, exact source links, and recorded-trace separation in `frontend/src/features/workbench/TaskJourneyGraph.tsx` and `frontend/src/types/taskWorkbench.ts`
- [ ] T024 [US2] Redesign the Task journey around current adopted decisions and unresolved topics with historical supersession paths in `frontend/src/features/workbench/TaskJourneyGraph.tsx` and `frontend/src/features/workbench/task-journey-graph.css`
- [x] T025 [US2] Add a keyboard-safe saved-detail modal/drawer with before/after/reason/evidence/result/status/links and focus return in `frontend/src/features/workbench/TaskJourneyGraph.tsx`
- [x] T026 [US2] Add native tests for new-decision identity, explicit status evidence, actor/epistemic separation, relationship citations, and completion snapshots in `src-tauri/src/native/task_distillation.rs`
- [ ] T027 [US2] Add component tests for current/historical hierarchy, superseded/withdrawn/unresolved states, prepared detail, source navigation, no model call, keyboard, and focus return in `frontend/src/features/workbench/TaskJourneyGraph.test.tsx`

---

## Phase 5: User Story 3 — Update and Recover Safely (P2)

**Goal**: Propagate only relevant source changes through durable batches while retaining the last-good view.

**Independent test**: Edit/delete one source, replay a batch 100 times, fail/retry, finish stale work late, and perform explicit repair; verify unaffected nodes remain byte-identical.

- [x] T028 [US3] Add source-edit/delete hooks that calculate direct claim/link dependencies and enqueue bounded Task source bundles in `src-tauri/src/application/task_distillation_service.rs`
- [x] T029 [US3] Implement batch coalescing, exact expected-revision checks, stale disposition, explicit retry, and atomic compare-and-publish through feature-015 helpers in `src-tauri/src/application/task_distillation_service.rs`
- [x] T030 [US3] Implement targeted downstream recomputation for topic state and prepared detail plus explicit full repair for rules/schema changes in `src-tauri/src/native/task_distillation.rs`
- [x] T031 [US3] Expose retry and repair operations without rerunning Codex or changing Task state in `src-tauri/src/lib.rs`, `frontend/src/services/taskClient.ts`, and `frontend/src/services/taskOperation.ts`
- [x] T032 [US3] Add native tests for targeted edit/delete propagation, atomic batch publication, 100-delivery idempotency, stale completion, retry, and repair in `src-tauri/src/application/task_distillation_service.rs` and `src-tauri/src/native/task_distillation.rs`
- [x] T033 [US3] Add component tests for retained last-good views and pending/stale/retryable/repair-required controls in `frontend/src/features/workbench/WorkLogDistillation.test.tsx` and `frontend/src/features/workbench/TaskJourneyGraph.test.tsx`

---

## Phase 6: Polish, Documentation, and Verification

- [x] T034 [P] Add aligned English/Korean user-facing copy and responsive/focus/status styles in `frontend/src/features/workbench/taskWorkbenchText.ts`, `frontend/src/features/workbench/task-workbench.css`, and `frontend/src/features/workbench/task-journey-graph.css`
- [x] T035 [P] Document observable Distillation, original inspection, decision status, retry/repair, and privacy behavior in `docs/features/task-work-sessions.md`, `docs/features/task-work-sessions.ko.md`, `docs/features/README.md`, and `docs/features/README.ko.md`
- [ ] T036 Run focused unit/integration checks, the stable `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`; record results in `specs/017-worklog-decision-lineage/quickstart.md`
- [x] T037 Profile collapsed Task/Work Log projections and node-detail local reads against the 100 ms p95 and 15% regression gates; record evidence in `specs/017-worklog-decision-lineage/quickstart.md`
- [ ] T038 Perform rendered wide/640 px English/Korean review for long, empty, pending, stale, failed, contradicted, superseded, withdrawn, unresolved, completion, keyboard, source-link, and focus-return states; store evidence under `.tmp/ui-review/work-distillation/` and record limitations in `specs/017-worklog-decision-lineage/quickstart.md`
- [ ] T039 Assess the concrete remaining native lifecycle risk after focused checks; run only the smallest targeted packaged scenario if needed and document the E2E run/skip decision in `specs/017-worklog-decision-lineage/quickstart.md`
- [ ] T040 Re-run cross-artifact analysis and update `docs/CONTINUATION.md` only if material implementation work, risk, or unavailable verification remains

## Dependencies

- T001–T004 block every implementation task.
- T005–T011 establish shared feature semantics and block all user stories.
- US1 and US2 are both P1, but US2 consumes the validated US1 structured result and therefore starts after T013.
- US3 depends on published Work Log and journey projections from US1/US2.
- T034–T040 follow final behavior and contract stabilization.

## Parallel Examples

- T007 fixture preparation can proceed alongside T005–T006 after the integration contract is fixed.
- T015 can proceed alongside native US1 projection work once the application response contract is fixed.
- T023 can proceed alongside T020–T022 once semantic types are fixed.
- T034 and T035 can proceed in parallel after UI behavior stabilizes.

## Implementation Strategy

Finish the foundation gate first. Deliver US1 as the readable, cited Run Work Log MVP, then reuse exactly that validated result for US2 Task journey increments. Add US3 targeted invalidation only after semantic identity and projection publication are stable. Never enqueue both `run_report_distillation` and `work_log_distillation` for one Run. Keep every task unchecked until implementation and its stated verification actually complete.

## Implementation checkpoint — 2026-09-26

Native authority and transaction repairs passed 32 focused tests. Frontend projections, raw disclosure, exact Run retry, prepared node detail and topic status are wired. Source fixtures live beside native tests rather than a separate fixture directory (T007 remains pending artifact reconciliation). T018/T024/T027/T036–T040 retain their complete acceptance scope; final rendered review, source-navigation review, broad integrated checks and projection profiling are not claimed complete.
