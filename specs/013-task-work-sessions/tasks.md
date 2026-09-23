# Tasks: Task Work Sessions

## Phase 1: Setup

- [X] T001 Review preserved initial implementation against spec and plan in `frontend/src/features/workbench/TaskWorkSessions.tsx` and `src-tauri/src/application/task_service.rs`
- [X] T002 [P] Add bilingual user guide shells in `docs/features/task-work-sessions.md` and `docs/features/task-work-sessions.ko.md`

## Phase 2: Foundational

- [X] T003 Add migration 14 and constrained session/record schema in `src-tauri/src/native/migrations.rs` and `src-tauri/src/native/task_work_session_schema.sql`
- [X] T004 Add shared session, entry, and attachment contracts in `frontend/src/types/taskWorkbench.ts`
- [X] T005 Map Task-scoped session routes and client calls in `frontend/src/services/taskOperation.ts` and `frontend/src/services/taskClient.ts`
- [X] T006 Add route trust-boundary tests in `frontend/src/services/taskOperation.test.ts`

## Phase 3: User Story 1 — Resume Task Work (P1)

**Independent test**: Create, save, fail/retry, close, and reopen one session without auto-creation or duplicate records.

- [X] T007 [US1] Implement idempotent create/list/get/append persistence without Task activity mutation in `src-tauri/src/application/task_service.rs`
- [X] T008 [US1] Add restart persistence, no-auto-create, retry, and Task-state native tests in `src-tauri/src/application/task_service.rs`
- [X] T009 [US1] Build empty/list/chat/composer flows with confirmed-save draft clearing in `frontend/src/features/workbench/TaskWorkSessions.tsx`
- [X] T010 [US1] Add create/reopen/failure component tests in `frontend/src/features/workbench/TaskWorkSessions.test.tsx`

## Phase 4: User Story 2 — Keep Sessions and Tasks Separate (P1)

**Independent test**: Switch across two sessions and two Tasks; saved and pending content never crosses owners and stale async completions do not replace the current view.

- [X] T011 [US2] Enforce compound Task/session ownership on every native read and write in `src-tauri/src/application/task_service.rs`
- [X] T012 [US2] Guard list/read/write/file-read async completions and persist per-session drafts through Task detail lifetime in `frontend/src/features/workbench/TaskWorkSessions.tsx` and `frontend/src/features/workbench/TaskDetail.tsx`
- [X] T013 [US2] Add same-Task session isolation, cross-Task rejection, rapid-switch, and busy-state tests in `frontend/src/features/workbench/TaskWorkSessions.test.tsx` and `src-tauri/src/application/task_service.rs`

## Phase 5: User Story 3 — Prepare Future Execution Context (P2)

**Independent test**: Save Codex/model/approval/path and pasted/file attachments, reopen them, preview/download bytes, and observe a clear no-execution/no-file-read explanation.

- [X] T014 [US3] Validate provider-independent lifecycle settings, Codex catalog values, opaque path, and bounded attachment payload in `src-tauri/src/application/task_service.rs`
- [X] T015 [US3] Implement accessible settings, file selection, paste preview, saved image preview/download, and explicit inactive semantics in `frontend/src/features/workbench/TaskWorkSessions.tsx`
- [X] T016 [P] [US3] Add Korean/English strings and responsive/focus states in `frontend/src/features/workbench/taskWorkbenchText.ts` and `frontend/src/features/workbench/task-workbench.css`
- [X] T017 [US3] Add settings/attachment/accessibility/component tests in `frontend/src/features/workbench/TaskWorkSessions.test.tsx`

## Phase 6: Polish and Verification

- [X] T018 Update bilingual feature docs and indexes in `docs/features/task-work-sessions.md`, `docs/features/task-work-sessions.ko.md`, `docs/features/README.md`, and `docs/features/README.ko.md`
- [X] T019 Run focused npm/Cargo checks, 100 ms p95 projection profile, and `git diff --check` per `specs/013-task-work-sessions/quickstart.md`
- [ ] T020 Perform wide/narrow, Korean/English, long/empty/error, image, and keyboard rendered review and capture evidence under `.tmp/ui-review/task-work-sessions/`
- [X] T021 Prepare and run only the packaged `task-work-session-restart` lifecycle scenario in `src-tauri/src/desktop_e2e.rs` and `frontend/src/test/desktopScenario.ts`

## Dependencies

- T001–T006 establish the shared contract.
- US1 requires foundational work. US2 depends on US1 persistence and UI. US3 depends on US1 and may proceed alongside US2 only where files do not overlap.
- T018–T021 follow the final implementation.

## Parallel Examples

- T002 can proceed while T003–T006 establish code contracts.
- T016 can proceed while native validation T014 is completed.

## Implementation Strategy

Complete US1 first as the durable-session MVP, then enforce multi-owner isolation in US2, then add inactive future execution settings and attachments in US3. Mark tasks complete only after review and the specified verification succeeds.

## Phase 7: Convergence

- [ ] T022 Complete the remaining actual UI checks from T020: narrow 640 px English/Korean layouts, empty/error states, and native file-picker completion; record results in `specs/013-task-work-sessions/quickstart.md` and evidence under `.tmp/ui-review/task-work-sessions/` per FR-012, FR-014, SC-005 and plan: rendered review (partial). The final packaged restart scenario and normal-width English/Korean keyboard save/reopen checks pass; CUA/AppKit access loss after the picker and unavailable window-resize controls prevent claiming full visual verification. Windows execution is also unverified.
