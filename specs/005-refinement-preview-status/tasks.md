# Tasks: Refinement Preview Status

**Input**: Design documents from `/specs/005-refinement-preview-status/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api.md, quickstart.md

**Tests**: UI state verification is an explicit acceptance criterion, so tests are written before each implementation slice.

**Organization**: Tasks are grouped by user story and executed in dependency order within the dedicated feature worktree.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it touches a different file and has no incomplete dependency.
- **[Story]**: Maps the task to the corresponding user story in spec.md.

## Phase 1: Setup and Baseline

**Purpose**: Confirm the existing worktree and refinement behavior are healthy before feature edits.

- [X] T001 Run the existing workflow and browser-menu baselines with `uv run pytest -q tests/test_workflow.py tests/test_workbench_flow.py tests/test_browser_menu.py` and record any pre-existing failure before editing `llm_wiki/` or `tests/`
- [X] T002 Verify `.gitignore` already covers Python caches, virtual environments, build output, logs, environment files, editor files, and OS metadata in `.gitignore`

---

## Phase 2: Foundational Context Contract

**Purpose**: Provide one bounded, deterministic, same-item context source shared by loading, ready, and failure flows.

**⚠️ CRITICAL**: User-story UI work depends on this contract.

- [X] T003 [P] Add failing same-item selection, ordering, title-only, three-entry, 500-character, and timing tests for refinement context in `tests/test_workflow.py`
- [X] T004 [P] Add failing Problem/Solution success, missing-item, and Capture-exclusion contract tests in `tests/test_workbench_flow.py`
- [X] T005 Implement deterministic bounded refinement context selection without provider or vault calls in `llm_wiki/services/workflow.py`
- [X] T006 Implement the read-only Problem/Solution refinement-context endpoint and error mapping in `llm_wiki/api/app.py`
- [X] T007 Run `uv run pytest -q tests/test_workflow.py tests/test_workbench_flow.py` to validate the foundational context contract

**Checkpoint**: Same-item context is available independently of Preview generation, with no schema or dependency change.

---

## Phase 3: User Story 1 — Context While Preview Is Generated (Priority: P1) 🎯 MVP

**Goal**: Show `Refine 중...` and a bounded prior-context summary while Problem or Solution Preview generation is pending, then retain the summary in the completed editable Preview.

**Independent Test**: Hold Problem and Solution proposal requests pending and verify the correct item’s summary appears within one second, then resolve the request and verify the same summary remains beside the proposal.

- [X] T008 [US1] Add failing Problem and Solution loading/ready Preview tests, including exact status, bounded escaped content, and human-approval preservation, in `tests/test_browser_menu.py`
- [X] T009 [US1] Add the reusable loading/ready summary presentation and minimal responsive styling to `llm_wiki/static/index.html`
- [X] T010 [US1] Start context and refinement requests concurrently and retain attempt-local context through the ready state in `llm_wiki/static/index.html`
- [X] T011 [US1] Run `uv run pytest -q tests/test_browser_menu.py -k 'refinement_preview and (loading or ready)'` to validate User Story 1 independently

**Checkpoint**: Context-bearing Problem and Solution refinement provides a useful loading and completed Preview without applying content.

---

## Phase 4: User Story 2 — Preview Failure Warning (Priority: P1)

**Goal**: Remove the failed Preview, restore the usable refinement modal, and expose exactly one accessible corner warning with the required tooltip.

**Independent Test**: Force Problem and Solution generation failures and verify Preview absence, modal usability, one warning, pointer/keyboard tooltip access, and warning cleanup on retry.

- [X] T012 [US2] Add failing Problem and Solution failure, pointer tooltip, keyboard tooltip, retry cleanup, cancellation, and stale-response tests in `tests/test_browser_menu.py`
- [X] T013 [US2] Add the hidden accessible warning control and corner-safe tooltip styling to `llm_wiki/static/index.html`
- [X] T014 [US2] Implement attempt identity, cancellation, failure restoration, warning lifetime, and stale-response guards in `llm_wiki/static/index.html`
- [X] T015 [US2] Run `uv run pytest -q tests/test_browser_menu.py -k 'refinement_preview and (failure or warning or cancel or stale)'` to validate User Story 2 independently

**Checkpoint**: Failed Preview content is never displayed and the failure remains discoverable without an intrusive alert.

---

## Phase 5: User Story 3 — Preserve Existing Refinement Behavior (Priority: P2)

**Goal**: Keep no-context and Capture refinement behavior unchanged and avoid permanent context or warning UI.

**Independent Test**: Hold a no-context request pending and refine a Capture, verifying no new loading Preview, context endpoint call, permanent summary, or corner warning appears.

- [X] T016 [US3] Add failing no-context, ordinary-modal, Capture-exclusion, successful-retry, item-switch, and reduced-motion regression tests in `tests/test_browser_menu.py`
- [X] T017 [US3] Tighten type, no-context, close, retry, and item-switch boundaries without changing existing Capture behavior in `llm_wiki/static/index.html`
- [X] T018 [US3] Run `uv run pytest -q tests/test_browser_menu.py -k 'refinement_preview'` to validate the complete UI state matrix

**Checkpoint**: Requested states work while unaffected refinement states retain the current UX.

---

## Phase 6: Polish and Cross-Cutting Validation

**Purpose**: Verify code quality, performance budget, compatibility, and full feature convergence.

- [X] T019 Run the full automated suite with `uv run pytest -q` and address feature-related failures in `llm_wiki/` or `tests/`
- [X] T020 Parse the browser script from `llm_wiki/static/index.html` with Node and run `git diff --check`
- [X] T021 Execute every scenario in `specs/005-refinement-preview-status/quickstart.md` and confirm the documented 100 ms service and one-second loading-state budgets
- [X] T022 Review `llm_wiki/services/workflow.py`, `llm_wiki/api/app.py`, and `llm_wiki/static/index.html` against all FR-001–FR-019 and mark all completed tasks in `specs/005-refinement-preview-status/tasks.md`

---

## Dependencies and Execution Order

### Phase Dependencies

- **Phase 1** has no dependency.
- **Phase 2** depends on Phase 1 and blocks all user stories.
- **Phase 3 / US1** depends on Phase 2.
- **Phase 4 / US2** depends on the shared Preview state introduced by US1.
- **Phase 5 / US3** depends on US1 and US2 so it can lock down their boundaries.
- **Phase 6** depends on all user stories.

### User Story Dependencies

- **US1 (P1)** establishes the loading and ready Preview state machine.
- **US2 (P1)** extends that state machine with failure and cancellation behavior.
- **US3 (P2)** verifies excluded states and adds only boundary corrections.

### Within Each User Story

- Add the story’s failing UI tests before changing browser behavior.
- Keep changes to `llm_wiki/static/index.html` sequential.
- Complete and validate each story before beginning the next.

### Parallel Opportunities

- T003 and T004 can run in parallel because they modify different test files.
- After T003/T004, service implementation T005 precedes endpoint T006.
- Browser test and browser implementation tasks are intentionally sequential because they share the same files and follow TDD.

---

## Parallel Example: Foundational Context Contract

```text
Task T003: Add workflow summary tests in tests/test_workflow.py
Task T004: Add endpoint contract tests in tests/test_workbench_flow.py
```

---

## Implementation Strategy

### MVP First

1. Complete setup and the bounded context contract.
2. Complete US1 and validate loading/ready Problem and Solution Preview states.
3. Pause point: the user can already understand prior context while waiting.

### Incremental Delivery

1. Add failure warning and cancellation isolation through US2.
2. Lock down no-context and Capture exclusions through US3.
3. Run the full validation matrix and convergence review.

## Notes

- Every task uses an exact repository path or runnable validation command tied to exact paths.
- No new dependency, schema migration, vault access, or provider call is expected for context summary construction.
- Commit implementation and completed artifacts only from `codex/refinement-preview-status` in the dedicated worktree.
