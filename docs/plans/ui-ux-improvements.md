# UI/UX Improvement Implementation Plan

> **Execution status — implementation, packaged E2E, and documentation captures complete.** Draft protection,
> window-size layout, selected-locale copy, focus/IME composition guards, and Queue recovery are implemented.
> Real Korean IME, VoiceOver, and OS reduced-motion manual verification remain unverified.

**English** | [한국어](ui-ux-improvements.ko.md)

> Status: **implemented follow-up; automated release acceptance complete.** The implementation is based on
> `de01ff47398871902a765d43b5a4060161316f92` on `fix/ui-ux-improvements`.

## Evidence and scope

**Final package result:** the signed `9b2b81082a43de0637bedd33a8cc670709ff2b21` bundle passed 32/32 packaged scenarios. The generated inventory recorded 188 scanned source controls and 175 rendered/exercised/asserted controls, and all six gap arrays were empty. Manual Korean IME, VoiceOver, OS reduced motion, Windows, zoom, and native quit/crash draft durability remain unverified or out of scope.

The signed macOS release bundle built on 2026-09-12 at 11:44:31 AM by `LLM Wiki Local Signing` has CDHash `9b2b81082a43de0637bedd33a8cc670709ff2b21`. It is a dirty-worktree build identified by the base and branch above, not a final commit. Unit/runtime, typecheck, scoped ESLint, native tests, and build checks pass; full packaged E2E passed 32/32 with 175 covered controls. Real Korean IME/VoiceOver manual review remains unverified.

The scope is incremental improvements to Task detail, Workbench, Refinement, AI setup, and Queue. Review remains limited to the supported light desktop surface. A wholesale information-architecture redesign, a new UI library, dark mode, mobile, unreviewed autosave, and a new durable Task-draft schema are outside the default scope.

## Implementation start and file boundaries

- [x] **T0 · Prepare the implementation worktree (P1, small).** Record primary-checkout `git status --short` and the starting HEAD, preserving existing user changes. Create `<repo>/.worktrees/ui-ux-improvements` with `scripts/create_task_worktree.sh ui-ux-improvements fix/ui-ux-improvements HEAD`. Run it from the primary checkout and explicitly pass the verified HEAD as the third argument rather than using the default main. Confirm any alternate commit/ref first. Do not copy, clean up, or revert existing user changes arbitrarily. Treat linked `node_modules`, Cargo `target`, verified embedding assets, and copied `dist/` as build caches; never run `npm ci` against linked dependencies.
- [x] **T1 · Reconfirm current contracts (P1, small).** In the worktree, inspect `frontend/src/services/taskClient.ts`, `frontend/src/types/taskWorkbench.ts`, Task API/IPC, and revision-conflict responses. Preserve the expected `taskRevision` contracts used by `revise`, `workLog`, checklist/decision/transition/complete. Do not assume a new API or persistence schema.
- [ ] **T2 · Discover native window-close capability (optional, bounded).** Implement React panel close and Task switching without waiting for T2. Include a native app close/quit guard only after proving current Tauri close interception, async save, and cancel/keep-open behavior with a small spike or native test. If it cannot be safely guaranteed, leave it as a follow-up; do not claim quit/crash draft persistence.

Likely boundaries are `frontend/src/features/workbench/TaskDetail.tsx`, `WorkbenchView.tsx`, `RefinementPanel.tsx`, `task-workbench.css`, `taskWorkbenchText.ts`, `frontend/src/features/settings/SettingsView.tsx`, the Queue runtime/overlay owner, and matching React/runtime/desktop scenarios plus the interaction coverage manifest. Add `src-tauri` and Rust tests only if the native close guard is proven and adopted. After implementation, review and update only the matching user-facing behavior in `docs/features/conflict-gated-workflow(.ko).md`, `background-ai-queue(.ko).md`, `visual-guide(.ko).md`, and coverage documentation. Correct stale mixed artifact identity in coverage documentation only with actual final implementation evidence.

## Priorities and issue mapping

## Implementation status (2026-09-12)

| Delivered scope | Status |
| --- | --- |
| I1.1–I1.4 Task definition draft, independent-mutation merge, expected-revision save, close/switch guard | Implemented and covered by focused tests |
| I2.1–I2.2 shortcut layout and title-input width | Implemented; focused native geometry and full E2E pass |
| I3.1–I3.3 selected-locale copy and assertions | Implemented and covered by focused tests |
| I4.1–I4.3 refinement spacing, nonmodal focus/Escape, composition safety | Implemented; real IME and VoiceOver remain manual checks |
| I5.1–I5.2 Queue Open AI setup and failed-job return/retry | Implemented and covered by focused tests |
| I4.4 shared left-axis headings/counts and I6 12px-or-larger status/revision metadata | Implemented; broader detail regrouping is out of scope |
| T2 native quit/crash guard | Deferred |

The checklist below records delivery against the original plan. The table above and
[portable verification evidence](../testing/evidence/ui-ux-improvements.json) describe the final scope.

