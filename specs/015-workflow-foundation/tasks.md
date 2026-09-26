# Tasks: Workflow Foundation

**Input**: Design documents from `/specs/015-workflow-foundation/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

## Phase 1: Setup

- [X] T001 Confirm migration 16 ownership and current canonical version/decision stores in `src-tauri/src/native/migrations.rs` and schema files
- [X] T002 Add workflow foundation module exports in `src-tauri/src/lib.rs` and `src-tauri/src/native/mod.rs`

## Phase 2: Foundational Contracts

- [X] T003 Implement prompt IDs, prompt definitions, builders, validators, and operation policies in `src-tauri/src/workflow_foundation.rs`
- [X] T004 Implement retrieval intent, source attribution, version/provenance, document reference, decision status, and exact-head contracts in `src-tauri/src/workflow_foundation.rs`
- [X] T005 Add additive queue metadata, version provenance, and document reference schema and migration 16 in `src-tauri/src/native/schema.sql` and `src-tauri/src/native/migrations.rs`
- [X] T006 Implement SQLite provenance, reference, and application-disposition CAS helpers in `src-tauri/src/native/workflow_foundation.rs`
- [X] T007 Add contract, migration, CAS, and performance-focused unit tests in `src-tauri/src/workflow_foundation.rs`, `src-tauri/src/native/workflow_foundation.rs`, and `src-tauri/src/native/migrations.rs`

## Phase 3: User Story 1 - Trust AI operations across workflows

**Goal**: Separate queue execution from application outcome while preserving coalescing, cancellation, and retry.

**Independent Test**: Complete and supersede representative queue jobs and verify status, retry, result, and CAS behavior.

- [X] T008 [US1] Extend queue reads, enqueue metadata, retries, cancellation, and finalization outcomes in `src-tauri/src/native/jobs.rs`
- [X] T009 [US1] Add policy-based equivalent coalescing and latest-wins cancellation helpers in `src-tauri/src/native/jobs.rs`
- [X] T010 [US1] Add focused queue tests for status separation, duplicate coalescing, stale application, and retry metadata in `src-tauri/src/native/jobs.rs`

## Phase 4: User Story 2 - Reuse governed prompts and evidence

**Goal**: Route current structured native provider calls through the registry and reject malformed output.

**Independent Test**: Exercise registered builders and validators through queue, task assistance, and direct provider paths.

- [X] T011 [US2] Adopt registered prompts and output validation in `src-tauri/src/native/jobs.rs`
- [X] T012 [US2] Adopt registered prompts and output validation in `src-tauri/src/native/task_assistance.rs`
- [X] T013 [US2] Adopt registered problem-enrichment prompt and output validation in `src-tauri/src/provider.rs`
- [X] T014 [US2] Add focused adoption tests in `src-tauri/src/native/jobs.rs`, `src-tauri/src/native/task_assistance.rs`, and `src-tauri/src/provider.rs`

## Phase 5: User Story 3 - Resume and restore versioned work

**Goal**: Make exact-head, restoration provenance, source references, and decision status reusable without replacing canonical version stores.

**Independent Test**: Record append/restoration provenance and exact references, then reject a stale expected head.

- [X] T015 [US3] Verify provenance/reference persistence and canonical-version compatibility in `src-tauri/src/native/workflow_foundation.rs`
- [X] T016 [US3] Publish stable consumer signatures and examples in `specs/015-workflow-foundation/contracts/workflow-foundation-api.md`

## Phase 6: Polish and Cross-Cutting Concerns

- [X] T017 [P] Document foundation behavior in `docs/features/workflow-foundation.md` and `docs/features/workflow-foundation.ko.md`
- [X] T018 [P] Link the new guide from `docs/features/README.md` and `docs/features/README.ko.md`
- [X] T019 Mark all completed tasks and verify spec/plan/tasks consistency in `specs/015-workflow-foundation/tasks.md`
- [X] T020 Run focused native tests and `git diff --check`

## Dependencies

- Phase 2 depends on Phase 1.
- US1 depends on queue schema and foundation contracts from Phase 2.
- US2 depends on the prompt registry from Phase 2 and may proceed after its API is stable.
- US3 depends on the persistence helpers from Phase 2 and is otherwise independent of US1 and US2.
- Documentation and final verification depend on all user stories.

## Parallel Opportunities

- T003 and T004 execute sequentially because they share one contract module.
- T017 and T018 touch separate documentation files from implementation.
- No implementation is delegated because this bounded foundation owns tightly coupled queue, migration, and prompt contracts.

## Implementation Strategy

Complete the typed registry and additive migration first, then integrate the existing queue, then migrate provider call sites. Keep every canonical content and decision table in place. Deliver no downstream Capture, Work Log, retrieval UI, preview UI, Knowledge behavior, or archive behavior.
