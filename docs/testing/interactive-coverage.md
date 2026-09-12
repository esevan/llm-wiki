# Shipping interactive coverage inventory

**Inventory date:** 2026-09-12
**Scope:** shipping React shell, Task Workbench, legacy runtime loaded by `frontend/index.html`, and conditional overlays
**Evidence rule:** a control is marked packaged only when the signed application was driven through that control and its observable result was asserted. A component test, delegated DOM click, native route test, visual capture, or API-only mutation is not packaged interaction evidence.

## Artifact and runtime boundary

Final verification used CDHash `3be85b685e04f76596c914fafa4bf9639081958d`; the 32-case packaged run passed with exit 0. Earlier values are historical baseline evidence.

The current recorded release artifact is `src-tauri/target/release/bundle/macos/LLM Wiki.app`, code-directory hash `1988d962cfc113ca58786875eb7553c0cad5630b` (full recorded identity `1988d962cfc113ca58786875eb7553c0cad5630bcf1dc3ac5fd955d0f2fff6c3`). Its nine recorded packaged scenarios are `task-capture`, `task-worklog`, `task-refinement`, `task-relationships`, `task-review`, `task-publication`, `task-problem-resolution`, `task-persistence`, and `task-localization` in `.tmp/desktop-e2e-artifacts-cJf2wU/results.json`.

The application observed running from `/Applications/LLM Wiki.app` is a different installation: its executable is signed 2026-09-05 22:10:05, while the worktree bundle is signed 2026-09-08 20:38:34. An observation in the installed application therefore cannot confirm or refute the final Task artifact without first launching the recorded artifact identity. A separate isolated run of that installed executable completed its legacy packaged scenario in 12.5 seconds and recorded that Problem approval and refinement responded to their rendered clicks in `.tmp/desktop-e2e-artifacts-E0hxMm/results.json`. The reported inert click is therefore not reproduced by the deterministic happy path; the installation mismatch proves different UI versions, but does not establish the cause of the report.

`frontend/index.html` loads React and all eleven legacy runtime files: `foundation.js`, `jobs.js`, `workbench.js`, `conflicts.js`, `explore.js`, `work-tracking.js`, `manual.js`, `search-settings.js`, `solution-work.js`, `archive.js`, `transitions.js`, and `completed-workspace.js`. The Task React Workbench owns `#workbench`; legacy `loadBoard()` detects the absence of `#board` and dispatches `llm-wiki:task-workbench-refresh`. Legacy Search, Compass, Settings, chat, queue, notifications, detail readers, and modal markup still execute and ship. They remain in this inventory even when their old board-only trigger is unreachable from the Task Workbench.

## Refine Problem trace

The exact literal **Refine Problem** does not occur in the current React or legacy source. The current React Workbench has `Refine` only for canonical Capture and Task cards (`WorkbenchView.tsx`) and `RefinementPanel` accepts only `kind: "capture" | "task"`; Problem records are managed inside Task detail and are not React Workbench cards.

The legacy Problem card is still present in loaded source. In `frontend/public/runtime/workbench.js`, an unapproved Problem renders `Approve Problem`; an approved Problem renders `Explore next solution` through `draftButton(... data-next-chat-type="problems" ...)`. `frontend/public/runtime/conflicts.js` delegates that selector to `openNextChat("problems", id)`, and `frontend/public/runtime/explore.js` opens the modal, sets `chatTarget={type:"problems",mode:"next"}`, loads `/problems/{id}/refinement-context`, and after a successful chat queues `/problems/{id}/draft`. Thus the checked-in legacy action has a handler and an observable modal/job path.

The reproducible root cause of the reported mismatch is artifact identity: the click was observed in the older `/Applications` installation, while the final Task artifact has a different signature and UI ownership. The evidence does **not** yet prove why the older build's click looked inert; diagnosing that would require driving that old artifact or reproducing against an isolated copy. For the final artifact, the comparable gap is different: there is no first-class Problem refinement control in the React Task Workbench. A product decision and minimum behavior fix may be needed if Problem refinement is still required in the Task UI; this inventory does not make that code change.

## Control-family ledger

Legend: **P** = exercised through the recorded signed package with persisted/observable assertions; **C** = component/runtime/native tests only; **V** = visual presence only; **—** = no direct evidence. `P(part)` means one state or action in the family was packaged, while siblings and failure/keyboard states remain open.