| Phase | Sequence | Acceptance |
| --- | --- | --- |
| 1. Reproduced defects and small fixes | I1 → I2 → I3 → I4.1 | Draft-loss/shortcut-overlap regression checks, title width, selected-language copy, and role spacing |
| 2. Shared interaction and recovery | I4.2–I4.3 → I5 | Non-modal focus, safe close, setup return/retry, and meaningful assertions for new controls |
| 3. Reading space and hierarchy | I4.4 → I6 | Review long-content fixtures before local layout/size changes; do not block mandatory defect fixes |

Use local checks during each phase. Run the release build and full packaged E2E once after the integrated final changes. Optional native-exit discovery T2 is not a prerequisite for mandatory phase-1 fixes.

| Delivery | Review issue | Priority | Size assumption | Depends on |
| --- | --- | --- | --- | --- |
| I1 Protect Task drafts | UX-001 | P1 | Medium | T0, T1; T2 only for native exit |
| I2 Narrow window and title width | UX-012, UX-002 | P2 | Small–medium | T0 |
| I3 Locale and copy | UX-003, UX-006 and copy inventory | P2 | Small | T0 |
| I4 Refinement and detail reading axis | UX-004, UX-005, UX-008 | P2 | Small–medium | T0 |
| I5 Queue recovery | UX-007 | P2 | Small–medium | T0, current runtime ownership |
| I6 Important metadata | UX-011 | P3 | Small | I4.4, representative rendered review |

These are implementation assumptions, not dates or claims about user frequency. Finish I1 first. Other work may share a reviewable change, but their acceptance criteria remain independent.

## I1 — Separate Task draft from persisted snapshot

- [x] **I1.1 · State model (P1).** Replace `TaskDetail`’s single `task` state with (a) the last server `persisted` snapshot, (b) an editable `draft`, (c) `baseTaskRevision` and comparison `baseSnapshot` captured when that draft starts, and (d) a field-level dirty set. Draft fields are title, detail, outcome, scope, nonGoals, and validationCriteria.
- [x] **I1.2 · Merge independent mutations (P1).** Work Log, comments, checklist, decisions, relationships, readiness, transitions, and completion update the newest persisted snapshot. A mutation that increments Task revision must not silently move a pending draft’s base revision. Compare the new snapshot with `baseSnapshot`. Only after proving that no server change overlaps a dirty field, preserve dirty values, update untouched fields, and advance the comparison snapshot and base revision together. Mutation success alone must not move the base; when a changed server field overlaps a dirty field, or comparison is uncertain, keep the draft and require conflict review/retry rather than overwriting it.
- [x] **I1.3 · Explicit save and optimistic conflict (P1).** Send the original or verified disjoint-rebased `baseTaskRevision` as `expectedTaskRevision` to `revise`. On success, use the returned snapshot as the next persisted/draft baseline and clear dirtiness. On conflict or network failure, retain the input and base revision, expose a useful recovery action, and preserve existing mutation queueing without letting a reload overwrite draft state.
- [x] **I1.4 · Close and switch guard (P1).** On a dirty close or another Task selection, offer Save, Discard, and Keep editing. A save failure/conflict keeps the panel and switch blocked with the draft and error intact. Discard alone removes the draft; a successful save continues the requested close/switch.
- [ ] **I1.5 · Native exit limit (P1).** Only when T2 proves support, attach equivalent handling to native close/quit and test that a save failure can cancel exit. Otherwise finish the panel-close and Task-switch scope only; quit/crash protection, background flush, autosave, and a new DB schema remain excluded.

Acceptance: an unsaved Task definition survives a Work Log addition, independent mutation, and refresh; explicit save detects conflicts at the exact base revision and never overwrites unrelated changes; close/switch choices and save failures are safe.

## I2 — Narrow Workbench and input width

- [x] **I2.1 · Shortcut geometry (P2).** Make `WorkbenchView.tsx`’s full Capture/Task title and the `task-workbench.css` shortcut grid/card content-driven. Keep the action in the card’s bottom area and prevent overlap with the following `All work` heading. Reproduce the WebKit cascade/intrinsic sizing before attributing the issue to a fixed height. If title clamping is chosen, add keyboard access to the complete title in detail.
- [x] **I2.2 · Task title input (P2).** Keep a separate width rule only for checkboxes; let Task-detail text inputs use the same available width as other editor fields. A long title must remain editable to its horizontal-scroll end without pushing labels or save actions out of layout.

Acceptance: at 900×640, 904×768, and 1280×820, long Korean and English shortcut titles do not overlap their actions or the next heading. Input geometry uses the panel content width reasonably and verifies editable horizontal-scroll end; tests do not demand an entire long value be simultaneously visible in one input.

## I3 — System copy in the selected locale

- [x] **I3.1 · Inventory the source of truth (P2).** Locate the current locale source and React/legacy runtime boundaries for AI-setup privacy, prerequisites, readiness reason/outcome, completion CTA, and Queue failure/retry/setup copy, excluding user-authored content.
- [x] **I3.2 · Apply i18n (P2).** Move the hardcoded AI-setup privacy sentence to a shared locale key. Korean: `API 키는 이 기기의 로컬 설정 파일에만 저장됩니다. Vault나 앱 데이터베이스에는 저장되지 않습니다.` Preserve the English meaning. Replace the top action with matching visible and accessible labels: `Add completion evidence` / `완료 근거 추가`. Give prerequisite/reason outcome strings an explicit purpose.
- [x] **I3.3 · Assert copy (P2).** Add exact Korean/English system-copy assertions across locale switching and a relaunch fixture. Do not claim to replace every legacy string or user-authored English.

