# Task/session design review — approval required

Historical status: this document records the design-only approval phase. Implementation is now authorized on this branch; see [implementation checkpoint](IMPLEMENTATION.md) for current changes, evidence and visual-review blocker. During the original design phase no production files, tests, persistence, APIs or packaged app were changed. Mockup controls are simulated; only review navigation, native details disclosures, text entry and radio selection work. Approval/submission/run/stop/save/link buttons never invoke Codex or write data.

## Review entry points

Serve repository worktree root, then open `design/task-sessions/index.html`. Query `state` selects empty, setup, ready, running, approval, input, result, held, completed, uncertain. `narrow.html` embeds the input state in a real 560 CSS-pixel viewport. Review strip is tooling, excluded from proposed product UI.

## Actual workflow audit

Sources: `frontend/src/features/workbench/TaskWorkSessions.tsx`, its tests, `TaskDetail.tsx`, `frontend/src/types/taskWorkbench.ts`, `docs/features/task-work-sessions.md`, Korean counterpart, token stylesheet, UI design skill and product spirit. Current UI duplicates session picker/rail, run status/activity/history, and shows new/link/settings/note/run controls broadly. Title is manually editable despite being unnecessary for the user's work.

| State | User need | Proposed visible actions | Existing boundary |
| --- | --- | --- | --- |
| No session | Start focused work or attach prior conversation | New session, link Codex session | Creation/link does not run work |
| Unprepared | Choose execution context once | Folder + Browse, save settings, prepare conversation | Preparation stays separate from Run; model/approval options disclosed |
| Ready/finished active Task | Give the next instruction | Compact request composer, attach, Run; lower-emphasis save note | Explicit Run only; immutable request retained |
| Queued/running | Understand progress or interrupt | Live status/activity, Stop request; optional disclosed note | No Queue/Steering API, so neither offered |
| Approval needed | Decide actual request | Provider-supplied choices plus Stop | No fabricated approval choices; example labels only in mockup |
| Input needed | Answer actual questions | Only requested options/fields and explicit submit plus Stop | No send-on-selection; secret requests route to Codex; stale read-only |
| Terminal result | Read what happened | Work Log link, collapsed exact records, next request when active | Requested → Codex action → result; no implied Task completion |
| Failed/cancelled/interrupted | Review evidence then retry | Retry beside that attempt; reuse instruction only if draft empty | Preserve evidence and original Run link; current draft never sent by retry |
| Uncertain | Avoid duplicate execution | Prominent uncertainty + saved evidence, eligible retry | No invented reconciliation/force-clear capability |
| Held | Read/preserve; resume Task explicitly | Read history, optional note; resume through existing Task menu | New Run blocked until Task resumed |
| Completed | Read frozen work; optional addendum | History/Work Log, disclosed post-completion note | No Run, settings edit or external linking; append-only note path |

## Automatic label proposal (requires approval)

Remove every manual title-edit field, pencil, rename menu and title validation dependency. Derive display label deterministically from first nonempty saved Run instruction, else first saved note, else linked conversation title, else `세션 02 · 10월 23일`. Normalize whitespace; show up to 36 Unicode characters with full accessible label/tooltip. Keep stable ordinal/date as metadata. Never call a model or rewrite saved historical titles for this display change. Existing persisted title remains untouched. This display algorithm is proposed behavior, not an existing capability. Sidebar example is illustrative, not an actually generated AI summary.

## Layout/component acceptance criteria