| ID | Shipping control family and source | Handler / native operation | Preconditions and observable expected effect | Evidence and missing finite scenarios |
|---|---|---|---|---|
| F01 | Sidebar Workbench/Search/Compass/AI setup buttons; `app/Sidebar.tsx` | React `setActiveView` | ready Vault; exactly one view active, focus remains usable | C; package each destination, keyboard focus/order |
| F02 | Language select; `Sidebar.tsx`, `foundation.js` | locale resource swap | ready app; `html.lang`, labels and layout change, selection persists per contract | P(part): EN→KO→EN/narrow; reload persistence, open-modal swap, keyboard |
| F03 | Capture/Task radios and entry text; `WorkbenchView.tsx` | local state | ready Workbench; selected mode and text retained through validation/failure | P(part): mode selection; empty/failed/retry/keyboard submission unproved |
| F04 | Save Capture / Task; `WorkbenchView.tsx` | `POST /captures` or `POST /tasks` | nonempty, not busy; one canonical record and Task provenance, input clears only on success | P(part): both happy paths; injected failures, double-submit disabled, Enter |
| F05 | Workbench retry alert; `WorkbenchView.tsx` | `GET /workbench` | initial/refresh error; clears error and replaces projection on success | C; packaged fail→retry absent |
| F06 | Active shortcut Open; `WorkbenchView.tsx` | local `setDetail` then `GET /tasks/{id}` | active shortcut; matching Task detail opens | C/V; package identity, missing target, keyboard |
| F07 | Refining shortcut Refine; `WorkbenchView.tsx` | local `setRefining`, `POST /{captures|tasks}/{id}/refinement` | refining shortcut; correct saved session opens | P(part via Task detail only); shortcut/capture path absent |
| F08 | Canonical Task Open and Capture/Task Refine; `WorkbenchView.tsx` | detail/refinement local state + Task routes | canonical card; correct overlay, no duplicate item | P(part): Task Open/Refine; Capture Refine and keyboard absent |
| F09 | Task detail Close; `TaskDetail.tsx` | local `onClose` | detail open; closes without mutation and returns focus predictably | P; Escape/focus restoration absent |
| F10 | Task Refine; `TaskDetail.tsx` | opens `RefinementPanel` | loaded Task; exact Task session opens | P(part): open/close/restore; send/failure absent |
| F11 | Start / Complete-focus / Reopen; `TaskDetail.tsx` | `POST /tasks/{id}/transitions` or focus completion field | state-specific; persisted exact revision transition or focus change | P(part): Start and completion; Reopen, stale, failure, double click absent |
| F12 | Task title/detail/outcome/scope/non-goals/criteria + Save changes; `TaskDetail.tsx` | `POST /tasks/{id}/revisions` | loaded Task; new immutable revision and visible fields | P(part): title revision in review; all fields, no-op, stale/fail/retry absent |
| F13 | Work Log text/file/Add; `TaskDetail.tsx` | file base64 + `POST /tasks/{id}/work-log` | nonblank text; entry and optional bytes persist, inputs clear on success | P happy text/file; empty disabled, read error, stale/fail/retry/keyboard absent |
| F14 | Work Log comment input/Add; `TaskDetail.tsx` | `POST /work-log/{id}/comments` | existing entry/nonblank; comment persists and input clears | P happy; disabled/fail/retry/keyboard absent |
| F15 | Checklist add/toggle; `TaskDetail.tsx` | `POST /tasks/{id}/checklist`, `PUT .../{item}` | nonblank/existing item; item and checked state persist | P happy; uncheck, stale/fail/retry/keyboard absent |
| F16 | Decision add; `TaskDetail.tsx` | `POST /tasks/{id}/decisions` | nonblank; append-only decision persists | P happy; disabled/fail/retry/keyboard absent |
| F17 | Readiness `not applicable` reason/action; `TaskDetail.tsx` | `POST /tasks/{id}/readiness-decisions` | missing field + nonblank reason; revision-bound status/reason visible | C; all keys, disabled, stale/fail/retry absent |
| F18 | Problem Unlink / Resolve Problem; `TaskDetail.tsx` | `DELETE .../problem-links/{link}`, `POST /problems/{id}/resolutions` | exact links; link removed or exact Problem resolved independently | P(part): Resolve; Unlink, stale UI error/retry absent |
| F19 | Task-relationship Unlink; `TaskDetail.tsx` | `DELETE .../relationships/{id}` | existing edge; removed from both projections without deleting Tasks | C; package happy/stale/fail absent |
| F20 | Connection details disclosure; `TaskDetail.tsx` | native `<details>` | detail open; reveals/hides relationship editors without mutation | P open; close/keyboard absent |
| F21 | Create and link Problem; `TaskDetail.tsx` | `POST /problems`, then `POST /tasks/{id}/problem-links` | nonblank; Problem r1 and exact Task link visible | P happy; first/second operation failure and retry/atomicity UI absent |
| F22 | Revise Problem; `TaskDetail.tsx` | `POST /problems/{id}/revisions` | Problem ID + statement; revision increments, fields update | P happy; invalid/stale/failure/retry absent |
| F23 | Link Problem ID/revision; `TaskDetail.tsx` | `POST /tasks/{id}/problem-links` | ID and positive revision; exact link visible | P happy; unknown ID/revision, duplicate, fail/retry absent |
| F24 | Relationship kind select + Link Task; `TaskDetail.tsx` | `POST /tasks/{id}/relationships` | target ID; typed edge persists | P(part): prerequisite; related/split/self/duplicate/cycle failures and retry absent |
| F25 | Conflict Review Run/Retry/Cancel; `ConflictReviewPanel.tsx` | `POST /conflict-reviews`, `POST .../cancel`, polling GET | exact Task revision; queued/running/cancelled/stale/finding/current result rendered | P(part): queue/cancel/stale/retry/findings; clear/insufficient/failed/retry-error absent |
| F26 | Completion evidence + Complete; `TaskDetail.tsx` | `POST /tasks/{id}/completions` | in-progress/nonblank; Task only closes with evidence | P happy; disabled/stale/fail/retry and keyboard absent |
| F27 | Create Knowledge draft; `TaskDetail.tsx` | `POST /tasks/{id}/knowledge/drafts` | completed Task; exact draft revision/hash/body visible | P happy; failure/retry and incomplete disabled state absent |
| F28 | Correct / Publish draft; `TaskDetail.tsx` | `POST .../correction`, `POST .../publish` | current draft/hash; new hash or published file/state | P happy; stale hash/external change/failure/retry/double-submit absent |
| F29 | Regenerate / Withdraw Knowledge; `TaskDetail.tsx` | `POST .../regenerate`, `POST .../withdraw` | publication state permits; new draft or withdrawn state visible | P happy; external-change/failure/retry absent |
| F30 | Load lineage; `TaskDetail.tsx` | `GET /tasks/{id}/lineage` | Task available; ordered nodes render | C/V; package keyboard, empty/failure/retry absent |
| F31 | Refinement Close/autosave; `RefinementPanel.tsx` | `PUT /refinement/{id}/workspace` then close | session loaded; note/tab/scroll persist, close blocked on save failure | P(part): successful close/restore; close failure/retry and Escape absent |
| F32 | Refinement Conversation/Proposals tabs; `RefinementPanel.tsx` | local select + delayed workspace PUT | session loaded; selected tab persists and correct panel visible | P happy; keyboard tab semantics/fail absent |
| F33 | Refinement message Send; `RefinementPanel.tsx` | workspace PUT then `POST .../messages`, poll | nonblank/not busy; user message persists, AI terminal state/proposals appear | C; package success/fail/cancel/provider absence/retry/double-submit absent |
| F34 | Saved refinement note + message-scroll autosave; `RefinementPanel.tsx` | debounced workspace PUT | session loaded; input, tab and anchor resume after switch/relaunch | P(part): note/tab/scroll close/reopen; debounce failure and relaunch absent |
| F35 | Proposal Edit/Accept/Reject; `RefinementPanel.tsx` | local payload edit + `POST .../proposal-decisions` | proposal exact revision; only accepted operations persist, decision removed from pending | C; package edit/accept/reject/stale/fail/retry/keyboard absent |
| F36 | Search query/Semantic submit; `SearchView.tsx`, `search-settings.js` | `GET /search?...&semantic=true` | nonempty query; loading then lexical/semantic results or error | C; packaged lexical/semantic/empty/fail/retry/Enter absent |
| F37 | Search result / Show 20 more / Knowledge retry; runtime | result click or Enter/Space; paged GET; `GET /knowledge` | result/20 rows/read failure; opens exact reader, appends no duplicate, retry replaces error | C; packaged pointer+keyboard, slow-race, fail/retry absent |
| F38 | Compass goal Add; `CompassView.tsx`, `search-settings.js` | `POST /goals`, `GET /dashboard` | nonblank; goal persists and dashboard refreshes | C; packaged happy/fail/retry/Enter absent |
| F39 | Provider fields, 13 advanced toggles, worker count, Save; `SettingsView.tsx`, runtime | `PUT /provider/config` | valid config; status updates, secret field clears, nonsecret config reloads | C; package valid/invalid/fail/retry/keyboard and each toggle state absent |
| F40 | Test connection/list models; `SettingsView.tsx`, runtime | provider test route | configured/unconfigured provider; result or safe error status | C; packaged success/fail/timeout absent |
| F41 | MCP connection name/scopes/topics/Create; `McpConnections.tsx` | `POST /work-tracking/connections` | valid name; connection and exact grant set visible | C; package scope pairwise, validation/fail/retry/disabled absent |
| F42 | Connection access disclosure / Revoke; `McpConnections.tsx` | `<details>`, `DELETE /work-tracking/connections/{id}` | active connection; command/scopes visible or state revoked | C; packaged keyboard/happy/fail/retry absent |
| F43 | Topic membership disclosure/type/include select/Save; `McpConnections.tsx` | `PUT /work-tracking/topic-membership` | valid topic/member; status event and membership persistence | C; packaged include/remove, each entity type, fail/retry absent |
| F44 | Chat Track this chat toggle; `OverlayLayer.tsx`, `work-tracking.js` | tracking selection/events | chat target available; tracking surface attaches/detaches without duplicate | C; packaged both states/reopen/failure absent |
| F45 | Chat propose-action buttons; `ChatTrackingSurface.tsx` | `llm-wiki:chat-tracking-propose` | available action/not busy; matching proposal card appears | C; packaged each action/disabled/failure absent |
| F46 | Work-tracking card Reject/Edit/Accept, Defer/Review/Edit draft/Publish; `WorkTrackingCards.tsx` | tracking action events | stage/publication conditional; exact revision decision and projection status visible | C; package every conditional branch, stale/fail/retry/disabled absent |
| F47 | Chat JSON editor Cancel/Preview; `ChatTrackingSurface.tsx` | parse JSON + tracking edit event | editable card; cancel discards, valid object previews, invalid JSON alerts | C; package valid/invalid/array/cancel/keyboard absent |
| F48 | Feature proposal, draft review, manual update forms; `OverlayLayer.tsx`, `manual.js`, `solution-work.js` | legacy Problem→Solution/create, draft finalize, item PUT | legacy triggers; modal cancel leaves state, submit persists exact fields or error | C; old-board triggers unreachable in Task UI; packaged happy/fail/cancel/Enter/Escape absent |
| F49 | Legacy chat quick prompts, Ask, Close, preview Context/Detail, Apply; `OverlayLayer.tsx`, `explore.js` | workflow chat, draft/refine jobs, apply PUT/promote/create | legacy target; modal/poll/status/draft/application visible | C; no Task-artifact packaged coverage; success/fail/retry/cancel/race/keyboard absent |
| F50 | Notice Confirm/Cancel/Escape; `OverlayLayer.tsx`, `solution-work.js` | promise resolver | destructive/confirmation caller; true/false returned exactly once, modal closes | C; packaged caller matrix and keyboard absent |
| F51 | Transition modal generated fields/Continue/Cancel; `OverlayLayer.tsx`, `transitions.js` | transition-specific native route | legacy transition selected; conditional required fields enforced and state advances only on submit | C; packaged every transition, invalid/fail/retry/cancel/keyboard absent |
| F52 | AI Queue toggle/outside-close/header-close; `OverlayLayer.tsx`, `jobs.js` | local panel + `GET /jobs` | any ready view; `aria-expanded` and panel visibility agree | C; packaged open/each close/focus/keyboard absent |
| F53 | Queue job Cancel/Retry/Open result; `jobs.js` | `POST /jobs/{id}/{action}`, result router | status/result-interface pair; legal action shown, target status/result destination updates | C; packaged pairwise task-kind × terminal/active state absent |
| F54 | Notifications toggle/close/Open/Dismiss; `jobs.js` | GET, mark read, dismiss, result router | notifications exist; unread badge/read state/dismissal/result destination update | C; packaged empty/unread/open/dismiss/fail/keyboard absent |
| F55 | Queue toast Dismiss/timeout; `jobs.js` | local hide/timer | new unread notification; toast hides manually or at 5 s | C; packaged timing/new-notification race absent |
| F56 | Item/Knowledge detail Close; `OverlayLayer.tsx`, runtime | dialog close | detail reader open; closes and restores usable focus | C; packaged button/Escape/focus absent |
| F57 | First-run Skip/progress dots/Continue/Choose Vault/Try again; `FirstRunIntro.tsx` | scene state + `onFinish` | first run; scene/focus advances, picker starts once, failure exposes retry | C/V; package auto-advance/reduced-motion/manual/skip/error/retry/keyboard absent |
| F58 | Vault setup Choose/Try again busy states; `VaultSetupView.tsx`, `App.tsx` | native choose + status retry | setup required/error; app stays inert, cancel preserves requirement, success unlocks | C; packaged choose/cancel/error/retry/disabled/focus trap absent |
| F59 | Migration Restore verified backup / Retry migration; `MigrationRecoveryView.tsx` | native restore/retry | recovery manifest flags; app remains inert, selected recovery succeeds or safe error remains | C; package restore/retry/unavailable/busy/fail/relaunch absent |
| F60 | Legacy board card open, More actions, priority, manual, delete, drag; `workbench.js`, `manual.js`, `conflicts.js` | detail modal, priority PUT, item PUT/DELETE, category override | legacy board exists; correct card changes and reloads | C; trigger unreachable in Task Workbench; packaged pointer/keyboard/menu/outside/fail/cancel absent |
| F61 | Legacy Problem Approve / Explore next solution; `workbench.js`, `conflicts.js`, `explore.js` | `POST /problems/{id}/approve`; `openNextChat` then `/draft` | unapproved/approved Problem; state or chat preview changes visibly | C; trigger unreachable in Task Workbench; installed-app report requires isolated reproduction |
| F62 | Legacy Solution conflict/stage/completion controls; `solution-work.js`, `conflicts.js` | conflict review, state PUT, completion review | solution state/conflict pair; only legal action shown, status/detail changes | C; packaged pairwise states, fail/retry/cancel/continue absent |
| F63 | Legacy Solution Work composer, paste image, Enter/Shift+Enter, summarize, comments, checklist edit/toggle; `solution-work.js` | progress/comment/checklist routes and AI job | open Solution detail; bytes/text/checks persist and keyboard contract holds | C; no Task-artifact package evidence for this legacy surface |
| F64 | Legacy Workbench capture/organize/flow/in-progress cards; `workbench.js` | capture POST, organize job, local flow, stage/review | old board elements exist; board or flow/status changes | C; triggers unreachable in Task Workbench; packaged states/failure/keyboard absent |
| F65 | Archive open/show more/regenerate/delete/force-delete; `workbench.js`, `archive.js` | Knowledge GET, regenerate POST, DELETE with optional force | archived/missing/external-change states; exact file/result or confirmation outcome visible | C; packaged happy/cancel/external-change/fail/retry/keyboard absent |
| F66 | Completed workspace Result/Lineage/Evidence/Archive tabs, references, records, follow-up Problem, file open/regenerate/delete; `completed-workspace.js` | lineage/evidence reads and follow-up/archive routes | completed Solution; readonly record preserved and selected result opens | C; packaged tab/reference toggle/close/follow-up/archive state matrix absent |
| F67 | Conflict result note/decisions/continue and completion review choices; `conflicts.js` | decision/transition routes | findings or completion proposal; choice persists, cancel leaves state | A(part): packaged queue and notification results persist both clear/conflicted decisions with notes; findings/continue/fail/stale/cancel remain |
| F68 | Dialog native cancel/outside-panel handling across chat/draft/manual/notice/transition/queue/alert | `cancel`, close buttons, capture click handler | each overlay open; close is nondestructive, async work cancelled where specified, focus remains reachable | C; systematic packaged Escape/outside/focus restoration absent |