## I4 — Refinement, non-modal focus, and detail reading axis

- [x] **I4.1 · Role/body spacing (P2).** Split Refinement message role and body with a block or explicit gap, preserving scroll and heading hierarchy for long-text/code/image fixtures.
- [x] **I4.2 · Non-modal focus contract (P2).** Keep Task detail and Refinement non-modal for background comparison and parallel Work Log work. On open, focus the panel heading or compose control. Escape closes it and restores focus to the trigger when present, otherwise to a stable Workbench heading. Do not trap Tab; use normal non-modal tab order.
- [ ] **I4.3 · Save-failure safety and IME (P2).** Review the Refinement workspace flush/save path so a failure cannot silently lose input during close; retain/retry or keep open according to the existing safe path. Do not send Cmd/Ctrl+Enter during React `event.nativeEvent.isComposing` (with compatibility conditions only when verified necessary). Add a manual macOS Korean-IME and VoiceOver pass because deterministic E2E cannot substitute for either.
- [ ] **I4.4 · Local detail grouping (P2).** Without a full redesign, adjust headings/counts and the order of Task details, current work record, completion evidence, and metadata. Place Workbench heading/count/status on the left reading axis with their card group, and metadata after evidence. Fix spacing before considering a larger existing Refinement panel.

## I5 — Queue recovery when a provider is missing

- [x] **I5.1 · Confirm ownership and navigation (P2).** Confirm the legacy runtime/Overlay owner that renders Queue and the existing AI-setup navigation dispatch. `Open AI setup` must call the existing sidebar/app navigation, not add a new route.
- [x] **I5.2 · Failure → setup → return → retry (P2).** For an API-key-missing job, render cause and next action in the selected locale: `No API key is configured. Save your connection details in AI setup, then retry.` / `API 키가 설정되지 않았습니다. AI 설정에서 연결 정보를 저장한 뒤 다시 시도하세요.` Opening settings retains the failed job and Queue context; after a successful setting save, existing Retry targets the same job. Block duplicate retry only for that job and refresh its result state.

## I6 — Readability of important metadata

- [x] **I6.1 · Review role-based sizes (P3, small).** Separate UX-011 current status/revision from provenance and compare 12px-or-larger candidates for decision-relevant metadata in Korean/English rendered screens. Reuse body/mono tokens rather than making all text the same size. Adopt only changes that preserve card height, wrapping, and access when enlarged.

## Verification

- [x] Unit/runtime: cover I1 draft merge, disjoint rebase, conflict, save failure, and close/switch guard; I3 keys; I4 focus/Escape/IME; and I5 navigation/retry state.
- [x] Desktop scenario and interaction coverage: register every newly enabled guard action, setup CTA, and close control with scenario and source evidence. The final control total may exceed the 169 baseline; do not freeze acceptance at 32/169.
- [ ] Geometry and copy: collect Korean/English long-title screenshots and DOM geometry at 900×640, 904×768, and 1280×820. Check `scrollWidth/clientWidth`, card bounds, action/heading non-overlap, available input width, horizontal-scroll end, and exact localized copy.
- [ ] Accessibility/manual: record keyboard open→focus, non-trapping Tab, Escape→focus restore, real Korean IME composition, VoiceOver names/focus, and long text/code/image scrolling on macOS. Do not extend support claims to Windows, dark mode, or mobile.
- [x] Incremental checks: run relevant `npm test`, `npm run typecheck`, and `npm run lint`. Run `cargo test --manifest-path src-tauri/Cargo.toml` only when native code changes. Once after all code and documentation changes, run `npm run tauri:build` and `npm run test:desktop -- --full`, then `git diff --check`, changed-link validation, bilingual review, and worktree-status checks.

## Documentation and handoff

Implementation was completed in `.worktrees/ui-ux-improvements` on `fix/ui-ux-improvements`,
based on `de01ff47398871902a765d43b5a4060161316f92`. Primary-checkout user changes were preserved.
`docs/DOCUMENTATION_GUIDE.md` was reviewed. Current English/Korean user guides, contribution
instructions, navigation, workflow descriptions, and specification authority were updated.
Nine actual screenshots replace sixteen retired assets; all 187 reviewed Markdown files and
46 image references resolve locally. Historical Problem/Solution migration contracts remain history.

Automated results: 34 Vitest files / 176 tests, 106 Rust tests, typecheck, changed-code ESLint,
signed build, 32/32 packaged scenarios, and all 175 controls pass. Full lint retains 23 baseline
findings in two untouched runtime test files. Real IME, VoiceOver, OS reduced-motion, zoom,
Windows installed testing, and native quit/crash Task-draft durability remain unverified or out of scope.
See the [current visual guide](../features/visual-guide.md) and
[acceptance record](../../specs/012-task-centered-workbench/acceptance-verification.md).
