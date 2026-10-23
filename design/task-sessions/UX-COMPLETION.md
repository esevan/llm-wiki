# Approved session UX completion follow-up

This is the correction of gaps in shipped `d816280`, following the already approved mockups. It is separate from the preparation functionality hotfix `6333386`. The user subsequently authorized integration of these corrections and the separately reviewed preparation hotfix. Original commits remain preserved on the design branch; the dedicated integration worktree applies only these changes on Capture main `cbbc892`. **No rendered pixel acceptance has been performed:** the Mac remains locked; no unlock retry or alternate screen-control path was attempted.

## Component and state completion matrix

| Approved element / state | Shipped gap | Current source / DOM correction | Verification boundary |
| --- | --- | --- | --- |
| Automatic noneditable labels | Already connected | Retained; no title/provider form restored | Existing DOM tests |
| Empty state | Global static creation toolbar | New/link actions live once in empty state; in populated sessions they belong to navigation | Existing creation/link tests |
| Unprepared setup | Settings closed below transcript/composer | Setup automatically opens before context/transcript, with setup heading and project/model/effort fields | New DOM ordering/open-state test |
| Session environment | Scattered prepare/save and nested grids | Saved path/model/effort in summary; model+effort adjacent; approval advanced; fields and read-only connection separated; actions in one footer | Model/effort tests, typecheck; rendered spacing pending |
| Cancel settings | Missing from implementation | Restore last loaded/saved settings in memory without an API write; preserve composer draft and existing execution identity | New cancellation regression |
| Folder error | Only global message | Invalid-folder errors identify and describe the folder field; provider wording preserved | Source; actual provider error screenshot pending |
| Ready request | Generic instruction/note label | Visible next-work-request label, lower-emphasis note action; existing Enter/IME/attachment semantics retained | Existing request/attachment tests |
| Running | New behavior hidden behind generic wording | Visible additional-work-request label; same enabled enqueue path; empty queue destination visible while active | Running enqueue/draft test and new empty-queue test |
| Approval/input | Decision after history/tools | Active execution/decision block before context/transcript; provider choices before optional live tool output | DOM ordering + existing formal routing tests |
| Current versus old Run | Current state could be visually buried | Current Run block renders first; terminal selected detail remains after transcript/history | Source + existing current-Run selection behavior |
| Pending queue | Connected but crowded controls | Preserve snapshot/edit/move/remove/stop-and-drain behavior; controls have real gaps and move below item text on constrained widths | Queue interaction tests; pixel pending |
| Stop pending / uncertainty | Existing terminal gating | Retained exact Run target, confirmed termination requirement and explicit pause/reconciliation states | Existing native fake lifecycle tests; UI state fixture |
| Held | Execution guard and note path | Retained guard and readable history; environment stays optional | Existing held test |
| Completed | New session action remained visible | No create/link/Run/edit-settings; disclosed append-only note remains | Completed-note test now asserts absent create/link |
| Result/history / Work Log | Existing provenance | Final report, collapsed history and exact Work Log links retained; no synthetic summary replaces evidence | Existing result/Work Log tests |
| Wide layout | Partial scoped override | 1248px maximum, 224px rail, 24×36px content inset, 20px panel radius; navigation owns actions | CSS source only; pixel pending |
| Narrow layout | Nested form/queue/control compression | 760px rail transition, 20px content inset, 17px radius; settings/connection and workspace picker stack; queue controls wrap below text by 900px | CSS source only; pixel pending |

## Specific CSS collision corrections

Scoped rules reset legacy settings `display:grid` on the disclosure itself, provide an explicit two-column body with full-span heading/actions/effective settings, and remove legacy direct-child order as a source of visual/keyboard disagreement. Approval choice rows, fieldsets, environment footer, composer footer and queue controls have explicit 8px or larger gaps. Buttons use border-box sizing, 36px minimum height, wrapping and bounded width. Flex/grid children use `min-width:0`; long paths/messages wrap. Workspace picker becomes one column at narrow widths. Shared application tokens and existing focus/hover/pressed/disabled semantics remain.

These are source-level fixes for identified collision paths, **not a claim that every reported overlap has been reproduced or eliminated in pixels**. The surrounding shared Task modal/header is retained; this follow-up does not restyle other Task tabs. Session navigation is bounded to the approved width inside it.

## Review fixture and remaining acceptance

`frontend/session-review.html` renders actual shipping components with synthetic IPC/data and no provider/storage. States: `empty`, `setup`, `ready`, `environment`, `environment-error`, `running`, `approval`, `input`, `stop`, `queue-pause`, `uncertain`, `held`, `completed`. Add `width=narrow` for 560px component surface and `lang=en` for English. Environment-error is simulated only after Prepare is clicked; other buttons are fixture behavior, not evidence of native execution. Earlier screenshots 01–18 remain mockups.

After manual Mac unlock, inspect wide/narrow Korean and English states, button rectangles/gaps, long path and queue wrapping, decision visibility, keyboard order and focus. Capture actual comparison screenshots and upload verified Library IDs. No fresh implementation screenshots currently exist.

Verification: full `npm test` chain passed with 506 frontend tests. Typecheck, lint, frontend production build and whitespace checks are recorded in the follow-up handoff. Packaged E2E is skipped: existing native lifecycle/transaction coverage and current DOM behavior tests cover execution contracts; changes here are layout/composition and local cancellation. The remaining material check is actual rendered UI; do not substitute a release build or broad E2E for that missing evidence.

## Preparation incident diagnostics (not resolved by UI work)

Local installed CLI is `codex-cli 0.162.1`. Read-only generated protocol schema supports `thread/start` and `thread/resume`, optional historyMode (`legacy`/`paginated`), and documents resume by ID as loading from disk for non-running threads. No compatibility mismatch has been established from that schema. No provider generation, user settings modification or secret/session-internal read was performed. The existing adapter discards provider stderr, so retrospective provider stderr is unavailable through this integration; this follow-up does not enable broad or sensitive logging.

The separate hotfix prefixes new-thread RPC errors `conversation_start_failed:` and other resume errors `conversation_resume_failed:`. Missing-rollout resume errors begin `conversation_unavailable: no rollout found...`, followed by preservation/retry guidance. Fresh first preparation sends one start/no resume in the fake; the user's actual first-Prepare failure is still unconfirmed and unresolved. Obtain the full displayed error and action sequence, identifying first Prepare versus repeat/Check settings, whether settings were saved, and whether the app/provider restarted. Do not suggest that recreating a session is a proven remedy. Never clear IDs, replace history, or automatically resend work based on this diagnostic uncertainty.
