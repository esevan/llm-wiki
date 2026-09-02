# Tasks: Evidence-Rich Vault Conflict Review

## Phase 1: Setup

- [x] T001 Add conflict review service skeleton in `llm_wiki/services/conflict_review.py`

## Phase 2: Foundational

- [x] T002 Add independent semantic retrieval, exact passages, manifest hashing, and embedding cleanup in `llm_wiki/services/retrieval.py`
- [x] T003 Add extended review-run persistence and cached snapshot helpers in `llm_wiki/services/workflow.py`

## Phase 3: User Story 1 - Inspect a trustworthy review

- [x] T004 [US1] Add retrieval and evidence validation tests in `tests/test_retrieval.py` and `tests/test_conflict_review.py`
- [x] T005 [US1] Implement claim extraction, candidate merge/dedup, screening, strong review, findings, progress, and timings in `llm_wiki/services/conflict_review.py`
- [x] T006 [US1] Add asynchronous start and polling API integration in `llm_wiki/api/app.py`
- [x] T007 [US1] Render scope, coverage, progress, evidence, and timings in `llm_wiki/static/index.html`

## Phase 4: User Story 2 - Safe result semantics

- [x] T008 [US2] Test conservative clear gates and citation validation in `tests/test_conflict_review.py` and `tests/test_api.py`
- [x] T009 [US2] Implement explicit review state meanings and conservative terminal gates in `llm_wiki/services/conflict_review.py`
- [x] T010 [US2] Disable clear decisions for non-clear recommendations in `llm_wiki/static/index.html`

## Phase 5: User Story 3 - Reuse and cancellation

- [x] T011 [US3] Test hash reuse, invalidation, and server cancellation in `tests/test_conflict_review.py` and `tests/test_api.py`
- [x] T012 [US3] Implement hash cache and cancellation propagation in `llm_wiki/services/conflict_review.py`, `llm_wiki/services/provider.py`, and `llm_wiki/api/app.py`
- [x] T013 [US3] Connect browser cancellation to the server endpoint in `llm_wiki/static/index.html`

## Phase 6: Polish

- [x] T014 Update bilingual feature documentation and indexes in `docs/features/`
- [x] T015 Run `uv run pytest -q`, browser parse validation, and `git diff --check`
- [x] T016 Review `docs/DOCUMENTATION_GUIDE.md` and commit the completed task

## Dependencies

T001-T003 are foundational. US1 precedes US2 because terminal gates consume review progress. US3 depends on stable snapshots. Documentation and validation follow all stories.

## Independent Tests

- **US1**: A running review exposes scope/progress and exact cited partial findings.
- **US2**: Zero candidates, incomplete coverage, malformed screening, and invalid citations never yield clear.
- **US3**: Identical hashes reuse results, changed Vault content invalidates them, and cancellation prevents later provider calls.

## Implementation Strategy

Deliver US1 as the observable review pipeline, then add conservative state gates, then caching/cancellation. Tests precede the corresponding implementation behavior.