- Reuse existing cream, white, ink, pink, mint and lemon tokens exactly. Import `tokens.css`; do not load legacy component styles into mockup. Production must use scoped classes to avoid legacy collisions.
- Wide surface: max-width 1248px, outer vertical margin 24px, radius 20px. Task heading inset 36px horizontally, 28px top; gap 24px. Session rail 224px; content minmax(0,1fr). Session content padding 24px 36px. No permanent third settings column.
- At <=760px: outer margin 12px, panel radius 17px, content and Task header padding 20px; rail becomes one horizontal scroll row with 180px minimum session targets. Only the rail scrolls horizontally; content must wrap without horizontal overflow.
- Session heading 20px wide / 18px narrow, Task heading 24px / 20px, body 15px at 1.65. Metadata 12px, secondary ink. Body font Nunito/Noto Sans KR; mono only paths/commands. Long labels wrap, never cover status or close button.
- Entry groups 24px apart. Request has 3px neutral left rule and 16px inset. Compact event rows 10px vertical padding; details closed initially. Decision card radius 13px, padding 24px wide /16px narrow, 20px vertical margin. Active approval/input card precedes optional tool history, avoiding the first reviewed fold problem.
- Composer radius 13px, padding 16px, textarea minimum 72px. Controls minimum 36px height, radius 9px, actions wrap with 8px gap. One pink primary intent per state except mutually exclusive provider approval choices, whose labels come from provider.
- Settings, Task context, old run details and optional running/completed notes are collapsed initially. No title input or single-option provider selector. Creation/link toolbar is suppressed in completed history; no duplicate new/link controls in final empty state (current empty mockup hides the rail).
- Empty request/answer cannot submit in implementation; pending action disables repeated submissions. Empty mockup primary is a visual specimen, not functional validation. Preserve 10MB single attachment boundary, draft image preview, remove/download and failed-send retention.
- Focus ring is 3px strong pink with 2px offset; hover accent wash, pressed 1px offset, disabled token treatment. Tab order: Task close → session navigation → context → timeline disclosures → current decision/composer → settings. Escape/disclosure returns focus; radio selection never submits. IME Enter does not submit; Shift+Enter newline. Existing Enter semantics must be retained or separately approved, never silently changed by this visual design.
- Current run status, Stop, pending formal requests remain visible if an older Run is selected; production should pin this current-run strip above historical content. Mockup shows a single current run, not the older-run interaction.
- Close is always available, never calls interrupt. Existing Task detail view/tab/scroll/draft/attachment/formal answers and run identity retention must remain intact across close/reopen within the app lifetime; do not promise draft persistence after app restart. Stop requested remains pending until terminal provider status. Completion/hold guards remain unchanged.
- Result summary must not become a second competing chat transcript in implementation: the 3-row summary belongs to Work Log, shown here as its visual specimen. Sessions use final report + Work Log link. Original evidence is never rewritten.

## Visual review and verification

Native Dia browser pixels captured at 1920×960 screen size. Wide page content viewport approximately 1724px; panel max-width 1248px. Saved screenshots show running, request, revised approval, completed and actual 560px iframe input (top + scrolled footer). Narrow input long Korean request wraps cleanly; only intentional session rail clips/scrolls. Approval initial version put buttons below fold; revised capture replaces it and shows choices fully. Screenshots include browser chrome and routine public tab titles; no private app data or credentials included.

Reviewed tokens, alignment, whitespace, readable type, wrapping, radio controls and action visibility in the above pixels. Browser screenshots are real; content is synthetic. Full English strings, keyboard focus traversal, attachment preview, loading/error variants and 320px layout were not pixel-reviewed. These remain implementation acceptance checks, not claims of completion.

Additional state criteria: loading uses one inline status and no fake skeleton transcript; save error retains draft/attachment next to retry; request-submit error retains answers and actual provider error; stale requests show recorded response with no active buttons; queued uses running layout with queued label; stop-requested disables Stop and explains terminal status is pending; Work Log pending/failed remains separate from Run result with sync retry only when eligible. These are specified, not individually rendered mockups.

`node --check design/task-sessions/mockup.js` and `git diff --check` pass. E2E and release build skipped: only isolated design artifacts changed, no runtime/native/persistence behavior. Documentation guide reviewed; production feature guides remain accurate and unchanged because nothing is implemented. This review document records proposed behavior and residual approval/verification work. No commit, merge or push performed.

## Extension: execution environment setup (design only)

