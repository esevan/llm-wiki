# Tasks: Conflict Resolution Workflow

**Input**: Design documents from `specs/010-conflict-resolution-workflow/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Required by the feature request. Test tasks precede their implementation tasks.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm existing project safeguards before feature work.

- [X] T001 Verify repository ignore rules and current conflict-review touch points against `specs/010-conflict-resolution-workflow/plan.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish the durable Conflict and Resolution model used by every story.

- [X] T002 Add failing Conflict/Resolution validation and persistence tests in `tests/test_workflow.py`
- [X] T003 Add run-scoped conflict and resolution schema plus atomic workflow methods in `llm_wiki/services/workflow.py`
- [X] T004 Add resolution request schemas in `llm_wiki/controllers/schemas.py`

**Checkpoint**: Structured conflicts and human resolutions can be validated and stored without provider or UI dependencies.

---

## Phase 3: User Story 1 - Understand each conflict (Priority: P1) 🎯 MVP

**Goal**: Return normalized structured conflicts and render each as a scannable card with expandable evidence.

**Independent Test**: A multi-severity review result exposes and displays every required conflict field without parsing prose.

- [X] T005 [P] [US1] Add failing structured handler-result tests in `tests/test_ai_jobs.py`
- [X] T006 [P] [US1] Add failing conflict-card and severity browser contract tests in `tests/test_browser_menu.py`
- [X] T007 [US1] Expand and normalize the evidence-review result contract in `llm_wiki/services/handlers/conflict_review.py`
- [X] T008 [US1] Persist normalized conflicts when a review finishes in `llm_wiki/services/workflow.py`
- [X] T009 [US1] Replace primary raw report markup with conflict cards and expandable evidence in `llm_wiki/static/index.html`
- [X] T010 [P] [US1] Add English and Korean card labels and fallbacks in `llm_wiki/static/i18n/en.json` and `llm_wiki/static/i18n/ko.json`

**Checkpoint**: Multi-conflict reviews are independently readable while conflict-free results remain concise.

---

## Phase 4: User Story 2 - Resolve every conflict (Priority: P1)

**Goal**: Let the user choose and validate one action per card, save the complete set, and continue through existing gate semantics.

**Independent Test**: Mixed actions update exact counts; acceptance without rationale remains unresolved; complete valid submission persists atomically and maps to the correct Solution gate state.

- [X] T011 [P] [US2] Add failing resolution endpoint and atomic rollback tests in `tests/test_api.py`
- [X] T012 [P] [US2] Add failing radio, rationale, summary, Continue, and save-failure browser tests in `tests/test_browser_menu.py`
- [X] T013 [US2] Add the review resolution endpoint and stale-source validation in `llm_wiki/controllers/application.py`
- [X] T014 [US2] Implement per-card radio state, rationale validation, live counts, and persistence submission in `llm_wiki/static/index.html`
- [X] T015 [P] [US2] Add English and Korean resolution, validation, summary, and outcome copy in `llm_wiki/static/i18n/en.json` and `llm_wiki/static/i18n/ko.json`

**Checkpoint**: Every conflict requires a valid explicit human decision before Continue; gate behavior distinguishes accepted exceptions from required revision.

---

## Phase 5: User Story 3 - Resume and reuse resolution history (Priority: P2)

**Goal**: Restore persisted decisions after restart and keep legacy findings-only results readable.

**Independent Test**: Reopening a resolved run returns linked actions, rationales, and timestamps; an old report fixture renders safe card fallbacks.

- [X] T016 [P] [US3] Add failing persisted-read and legacy-report compatibility tests in `tests/test_api.py` and `tests/test_ai_jobs.py`
- [X] T017 [US3] Attach saved resolutions to review reads and normalize cached legacy findings in `llm_wiki/services/workflow.py` and `llm_wiki/services/handlers/conflict_review.py`
- [X] T018 [US3] Restore saved card state and legacy fallbacks in `llm_wiki/static/index.html`

**Checkpoint**: Conflict → Resolution → Rationale history survives restart and earlier queue results remain usable.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verify long-dialog usability, performance, documentation, and regression safety.

- [X] T019 Add sticky dialog layout, focus-visible states, non-color severity cues, and 20-card performance coverage in `llm_wiki/static/index.html` and `tests/test_browser_menu.py`
- [X] T020 [P] Document the user-facing workflow in `docs/features/conflict-resolution-workflow.md` and `docs/features/conflict-resolution-workflow.ko.md`
- [X] T021 [P] Add the new guide to `docs/features/README.md` and `docs/features/README.ko.md`
- [X] T022 Run the full pytest suite, exact browser script parse check, and `git diff --check` from `specs/010-conflict-resolution-workflow/quickstart.md`

---

## Dependencies & Execution Order

- Phase 1 precedes all work.
- Phase 2 blocks all user stories.
- User Story 1 establishes the structured result and card UI required by User Story 2.
- User Story 2 establishes persistence submission required by User Story 3 read/restore behavior.
- Phase 6 follows all user stories.
- Tasks marked `[P]` touch separate files or are test/documentation work that can be prepared independently after their phase prerequisite.

## Requirement Coverage

- **US1 / FR-001–FR-005, FR-010, FR-015, FR-020**: T005–T010
- **US2 / FR-006–FR-009, FR-011, FR-013–FR-017**: T011–T015
- **US3 / FR-012, FR-020**: T016–T018
- **FR-018–FR-019 and SC-005–SC-006**: T019–T022

## Implementation Strategy

1. Build and validate normalized persistence first.
2. Deliver the readable card result as the first independently demonstrable slice.
3. Add explicit resolution interactions and gate mapping.
4. Restore persisted/legacy histories.
5. Finish accessibility, long-review performance, bilingual documentation, and full regression verification.
