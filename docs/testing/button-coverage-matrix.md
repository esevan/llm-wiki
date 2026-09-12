# Button interaction evidence

> **Scope note:** This matrix combines current Task controls with loaded legacy runtime bindings. Rows naming board, Problem, Solution, Explore, manual transition, or completed-workspace actions are technical compatibility evidence only unless the current Task interface renders that control.

This audit follows action contracts rather than counting raw `<button>` strings.
A card renderer can emit many copies of one action, while translated React
actions can have several labels. Each row names the production render/event
path, the exact test, and the observable result. Disabled variants are included
where they are part of the contract.

| Rendered action contract | Exact automated evidence | Observable result |
| --- | --- | --- |
| Sidebar Workbench, Search, Compass, AI setup | `frontend/src/app/Sidebar.test.tsx` — **changes views only after each rendered navigation action is clicked**; packaged clicks in `frontend/src/test/desktopScenario.ts` | Controlled view changes; packaged hit testing rejects covered, disabled, or zero-area controls. |
| Save Capture | `desktopScenario.ts` — rendered **Save Capture** click | Capture persists through Tauri and appears on the board. |
| Search submit, Show 20 more, result open, document retry | `frontend/src/test/remainingButtonsRuntime.test.ts` — **clicks search more/open/retry and provider save/test with success and error outcomes**; stale/error cases in `frontend/src/test/searchRuntime.test.ts` | Results render, next offset loads, encoded Vault path opens, and request failure renders an error. |
| Compass goal save | `frontend/src/test/remainingButtonsRuntime.test.ts` — search/provider/goal case | Goal request submits and dashboard refreshes. |
| Provider save/test | `remainingButtonsRuntime.test.ts` search/provider case; packaged provider-test click | Save clears the key input; test success lists models and failure appears in status text. |
| Vault choose/retry | Three click cases in `frontend/src/features/vault-setup/VaultSetupView.test.tsx` | Picker callback runs once, duplicate initiation is disabled, and retry remains available after rejection. |
| First-run skip, scene dots, next/choose, retry | Advance, skip, exact-scene, and retry cases in `frontend/src/features/first-run-intro/FirstRunIntro.test.tsx` | Scene changes and setup callbacks occur only after clicks; working state locks navigation. |
| MCP create, revoke, topic membership save | Both cases in `frontend/src/features/settings/SettingsView.test.tsx`; packaged lifecycle in `desktopScenario.ts` | Requests contain the selected scope/topic and persisted revocation is read back. |
| Track Chat; proposal reject/edit/accept; edited payload | `frontend/src/features/chat/ChatTrackingSurface.test.tsx` — **dispatches proposal, decision, publication, and edited-payload actions only after their buttons are clicked**; four integration cases in `ChatTrackingRuntime.test.tsx` | Production events fire; persistence occurs only on accept and stale responses are ignored. |
| Completion defer/review/publish and saved-draft edit | Decision/edit cases in `frontend/src/features/chat/WorkTrackingCards.test.tsx` | Separate callbacks fire and missing/busy draft actions remain disabled. |
| Feature proposal save and AI draft finalize | `remainingButtonsRuntime.test.ts` — **submits feature and reviewed draft forms from their rendered primary buttons** | Final production handlers send validation criteria, refresh, and restore enabled state. |
| Manual save | `remainingButtonsRuntime.test.ts` — **submits manual edits through the rendered save button and restores it after failure** | PUT succeeds and refreshes, or failure shows a notice and re-enables Save. |
| Manual transition Continue | `remainingButtonsRuntime.test.ts` — **opens and submits each manual transition template and reports API failures** | All four transition IDs submit; success result and failure recovery are asserted. |
| Feature, Chat, draft, manual, transition Cancel/Close | Five close-isolation cases in `frontend/src/features/overlays/OverlayLayer.test.tsx`; `frontend/src/test/buttonDelegationRuntime.test.ts` — **binds refinement cancellation to the rendered Close action, never Track this chat** | Each click closes only its dialog; adjacent Track cannot be mistaken for Close. |
| Notice confirm/cancel | `frontend/src/test/completedButtonsRuntime.test.ts` — **resolves the production confirmation notice on both cancel and confirm clicks** | Pending promise resolves false/true and modal closes. |
| Card More menu, priority | `remainingButtonsRuntime.test.ts` — **renders and clicks menus, priority, manual, workflow, archive, flow, and organize actions** | Capture listener opens the menu; production priority PUT and visible status run. |
| Manual edit, delete, Approve Problem, Explore next | Same Workbench case; `buttonDelegationRuntime.test.ts` — **routes every board button action from nested targets to its production handler**; packaged Approve/Explore clicks | Nested targets resolve correctly; approval persists and Explore opens Chat. Delete is confirmation-gated. |
| Review conflict, Start/move proposed, completion review | Same runtime cases; open/failure cases in `frontend/src/test/conflictReviewRuntime.test.ts` | Correct Solution/action reaches production and failed busy state restores. |
| Copy handoff and four transition menu rows | Workbench/transition cases in `remainingButtonsRuntime.test.ts`; notice/external outcomes in `completedButtonsRuntime.test.ts` | Rendered inline action reaches the production function; every transition ID opens its form. |
| Flow open/close, archive more/fewer, organize | Workbench case plus packaged paired clicks | Hidden state, labels, limit, queued request, busy state, and Queue refresh change. |
| Archive document and completed Solution open | Workbench case in `remainingButtonsRuntime.test.ts` | Delegated click makes the encoded Knowledge request or renders completed detail. |
| Explore quick prompt and AI draft | `remainingButtonsRuntime.test.ts` — **clicks draft, tabs, apply, chat send, and preview retry through the rendered handlers**; quick-prompt case in `buttonDelegationRuntime.test.ts` | Prompt submits Chat; rendered draft reaches `draftWithAI`. |
| Preview Detail/Context tabs and Apply/Create | Same Explore case; completed tabs in `completedButtonsRuntime.test.ts` | Selected panel appears; Apply sends the correct endpoint, changes APPLIED/CREATED state, disables the applied action, and refreshes/closes. |
| Preview status/retry, detached warning retry, and Chat send | Same Explore case | Streaming request starts. Clicking error status returns to Context and reloads preview; when the preview itself is hidden after a context failure, the visible warning button retries and clears on recovery. |
| No conflict / Keep conflicted | `remainingButtonsRuntime.test.ts` — **clicks no-conflict decisions, every per-conflict resolution, continue, and failure recovery**; `conflictReviewDecision.test.ts` | Delegated choice sends state and refreshes; failure restores its action. |
| Apply recommendation / Ignore conflict / Continue | Same remaining-runtime case; labels in `conflictReviewChoiceLabels.test.ts` | Both choices update resolved state, Ignore requires rationale, Continue saves, and failed save shows inline error and re-enables. |
| Queue cancel/retry/result; notification open/dismiss | `frontend/src/test/jobsProductionRuntime.test.ts` — **renders production Queue and notification actions before dispatching their real handlers** | Production renders controls first; exact requests and result interface dispatch follow clicks. Disabled result variants are asserted. |
| Queue/Notifications toggle and close; toast/detail close | Same jobs test; outside-click cases in `OverlayLayer.test.tsx`; item-close click in `remainingButtonsRuntime.test.ts`; packaged panel clicks | Panels/modal/toast close, aria-expanded follows state, and outside dismissal preserves the underlying click. |
| Progress save, comment, checklist add/toggle, image summary | Work-log and image success/failure cases in `completedButtonsRuntime.test.ts` | Production-rendered controls issue exact requests; queued image summary waits for job completion before refresh and failure restores the action. |
| Completed Detail/Context/Work/Archive tabs; open/regenerate/delete; follow-up/reference/correction | `completedButtonsRuntime.test.ts` — **uses completed workspace tabs, archive actions, follow-up, lineage references, and corrections from production markup** | Each panel appears and every native/API outcome is asserted, including confirmation and safe refresh. |
| Archive row regenerate/delete and missing file | `completedButtonsRuntime.test.ts` — **clicks archive-row regenerate and delete controls and keeps the missing-file icon disabled** | Delegated actions reach exact endpoints; missing file control is disabled. |

`clickRenderedButton` in `frontend/src/test/desktopScenario.ts` scrolls the
control into view, rejects disabled and zero-area elements, checks that its
center resolves to the control or a child, then clicks. For each covered button, packaged assertions read visible DOM or isolated native
state after the click. Other API calls prepare fixtures and verify native contracts;
they are not counted as button-click evidence.

The macOS folder chooser, keychain consent UI, host file opener, and image
clipboard contents are outside the WebView. Tests click their in-app initiating
controls and assert the callback/native request, disabled or retry state, and
isolated persisted result. They do not automate the OS-owned confirmation
surface or touch a user Vault, keychain, or database.