Read additional actual sources: `SettingsView.tsx`, `taskExecutionClient.ts`, native `task_execution_service.rs::prepared/bind_thread/create_run` and `task_execution_runtime.rs::prepare_inner`. Do not infer current user configuration from examples: all paths/models/home labels in mockups are synthetic fixtures. No real settings or credentials were read or changed.

Existing setting scopes:

| Scope | Existing capability | Design treatment |
| --- | --- | --- |
| App common Codex connection | Default local Codex home or existing alternate path, app restart to apply alternate | Read-only summary plus collapsed problem-help path to AI setup |
| Other AI provider configuration | Endpoint, timeout 1–3600s, API key, default/advanced models and routing | Separate AI setup; these do not become the Sessions Codex login/model configuration |
| Per-session reusable settings | Codex provider, selected model, project folder, approval reviewer | One focused form: folder/Browse, recent folders, model; approval choices disclosed |
| Prepared effective settings | Provider-returned model/cwd/approval policy/sandbox/capability/settings revision | Distinguish requested values from verified effective values; never label an example as connected |
| Per-run override | **Not supported**: execute accepts instruction/operation/settings revision, not arbitrary model/cwd/approval patches | No active “only this run” selector. Disclosed explanation explicitly says unsupported |
| Cross-session presets | **Proposed**, not an existing save contract | Only named as a future proposal inside scope disclosure; no working preset action |

Validation and change flow:

1. Enter/browse existing local project directory or pick from the existing recent-10 list. No server provisioning, container, SSH, cloud machine, environment image or login wizard is invented.
2. Choose existing model options (preserve an unknown stored model in production). Provider is always Codex, so no single-choice provider dropdown.
3. Save session settings, then prepare conversation. Save invalidates prior effective readiness; prepare checks the canonical existing directory, exact conversation binding, provider startup/connection and returned effective settings. A prepared conversation is not a started Run.
4. Show exact readiness error near the relevant field. `env-error` illustrates invalid_folder with aria-invalid + linked error and blocked preparation until the invalid input is corrected. Connection/provider errors must appear at the connection summary with Retry preparation; do not clear settings, draft or queue. Pending validation disables repeat submission; error is not communicated only by color.
5. `env-change` shows a bound conversation's folder read-only and a separate new-session action. Actual backend rejects workspace_thread_mismatch; this is not an inline folder relocation. Save/model/reviewer changes require new preparation and a fresh revision before execution. Do not assume selected model overrides a provider-returned model until preparation succeeds.
6. App common Codex-home change remains in AI setup with its current existing-path validation and restart requirement. Do not copy API secrets into session setup or treat provider endpoint credentials as Codex authentication.
7. During active work, show execution environment summary/read-only and preserve active Run settings. Completed Task settings stay read-only. Cancelling the editor returns to the preserved composer; no implicit save/start/queue dispatch.

Environment layout criteria: summary padding 20px (16px narrow), radius 13px, 16px top/24px bottom margin; requested settings and connection info use 1.25fr/1fr columns with 28px gap. At <=760px use one column/20px gap; connection info must be display:block even though session rail uses flex. Folder input min-width:0 plus 8px Browse gap, model/input minimum 40px high. Fields have 20px separation, helper text 12px. Error inset 12px 16px and 16px vertical margin. Footer top border +16px inset/24px top gap; Save is secondary, Prepare primary. Advanced approval fields, recent folders, connection help and scope explanation remain collapsed initially. No giant permanent form on normal chat: use compact summary disclosure as entry point.

## Extension: ordered follow-up queue (user-requested proposal)

This section supersedes the initial running-state audit's “no Queue offered” design choice. The **current product still has no queue/steer API** (`TaskWorkSessions.tsx` explicitly says unavailable); the user now requested a queue, so these mockups depict **new backend work requiring approval**. Do not describe it as implemented. Current `TaskExecutionStatus.queued` is a Run's dispatch state, not a persisted list of future instructions.

`running`/`queue` now show enabled additional-request composer, numbered pending list, edit/delete and explicit up/down reorder. Pink action **대기 요청에 추가** enqueues only; it never steers or interrupts the active turn. **메모로 저장** remains separate and never enters queue. Ordinary **중지 요청** remains available independently of the explicit **현재 작업 중단 후 대기 요청 실행** action. Queue is shown between compact current activity and composer, not as a separate screen or permanent settings form.

