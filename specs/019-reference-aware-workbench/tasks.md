# Tasks: Reference-aware workbench

**Input**: Design documents from `/specs/019-reference-aware-workbench/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, integrated feature 015, full feature 016 Capture wiring, and integrated feature 018 persistence/application contracts

**Tests**: Required by the program’s stale/concurrency/restart/migration, evidence-fidelity, accessibility, EN/KO, reduced-motion, and selected E2E requirements.

## Phase 1: Dependency integration and setup

**Purpose**: Begin implementation only from integrated upstream behavior and a contiguous migration sequence.

- [ ] T001 Confirm integrated feature 015 prompt/job/provenance/reference contracts, feature 016 Capture wiring and `ContentRevisionTransition`, and feature 018 retrieval persistence/API; record accepted hashes and signatures in `specs/019-reference-aware-workbench/plan.md`
- [ ] T002 Confirm E owns shared `ReferenceViewer`, mention types, and `DraftVersionControls` for F/G reuse, and reconcile feature 020 imports/contracts in `specs/019-reference-aware-workbench/contracts/interaction.md` and `specs/020-knowledge-idea-distillation/plan.md`
- [x] T003 Re-read the central migration head after B17/C18/D19 integration, replace tentative E20 only when predecessors exist, and record the assigned version in `specs/019-reference-aware-workbench/plan.md` and `src-tauri/src/native/migrations.rs`
- [x] T004 Reconfirm current refinement routes, queue behavior, Task mutation path, retrieval adapter, and UI state owners before edits in `specs/019-reference-aware-workbench/plan.md`

---

## Phase 2: Foundational preview and reference infrastructure

**Purpose**: Establish immutable storage, contracts, typed clients, and module boundaries that block every user story.

**Critical**: Do not start user-story implementation until the integrated dependencies and contiguous migration are confirmed.

- [ ] T005 Add failing migration tests for immutable preview versions, current pointer, assumptions, reference snapshots, investigations, mention bindings, usage facts, existing-session preservation, idempotence, foreign keys, and rollback in `src-tauri/src/native/migrations.rs`
- [x] T006 Implement the centrally assigned contiguous migration for feature-owned preview/reference tables and additive refinement prompt/version metadata in `src-tauri/src/native/migrations.rs` and `src-tauri/src/native/schema.sql`
- [x] T007 Create `WorkPreviewVersion`, `DraftAssumption`, `PreviewReferenceSnapshot`, `Investigation`, `DocumentMention`, and `ReferenceInteraction` domain types with canonical serialization/hash/validation in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T008 Register the complete E planner/finalizer prompt instructions and typed input/output validation in `src-tauri/src/workflow_foundation/prompts/reference_aware_workbench.rs` and route them from `src-tauri/src/workflow_foundation/prompts/mod.rs`
- [x] T009 Add minimal module, compatibility-route, and job dispatch wiring without moving canonical Task/session ownership in `src-tauri/src/native/mod.rs`, `src-tauri/src/native/task_assistance.rs`, and `src-tauri/src/native/jobs.rs`
- [x] T010 [P] Add shared exact-reference, mention-token, preview-version, comparison, generation, investigation, and view-state types in `frontend/src/types/taskWorkbench.ts`
- [x] T011 Extend typed application client operations for workspace reads, generation, investigation, references, mention drafts, edit, compare, restore, usage, and apply in `frontend/src/services/taskClient.ts`
- [x] T012 [P] Define the public generic props/state contracts for `ReferenceViewer` and `DraftVersionControls`, including F/G reuse examples, in `frontend/src/components/ReferenceViewer.tsx` and `frontend/src/components/DraftVersionControls.tsx`

**Checkpoint**: Immutable append/read primitives, central prompt definitions, and reusable UI/client contracts exist; no observable feature behavior is implied yet.

---

## Phase 3: User Story 1 - Evidence-backed work preview first (Priority: P1) MVP

**Goal**: Prepare a complete structured preview without routine clarification, performing necessary retrieval before exposing the first useful preview and preserving explicit adoption authority.

**Independent Test**: Submit the contract-change fixture once, verify required retrieval finishes before a complete preview appears, inspect all nine structured areas/assumptions/exact citations, and prove the canonical Task is unchanged until an explicit apply succeeds.

### Tests for User Story 1

- [ ] T013 [P] [US1] Add failing Rust prompt/fixture tests for all structured fields, no mandatory clarification, needed/unneeded retrieval intent, no-result assumptions, exact claim sources, unsupported additions, and no adoption/execution/finality claims in `src-tauri/tests/reference_aware_workbench.rs`
- [ ] T014 [P] [US1] Add failing job/application tests for planner→necessary-retrieval→finalizer order, visible requested status/retry, equivalent coalescing, save-time source/context mismatch, and atomic review-needed disposition in `src-tauri/src/native/reference_aware_workbench.rs` and `src-tauri/src/native/jobs.rs`
- [ ] T015 [P] [US1] Add failing component tests for preview-first hierarchy, every field, assumptions/provenance, generation/search/no-result/failure states, dirty-field protection, explicit apply, focus, reduced motion, and long EN/KO copy in `frontend/src/features/workbench/ReferenceAwarePreview.test.tsx`

### Implementation for User Story 1

- [x] T016 [US1] Implement exact planner input from current refinement subject/messages/hierarchy and validate the central `RetrievalIntent` contract in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T017 [US1] Execute necessary feature 018 retrieval inside the requested generation, validate grants/current revisions, and build a bounded exact reference bundle before finalization in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T018 [US1] Implement final preview validation, assumption/evidence separation, claim-source binding, source-bundle hashing, and immutable append with `review_needed` disposition in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T019 [US1] Implement generation read/retry projections that keep the last good preview and distinguish necessary no-result/failure from optional investigation state in `src-tauri/src/native/reference_aware_workbench.rs` and `src-tauri/src/native/task_assistance.rs`
- [x] T020 [US1] Implement exact-head explicit apply through the existing canonical Task mutation transaction, recording application disposition and actual used/adopted references atomically in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T021 [US1] Build the structured preview, assumptions, provenance links, dirty-edit/queued-proposal behavior, and explicit apply controls in `frontend/src/features/workbench/ReferenceAwarePreview.tsx`
- [x] T022 [US1] Integrate `ReferenceAwarePreview` into the existing refinement workspace while keeping conversation/composer available and reusing `ContentRevisionTransition` only for confirmed apply in `frontend/src/features/workbench/RefinementPanel.tsx`
- [x] T023 [US1] Add complete English/Korean preview, assumption, provenance, requested-generation, no-result, retry, conflict, and apply copy using 사용자 in Korean in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: A full evidence-backed preview is independently usable without optional investigation, reference browsing, mentions, or version restore.

---

## Phase 4: User Story 2 - Keep talking while investigation progresses (Priority: P1)

**Goal**: Continue conversation during latest-context optional investigation and route current critical/supporting/ancillary findings without stale advice.

**Independent Test**: Start investigation for revision 1, change approver/customer count in revision 2, and verify revision-1 output becomes changed-context while revision-2 critical evidence surfaces, supporting evidence grounds the next reply, ancillary evidence stays in references, and the composer never blocks.

### Tests for User Story 2

- [ ] T024 [P] [US2] Add failing Rust concurrency tests for latest-context cancellation, changed-context disposition, no automatic durable retry, direct-investigation explicit retry, priority validation, and exactly-once supporting-evidence delivery in `src-tauri/tests/reference_aware_workbench.rs`
- [ ] T025 [P] [US2] Add failing refinement integration tests for non-blocking composer use, current critical alert, next-reply supporting context, ancillary-only references, optional failure, and no duplicate messages/findings in `src-tauri/src/native/task_assistance.rs`
- [ ] T026 [P] [US2] Add failing component tests for conversation continuity, separate optional status/failure, critical alert, no focus/scroll theft, and stale arrival suppression in `frontend/src/features/workbench/RefinementPanel.test.tsx`

### Implementation for User Story 2

- [x] T027 [US2] Implement latest-wins optional search/conflict investigation keyed by exact context revision using the foundation speculative policy in `src-tauri/src/native/reference_aware_workbench.rs` and `src-tauri/src/native/jobs.rs`
- [x] T028 [US2] Validate and persist critical/supporting/ancillary findings with exact source bindings and changed-context guards in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T029 [US2] Inject each current supporting finding at most once into the next reply context while keeping ancillary findings out of unsolicited chat in `src-tauri/src/native/task_assistance.rs`
- [x] T030 [US2] Render current critical findings and separate optional investigation searching/no-result/failure/changed-context states without disabling the composer in `frontend/src/features/workbench/RefinementPanel.tsx` and `frontend/src/features/workbench/ReferenceAwarePreview.tsx`

**Checkpoint**: Conversation and current evidence remain coherent under changing context without the reference browser or version UI.

---

## Phase 5: User Story 3 - Inspect and mention many references (Priority: P2)

**Goal**: Search/filter/sort stable reference rows, open exact safe Markdown passages, and insert one or many bound document/section mentions without sending.

**Independent Test**: With duplicate titles, changing paths, multiple versions/sections, redirects, and asynchronous arrivals, keep the row under focus stable, navigate exact passages in the viewer, restore focus/input/scroll on close, insert multiple mentions, and prove viewed/mentioned/used/adopted/excluded remain distinct.

### Tests for User Story 3

- [ ] T031 [P] [US3] Add failing Rust/API tests for dedupe, stable epoch ordering, deterministic sorts, paging, exact grant-bound reads, archive/path survival, usage separation, and actual-used/counterevidence export in `src-tauri/tests/reference_aware_workbench.rs`
- [ ] T032 [P] [US3] Add failing component tests for 1,000-row search/filter/sort, arrival stability, duplicate titles, all empty/error states, keyboard operation, and narrow layout in `frontend/src/features/workbench/ReferenceList.test.tsx`
- [ ] T033 [P] [US3] Add failing shared viewer tests for safe Markdown/links, exact version/section, back/forward/previous/next, Escape/focus trap, origin/input/selection/scroll restoration, long EN/KO, and revoked grants in `frontend/src/components/ReferenceViewer.test.tsx`
- [ ] T034 [P] [US3] Add failing mention tests for document/section lookup, duplicate-title disambiguation, multiple ordered tokens, keyboard/IME behavior, deletion, save/reload, plain-text paste fallback, and no auto-send in `frontend/src/features/workbench/ReferenceMentionInput.test.tsx`

### Implementation for User Story 3

- [x] T035 [US3] Implement reference projection/search/filter/stable-sort epochs over feature 018 results without copying canonical document/search storage in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T036 [US3] Implement exact grant-bound reference opening plus separate viewed/mentioned/used/excluded facts and actual-use/counterevidence export in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T037 [US3] Build the shared safe `ReferenceViewer` with exact section focus, navigation history, previous/next, keyboard close, focus trap, and context restoration in `frontend/src/components/ReferenceViewer.tsx`
- [x] T038 [US3] Build the stable searchable/filterable/sortable reference rail with pending-arrival grouping and explicit reorder activation in `frontend/src/features/workbench/ReferenceList.tsx`
- [x] T039 [US3] Build the bound document/section/multi-mention composer lookup with no auto-send in `frontend/src/features/workbench/ReferenceMentionInput.tsx` and integrate it in `frontend/src/features/workbench/RefinementPanel.tsx`
- [x] T040 [US3] Add reusable viewer/list/mention styling with visible focus, roomy targets, stable wide/narrow hierarchy, existing tokens, and reduced-motion rules in `frontend/src/features/workbench/reference-aware-workbench.css`
- [x] T041 [US3] Add complete English/Korean reference, filter/sort, viewer navigation, usage, grant, and mention copy in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: Reference browsing and mentions work independently of preview-version restore; F/G can reuse the integrated viewer contract.

---

## Phase 6: User Story 4 - Choose a working version (Priority: P2)

**Goal**: Read and compare immutable previews and restore an old version as a new preview without changing actual work history or overwriting edits.

**Independent Test**: Create versions 1–3, compare exact fields/assumptions/reference snapshots, restore 1 as 4, and prove versions 1–3 plus Task/Work Log/decision/journey/message state remain unchanged; reading old versions provides no apply action.

### Tests for User Story 4

- [ ] T042 [P] [US4] Add failing Rust/API tests for immutable edit append, exact version reads, field/reference comparison, restore-as-new, operation replay/conflict, queued proposals during editing, and zero Task/history mutation in `src-tauri/tests/reference_aware_workbench.rs`
- [ ] T043 [P] [US4] Add failing shared control tests for current/historical/derived labels, two-version field diff, assumption/reference diff, wide/narrow semantics, read-only history, restore conflict, keyboard order, and focus retention in `frontend/src/components/DraftVersionControls.test.tsx`
- [ ] T044 [P] [US4] Add failing integration tests proving historical navigation/rerender produces no transition, successful restore/apply uses the right transition cause, reduced motion suppresses it, and dirty edits survive arrivals in `frontend/src/features/workbench/ReferenceAwarePreview.test.tsx`

### Implementation for User Story 4

- [x] T045 [US4] Implement immutable edited preview versions, exact reads, current-pointer projection, and deterministic field/assumption/reference comparison in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T046 [US4] Implement restore-as-new with exact expected head, copied source snapshot, derivation provenance, and assertions that Task/Work Log/decision/journey/message stores remain untouched in `src-tauri/src/native/reference_aware_workbench.rs`
- [x] T047 [US4] Build generic version selection, two-version comparison, responsive field diffs, and restore-as-new controls for E/F reuse in `frontend/src/components/DraftVersionControls.tsx`
- [x] T048 [US4] Integrate historical read-only mode, queued proposal badges, dirty-edit protection, and shared transition causes in `frontend/src/features/workbench/ReferenceAwarePreview.tsx`
- [x] T049 [US4] Add complete English/Korean version, comparison, derivation, current/historical, restore, dirty-edit, queued-proposal, and conflict copy in `frontend/src/features/workbench/taskWorkbenchText.ts`

**Checkpoint**: All four stories work while Knowledge generation/publication and archive organization remain out of scope.

---

## Phase 7: Polish, documentation, and risk-based validation

**Purpose**: Complete current-behavior documentation, performance evidence, accessibility review, and integration checks after implementation.

- [x] T050 [P] Add paired current-behavior guides for structured previews, background investigation, exact references/mentions, immutable versions, restore-as-new, and explicit adoption in `docs/features/reference-aware-workbench.md` and `docs/features/reference-aware-workbench.ko.md`
- [x] T051 [P] Link the paired guides from `docs/features/README.md` and `docs/features/README.ko.md` and review `docs/DOCUMENTATION_GUIDE.md` for any other affected shipped-behavior docs
- [ ] T052 Add stable performance tests for a 1,000-reference projection/filter/sort and record preview planning, retrieval, provider, validation, and application stage measurements without a new unmeasured latency promise in `src-tauri/src/native/reference_aware_workbench.rs` and `specs/019-reference-aware-workbench/quickstart.md`
- [x] T053 Run focused `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, retrieval regression tests, migration tests, and `git diff --check` separately and record actual results in `specs/019-reference-aware-workbench/quickstart.md`
- [ ] T054 Perform rendered wide/narrow English/Korean keyboard/focus/IME/reduced-motion review using the full interaction matrix and record evidence/limitations in `specs/019-reference-aware-workbench/quickstart.md`
- [x] T055 Assess remaining Tauri restart/modal/deep-link risk after focused tests; record the E2E skip rationale or run only the smallest named desktop scenario after one final release build and record it in `specs/019-reference-aware-workbench/quickstart.md`
- [ ] T056 Run `$speckit-converge` against the integrated implementation and append only demonstrably unbuilt work to `specs/019-reference-aware-workbench/tasks.md`