The inventory contains **68 control families**: 47 React/current shared families (F01–F47), 12 shared overlay/runtime families (F48–F59), and 9 legacy dynamic families (F60–F68). This is a finite family count, not an element count: repeated checklist items, generated job cards, 13 advanced-model checkboxes, 10 MCP scopes, proposal cards, and status-dependent buttons are covered by pairwise state scenarios rather than an impossible Cartesian product.

The executable source registry currently scans **27 shipping frontend/runtime files** and finds **167 interactive-source matches** mapped to **169 manifest controls**. That element-level scan supplements the 68 semantic families; it does not turn a source match into package coverage until the control is rendered, exercised, and followed by an independent effect assertion.

## Genuine packaged coverage and untested set

The nine passing package journeys directly exercise parts of **26 families**: F02–F04, F08–F16, F18, F20–F29, F31–F32, and F34. Even within these families, only the states stated in the ledger are proven. No packaged proof exists for 42 whole families, and failure/retry/disabled/cancel/keyboard coverage is incomplete in every family.

Highest-risk untested groups are:

1. F31–F35 refinement send, provider failure, proposal decisions, stale revision and close-save failure. This includes the user's refinement concern in the actual Task surface.
2. F01, F36–F43 shell navigation, Search, Compass, provider settings and MCP settings. These ship but none is in the nine package journeys except locale switching.
3. F48–F59 conditional modals, queue, notifications, onboarding, Vault selection and migration recovery. Most are hidden by normal fixtures, so click sweeps miss them.
4. F60–F68 legacy dynamic runtime. It is loaded into the package; some triggers are unreachable from the Task Workbench, which must be asserted explicitly rather than silently excluded.
5. Failure and recovery for F04–F29. At the initial nine-case baseline, happy paths dominated package evidence; optimistic conflicts, partial failures, retry enablement and duplicate suppression need deterministic injection.

