# Tasks: Korean-English Localization

**Input**: Design documents from `specs/007-bilingual-localization/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Required by the validation criteria and constitution. Story tests are written and observed failing before implementation.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish packaged locale resources and localization module boundaries.

- [X] T001 Add parity-matched English and Korean UI resources in `llm_wiki/static/i18n/en.json` and `llm_wiki/static/i18n/ko.json`
- [X] T002 Update static resource packaging rules in `pyproject.toml` and verify repository ignore patterns in `.gitignore`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement shared locale validation, persistence, content overlays, and prompt helpers.

- [X] T003 Add failing locale/resource/storage/cache tests in `tests/test_localization.py`
- [X] T004 Implement locale normalization, settings persistence, localized-content schema/registry, bulk overlays, and Knowledge cache primitives in `llm_wiki/services/localization.py`
- [X] T005 Integrate localization schema and field-version helpers with workflow transactions in `llm_wiki/services/workflow.py`
- [X] T006 Add request-locale, locale settings, and resource endpoints from the contract in `llm_wiki/api/app.py`

**Checkpoint**: Locale infrastructure is available without changing existing data or invoking AI.

---

## Phase 3: User Story 1 - Switch the application language (Priority: P1) 🎯 MVP

**Goal**: Persist one locale and translate all primary static UI surfaces immediately without losing context.

**Independent Test**: Switch an open populated workbench and dialog between English and Korean; static text and accessibility attributes change while typed input and navigation remain.

- [X] T007 [US1] Add failing browser tests for first-run locale, persistence, resource parity, request headers, open-dialog preservation, and Korean accessible labels in `tests/test_browser_menu.py`
- [X] T008 [US1] Add the locale toggle, resource bootstrap, DOM/resource translation, locale-aware dates, and locale request headers in `llm_wiki/static/index.html`
- [X] T009 [US1] Localize server-provided transition descriptors and stable error presentation in `llm_wiki/services/workflow.py` and `llm_wiki/api/app.py`
- [X] T010 [US1] Add a locale-switch performance/provider-call regression check in `tests/test_browser_menu.py` and `tests/test_localization.py`

**Checkpoint**: The complete static shell works in Korean and English as an independently demonstrable MVP.

---

## Phase 4: User Story 2 - Reuse bilingual versions of newly saved content (Priority: P1)

**Goal**: Generate, review, save, and display KO+EN versions of new AI-produced Problems and Solutions with no AI call on switching.

**Independent Test**: Approve a bilingual Problem and Solution draft, reload both locales, and observe stored variants with zero provider calls during reads/switches.

- [X] T011 [US2] Add failing bilingual draft/apply/restore and read-overlay tests in `tests/test_workflow.py` and `tests/test_api.py`
- [X] T012 [US2] Extend Problem/Solution draft and refinement prompts and validation for one-call KO+EN payloads in `llm_wiki/services/conversation.py` and `llm_wiki/api/app.py`
- [X] T013 [US2] Store approved localized variants atomically and restore bilingual drafts in `llm_wiki/services/workflow.py`
- [X] T014 [US2] Carry reviewed variants through proposal/apply UI and switch cached item views without AI in `llm_wiki/static/index.html`

**Checkpoint**: New generated durable Problem/Solution content is bilingual and language switches are provider-free.

---

## Phase 5: User Story 3 - Preserve legacy content without migration (Priority: P1)

**Goal**: Leave all existing rows and Vault files unchanged and display originals when localized versions are absent.

**Independent Test**: Initialize over a legacy fixture, read under both locales, and assert identical source bytes/rows and zero translation calls.

- [X] T015 [US3] Add failing legacy database/Vault no-migration, no-provider, and manual-supplement tests in `tests/test_localization.py` and `tests/test_api.py`
- [X] T016 [US3] Implement field-level original fallback and manual locale supplementation API in `llm_wiki/services/localization.py` and `llm_wiki/api/app.py`
- [X] T017 [US3] Surface missing-version/fallback state and manual supplementation controls without changing identity in `llm_wiki/static/index.html`

**Checkpoint**: Existing user data remains untouched, readable, and manually supplementable.

---

## Phase 6: User Story 4 - Generate live content in the active language (Priority: P2)

**Goal**: Bind all live AI operations to one request-start locale without dual generation or retroactive translation.

**Independent Test**: Exercise live AI endpoints in both locales and verify one provider call, one locale instruction, and unchanged prior output after switching.

- [X] T018 [US4] Add failing locale-bound prompt and single-provider-call tests for all live AI routes in `tests/test_conversation.py`, `tests/test_ai.py`, and `tests/test_api.py`
- [X] T019 [US4] Add reusable response-language instructions and apply them to live AI operations in `llm_wiki/services/conversation.py` and `llm_wiki/api/app.py`
- [X] T020 [US4] Bind SSE chat to request-start locale and preserve already rendered responses during switching in `llm_wiki/static/index.html`

**Checkpoint**: Live content is generated exactly once in the active request language.

---

## Phase 7: User Story 5 - Read portable Knowledge in Korean (Priority: P2)

**Goal**: Publish new managed Knowledge as English canonical Markdown and provide safe on-demand cached Korean reading.

**Independent Test**: Publish under Korean UI, verify English canonical content, observe Korean miss then hit, edit canonical externally, and observe stale-cache rejection.

- [X] T021 [US5] Add failing English-canonical publication, legacy fallback, cache hit/miss/invalidation/race/failure tests in `tests/test_localization.py`, `tests/test_api.py`, and `tests/test_projections.py`
- [X] T022 [US5] Standardize managed projection and completion Knowledge generation to English with canonical metadata in `llm_wiki/services/workflow.py` and `llm_wiki/api/app.py`
- [X] T023 [US5] Implement the adapter-bounded Knowledge read/translate/cache contract in `llm_wiki/services/localization.py` and `llm_wiki/api/app.py`
- [X] T024 [US5] Add a locale-aware Knowledge reading surface and cache/fallback states in `llm_wiki/static/index.html`
- [X] T025 [US5] Deprecate canonical `report_language` behavior while retaining configuration compatibility in `llm_wiki/services/settings.py` and `specs/006-ai-task-model-routing/contracts/provider-config.md`

**Checkpoint**: Newly managed Knowledge is portable English canonical data with safe derived Korean viewing.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Close coverage, performance, cross-platform, and documentation gates.

- [X] T026 [P] Add paired feature guides and index links in `docs/features/bilingual-localization.md`, `docs/features/bilingual-localization.ko.md`, `docs/features/README.md`, and `docs/features/README.ko.md`
- [X] T027 [P] Update locale and Knowledge behavior in `README.md`, `README.ko.md`, `docs/product-spirit.md`, and `docs/product-spirit.ko.md`
- [X] T028 Extend Unicode/cross-platform and 1,000-note performance coverage in `tests/test_markdown.py`, `tests/test_retrieval.py`, and `.github/workflows/cross-platform.yml`
- [X] T029 Run `uv run pytest -q` and record all tests passing
- [X] T030 Run the exact browser-script syntax validation command and record success
- [X] T031 Run `git diff --check` and complete `specs/007-bilingual-localization/quickstart.md` validation
- [X] T032 Review `docs/DOCUMENTATION_GUIDE.md` and align all affected documentation pairs

---

## Dependencies & Execution Order

- Setup → Foundational → all user stories.
- US1 is the UI MVP and supplies locale propagation used by US2–US5.
- US2 and US3 share localized sidecar storage and proceed in that order to establish then validate fallback behavior.
- US4 depends only on locale propagation and can proceed after US1.
- US5 depends on locale propagation and cache primitives, not on US2 storage behavior.
- Polish follows all selected stories.

## Parallel Opportunities

- T001 and T002 touch independent setup files.
- After T006, backend tests for US2/US3/US4/US5 can be authored independently before their implementations.
- T026 and T027 can proceed in parallel because they affect separate documentation pairs.
- No tasks sharing `llm_wiki/static/index.html`, `llm_wiki/api/app.py`, or `llm_wiki/services/workflow.py` should execute concurrently.

## Implementation Strategy

1. Deliver US1 as the static bilingual demo MVP.
2. Add stored bilingual generated content and prove provider-free switching.
3. Lock legacy fallback and no-migration behavior.
4. Bind live AI to locale.
5. Add English-canonical Knowledge and Korean cached reading.
6. Finish paired documentation, cross-platform checks, and stable verification.

## Phase 9: Convergence

- [X] T033 Complete bilingual variant propagation and atomic persistence for Explore preview creation and Problem/Solution refinement apply paths per FR-007, FR-008, FR-009, T012, and T014 (partial)
- [X] T034 Normalize approved Korean-input patches for app-managed Knowledge to English before canonical write while preserving review, failure safety, and adapter boundaries per FR-015, SC-005, and US5/AC1 (partial)

---

## Phase 10: Bilingual Image Summary Convergence

**Goal**: Generate and persist KO+EN Image Summary variants in the existing explicit AI request while preserving raw Work Log evidence and provider-free locale switching.

**Independent Test**: Summarize one Work Log image, read progress in both locales, and verify one provider call, matching stored summaries, unchanged raw evidence, legacy fallback, and atomic failure behavior.

- [X] T035 [US2] Add failing workflow storage and locale-overlay tests for bilingual and legacy Image Summaries in `tests/test_workflow.py` and `tests/test_localization.py`
- [X] T036 [US2] Add failing API tests for one-call bilingual Image Summary generation, locale reads, malformed response safety, and zero-provider switching in `tests/test_api.py`
- [X] T037 [US2] Register and atomically persist localized Image Summary variants in `llm_wiki/services/localization.py` and `llm_wiki/services/workflow.py`
- [X] T038 [US2] Request, validate, and return KO+EN Image Summary variants while binding progress reads to the request locale in `llm_wiki/api/app.py`
- [X] T039 [US2] Refresh an open Work Log after locale changes and render stored summary fallback state in `llm_wiki/static/index.html` and `tests/test_browser_menu.py`
- [X] T040 Update paired localization documentation and complete stable verification in `docs/features/bilingual-localization.md`, `docs/features/bilingual-localization.ko.md`, and `specs/007-bilingual-localization/quickstart.md`

---

## Phase 11: Progressive Korean Knowledge Reading

**Goal**: Render canonical or cached Knowledge within one second, then replace only completed paragraphs while preserving a cancellable, recoverable reading experience.

- [X] T041 Add failing API and browser tests for provider-free first render, paragraph progress, cached immediate display, server cancellation, failure fallback, retry, and whole-paragraph fade in `tests/test_api.py`, `tests/test_localization.py`, and `tests/test_browser_menu.py`
- [X] T042 Add stable Markdown paragraph segmentation and progressive exact-hash cache assembly in `llm_wiki/services/localization.py`
- [X] T043 Add fast-read, NDJSON paragraph translation, and server cancellation contracts in `llm_wiki/api/app.py`
- [X] T044 Render canonical/cache immediately, stream completed Korean paragraphs with a 150 ms fade, cancel superseded jobs, and retain retryable English fallback in `llm_wiki/static/index.html` and paired locale resources
- [X] T045 Update paired feature documentation, contract/quickstart records, and complete stable verification

---

## Phase 12: Progressive Reading Visibility and Motion

- [X] T046 Add failing browser-source coverage for a sticky high-contrast progress live region and slow left-to-right two-layer wave replacement in `tests/test_browser_menu.py`
- [X] T047 Implement the prominent progress badge and approximately 900 ms English-out/Korean-in paragraph wave with reduced-motion fallback in `llm_wiki/static/index.html`
- [X] T048 Update paired feature documentation and complete stable verification

---

## Phase 13: Vault-backed Korean Reading Files

- [X] T049 Add failing storage, migration, indexing-exclusion, invalidation, and orphan-cleanup tests in `tests/test_localization.py`, `tests/test_retrieval.py`, and `tests/test_api.py`
- [X] T050 Add safe `Translations/ko` path, discovery exclusion, and derived-file lifecycle operations to `llm_wiki/services/vault.py`
- [X] T051 Implement exact-hash Vault translation storage with canonical-link frontmatter and legacy SQLite promotion in `llm_wiki/services/localization.py`
- [X] T052 Integrate Vault-backed translation storage and watcher cleanup in `llm_wiki/api/app.py`
- [X] T053 Update paired documentation and complete stable verification