---

## Dependencies and execution order

### Phase dependencies

- Phase 1 blocks all code/schema work and prevents migration gaps.
- Phase 2 blocks every user story.
- User Story 1 is the MVP and supplies preview/reference snapshots.
- User Story 2 depends on the current context and reference snapshot primitives from User Story 1, but not on reference UI or versions.
- User Story 3 depends on foundational exact reference types and may proceed after User Story 1’s reference snapshot contract stabilizes.
- User Story 4 depends on immutable preview versions from User Story 1 and shared component ownership from Phase 2; it does not depend on optional investigation.
- Phase 7 follows all stories.

### Parallel opportunities

- In Phase 2, frontend types/public component shells can proceed after native schema/type contracts are fixed and touch separate files.
- Within each story, Rust fixtures and separate component test files marked `[P]` can proceed independently.
- User Story 2 native work and User Story 3 shared viewer/list work can proceed after User Story 1 contracts are integrated, provided `RefinementPanel.tsx` edits are serialized.
- Feature 020 may adapt to the shared viewer/version contracts only after T037/T047 integrate; it must not duplicate them.

## Implementation strategy

Deliver User Story 1 first: one structured preview, necessary retrieval, exact provenance, and explicit apply. Add non-blocking latest-context investigation next. Then add the many-reference viewer/mentions and immutable version comparison/restore. Keep `task_assistance.rs` and `RefinementPanel.tsx` as thin integration shells, centralize E domain logic, and preserve B cleanup, F Knowledge, and G archive ownership boundaries.