Proposed lifecycle and safety contract (backend changes needed):

- Scope: one ordered queue per exact Task/session/bound Codex conversation. Store immutable request revision, attachment reference, selected effective-environment revision, operation ID, ordering and dispatch link. Do not merge across sessions. Enqueue success clears only the submitted draft/attachment; failure retains both. Empty instruction cannot enqueue. A later draft is never cleared by an earlier async response.
- Pending → dispatching → linked Run → terminal. Create a real immutable Run/Work Log only when dispatching the head, not when merely typing or enqueueing. Work Log still records request → Codex action → result for that exact Run. Pending list is not execution evidence.
- Normal successful terminal Run triggers the next pending request sequentially without another user click because enqueue explicitly authorizes follow-up dispatch. Use authoritative native lifecycle, atomic dequeue/claim and idempotent operation identity. Never dispatch from component mount, a local timer, chat rendering, or an unconfirmed response.
- Exactly one active execution; verify task eligibility, session ownership, settings revision, bound thread and queue-generation token at dispatch. Repeated notifications, reconnect or reopening Task cannot dispatch twice. App UI close must not cancel active Run or queue. Durable queue recovery is proposed backend behavior; after app restart first reconcile authoritative active-run evidence before continuation, never replay uncertain work automatically.
- **현재 작업 중단 후 대기 요청 실행** records an explicit stop-and-drain intent and interrupts the exact active Run once. `queue-stop` shows waiting status, disables edits/reorder/delete for the claimed list, and offers **대기 실행 취소**. Only confirmed provider terminal status releases the next item. Interrupt ACK, timeout, disconnect, stale event or stopRequested flag alone must never release it. Uncertain stop retains list + intent and requires authoritative reconciliation; current backend offers no force-clear, so do not invent one.
- If active Run succeeds before interrupt arrives, atomically consume the same stop-and-drain intent once. Do not issue a stale interrupt to the next Run. Cancelling the drain intent keeps pending requests and pauses automatic continuation; it cannot retract an interrupt already delivered or undo side effects. Show this limitation before cancellation where relevant.
- Explicit Stop alone pauses queue auto-dispatch; it does not silently drain. User can separately resume pending dispatch after terminal confirmation. Queue pause prevents the next request from starting, never cancels current work. Delete cancels a still-pending item only. Once claimed, it is a Run and requires explicit Stop; no disappearing execution evidence.
- Approval/input waiting keeps that Run active and blocks the queue. Show live provider request above the queue; actual choice/answer submission is separate from the additional-request composer. Nonblocking requests may coexist with continuing active work; do not dispatch another Run merely because it is nonblocking. Secret/stale/error semantics remain unchanged.
- Failed/cancelled/interrupted/needs_attention pauses queue, leaves evidence and pending requests, and asks review/retry/resume. The only intentional cancellation that can continue automatically is the confirmed cancellation covered by the explicit stop-and-drain intent. No automatic retries or inferred success. `queue-pause` is the representative review state.
- Edit/reorder/delete only unclaimed requests, with optimistic revision/conflict handling in future implementation; accessible up/down buttons supplement any drag interaction. Editing uses inline editor and Save/Cancel without overwriting active composer. Removing the last request removes drain action and shows a compact empty queue hint. App submission failure keeps pending order stable.
- Enqueue uses the session environment snapshot; requested per-run overrides are currently unsupported and remain absent. Show project/model summary per pending item; if environment changes, mark pending requests “환경 재확인 필요”, pause dispatch and require explicit revalidation/reapply. Do not silently execute old requests in a different folder/model or copy auto-review consent across new sessions. Existing current opt-in/security policy/sandbox boundaries remain authoritative.
- Completed Tasks cannot enqueue, dispatch, edit execution settings or retry Runs. The completion guard must additionally reject nonempty/claimed queue or require explicit cancellation of pending requests first. Held Tasks cannot dispatch/enqueue work until resumed; pending queue needs explicit preservation/pause in proposed backend. No queue action completes/reopens Task, publishes Knowledge or resolves a Problem.

