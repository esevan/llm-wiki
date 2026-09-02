# Tasks: Lineage Knowledge Layer

**Input**: Design documents from `specs/010-lineage-knowledge-layer/`

## Phase 1: Setup

- [x] T001 Verify repository ignore rules and implementation prerequisites in `.gitignore` and `.specify/feature.json`
- [x] T002 Add the lineage service module skeleton in `llm_wiki/services/lineage.py`

## Phase 2: Foundational

- [x] T003 Add failing schema and decision-event tests in `tests/test_workflow.py`
- [x] T004 Add lineage, conflict-address, correction, and completion contract tests in `tests/test_api.py`
- [x] T005 Add completed-workspace Lineage graph and responsive interaction tests in `tests/test_browser_menu.py`
- [x] T006 Add append-only decision, conflict-address, snapshot, claim, evidence, revision, and Playbook metadata tables in `llm_wiki/services/workflow.py`
- [x] T007 Implement decision-event recording for Solution creation, edits, approval, conflict address, and completion in `llm_wiki/services/workflow.py`
- [x] T008 Implement provenance, evidence, conflict-state, and inference validation primitives in `llm_wiki/services/lineage.py`

## Phase 3: User Story 1 — Understand completed work from origin to evidence (P1)

**Goal**: Automatically preserve and display the four-stage lineage and generate final documents from it.

**Independent Test**: Complete a sourced Solution and verify the Lineage API, completed workspace, and Markdown contain Capture → Problem → Solution → Complete with evidence-backed transitions.

- [x] T009 [US1] Implement deterministic four-stage snapshot assembly and idempotent persistence in `llm_wiki/services/lineage.py` and `llm_wiki/services/workflow.py`
- [x] T010 [US1] Implement Lineage-based report context and final Markdown section rendering in `llm_wiki/services/lineage.py` and `llm_wiki/services/workflow.py`
- [x] T011 [US1] Integrate snapshot-before-report generation into both completion and Playbook regeneration paths in `llm_wiki/api/app.py`
- [x] T012 [US1] Add `GET /api/features/{feature_id}/lineage` and evidence read endpoints in `llm_wiki/api/app.py`
- [x] T013 [US1] Render responsive four-card Lineage graph, transition decisions, provenance badges, and evidence drill-down in `llm_wiki/static/index.html`

## Phase 4: User Story 2 — Trace decisions and conflicts to evidence (P1)

**Goal**: Expose source-backed decisions and strict conflict-address semantics without invented resolution.

**Independent Test**: Record unsupported and supported conflict transitions and verify only the supported address becomes Addressed with basis, disposition, and evidence.

- [x] T014 [US2] Extend conflict decision request validation and persistence in `llm_wiki/api/app.py` and `llm_wiki/services/workflow.py`
- [x] T015 [US2] Assemble Decision Changes and Conflicts & Addresses with material-conflict selection in `llm_wiki/services/lineage.py`
- [x] T016 [US2] Validate optional inference against opaque evidence IDs and add inference-enabled regeneration in `llm_wiki/api/app.py` and `llm_wiki/services/lineage.py`
- [x] T017 [US2] Render expandable decision/conflict detail and source basis in `llm_wiki/static/index.html`

## Phase 5: User Story 3 — Navigate from lineage to the Problem (P2)

**Goal**: Open the source Problem from the graph without reopening completed refinement.

**Independent Test**: Select the Problem card and verify the correct completed Problem opens read-only; deleted live records retain their snapshot excerpt.

- [x] T018 [US3] Add completed Problem read-only context to the lineage response in `llm_wiki/services/workflow.py` and `llm_wiki/api/app.py`
- [x] T019 [US3] Implement Problem-card navigation and missing-live-record fallback in `llm_wiki/static/index.html`

## Phase 6: User Story 4 — Correct knowledge without erasing history (P2)

**Goal**: Append user corrections while preserving prior AI interpretations and source evidence.

**Independent Test**: Correct an inferred claim and verify the new current revision, old AI revision, and immutable evidence survive reload and regeneration.

- [x] T020 [US4] Implement atomic append-only claim correction and correction carry-forward in `llm_wiki/services/workflow.py` and `llm_wiki/services/lineage.py`
- [x] T021 [US4] Add correction endpoint with stale-revision and external-document conflict handling in `llm_wiki/api/app.py`
- [x] T022 [US4] Add correction and audit-history UI for inferred claims in `llm_wiki/static/index.html`

## Phase 7: Polish & Cross-Cutting Concerns

- [x] T023 Add deterministic assembly/read performance coverage in `tests/test_performance.py`
- [x] T024 Complete API, workflow, transition, provider-failure, document-input, and browser regression coverage in `tests/`
- [x] T025 Update English and Korean completion/Lineage feature guides and indexes in `docs/features/`
- [x] T026 Run `uv run pytest -q`, exact browser-script parse validation, and `git diff --check`
- [x] T027 Mark completed tasks, review all requirements against implementation, and commit the feature worktree in `specs/010-lineage-knowledge-layer/tasks.md`

## Follow-up: Document-coupled lifecycle

- [x] T028 Rebuild current Lineage before completion-document generation and regeneration in `llm_wiki/api/app.py`
- [x] T029 Prevent external document conflicts from advancing Lineage independently in `llm_wiki/api/app.py`
- [x] T030 Add dedicated Advanced-model routing for Lineage interpretation in `llm_wiki/services/settings.py` and `llm_wiki/static/index.html`
- [x] T031 Remove user-facing Lineage version presentation while preserving internal audit and correction history
- [x] T032 Add lifecycle, model-routing, conflict, and browser regression tests and update paired documentation

## Follow-up: Reference and stage readability

- [x] T033 Replace per-claim Source labels with stable Lineage-wide Reference numbers and contextual popovers
- [x] T034 Add system-locale stage timestamps and read-only navigation for Capture, Problem, and Solution
- [x] T035 Replace absent transition rationale with deterministic Recorded change summaries
- [x] T036 Add responsive browser, workflow, API, and paired-documentation coverage for the feedback

## Dependencies

- Setup completes before Foundational work.
- T006–T008 block every user story.
- US1 establishes the snapshot and projection consumed by US2–US4.
- US2 and US3 can proceed independently after US1.
- US4 depends on persisted claims/revisions from US1 and evidence validation from US2.
- Polish follows all user stories.

## Parallel opportunities

- T003, T004, and T005 touch separate test files and can be prepared in parallel.
- After T009–T012 establish the backend contract, T013 UI work can proceed alongside T014 conflict persistence.
- T018 backend navigation context can proceed alongside T017 decision/conflict UI.
- Documentation T025 can begin after API/UI behavior stabilizes while remaining tests are completed.

## Implementation strategy

1. Deliver the deterministic snapshot, final-document pipeline, API, and four-stage graph as the MVP (US1).
2. Add strict decision/conflict provenance (US2).
3. Add Problem navigation (US3).
4. Add append-only corrections (US4).
5. Finish performance, regression, documentation, and stable verification gates.

## Follow-up: Human-readable document evidence

- [X] T037 Add failing report-context and completion-document tests proving internal UUIDs are absent and readable evidence labels remain in `tests/test_api.py`
- [X] T038 Add a dedicated human-readable completion-report projection while retaining opaque IDs for inference validation in `llm_wiki/services/lineage.py`
- [X] T039 Bind completion report generation to readable labels and explicitly prohibit internal IDs in `llm_wiki/api/app.py`
- [X] T040 Update paired Lineage documentation and complete stable verification