## Native runtime checkpoint (2026-09-26)

The existing refinement message and durable preview job now execute registered reply,
planner, retrieval, and finalizer calls. The native implementation lives in
`native/reference_aware_workbench/runtime.rs`; provider/command regression fixtures live
in `runtime_tests.rs`. The schema/domain checkpoints alone are not considered runtime
completion. The later live-consumer checkpoint below records the implemented React integration
and the remaining portions of the broader acceptance tasks.

Native reads use D's persisted search units and exact local Vault revision reads.
This desktop-owned scope does not borrow MCP connection grants or expose arbitrary
path reads; MCP access remains on D's connection/grant boundary. All accepted planner
aspects/filters affect retrieval, and requery is bounded to two extra queries.

Standalone generation and explicit investigation were subsequently wired in the live
consumer rescue. Requested generation uses the registered durable job; optional
investigation remains bounded and does not create an automatic durable retry job.


## Live consumer task reconciliation (2026-09-26, before completion follow-up)

This is an evidence-based status update for the E checkpoints through `7a1ff46`,
including shared controls `86eef81` and `67a51e2`. Checked implementation tasks mean
that their described behavior exists; they do not imply every broader acceptance test
or final packaged review has passed. No application code changed in this audit.
The actual implementation uses `reference_aware_workbench/runtime.rs`,
`runtime_tests.rs`, `referenceWorkbenchText.ts`, and the mounted
`RefinementPanel.references.test.tsx`; the planned filenames are not completion gates.

