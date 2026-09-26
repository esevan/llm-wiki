# Tasks: Organized Knowledge and idea archive

**Input**: Design documents from `/specs/021-organized-knowledge-archive/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, integrated D19, E20 shared-control contract/implementation, and F21 immutable Knowledge/idea publication handoff

**Tests**: Required for byte fidelity, external-edit safety, migration preservation, crash recovery, index freshness, UI/accessibility, and binding performance. Keep E2E selected by concrete residual risk.

## Phase 1: Dependency integration and setup

**Purpose**: Resolve upstream signatures and serialize shared-file ownership before implementation.

- [x] T001 Confirm integrated feature 015 job/prompt/reference contracts, feature 018 document/index contract and migration 19, feature 019 `ReferenceViewer`/`DraftVersionControls` and migration 20, and feature 020 revision/publication handoff and migration 21; record resolved hashes and signatures in `specs/021-organized-knowledge-archive/plan.md`
- [x] T002 Confirm feature 017 source-fidelity fixes consumed by feature 020 and record any still-gated fixtures without weakening G scope in `specs/021-organized-knowledge-archive/plan.md`
- [x] T003 Inspect the integrated schema head and reserve migration 22 only when `user_version` is exactly 21 in `src-tauri/src/native/migrations.rs`
- [x] T004 Sequence the single F panel integration edit after F lands while assigning G ownership to `frontend/src/features/knowledge/KnowledgeArchiveReview.tsx` in `specs/021-organized-knowledge-archive/plan.md`

---

## Phase 2: Foundational archive contracts and persistence

**Purpose**: Establish stable published identity, immutable proposal/revision storage, and recoverable cross-store operations.

**Critical**: No user-story apply behavior starts until this phase is complete.

- [x] T005 Add failing migration tests for fresh 22 bootstrap, populated 21 upgrade, second open, rollback, foreign keys, preserved existing publication rows/hashes, and lazy adoption without file rewrite in `src-tauri/tests/work_tracking_migrations.rs`
- [x] T006 Implement additive migration 22 for archive proposals, documents, revisions, path history, published references, publication operations, and steps with pre/post invariants in `src-tauri/src/native/migrations.rs`
- [x] T007 [P] Define outcomes, artifacts, stable identities, reference eligibility, managed patch validation, operation phases, and canonical hashing in `src-tauri/src/domain/knowledge_archive.rs`
- [x] T008 [P] Extend structured vault port operations for exact byte inventory/read/hash/stage/replace/move/withdraw/recovery without exposing raw filesystem access in `src-tauri/src/ports/vault_repository.rs`
- [x] T009 Add the dedicated `knowledge_archive_proposal` prompt identity under durable `user_generation`, implement its complete registry-owned semantics/schema in `src-tauri/src/workflow_foundation/prompts/archive.rs`, and route it from `src-tauri/src/workflow_foundation.rs` and `src-tauri/src/workflow_foundation/prompts/mod.rs` without changing `publication_index`
- [x] T010 Wire the archive module and thin compatibility dispatch while removing publication organization logic from the large command handler in `src-tauri/src/native/mod.rs` and `src-tauri/src/native/task_assistance.rs`
- [x] T011 [P] Define archive proposal, operation, artifact, path, MOC, and source-link frontend types in `frontend/src/features/knowledge/archiveTypes.ts`
- [x] T012 [P] Add typed prepare/read/publish/status/retry/recover client calls with idempotency and expected hashes in `frontend/src/services/applicationClient.ts`

**Checkpoint**: Migration and shared types exist; no file apply or UI publication is implied.

---

## Phase 3: User Story 1 - Organize without duplicating existing knowledge (Priority: P1) MVP

**Goal**: Prepare a complete reviewable new/update/merge/conflict/supersede proposal using existing taxonomy and overlap before creating a duplicate.

**Independent Test**: Given existing overlapping and non-overlapping articles, prepare proposals and verify outcome, stable target, category choice/rationale, exact frozen artifacts, and stale rejection without modifying files or workflow state.

### Tests for User Story 1

- [x] T013 [P] [US1] Add failing domain tests for all five outcomes, stable ID/path validation, existing-taxonomy preference, required new-category rationale, case/path collisions, idea separation, and proposal canonical hashes in `src-tauri/src/domain/knowledge_archive.rs`
- [ ] T014 [P] [US1] Add failing native tests for feature 018 overlap input, the eight-passage/6,000-token context cap, taxonomy snapshot changes, duplicate titles, exact feature 020 source heads, provider failure, and zero file/Task mutation during preparation in `src-tauri/tests/knowledge_archive.rs`
- [x] T015 [P] [US1] Add failing prompt registry tests proving full semantic instructions/schema are registry-owned, the identity uses `user_generation`, and version changes cover behavior changes in `src-tauri/src/workflow_foundation/prompts/archive.rs`

### Implementation for User Story 1

- [x] T016 [US1] Implement bounded Wiki/taxonomy inventory through `MarkdownVaultAdapter` with explicit metadata, tags, aliases, MOCs, current file hashes, and normalized snapshot hashing in `src-tauri/src/adapters/vault/mod.rs`
- [x] T017 [US1] Implement `KnowledgeArchiveService::prepare` using exact feature 020 revisions, eligible feature 019 facts, feature 018 overlap units, and the registered organization prompt in `src-tauri/src/application/knowledge_archive_service.rs`
- [x] T018 [US1] Deterministically serialize exact final Knowledge and separately selected idea artifacts, validate all bytes/hashes/paths, and persist immutable proposal versions in `src-tauri/src/native/knowledge_archive.rs`
- [x] T019 [US1] Implement prepare/read proposal routes and visible user-requested generation failure/retry status without publication side effects in `src-tauri/src/native/knowledge_archive.rs` and `src-tauri/src/native/task_assistance.rs`

**Checkpoint**: User Story 1 independently produces reviewable proposals but cannot publish them.

---

## Phase 4: User Story 2 - Publish linked portable documents (Priority: P1)

**Goal**: On explicit action, publish the exact reviewed final/idea bytes, eligible source wikilinks, and managed MOC patches with stable identity and durable indexing.

**Independent Test**: Review and publish final Knowledge plus selected ideas; verify exact artifact hashes, no final-body exploration, actual-use/counterevidence links only, preserved MOC user bytes, stable revisions, and distinct index-pending/completed states.

### Tests for User Story 2

- [x] T020 [P] [US2] Add failing adapter tests for regular-file/root/symlink checks, exact create/replace/move CAS, same-filesystem staging, recovery copies, managed-region byte preservation, duplicate markers, and platform path cases in `src-tauri/src/adapters/vault/mod.rs`
- [ ] T021 [P] [US2] Add failing integration tests for exact final/idea bytes, all idea dispositions, selected-only publication, eligible source links/rationales, stable revisions, operation replay, and feature 020 pointer transaction in `src-tauri/tests/knowledge_archive.rs`
- [ ] T022 [P] [US2] Add failing index tests for publication job coalescing, bounded retry/restart, exact revision CAS, lexical fallback, final-over-idea ranking, and no file rewrite on retry in `src-tauri/src/native/retrieval_index.rs`
- [ ] T023 [P] [US2] Add failing component tests for proposal hierarchy, outcome/path/category, artifact and MOC/source diffs, explicit Publish, writing/index-pending/complete states, keyboard order, and long EN/KO content in `frontend/src/features/knowledge/KnowledgeArchiveReview.test.tsx`

### Implementation for User Story 2

- [x] T024 [US2] Implement exact byte stage/create/replace and managed MOC region application with whole-file and region CAS in `src-tauri/src/adapters/vault/mod.rs`
- [x] T025 [US2] Implement journal-first publish phases without holding a database write transaction during filesystem IO in `src-tauri/src/application/knowledge_archive_service.rs`
- [x] T026 [US2] Record pending archive revisions/path history/eligible exact reference links atomically after verified file writes while leaving G and feature 020 applied pointers unchanged in `src-tauri/src/native/knowledge_archive.rs`
- [x] T027 [US2] Enqueue/coalesce durable `publication_index` work, validate exact receipts, and advance G/feature 020 pointers together only after indexing while keeping file/database/index/application states distinct in `src-tauri/src/native/knowledge_archive.rs` and `src-tauri/src/native/jobs.rs`
- [x] T028 [US2] Implement publish/status/index-retry routes and compatibility delegation for the existing Knowledge publish route in `src-tauri/src/native/knowledge_archive.rs` and `src-tauri/src/native/task_assistance.rs`
- [x] T029 [US2] Build G-owned `KnowledgeArchiveReview` with exact artifact preview, organization summary, progressive MOC/source/link details, conflict and publication states, and reused E `ReferenceViewer` in `frontend/src/features/knowledge/KnowledgeArchiveReview.tsx`
- [x] T030 [US2] Add responsive archive-review styles using shared tokens, visible focus, text-plus-color states, reduced motion, and narrow stacking in `frontend/src/features/knowledge/knowledge-archive.css`
- [x] T031 [US2] Integrate `KnowledgeArchiveReview` once into F's `KnowledgeReviewPanel` without duplicating E version/reference controls or disturbing current selection/focus/scroll in `frontend/src/features/workbench/KnowledgeReviewPanel.tsx`
- [x] T032 [US2] Add complete English/Korean archive outcome, path, idea, MOC, source-rationale, conflict, publication, and index-state copy using 사용자 in Korean in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: User Stories 1 and 2 publish portable files only through explicit reviewed apply.

---

## Phase 5: User Story 3 - Recover without losing external edits (Priority: P2)

**Goal**: Resume or repair partial publication and maintain stable identity/index/managed links through move, rename, supersede, and withdrawal without overwriting external edits.

**Independent Test**: Interrupt every publication phase, restart, and verify hash-directed completion/compensation; add an external edit and verify it is preserved with recovery material and a review-required state.

### Tests for User Story 3

- [x] T033 [US3] Add failing crash-point/restart tests after journal, stage, partial files, all files, database record, and index enqueue with idempotent row/file/job counts in `src-tauri/tests/knowledge_archive.rs`
- [x] T034 [US3] Add failing external-edit tests for post-write races, changed recovery bytes, safe compensation limits, explicit recovery choices, and preserved current/recovery content in `src-tauri/tests/knowledge_archive.rs`
- [x] T035 [US3] Add failing move/rename/supersede/withdraw tests for stable IDs, immutable revisions/path history, managed inbound link repair, unresolved unmanaged links, recovery withdrawal, and index remove/upsert in `src-tauri/tests/knowledge_archive.rs`
- [ ] T036 [P] [US3] Add failing component tests for stale/conflict/index-failed/repair-required/recovery choices and preserved version/reference/focus/scroll state during async updates in `frontend/src/features/knowledge/KnowledgeArchiveReview.test.tsx`

### Implementation for User Story 3

- [x] T037 [US3] Implement restart reconciliation from exact journal/file/database/index hashes, idempotent resume, and compensation only for unchanged operation-owned bytes in `src-tauri/src/application/knowledge_archive_service.rs`
- [x] T038 [US3] Implement safe recover/adopt-through-new-review operations with retained recovery material and bounded user-safe diagnostics in `src-tauri/src/native/knowledge_archive.rs`
- [x] T039 [US3] Implement reviewed move/rename/supersede/withdraw proposals, stable revision/path history, and managed-link/MOC repair without broad rewrites in `src-tauri/src/application/knowledge_archive_service.rs`
- [x] T040 [US3] Extend `KnowledgeArchiveReview` with recovery, unresolved-link, and existing-publication organization review while preserving async continuity in `frontend/src/features/knowledge/KnowledgeArchiveReview.tsx`

**Checkpoint**: All stories preserve external edits and recover file/database/index partial states.

---

## Phase 6: Performance, documentation, and validation

**Purpose**: Prove budgets, portability, accessibility, and truthful current-behavior documentation.

- [ ] T041 [P] Add representative 1,000-note/10 MB timing fixtures that report inventory, validation, file, database, indexing, and UI projection timings separately while retaining 3 s/75 ms/100 ms gates in `src-tauri/tests/knowledge_archive.rs`
- [x] T042 [P] Add portable YAML/Markdown parsing and future typed-section preservation fixtures in `src-tauri/src/domain/knowledge_archive.rs`
- [x] T043 [P] Add paired user guides for explicit archive review, final/idea separation, portable metadata, external-edit conflicts, recovery, moves/withdrawal, and index-pending behavior in `docs/features/knowledge-archive.md` and `docs/features/knowledge-archive.ko.md`
- [x] T044 Link the paired archive guide from `docs/features/README.md` and `docs/features/README.ko.md` and review `docs/DOCUMENTATION_GUIDE.md` for any other affected current-behavior docs
- [ ] T045 Run focused Rust archive/domain/adapter/migration/index tests, focused frontend component/client tests, and `git diff --check` as separate commands; record exact results in `specs/021-organized-knowledge-archive/quickstart.md`
- [ ] T046 Perform rendered wide/narrow English/Korean review with long/empty/conflict/recovery/index states plus keyboard, focus, screen-reader labels, and reduced motion; record evidence and limitations in `specs/021-organized-knowledge-archive/quickstart.md`
- [ ] T047 Assess the remaining native file/database/restart boundary after focused tests; run only the smallest selected packaged desktop scenario if a concrete residual risk remains, otherwise record the E2E skip rationale in `specs/021-organized-knowledge-archive/quickstart.md`
- [ ] T048 Run Spec Kit analyze and converge, resolve every critical/high inconsistency and all unfinished required behavior, and update `specs/021-organized-knowledge-archive/tasks.md`

## Dependencies and execution order

- Phase 1 precedes all implementation. T003 is blocked until the integrated schema head is exactly 21.
- Phase 2 blocks all user stories.
- US1 proposal preparation blocks US2 publication and US3 lifecycle operations.
- US2's journal/apply/index path blocks US3 recovery and move/withdraw reuse.
- Backend proposal/domain tasks may proceed before E/F UI wiring, but T031 is serialized after F owns its panel and E controls exist.
- `[P]` tasks touch different primary files or independent fixtures; shared adapter, migration, command, panel, and registry edits remain sequential.


## Implementation checkpoint — native archive boundary

The resolved native/domain file mapping is recorded in plan.md. Completed tasks
above describe behavior verified there, not nonexistent facade files. Native
archive/migration selection passed 16 tests: real HTTP proposal dispatch and
idempotent replay, all five outcomes, topic-specific supersession, metadata/MOC
identity preservation, exact reviewed bytes and deferred F pointers, external
edits, partial-file/restart recovery, compensation, withdrawal/search removal,
actual-use source eligibility, and additive/rollback migration. Inventory of
1,001 notes/~10 MB measured 1,569 ms. Full index/warm retrieval budgets are feature
018's independently measured acceptance; final integrated UI projection timing,
G/F bindings, rendered review, and native desktop E2E remain open.

T008's facade expansion is replaced by the resolved native adapter boundary;
T011/T012/T023/T029-T032/T036/T040/T046 are implemented with F-owned UI integration
and require that checkpoint before being checked here. T014/T021/T022/T041/T045
remain partially verified until final cross-feature acceptance. T048 has not yet
been declared converged.


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