## Finite scenario design

For each family, use the smallest pairwise matrix that changes behavior:

- availability: visible/enabled, visible/disabled, conditionally absent;
- operation: happy, deterministic native/provider failure, retry success;
- interruption: cancel/close before commit and close during pending work where supported;
- input: pointer and keyboard (`Tab`, `Shift+Tab`, `Enter`, `Space`, `Escape`), plus IME-safe Enter where the handler has special logic;
- persistence: immediate DOM result, native record/readback, close/reopen, and relaunch only where the contract promises it;
- concurrency: current revision, stale revision, double activation while busy;
- layout/localization: one wide English and one narrow Korean pass, with long content and reduced motion for families whose layout or animation changes.

Generated collection controls use representative pairwise cases: first/middle/last item, empty/nonempty, active/terminal/error, and current/stale. Every scenario must name the selector or accessible label driven, the native operation observed, and the persisted or user-visible assertion. A successful click dispatch alone is never a pass.

The automated source gate discovers shipping React control sources with a Vite glob and derives the legacy runtime set from the scripts loaded by `frontend/index.html`. It rejects a newly added or conditional control when the source has no stable control identity, manifest record, and effect-bearing scenario. Old-board-only markup may appear only in the explicit evidence list in `legacyReachability.ts`; those entries describe missing Task-artifact entrypoints and never count as exercised controls. Packaged scenario processes report control IDs, and the final gate unions those IDs before failing on controls that were not rendered, exercised, or independently asserted.

## Historical implementation split

- Harness/manifest: enumerate F01–F68 with stable accessible selectors and reject scenarios that lack an effect assertion.
- Current Workbench core: F03–F30 happy/fail/retry/disabled/stale, preserving the existing nine journeys as smoke coverage.
- Refinement/chat: F31–F35 and F44–F49, including provider absence, polling terminal states, proposal decisions and A→B→A/relaunch.
- Shell/tools: F01–F02 and F36–F43 for navigation, Search, Compass, provider and MCP settings.
- Conditional system UI: F50–F59 for confirmations, transition forms, queue, notifications, onboarding, Vault setup and migration recovery fixtures.
- Legacy runtime: F60–F68 with explicit reachability assertions from the final Task artifact; drive reachable surfaces and record unreachable old-board triggers as removal/dead-code decisions, not interaction passes.
- Final artifact gate: build once, record hash/signing time, run the complete manifest against that exact path with isolated state, then compare the launched executable path and code hash before accepting results.