### Completed implementation evidence

| Tasks | Evidence |
| --- | --- |
| T003, T004, T006 | `plan.md` native checkpoint records D `1e9c80c`, E20 `c3302c6`, migration test `bde8703`, domain `43b484c`, and dispatch/Task transaction owners. `migrations.rs::add_reference_aware_workbench` applies the feature schema after D19. |
| T010, T011, T012 | `types/taskWorkbench.ts`, reference operations in `services/taskClient.ts`, exported `ReferenceViewerProps` and `DraftVersionControlsProps`; public consumer notes in `contracts/interaction.md`. |
| T017, T019 | `runtime.rs::retrieve`, `prepare`, workspace read, and standalone generation/investigation routing in `task_assistance.rs`; provider tests `actual_retrieval_applies_aspects_filters_and_bounded_requery` and `standalone_initial_generation_uses_exact_heads_and_registered_job`. Local desktop authority is checked; connection-scoped MCP grants remain owned by D. |
| T022, T030 | `RefinementPanel.tsx` initial load, polling, automatic generation, persisted mentions, exact edit/apply adapters, and separate optional investigation. Mounted tests cover first generation, editing conflicts, version/source reads, and conversation during investigation. `ReferenceAwarePreview.tsx` renders critical findings and distinct optional states. |
| T037, T040 | `ReferenceViewer.tsx`, shared modal hook, safe Markdown renderer and `reference-aware-workbench.css`; viewer tests cover exact history, late-response rejection, retry, relative section links and opener focus. Rendered wide/narrow EN/KO and keyboard evidence is recorded in `quickstart.md`. |
| T047, T048, T049 | Shared version controls plus exact native fetch/compare adapters, historical Apply suppression, restore-as-new, dirty edits and queued-head notice. `ReferenceAwarePreview.test.tsx` and mounted panel tests exercise these boundaries. English/Korean labels live in `referenceWorkbenchText.ts`. |
| T050, T051 | Paired `docs/features/reference-aware-workbench.md` guides and feature indexes describe the wired behavior without claiming the remaining reference/mention options. |
| T053, T055 | Actual focused/full npm, native provider/regression, typecheck/lint and whitespace results plus the risk-based packaged deferral are recorded in `quickstart.md`. The final integrated packaged lifecycle remains a separate program verification step. |

