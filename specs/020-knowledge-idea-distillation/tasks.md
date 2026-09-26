# Tasks: Final Knowledge and separate idea drafts

**Input**: Design documents from `/specs/020-knowledge-idea-distillation/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, integrated features 015/017/018, and accepted feature 021 handoff

**Tests**: Required by the program's evidence-fidelity, stale/concurrency, migration, UI/accessibility, locale, and publication checks. Keep E2E selected by concrete residual risk.

## Phase 1: Dependency integration and setup

**Purpose**: Start only from accepted upstream contracts and reserve shared edit order.

- [x] T001 Confirm the integration branch contains feature 015 prompt/job/reference/version contracts, feature 017 Distillation/journey freshness and completion snapshots, and feature 018 canonical `decisions[]` metadata; record resolved contract versions in `specs/020-knowledge-idea-distillation/plan.md`
- [x] T002 Confirm the feature 021 exact reviewed-body publication handoff and idea-unit ownership in `specs/020-knowledge-idea-distillation/contracts/publication-metadata.md`
- [x] T003 Reconfirm the central migration head, replace tentative `F21`, and record the assigned version and preservation invariants in `specs/020-knowledge-idea-distillation/plan.md` and `src-tauri/src/native/migrations.rs`
- [x] T004 Reconfirm current Knowledge routes/jobs/types and sequence the minimal shared-file patches after C/E integration in `specs/020-knowledge-idea-distillation/plan.md`

---

## Phase 2: Foundational immutable Knowledge infrastructure

**Purpose**: Establish feature-owned storage, exact source snapshots, and shared contracts before story behavior.

**Critical**: No user-story work starts until this phase is complete.

- [x] T005 Add failing migration tests for immutable versions, pointer derivation, existing body/hash/path preservation, idempotence, rollback, and foreign keys in `src-tauri/src/native/migrations.rs`
- [x] T006 Implement the centrally assigned migration for Knowledge versions, current/published pointers, evidence snapshots, applicability, idea revisions, and quality findings in `src-tauri/src/native/migrations.rs`
- [x] T007 Create feature-owned domain types, canonical serialization, hashing, and exact reference validation in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T008 Register the versioned Knowledge generation operation/result schema through the integrated feature 015 registry in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T009 Add minimal module/route/job wiring while preserving compatibility route names in `src-tauri/src/native/mod.rs`, `src-tauri/src/native/task_assistance.rs`, and `src-tauri/src/native/jobs.rs`
- [x] T010 Add shared frontend Knowledge review/version/idea types matching `contracts/application-api.md` in `frontend/src/types/taskWorkbench.ts`
- [x] T011 Extend typed read/generate/edit/restore/publish client operations with idempotency and expected hashes in `frontend/src/services/taskClient.ts`

**Checkpoint**: Immutable append/read primitives and accepted contracts are available; no final body, idea, or version UI behavior is implied yet.

---

## Phase 3: User Story 1 - Read final outcomes without reconstructing the session (Priority: P1) MVP

**Goal**: Generate and review a standalone, source-grounded final Knowledge article whose per-topic outcomes, conditions, applicability, and limits are independently readable.

**Independent Test**: With receipt/approval, contradictory-report, missing-report, and historical-choice fixtures, generate a draft and verify exact claim sources, current per-topic finality, retained conditions/limits, complete applicability, and zero unsupported verification/effectiveness claims without publication.

### Tests for User Story 1

- [ ] T012 [P] [US1] Add failing Rust fixture tests for receipt-not-approval, report contradiction, missing final report, per-topic final decisions, non-decision article types, omissions, assumptions, and applicability overreach in `src-tauri/tests/knowledge_distillation.rs`
- [ ] T013 [P] [US1] Add failing API/job tests for waiting on current journey, bounded transient retry result states, exact generation snapshot capture, save-time stale rejection, and last-good stale visibility in `src-tauri/src/native/knowledge_distillation.rs` and `src-tauri/src/native/jobs.rs`
- [ ] T014 [P] [US1] Add failing component tests for final-outcome-first hierarchy, private/published labels, applicability, source/assumption disclosure, empty/loading/error/stale states, focus, and long EN/KO content in `frontend/src/features/workbench/KnowledgeReviewPanel.test.tsx`

### Implementation for User Story 1

- [x] T015 [US1] Implement journey reconciliation and one-transaction EvidenceSnapshot capture from completed Task definition, primary final-report Distillation, exact journey/completion snapshot, per-topic decisions, and actual-use/counterevidence references in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T016 [US1] Implement generation for concept, guide, comparison/decision, and research-result articles with final outcomes first and exact claim/assumption bindings in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T017 [US1] Implement separate deterministic validation for unsupported additions, material omissions, idea leakage, invalid finality/verification, missing references, and applicability overreach in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T018 [US1] Implement transactional save-time manifest recomputation so changed Task, Distillation, journey, completion, or actual-use reference evidence appends no revision in `src-tauri/src/native/knowledge_distillation.rs` and `src-tauri/src/native/jobs.rs`
- [x] T019 [US1] Implement the current Knowledge review projection and generation application routes without changing Task workflow state in `src-tauri/src/native/knowledge_distillation.rs` and `src-tauri/src/native/task_assistance.rs`
- [x] T020 [US1] Build the primary article, applicability, provenance disclosure, quality state, and private/published header in `frontend/src/features/workbench/KnowledgeReviewPanel.tsx` and `frontend/src/features/workbench/knowledge-review.css`
- [x] T021 [US1] Replace only the existing Task Review Knowledge section with `KnowledgeReviewPanel` and preserve Task-detail session/focus state in `frontend/src/features/workbench/TaskDetail.tsx`
- [x] T022 [US1] Add complete English/Korean Knowledge review, applicability, provenance, freshness, and validation copy using 사용자 in Korean in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: User Story 1 works without idea archival, version comparison, restoration, or publication.

---

## Phase 4: User Story 2 - Preserve useful unverified ideas separately (Priority: P1)

**Goal**: Preserve only reusable exploration as separate idea units without inserting it into the final Knowledge body or overstating its evidence.

**Independent Test**: With the reminder-automation fixture, verify the final body has no exploration passage and a separate optional idea shows its exact disposition, sources, Task/final-Knowledge links, and reconsideration conditions; verify no idea is created for valueless exploration.

### Tests for User Story 2

- [ ] T023 [P] [US2] Add failing Rust tests for optional idea extraction, all four dispositions, required links/reconsideration conditions, idea-leak rejection, no archival promotion, and no-valueless-idea output in `src-tauri/tests/knowledge_distillation.rs`
- [ ] T024 [P] [US2] Add failing component tests for the absent/available collapsed idea region and exact `unverified`, `deferred`, `out_of_scope`, and `rejected` labels in English/Korean in `frontend/src/features/workbench/KnowledgeReviewPanel.test.tsx`

### Implementation for User Story 2

- [x] T025 [US2] Implement separate immutable idea revision creation and validation with exact source, Task, final-Knowledge, disposition, and reconsideration links in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T026 [US2] Enforce normalized passage/source-role separation so idea-only exploration cannot appear in final Knowledge Markdown in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T027 [US2] Add the secondary `Ideas to revisit` UI with exact text dispositions, reconsideration conditions, and navigable provenance in `frontend/src/features/workbench/KnowledgeReviewPanel.tsx` and `frontend/src/features/workbench/knowledge-review.css`
- [x] T028 [US2] Serialize the agreed feature 018 compatibility metadata (`out_of_scope` displayed exactly, top-level `deferred`) in the feature 021 handoff builder in `src-tauri/src/native/knowledge_distillation.rs`

**Checkpoint**: User Stories 1 and 2 independently prove final-body/idea separation; no filesystem publication occurs.

---

## Phase 5: User Story 3 - Review exact draft versions (Priority: P2)

**Goal**: Select and compare exact immutable versions, restore an older version as a new private revision, and publish only the exact reviewed version.

**Independent Test**: With versions 1–3, compare exact bodies/snapshots, restore version 1 as version 4, and prove versions 1–3, Task logs/decisions/journey, and the published pointer are byte-equivalent; publish through a feature 021 substitute and verify exact handed-off bytes/hash with no provider call.

### Tests for User Story 3

- [ ] T029 [P] [US3] Add failing Rust/API tests for immutable edits, version reads/comparison, restore-as-new, operation replay/conflict, current/published pointer independence, and zero Task-history mutation in `src-tauri/tests/knowledge_distillation.rs`
- [ ] T030 [P] [US3] Add failing publication handoff tests for exact reviewed bytes, canonical multi-topic `decisions[]`, actual-use references only, no provider call, feature 021 failure rollback, and actual-content hash authority in `src-tauri/tests/knowledge_distillation.rs`
- [ ] T031 [P] [US3] Add failing component tests for version selection, current-private/published badges, wide and narrow compare semantics, restore-as-new, keyboard order, disabled/busy/conflict states, and preserved focus in `frontend/src/features/workbench/KnowledgeReviewPanel.test.tsx`

### Implementation for User Story 3

- [x] T032 [US3] Implement immutable edited revisions, exact version reads/comparison, and current/published pointer projections in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T033 [US3] Implement restore-as-new with derived provenance and assertions that Work Logs, decisions, completion, Distillation, and journey stores are untouched in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T034 [US3] Implement exact reviewed-revision handoff with canonical `llm_wiki.decisions[]`, applicability, source/assumption provenance, separately selected ideas, and rollback-safe feature 021 response recording in `src-tauri/src/native/knowledge_distillation.rs`
- [x] T035 [US3] Add compact version selection, two-revision compare, responsive stacked comparison, and restore-as-new controls in `frontend/src/features/workbench/KnowledgeReviewPanel.tsx` and `frontend/src/features/workbench/knowledge-review.css`
- [x] T036 [US3] Migrate the old correction/regenerate/publish UI actions to immutable version semantics and preserve unsaved-input/Task-switch guards in `frontend/src/features/workbench/TaskDetail.tsx` and `frontend/src/features/workbench/WorkbenchView.tsx`
- [x] T037 [US3] Add English/Korean version, compare, restore, current-private, published, and publication-conflict copy in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: All three stories work; feature 021 remains the only portable file writer.

---

## Phase 6: Polish, documentation, and risk-based validation

**Purpose**: Complete cross-cutting fidelity, performance, accessibility, and handoff evidence.

- [x] T038 [P] Add paired user guides for final Knowledge, applicability, separate ideas, immutable versions, restore-as-new, and publication authority in `docs/features/knowledge-distillation.md` and `docs/features/knowledge-distillation.ko.md`
- [x] T039 [P] Link the paired guides from `docs/features/README.md` and `docs/features/README.ko.md` and review `docs/DOCUMENTATION_GUIDE.md` for any further affected current-behavior docs
- [ ] T040 Add stable profiling for Task Review projection/version selection and generation stages, recording results without a new unmeasured latency promise in `src-tauri/src/native/knowledge_distillation.rs` and `specs/020-knowledge-idea-distillation/quickstart.md`
- [ ] T041 Run focused `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check` separately and record results in `specs/020-knowledge-idea-distillation/quickstart.md`
- [ ] T042 Perform rendered wide/narrow English/Korean and keyboard/focus/reduced-motion review with long, empty, loading, stale, conflict, and error states; record evidence and limitations in `specs/020-knowledge-idea-distillation/quickstart.md`
- [ ] T043 Assess remaining Tauri publication/restore risk after focused tests; record the E2E skip rationale or run only the smallest named desktop scenario after one final release build and record it in `specs/020-knowledge-idea-distillation/quickstart.md`
- [x] T044 Run `$speckit-converge` against the integrated implementation and append only demonstrably unbuilt work to `specs/020-knowledge-idea-distillation/tasks.md`

---

## Dependencies and execution order

### Phase dependencies

- Phase 1 blocks every implementation edit.
- Phase 2 depends on accepted dependencies and blocks all user stories.
- User Story 1 establishes the article and snapshot model used by User Story 2 and User Story 3.
- User Story 2 and User Story 3 may proceed after User Story 1; keep same-file Rust/UI edits sequential even if their tests are prepared in parallel.
- Phase 6 depends on all desired stories.

### Parallel opportunities

- T012–T014 prepare independent Rust job/component tests before US1 implementation.
- T023 and T024 prepare separate backend/UI idea tests.
- T029–T031 prepare separate domain/publication/UI version tests.
- T038 and T039 may proceed together after observable behavior stabilizes.
- Do not parallelize edits to `knowledge_distillation.rs`, `KnowledgeReviewPanel.tsx`, `TaskDetail.tsx`, `taskWorkbenchText.ts`, `jobs.rs`, or `migrations.rs` with other tasks touching the same file.

## Implementation strategy

### MVP first

1. Complete dependency gates and foundational immutable storage.
2. Deliver User Story 1 with exact snapshots, fidelity validation, standalone article, and applicability.
3. Validate it independently without publication.
4. Add separate idea units, then version/restore/publication review.

### Integration discipline

- Rebase/recreate from the integrated dependency commit before implementation.
- Keep most logic in the feature-owned Rust module and React panel.
- Apply shared migration/router/job/TaskDetail/client/type/localization patches sequentially after their upstream owners land.
- Do not implement D ranking or G filesystem organization in this feature.

## Notes

- `[P]` means different files or independent failing-test preparation with no incomplete dependency.
- Every story task carries `[US1]`, `[US2]`, or `[US3]`; setup/foundation/polish tasks do not.
- Tests must first demonstrate the missing behavior, then pass with implementation.
- E2E is skipped unless a concrete residual native boundary risk remains; full-suite E2E is not planned.


### Integrated ownership reconciliation

Implementation tasks above use the resolved locations in the plan: native archive
orchestration reuses the existing vault boundary; frontend archive state/types and
commands are in KnowledgeReviewPanel, taskWorkbench and taskClient. Exact published
bytes, out-of-scope-as-deferred metadata, selected ideas and rollback-safe pointers
are exercised by native archive and actual application-command tests. Dependency
contracts are integrated in main (schema 22, Knowledge prompt 3, Distillation prompt
3 and exact reference version/section usage). Unchecked aggregate test, profiling
and rendered-review items retain their broader original acceptance scope; they are
not a declaration that their associated implemented runtime is absent. No blanket
convergence claim is made for an unperformed visual matrix.
