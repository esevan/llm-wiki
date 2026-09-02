# Tasks: Task-Level AI Model Routing

**Input**: Design documents from `/specs/006-ai-task-model-routing/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/provider-config.md, quickstart.md

**Tests**: Provider-routing, configuration-route, full-suite, and browser-script checks are required because the feature changes persisted settings and every AI invocation path.

## Phase 1: Setup and Baseline

- [X] T001 Record the two-tier task-routing behavior and validation cases in `specs/006-ai-task-model-routing/spec.md` and `specs/006-ai-task-model-routing/quickstart.md`

---

## Phase 2: Foundational Model Resolution

- [X] T002 Add stable AI task identifiers, documented defaults, persisted Advanced model, and persisted task selections in `llm_wiki/services/settings.py`
- [X] T003 Add provider-setting tests for Advanced selection, Default selection, and blank-Advanced fallback in `tests/test_provider.py`
- [X] T004 Implement two-tier resolution so selected tasks use Advanced only when configured and otherwise use Default in `llm_wiki/services/settings.py`

**Checkpoint**: Every named task resolves a model without stage-specific configuration.

---

## Phase 3: User Story 1 - Configure Two Model Tiers (Priority: P1)

**Goal**: A person can save and reopen exactly a Default and an Advanced model.

**Independent Test**: Save configuration through the provider route and confirm the returned public configuration contains the two model tiers without API-key disclosure.

- [X] T005 [US1] Update the provider configuration input and save route for Default model, Advanced model, and task selections in `llm_wiki/api/app.py`
- [X] T006 [US1] Cover the updated provider configuration route in `tests/test_api.py`
- [X] T007 [US1] Replace stage-model fields with Default and Advanced model inputs and explanatory tooltips in `llm_wiki/static/index.html`

---

## Phase 4: User Story 2 - Choose a Model Tier Per AI Task (Priority: P1)

**Goal**: A person can expand Advanced options and set the Advanced tier for each named AI task.

**Independent Test**: Toggle a task, save, reload settings, and verify the selection is retained.

- [X] T008 [US2] Add the collapsed Advanced options task list and persistence wiring in `llm_wiki/static/index.html`
- [X] T009 [US2] Route discussion, refinement, drafting, organization, completed-record discussion, conflict review, image summary, completion review, completion report, and enrichment by named task in `llm_wiki/api/app.py`
- [X] T010 [US2] Set Advanced defaults for discussion/refinement, drafting, review, image summary, and reports while leaving the documented routine tasks on Default in `llm_wiki/services/settings.py`
- [X] T011 [US2] Add regression coverage that Capture, Problem, and Solution discussion/refinement use the Advanced tier by default in `tests/test_provider.py`

---

## Phase 5: User Story 3 - Receive Safe Model Fallbacks (Priority: P2)

**Goal**: A selected Advanced task remains usable with the Default model when no Advanced model is set.

**Independent Test**: Select image summary for Advanced, leave Advanced model blank, and verify it resolves the Default model.

- [X] T012 [US3] Add the Advanced-to-Default fallback regression case in `tests/test_provider.py`
- [X] T013 [US3] Correct independent task routing for conflict review and image summary in `llm_wiki/api/app.py`
- [X] T014 [US3] Explain the fallback behavior in the Advanced options status copy in `llm_wiki/static/index.html`

---

## Phase 6: Documentation and Validation

- [X] T015 Update the two-tier model-routing user documentation in `README.md`, `docs/CONTINUATION.md`, and `docs/WORKBENCH_CHECKLIST.md`
- [X] T016 Run the full test suite with `uv run pytest -q` and address feature-related failures in `tests/` and `llm_wiki/`
- [X] T017 Parse the browser script from `llm_wiki/static/index.html` with the repository's required Node command and run `git diff --check`
- [X] T018 Review `llm_wiki/services/settings.py`, `llm_wiki/api/app.py`, `llm_wiki/static/index.html`, and `tests/` against FR-001 through FR-010 in `specs/006-ai-task-model-routing/spec.md`

## Dependencies and Execution Order

- Phase 2 establishes routing and blocks user-facing configuration work.
- US1 requires Phase 2; US2 requires the two-tier configuration from US1; US3 requires both the task map and the Advanced model field.
- Documentation and final validation follow all user stories.

## Parallel Opportunities

- T003 and T006 can run in parallel because they modify different test files.
- Documentation task T015 can run in parallel with final validation after the implementation tasks are complete.

## Implementation Strategy

The MVP is Phase 2 plus User Story 1: a two-tier configuration with safe Default fallback. User Story 2 adds user-controlled task routing, and User Story 3 locks down fallback behavior. All listed tasks are marked complete because implementation and verification preceded this backfilled Spec-Kit record.