### Remaining implementation or contract work

These tasks remain unchecked because concrete portions are still absent or partial.

| Tasks | Remaining work and current evidence |
| --- | --- |
| T001, T002 | Finish the accepted A/B dependency signatures and F/G ownership reconciliation in the original cross-feature planning records. E shared props are exported and F consumes them, but this audit did not complete feature 020's plan record. |
| T007 | Native operations validate and hash JSON values. The full planned named Rust domain-type layer is not implemented; do not claim those named types exist. |
| T021, T023 | Structured fields, assumption text/basis, sources, editing and apply exist. Assumption status is not rendered or localized, so the complete assumption presentation remains partial. |
| T035, T038, T041 | The rail supports text/status filtering, reading/title order and stable append behavior. It does not yet implement document-level section grouping, aspect/type/usage filters, paging, or a separate pending-arrival group with explicit acceptance. Related full option copy remains incomplete. Native reference-list returns the snapshot or bounded lookup results. |
| T036 | Exact local desktop reads, moved-source resolution and separate usage facts exist. The planned connection/grant-bound read and complete actual-use/counterevidence export acceptance must be reconciled with D/F ownership and verified; desktop scope is not proof of MCP grant behavior. |
| T039 | Repeated exact document/section insertion, persisted unsent bindings and no auto-send work. Batch multi-select, Space insertion and atomic Backspace/Delete token editing are not implemented. |
| T052 | The 1,000-reference fixture measures filter/order rendering, and native runs record planner/retrieval/finalizer durations. Separate validation/application stage measurements are not recorded. |