Queue layout acceptance: 13px-radius panel, 16px padding (12px narrow), 20px top gap; header 16px, count/status 12px. Ordered rows use 24px ordinal + flexible text + controls with 8px gaps and 12px vertical padding. At <=760px controls move beneath request text; numbering and request body stay aligned. Buttons minimum 36px height. Interrupt-and-drain is secondary, clearly separate from pink enqueue. Composer 16px gap/16px padding, textarea >=56px. Long Korean interrupt action wraps rather than clipping. Empty queue must not show irrelevant edit/reorder/delete actions. Preparation and queue state cannot compete as simultaneous primary actions.

## Extension validation

Actual Dia pixels reviewed for wide environment (top/footer), narrow 560px invalid-folder and stacked connection/footer, wide queue + active composer, narrow queue + active composer, stop-confirmation and bound-folder change. The first narrow environment pass revealed inherited flex on the connection aside; fixed with explicit narrow display:block, reloaded and replaced the affected screenshot. Prior `01-running.png` is superseded by queue captures after the user's later request. New screenshots remain synthetic-content, real-rendered design specimens; buttons never change settings, queue work or execute Codex. Queue editing/reordering/dispatch, backend reconciliation, error recovery and actual credential/model availability were not exercised.

JS syntax/whitespace checks only; E2E/release build skipped because this extension edits design artifacts only. Documentation guide reread; current feature guides deliberately remain unchanged. No production implementation, actual settings mutation, commit, merge or push.

## Reasoning effort addition (design only)

Installed Codex protocol generated locally into ignored `.tmp/session-reasoning-protocol` documents `Model.supportedReasoningEfforts`, `defaultReasoningEffort` and `TurnStartParams.effort` (applies to this and subsequent turns). Current app session/effective config/Run types and runtime turn/start have **no effort field or capability retrieval**; implementation requires additive storage/types/native forwarding and capability validation, not just a dropdown.

Mockup options are a dated fixture from local `~/.codex/models_cache.json`, containing model catalog metadata only; no credentials/config secrets were read. Sol supports low/medium/high/xhigh/max/ultra in that snapshot; Luna supports low/medium/high/xhigh/max, default medium. A cache is not proof of live account availability or effective inherited effort. Production must use selected-model provider capabilities (not the global ReasoningEffort string type) and refresh/error states. Unknown model shows inherit only; never guess identical levels. Dynamic fixture model selection/effort filtering works inside mockup only; other buttons remain simulated.

Model and reasoning controls appear side-by-side (1fr/1fr with16px gap) wide, stacked with16px gap at <=760px. Every model has explicit **기본값 상속 · Codex 설정 사용** option. Catalog default and effective inherited value remain distinct. Model change retaining supported choice preserves it; unsupported old effort switches to inherit with visible warning naming removed choice/new model and requires save/reprepare confirmation. `env-effort-change` illustrates Sol ultra → Luna inherit. Do not silently downgrade and dispatch. Queue item metadata now shows model plus recorded explicit/resolved reasoning: item1 high explicit, item2 inherit resolved low (prepared-result example, not measured effective setting).

Decision: capture settings at enqueue, not dispatch inheritance. For inherit, prepare must resolve effective effort before a queue item is runnable; if it cannot, item remains pending_validation. Persist requestedMode + resolvedEffort + capability/settings provenance. Dispatch exact resolved value per item, because provider turn effort may persist to later turns; omitting it could accidentally inherit the previous Run. Changing session defaults never edits existing items; unsupported/revoked settings pause for explicit revalidation. New reasoning contract is proposed; no actual settings were changed.

See [compatibility matrix and release gate](COMPATIBILITY.md) for existing feature protection, specific contracts, test evidence, missing regressions and integration collisions. This extension does not establish any current feature regression; no implementation exists yet.