### Remaining verification and convergence

The following unchecked tasks have existing partial coverage; unchecked does not mean
that their underlying feature is wholly absent.

| Tasks | Evidence already present; remaining verification |
| --- | --- |
| T005 | `v20_preserves_existing_sessions_and_reopens_idempotently` checks session preservation, table creation, repeat migration and foreign keys. Complete E-specific immutability/rollback/table coverage remains. |
| T013, T014 | Runtime provider fixtures cover retrieval ordering/outcomes, actual claim use, no automatic adoption, delayed source/user edits and atomic apply. The complete prompt-semantic and equivalent-job coalescing/retry matrix is not established by these E fixtures. |
| T015, T026 | Mounted tests cover first preview, dirty edits, source/version adapters and composer continuity. Full field/assumption/error, critical-arrival, scroll and reduced-motion matrices remain unverified. |
| T024, T025 | Native tests cover delayed optional work, changed context, no durable speculative job, once-only supporting delivery and ancillary exclusion. Explicit investigation retry/failure and every critical-priority integration case need dedicated assertions. |
| T031, T032 | Native exact-source/move and usage tests plus the 1,000-row component fixture exist. Full paging/filter/grouping, grant, archive survival, export and accessibility cases depend on the incomplete implementation above. |
| T033, T034 | Viewer history/race/relative-link/focus and mention persistence/IME-composition tests pass. Full duplicate-title, multi-token editing, paste fallback, origin selection/scroll and revoked-grant cases remain. |
| T042, T043, T044 | Native append/restore and mounted exact compare/restore/edit conflict tests pass. Full untouched Work Log/decision/journey assertions, assumption/reference diff semantics and transition-cause/reduced-motion matrix remain. |
| T054 | Wide/narrow EN/KO production fixtures and keyboard source navigation were rendered. Physical Korean IME, OS reduced-motion and the complete interaction matrix were not reviewed. |
| T056 | A full integrated Spec Kit convergence pass has not run. This bounded documentation audit is not a claim of convergence. |

No new task IDs are appended here: all observed remaining work maps to existing
unchecked tasks. The final integrated convergence pass remains responsible for any
additional gaps discovered after the E/F/G merge.


## Observable interaction completion follow-up (2026-09-26)

This later checkpoint resolves the implementation gaps from the preceding audit:

- T007 uses the approved strict JSON boundary described in `plan.md`; no unused Rust
  facade is added. Runtime schema/validation/CAS checks remain authoritative.
- T021/T023 show and localize persisted assumption status independently of basis.
  `ReferenceAwarePreview.test.tsx` tests the exact persisted status projection.
- T035/T038/T041 are implemented in `ReferenceList.tsx`: text/status/aspect/type/use
  filters, document-version grouping, 20-document paging, stable reading epochs and
  explicit acceptance of pending sources/sections. Native reference metadata and
  actual usage facts feed this derived view. `ReferenceList.test.tsx` exercises 1,000
  documents, exact sections, combined filters, discovery/use separation, paging and
  arrival/focus preservation; the mounted preview test exercises its real consumer.
- T039 now supports ordered batch selection, Enter/Space insertion and atomic
  Backspace/Delete. `referenceMentions.test.ts` covers duplicate visible tokens,
  exact binding removal, partial selections, offset shifts and plain-text paste.
  Mounted panel tests assert persistence through the native adapter and no auto-send.
- The canonical apply callback now carries the exact successful Task revision as
  `TaskApplicationEvent`; unrelated workflow changes carry no adoption cause. The
  mounted provider-response fixture uses the native `{task, transitionCause, version}`
  shape and asserts the event.

Remaining unchecked implementation/contract work is T001/T002 (cross-feature records),
T036 (D grant/F export boundary acceptance) and T052 (validation/application timing).
Unchecked test/review tasks remain T005, T013–T015, T024–T026, T031–T034, T042–T044,
T054 and T056; their earlier partial evidence remains valid. The new component layout
could not be rendered in this turn because no CUA browser surface or local browser
runtime was available. This does not erase the earlier rendered rescue evidence and
is not a claim that the new controls received visual approval.


## T036 provenance and authority closure (2026-09-26)

The earlier T036 audit gap is resolved by `native/reference_provenance.rs` and its
command/provider integration tests in `native/reference_provenance_tests.rs`. Export
joins Capture work only through a recorded successful acceptance/application decision
for the exact Task; it never guesses from `origin_capture_id`. Task-owned and adopted
Capture usage coalesce by document/version/section, retain recorded claim statements
and counterevidence, and omit discovery/view/mention-only or excluded sources. Missing
legacy rationale fails visibly. Usage changes invalidate F/G source snapshot guards.

Desktop exact opening uses the visible refinement subject and stored exact binding,
then D's index and Vault hash validation (`reference_aware_workbench/runtime.rs`). MCP
reads separately retain connection, expiry and revision grant validation in
`adapters/vault/mod.rs::evidence_read`; desktop reads do not borrow MCP grants. This
reconciles the original generic grant wording with the two real authority boundaries.

Verification: 2 provenance tests and all 14 reference runtime tests passed. The actual
Capture generation/apply → Task completion → F generation snapshot/private draft → G
archive preparation test checks exact counterevidence links, excluded/viewed/candidate
non-export, unrelated Task isolation, and stale snapshots after exclusion. The broader
knowledge filter passed 36/39 on this pre-integration branch; its three failures are
existing schema/prompt and Knowledge fixture expectations repaired separately on main.
No other unchecked task is closed by this checkpoint.
