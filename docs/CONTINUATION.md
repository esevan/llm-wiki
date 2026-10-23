# LLM Wiki — Continuation handoff

## Simple publication, chat sessions and images (2026-10-21)

Current task: .worktrees/simple-knowledge-publication, base origin/main 0589850.
User approved main merge/push after verification. Normal publication is one explicit
button through existing journal/index storage, with semantic findings advisory.
User statements are primary sources; image bytes reach draft/revision multimodal
requests. AI origin and unverified authority remain explicit in search. Chat keyboard,
transcript scroll and same-app close/reopen are repaired. No Agent abstraction,
original DB read, real-data publication or app installation is included.

See [verification and limitations](testing/simple-knowledge-publication.md). Compare
the exact pushed commit's terminal CI with baseline failures. Existing v9 migration
fixtures expect schema24 while base already has25; no unrelated cleanup is included.

## Knowledge diagnostics and revision chat (2026-10-21)

Implementation lives in `.worktrees/knowledge-publication-contract`, branch
`fix/knowledge-publication-contract`. The prior contract correction was rebased
onto the supplied `origin/main` reference `1a57151` and is now `44c7cd9`.
The chat feature is a separate following commit. The fluid modal widths from
main are retained. No TaskDetail/WorkbenchView/RefinementPanel implementation
or their modal CSS was changed; their parallel owner confirmed no migration
conflict. KnowledgeReviewPanel's upper props/archive callbacks are unchanged.

The left article stays canonical until explicit proposal apply. The right chat
projects sentence/excerpt/reason/direction diagnostics, uses the selected
private or published version plus exact current sources, and proposes a full
GenerationResult rather than a body-only edit. Private review notes are not
chat and are excluded from model inputs. Changed sentences appear first;
complete body and binding comparisons are available before apply/cancel.
Published versions are labelled and selectable as correction bases.

Schema 25 adds one durable turn/proposal table, including base/hash, expected
private head, input evidence snapshot, prompt version, messages, disposition
and applied revision. It reuses the prompt registry/strict schema/provider,
JobRegistry cancellation, existing Knowledge revisions and archive publication.
There is one model call per turn and no added semantic-verifier call. Prompt
`knowledge_revision_chat` is v1. Conversation history is Task-scoped; provider
context includes 16 recent message pairs and one pending proposal, avoiding
repeated full candidates while retaining the complete current evidence.

Only a server-stored prepared valid proposal can atomically append a private
revision; apply rechecks source freshness and the private head. Full body,
claim/assumption/applicability/source bindings and quality findings change
as one revision. Original versions and published pointers/files remain intact.
Bad proposals stay repairable but cannot apply. Cancelling discards late replies
and cancels the provider token through the existing registry. Interrupted turns
remain visibly cancellable. Existing same-Task idea identities are preserved.
A published-file hash guard now also blocks external edits made before archive
preparation; existing write-time guards catch changes after preparation.

Verification and screenshots are detailed in
[the revision-chat test report](testing/knowledge-revision-chat.md). Knowledge
native verification passes 55 unit and 4 integration tests; foundation passes
26 tests; React passes 449 tests. Typecheck, lint, build and whitespace pass. No original
user database was read, no real-data file was published, and no installation,
merge or push was performed. Synthetic provider replies prove flow and integrity
boundaries, not measured real-model quality or arbitrary semantic entailment.

Remaining environment blocker: native app inventory/window capture on the Mac
failed because the Mac is locked and automatic unlock failed. Native window
screenshots and packaged WebView E2E remain unavailable until the host is
unlocked. Actual React App fixture screenshots were captured in the in-app
browser instead; they must not be described as real-user or packaged Tauri
screenshots. No release build was run solely for unavailable E2E.

## Knowledge generation contract investigation (2026-10-21)

Worktree: `.worktrees/knowledge-publication-contract`; branch:
`fix/knowledge-publication-contract`. The connected checkout's `main` was
`4dfb363`, an ancestor of the supplied `e59de51`. The dedicated branch uses
`e59de51` so the existing repair/UI work is retained. No other worktree,
real application database, installation, or publication was modified.

The screenshot error establishes a blocked saved candidate, not its precise
cause. Exact quote validation searches individual decoded JSON strings in
the identified manifest entry. It does not join fields, search another source,
or accept JSON serialization as a quote. Save reuses the frozen snapshot and
revalidates the result; append also rebuilds current evidence for freshness.
Editing revalidates the same evidence and restoration preserves quality;
neither action removes quarantined-citation blockers. Archive prepare checks
valid quality/current private revision/hash/freshness; approval rechecks source
and file guards, and index receipts precede the published pointer update.
No confirmed fresh-save/reload validation mismatch was found.

Generation now receives a provider-only citationCatalog pairing decoded strings
with exact identity; the frozen snapshot/hash remains unchanged. Prompt v5
explains that reuse prose is an interpretation, not a quotation or new verified
fact. The overstrict source-quote substring gate now also accepts a nonverbatim
applicability phrase when it is in the body, visibly labelled Reuse interpretation
(or 활용 해석), and bound by an existing assumptionBinding with an applicability:
ID, exact phrase bodyLocator and supporting exact citations shared by applicability.
Existing literal versions still validate unchanged. Unlabelled or unbound new
phrases remain blocked and can take the bounded same-snapshot repair path.
No extra LLM call, schema migration, or factual authority is added. All existing
claim, citation, material-omission, quality-finding and publication guards remain.
The review UI identifies these interpretations as suggestions requiring review.
Context size grows with the catalog; real-provider latency is unmeasured.

Regression coverage includes decoded Korean/quotes/newlines/JSON-pointer
escaping, wrong-source and joined-field rejection, unchanged snapshot identity,
provider repair success/rejection/outage and applicability-only repair. A new
temporary-vault native test saves a rejected version, verifies prepare fails,
appends a valid regenerated version, explicitly approves files, finishes index,
and checks the published pointer while retaining the rejected history. Existing
native tests cover stale evidence, edits/restores, authority and actual-use
metadata. Existing mounted App tests cover blocked publication, regeneration,
file preview and approval through the production adapter with mocked native
responses; they are not a live WebView-to-native E2E run.

Verification: `cargo test --manifest-path src-tauri/Cargo.toml knowledge`
passes all selected native unit and command/search/service integration tests,
including labelled reuse interpretation and rejection→regeneration→publish/index.
One existing synthetic timing test is ignored. `npm test` passes all 441 React
tests and helper/provider/macOS/runtime/boundary checks. `npm run typecheck`,
`npm run lint` and `git diff --check` pass. Initial concurrent UI budget failures
passed the complete rerun; the native fixture .tmp directory was prepared.
Packaged desktop E2E is skipped: exact-source provider fixtures, mounted App
adapter publication flows, the new interpretation review test and native real
SQLite/temp-vault publication/index integration cover the changed boundaries.
Real WebView plus a real provider and the original failing data remain unverified.
No release build, install, merge or push was performed.

Original-case diagnosis still requires only: the affected Task ID, selected
Knowledge revision, that version's `quality_state`, `model_error`, relevant
`result_json` bindings/findings, and its referenced evidence snapshot entry
(type/id/revision/locator/content plus generation snapshot hash). For a freshness
error also provide current Task revision/completion ID and source hash. These
can be redacted excerpts; no full user database is needed. Do not claim that
the connected machine contains the user's failing data or that this synthetic
repair proves all real model outputs will publish.

### Spec review: applicability is not a direct quotation

FR-004 requires use situations, questions, actual help and scope bounded by the
body. The data model calls summary a concise use situation supported by the body,
and the generation contract rule 6 forbids promises beyond supported body and
bindings. None requires summary/helpsWith/conditions/exclusions to be substrings
of source quotes. The existing validator's normalized source-quote substring
check is stronger than this specification; it conflates semantic scope support
with direct-quote fidelity. This can reject a supported paraphrase. The earlier extractive generation workaround has been removed. The current
implementation distinguishes source-linked interpretations from factual claims.
Do not claim the original user case was reproduced.

Implemented minimal correction reuses existing assumptionBindings rather than
adding a semantic-verifier service: exact citation checks still authenticate the
source; labelled nonverbatim applicability remains an explicit source-linked
interpretation. Factual statements about effects, observations, decisions or
scope still need claimBindings and retain authoritative statement/state checks.
Existing blocking qualityFindings remain blockers. This distinguishes reuse
explanation from factual authority without pretending substring matching proves
meaning. The UI and portable article explicitly identify reuse interpretations.

Regression fixtures separately cover: supported nonverbatim interpretation;
missing label/binding or fabricated source; contradictory factual claim;
unobserved effectiveness; unsupported factual scope expansion; blocking
unsupported-addition/overreach/omission findings; and rejection→regeneration→
publication/index with an interpreted applicability summary. Negative factual
fixtures exercise claim guards, and quality fixtures supply explicit findings.
They are not an automatic detector of arbitrary contradictory natural-language
interpretations. A model could misclassify an unsupported assertion as a labelled
assumption; source/assumption visibility and explicit human review remain necessary.
No existing independent semantic evaluator was found. No extra model calls or
keyword heuristics were introduced, and qualityState=valid is publication
eligibility after structural/evidence checks, not proof of every semantic claim.

Prompt version is explicitly 5, with regression coverage for registry/build
version and decoded catalog delivery; other prompt versions remain unchanged.
Existing mock repair tests select their responses; they prove control-flow and
storage boundaries, not measured behavior of a real model under old versus new
contracts. No original-database read, install, merge or push was performed.

## Knowledge review integration verification (2026-10-21)

Integrate only the approved Compass publication callbacks, Workbench wiring
fixtures, compact generation instructions, and simplified review UI on
`4dfb363`. Preserve the existing strict data contracts, exact evidence,
version-bound private notes, explicit publication approval, and file guards.
The deferred dynamic output contract and uncommitted publication-review
worktree files are excluded. Do not infer that the reported Workbench
publication-disabled state has been reproduced or resolved.

The measurement helper uses Node.js to preserve the native-only source rule.
Its initial Python version was rejected by post-push CI and is replaced without
changing the measurement.

The integrated React/runtime suite passes 438 tests; typecheck, lint and
frontend build pass. Full native execution passes 300 tests with three
ignored and two time-budget failures in unchanged retrieval-index and
execution-routing modules; both failed tests pass isolated reruns. The
seven reference-search integration tests pass. Existing main CI already
fails Rust formatting and synthetic React performance budgets. Keep those
unrelated repairs outside this integration. Packaged E2E is not run locally:
mounted adapter flows and native temporary-vault tests cover this change.

## Workbench publication availability investigation (2026-10-20)

The user clarified that publication was disabled after completing a Task
in Workbench. The separate Compass callback fix in `b3ddfb4` fixes a real
Compass defect but does not establish the cause of that Workbench report.
Do not describe the reported Workbench issue as resolved by that commit.

At base `4dfb363`, App supplies all six archive callbacks to Workbench,
which forwards them to its local TaskDetail and KnowledgeReviewPanel.
Completion reloads that same detail; recent-completed opens it locally.
Neither action routes through Compass. There is one KnowledgeReviewPanel
render in TaskDetail and it forwards the callbacks.

The new App wiring fixtures exercise completion then review, and reopening
from Workbench recent-completed, through the real taskClient and Tauri
adapter. Current valid Knowledge can prepare, inspect exact files and
explicitly publish in both fixtures. Settings/Search and jsdom modal focus
repair are excluded; their behavior is outside these wiring tests.
The user's disabled state and original integration placeholder have not
been reproduced in the Workbench route. Actual runtime callback presence,
draft freshness/quality, dirty state and pending operation remain unknown.
Do not infer an old installed build or missing setup without evidence.
No real Knowledge or installed application was changed for this inquiry.

## Refinement modal layer follow-up (2026-10-19)

Local branch `codex/refinement-modal-layer` is based on published main
`c2e62d3`. Do not merge, push, or replace the installed application without
new authorization. Preserve the separate CPU worktree and existing edits.

A user photograph identifies the legacy Problem “Explore this problem”
refinement below the entire Workbench and sidebar. Its dialog is outside
`.app`, so the old `.app:has(...)` grid cannot contain it; `show()` and sticky
positioning put it into document flow. It now uses `showModal()` and bounded
fixed positioning while retaining its context controls and ownership marker.
Repeated opening does not call `showModal()` again. Modern Task/Capture
refinements retain body portals. Obsolete inline-column rules, including the
narrow-screen background hiding rule, are removed; focus uses preventScroll.

The selected `task-refinement-modal-layout` desktop scenario covers actual
WebKit geometry, scrolled Capture/Task reopen with retained drafts, nested
Task detail, and legacy native modal placement at 1200 and 900 pixels. It
uses a temporary home and local fake provider. Initial diagnostics showed natural document-end scroll clamping at
1200 pixels. At 900 pixels the lock captured a different position before
closing; removing the test helper's second scroll did not resolve it, leading
to identification of the narrow-screen background hiding rule. Final verification
results are recorded in the task handoff. The Mac screen is locked, so numeric
DOM geometry is not a completed post-fix pixel review. No live data changed.

## Main integration authorization (2026-10-19)

The user authorized merging and pushing this retrieval/draft-save feature.
The implementation commits `90904c9` and `6380013` are combined for publication
under the repository's single-feature-commit convention. A local backup branch
retains the original two commits. This supersedes earlier local-only notes for
this feature; the CPU optimization worktree and installed app are excluded.
Application and test source remain unchanged during integration. Final checks
and CI for the published main SHA are reported in the integration handoff.

## Knowledge save recovery and timeout verification (2026-10-19)

Follow-up work remains on `codex/retrieval-draft-save`, after local commit
`90904c9`, without merge, push, installation or external publication.

A failing regression reproduced Knowledge correction committing successfully
followed by a projection-read failure: the old body/revision/hash reappeared.
The UI now retains acknowledged body/revision/hash separately and hides all
old-version mutation controls until a canonical read confirms Task identity,
revision, content hash, body and a non-regressed head. Recovery repeats reads,
not writes. Source/quality metadata is never synthesized. Repeated read failures
and mismatched canonical hashes remain blocked. Recovery then allows another
edit using the new revision/hash. Late acknowledgements and late post-save
reads/status updates cannot overwrite a different Task after navigation.

Verification: 400 React/unit tests and npm helper checks, typecheck, lint,
whitespace checks and the final no-bundle release build passed. Independent
review found and closed both Task-identity races; final review found no further
confirmed defect. No Rust source changed in this follow-up; the preceding
366-pass/3-ignored Rust result remains the relevant baseline.

Two focused real-app scenarios passed in three consecutive independent homes:
`task-refinement-title-apply` and `task-publication`. The latter injects a read
failure after the real correction IPC has returned, reads the temporary DB
through the original IPC adapter, checks exact body/snapshot/claim bindings,
then recovers the canonical hash without another correction. Across the three
refinement runs, 12 edit requests and 6 Apply requests returned HTTP 200.
Results are under `.tmp/desktop-e2e-artifacts-{JcXfHQ,yPFPXb,sZcok4}/results.json`.
These use a local fake provider and temporary Vault; live models and installed
or signed packages are outside this coverage.

Timeout diagnosis is separate from storage correctness. Instrumented failure
`rlRRSb` observed a hidden WebView, successful edit/refresh IPC, then roughly
23 seconds without JavaScript observations. The old condition-wait helpers
checked their deadline before checking the condition after resume. Two unit
regressions reproduced false timeouts for already-satisfied conditions; both
now pass, while two still-false cases continue to time out. Strict latency
assertions are unchanged. Final passing traces still contain 22-27 second
observation gaps, confirming the new condition check handles suspension rather
than proving foreground performance. The two original Q7Q61z/8CLQla failures
lack enough telemetry to assign their individual causes conclusively; a later
pass alone is not evidence that they were resolved. No currently reproducible
save blocker remains in the tested flows, and no office/user logs are needed
to reproduce these fixes. The original model response would only help attribute
the old retrieval incident, not unblock this local work. Existing 31-file fmt
failures remain a separate baseline.

## Retrieval contract and draft-save correction (2026-10-19)

Local branch `codex/retrieval-draft-save` in
`.worktrees/retrieval-draft-save` is based on main `16698ac` and is not merged
or pushed. The reported model response was not supplied, so the exact incident
cannot be attributed conclusively. A regression test reproduced schema-valid
no-search queries rejected by the runtime. Search/no-search schema branches now
match runtime intent constraints; query limits count Unicode characters.
Acknowledged refinement edits advance the local version/hash even when the
following read fails. Legacy Knowledge without an evidence snapshot explains
its read-only state before entering an unsaveable editor.

Validation: npm test (393 React tests plus helper checks), typecheck, lint,
366 Rust tests (3 ignored), whitespace check and no-bundle release build passed.
Independent read-only review found no additional confirmed defect. The focused
`task-refinement-title-apply` app scenario passed for Capture and Task with
consecutive title-only and body-only saves using an isolated DB and fake server.
Final artifact: `.tmp/desktop-e2e-artifacts-NJy849/results.json`.

Two earlier focused runs timed out: one before the Task chat was recorded, and
one awaiting navigation after Capture edits and Apply had committed. Initial
preview readiness and failure DOM diagnostics were added; the final run passed,
but the intermittent timeout cause is unconfirmed. Keep this limitation visible.
Whole-repository fmt still fails in the same 31 files as the recorded baseline.
No installed app, live provider, user database, signing identity or remote was
changed. Legacy Knowledge editing without a snapshot remains unsupported; no
source provenance is synthesized. The follow-up above covers Knowledge post-save read failure with fault injection.

## Shared AI timeout and settings singleton (2026-10-13)

Local work is in `.worktrees/generation-timeout` on `codex/generation-timeout`.
AI setup now saves a shared 1–3600-second timeout (default 180). All provider
HTTP requests, including streams and connection tests, use one client factory.
The settings singleton caches isolated homes and publishes changes only after
atomic persistence. Manual JSON edits require restarting the process.

`npm test`, typecheck, lint and whitespace checks passed. Rustup installed
Cargo/Rust 1.99.0; all 12 native settings tests passed, including concurrency,
failed writes and whole-response timeout behavior. Wide English and narrow
Korean previews passed visual, focus and overflow review.

The release executable built successfully. The selected native in-app scenarios
`task-refinement` and `task-publication` passed in isolated homes using the local
fake provider. They cover preview creation/edit/adoption and completion Knowledge
generation/review. No full E2E was needed. These tests do not measure live-model
latency or validate signed packaging; no real provider is configured here.

The implementation was fast-forwarded into main as `8cf6013`. The user then
authorized creating a new local signing identity and requested no old-app backup.
The new `LLM Wiki Local Signing` fingerprint is
`797FF6B7B6C50CCDBFDE1C5B7D80DCFEA6A31290`, registered in the local signing
configuration; its private key remains in the login keychain. Temporary PEM and
PKCS12 exports were deleted. User trust is scoped to the code-signing policy.

The signed release bundle passed strict/deep signature verification and the same
two selected in-app scenarios. The guarded installer replaced
`/Applications/LLM Wiki.app` with an explicitly accepted designated-requirement
change. The new app launched normally and its AI settings show Timeout 180.
The replacement backup was deleted after launch verification as requested.
App data, provider settings and Vault contents were preserved. The installed
CDHash is `1409075b94318a17a07ffe64f9bb4e7a5efa3fb8`. The user authorized pushing the merged changes after installation.


## Authorized push and local installation (2026-09-26)

The user authorized pushing the integrated workflow and installing the tested
application. The 82 unpublished integration commits were retained on local
`backup/knowledge-workflow-before-push-20260926` and consolidated for one normal
fast-forward push over `0a7da6a`; remote history rewriting is unnecessary.

The signed bundle built from `db4ada7` was installed at `/Applications/LLM Wiki.app`
and launched normally (PID 99227 at verification). Later changes only update docs.
Its CDHash is `e5cb2b5d549b9277f5f9cf1d02d2917bf80d3162`; strict signature verification
and the existing designated requirement passed. The guarded installer retained
`/Applications/LLM Wiki.app.previous-1790443863821-99146` and did not reset app data,
Keychain or TCC permissions. Previous selected Capture/refinement/publication
checks apply to the installed code; no redundant full E2E or rebuild was needed.
The earlier visual/extended-coverage limitations remain; installing does not
complete those checks. Earlier “no install/push” entries below are historical.

## Active knowledge workflow program (2026-09-26)

Work packages 1–15 and 17 are implemented and merged into local main through
`db4ada7`; work 16 focused automated checks passed, with the recorded visual and extended
coverage limitations still open. Report progress by number using
[the program ledger](../specs/knowledge-workflow-program.md). The installed app,
remote branches and personal Obsidian Vault remain unchanged.

- Specs 015–021 cover the workflow; schema head is 22. Final fixes live in
  `.worktrees/knowledge-workflow-final-checks` before main integration. Cargo uses
  shared cache; serialize it with release builds.
- Frontend 375 tests passed; later changes passed focused suites, lint/typecheck.
  Sequential native units passed 252 with one authenticated live test ignored.
  All 15 integration binaries passed across the targeted final runs; historical
  v8 fixture cleanup and schema assertions were repaired without weakening actual
  migrations. Latest native provenance 17, archive 16 and Knowledge 14 passed.
- Capture-adoption references retain exact actual-use/counterevidence authority
  through Knowledge/archive; unrelated/candidate/viewed/excluded sources cannot
  become published authority. Archive overlap now drops full viewer bodies and
  caps ranked passages at eight and 6,000 estimated tokens.
- Packaged `task-capture` passed (artifacts `desktop-e2e-artifacts-2ZohfQ`).
  `task-publication` passed after current-UI harness corrections (artifacts
  `desktop-e2e-artifacts-qZOCVD`): immutable correction, reviewed exact artifact
  hash/index receipt, explicit withdrawal and retained private history.
- `task-refinement` passed on the final signed `db4ada7` build (artifacts
  `desktop-e2e-artifacts-i2GYOb`): note/image restore, completed chat, overflow
  scroll, editable preview, immutable save and exact adoption into the original
  Task ID. All three selected scenarios now pass. Harness corrections retain
  every behavioral assertion. One initial 180s timeout was not reproduced and
  must not be described as a diagnosed app deadlock.
- Knowledge Review timing: 25 independent 40-revision cycles, p95 projection
  18.67 ms/selection 17.31 ms. Reference list 1,000-document timing: p95 mount
  6.84 ms/filter-sort 12.59 ms. These are mounted jsdom results, not native visual
  checks. Missing-report factual fallback already has executable coverage; the
  all-four-idea-disposition regression was added to the actual validator.
- Latest rendered wide/narrow EN/KO list/artifact/transient state matrix remains
  blocked by locked Mac; user was asked to unlock while automated work continued.
  Full generation/archive-stage timing and expanded retry/idea-publication
  permutations remain unverified; no blanket Spec Kit convergence is claimed.
- Only the three named packaged scenarios are justified by native background
  application, modal/adoption and file/index risks. No full E2E, install or remote
  push. Older entries below are historical and do not supersede this program.

**Updated:** 2026-09-22
**Current status:** The React/Tauri/Rust desktop now uses the Task as the canonical unit of work.
Capture remains a lightweight input; Work Log, refinement, exact Problem revisions, Task
relationships, advisory review, completion, and Knowledge publication are independent decisions.
The Python/FastAPI browser delivery remains retired in Git history at `caef236`.

## Task Codex execution follow-up (2026-09-22)

- Implementation is complete on `feature/task-session-execution`, including the inherited session MVP work. The feature commit is prepared for integration; verify the current local and remote Git state directly when resuming integration or release work. The user's installed application was not replaced.
- Spec Kit [feature 014](../specs/014-task-codex-execution/spec.md) extends existing Task sessions with persistent Codex threads, explicit preparation/execution, durable turns and canonical Work Log projections, provider-derived approval/question buttons, and recovery without automatic resend or Task completion. Existing Task/session/attachment storage and Work Log paths are reused.
- Astra High completed the **UI/UX design review**, as the user clarified. Actual rendered verification is separate. E2E ran with Luna; concrete E2E failures were diagnosed with Astra.
- Final checks: frontend 294 passed; native 186 passed, zero failed, one opt-in live test ignored in the normal suite; typecheck and whitespace checks passed. The unchanged 100 ms projection gate passed at p95 11.259417 ms. The separately run actual authenticated Codex test passed two same-thread turns with a first-turn-only nonce recalled in the second and two durable Work Log projections.
- After rebasing onto main's released v13 Task journey cache, the combined tree passed 303 frontend tests and 196 native tests with the same one opt-in live test ignored. The merged migration chain retains v13 for journey graphs, adds sessions at v14 and execution at v15; populated main-v13 journey/manual data, populated v14 session data, rollback, retry, and foreign-key checks passed. Typecheck and whitespace validation also passed.
- The final signed release and selected `task-codex-execution` packaged E2E passed. Evidence: `.tmp/task-codex-execution-artifacts-5/` inside the execution worktree. No full suite was run. Three initial harness failures and the actual restart-stale-request defect were diagnosed and corrected before the final pass; [quickstart](../specs/014-task-codex-execution/quickstart.md) distinguishes each evidence type.
- Actual CUA inspection covered EN/KO session UI, keyboard note save/reopen, 900 px native width, completed Run/answer restoration, report/evidence attribution, compact Work Log display, exact reverse-link heading focus, and restart recovery with stale questions read-only. All temporary review processes were stopped; the user's installed app was preserved.
- Residual limits: 640 px is unavailable under the existing native minimum; Windows and real-provider approval/interruption/auth-failure cases were not exercised; not every transient state was visually inspected. The Work-panel native accessibility tree was intermittently absent after a deep link but present on direct tab access; full assistive-technology validation is not claimed. Attachments are saved as notes and are not sent to Codex in this increment.
- No further feature implementation is queued. Resume only explicitly requested integration/install work or a recorded validation gap; do not restart broad inventory or the full E2E suite. Earlier handoff entries below are historical.

## Current handoff

- The Task work-session MVP is implemented in `.worktrees/task-session-mvp`. Sessions are explicitly created and Task-bound, persist chat-style records/settings/attachments across restart, preserve failed input for retry, and do not mutate Task state or run AI. The selected packaged `task-work-session-restart` scenario passed after the final release build; npm passed 268 tests and the full Cargo suite passed.
- An isolated normal macOS review bundle showed the Sessions UI in English and Korean, restored saved settings and an image, saved a long mixed Korean/English note using keyboard navigation, and restored it after a real restart. Native file selection reached a real 400×160 PNG, but UI automation lost AppKit access before picker completion could be observed; process samples showed the app and WebContent remained healthy. Narrow, explicit error-state, and Windows rendering remain unverified and should be treated as validation follow-up rather than a confirmed code defect.

- The Task-centered redesign is complete in single commit `de01ff47398871902a765d43b5a4060161316f92`; `main` and `origin/main` match, and the existing seven unpublished commits were pushed with it.
- The signed application was installed at `/Applications/LLM Wiki.app` and launched successfully (PID `28832` at handoff). The verified release identity was CDHash `3be85b685e04f76596c914fafa4bf9639081958d`, signature `LLM Wiki Local Signing`, timestamp `2026-09-12 08:58:37`, with the designated requirement matching strict verification. Previous app backup: `/Applications/LLM Wiki.app.previous-1789222006550-28769`.
- Packaged macOS E2E completed 32/32 scenarios with 169 rendered/exercised/asserted controls and no evidence gaps. Artifacts are retained in `.worktrees/task-centered-workbench/.tmp/desktop-e2e-artifacts-FcAbm6`; the documented EN/KO runbook supports `--list`, `--scenario`, `--full`, `--artifact-dir`, and `--keep-state`.
- Scoped npm (149), Cargo (106), lint, typecheck, format, and clippy checks passed. Full-repository lint still reports 21 pre-existing findings. Real-provider quality/latency, real Vault data, and installed Windows/Linux or OS reduced-motion coverage were not tested.
- Installer data, keychain, and TCC state were not reset; normal launch-time migration remains available. No new code, test, build, commit, push, or launch work remains.
- Deferred follow-up is recorded in [Task-centered architecture backlog](backlog/task-centered-architecture.md). The LLM Wiki MCP tracking attempt returned `elicitation_required`; no new remote record was saved.
- Preserve current user-owned changes: `AGENTS.md`, `.agents/skills/commit-convention/SKILL.md`, `docs/plans/task-centered-workbench.md`, and the untracked `docs/plans/ui-ux-improvements.md` / `docs/plans/ui-ux-improvements.ko.md`. The original task-centered plan was restored with SHA-256 `820d61b0fab5b1ce293aed6c7b81e0062a1765a8222da973887daa2207999d12`; its backup is `.tmp/pre-merge-user-plan-p1c5jb0a/task-centered-workbench.md`.
- There is no remaining implementation objective. Resume only an explicitly requested backlog or evidence gap; do not automatically restart the redesign or full E2E run.

- The lessons are maintained in [CONTRIBUTING.md](../CONTRIBUTING.md) and [CONTRIBUTING.ko.md](../CONTRIBUTING.ko.md), linked from both README contribution sections.

## Main verification and installation fixes (2026-09-16)

- Refinement restores its opener after the panel unmounts and releases its focus trap,
  including when a parent defers removal after saving. The interactive source registry
  includes the shared modal keyboard handler and the original-Capture disclosure.
  Packaged scenarios follow Task detail modal navigation, resumed-refinement controls,
  asynchronous close/status updates, and docked legacy-preview disclosure semantics.
  Docked Context toggles retain their own disclosure handler when the completed-work
  runtime is loaded. Legacy chat releases Send when the response finishes, even
  while presentation animation is pending, and flushes animation in hidden windows.
  Capture attachment coverage follows the visible chooser through
  its hidden native file input, image preview, and removal.
  Review cancellation uses a separately delayed fake response and exact buttons;
  polling deadlines use elapsed time so background WebKit timers cannot stretch them.
- Workbench collects Task-origin Capture IDs once instead of scanning all Tasks for
  each Capture. Null origins remain excluded from that set. Category responses move
  their JSON arrays without duplicate serialization. Native outbox checks run on
  completion and Task detail access, retaining startup/MCP retries while avoiding
  extra database connections on unrelated Capture and Workbench requests.
- Checkpoint appends enforce write authorization in the transaction before replay
  lookup and record successful activity in that same transaction. Rejected requests
  retain separate audit entries. The native adapter no longer drains publication
  after every checkpoint; accepted completion retains its service-owned drain.
  Native release verification uses `RUST_TEST_THREADS=1` so latency budgets are not
  measured concurrently with the independent 10,000-event stress fixture.
- The macOS package command detects the installed Intel ONNX Runtime, copies its
  transitive libraries into the bundle, rewrites their references, signs them through
  Tauri, and verifies that no non-system dependency remains outside the app. Local
  certificates use the documented app-scoped entitlement. This replaces the manual
  artifact repair described in the historical installation record below.
- Verification and reinstall work is isolated in `chore/main-verify-reinstall`, based
  on `main` at `38b12e2`. Packaged E2E evidence is retained under that worktree's `.tmp`.

## Latest local installation verification (2026-09-16)

- Reinstalled `/Applications/LLM Wiki.app` from `main` at `38b12e2` plus the
  verification fixes above. The guarded installer preserved the prior bundle at
  `/Applications/LLM Wiki.app.previous-1789580338205-70531`; it did not reset app
  data, provider settings, Keychain items, or TCC permissions.
- Validation passed: 245 frontend/runtime tests, 6 desktop-runner tests, 3 fake
  provider tests, 5 signing/runtime tests, runtime/native-boundary guards, type
  checking, lint, whitespace checks, and all 149 native tests. Native performance
  thresholds were unchanged and checked with sequential test execution.
- The final signed package passed all 33 desktop E2E scenarios and all 205
  interactive controls (rendered, exercised, and asserted), with no missing
  source evidence, unknown controls, or undocumented disabled states. Evidence:
  `.worktrees/main-verify-reinstall/.tmp/main-release-install-gate/`.
- Verified all 86 bundled runtime libraries have no external non-system
  dependency. The installed bundle retained the same local signing identity and
  designated requirement. Installed and tested executable SHA-256:
  `8999b02b4384ee8a2dead23f64dd6302e629fd0e503ec0a05300caf4cee0c8da`.
- Launched the installed app normally and confirmed a visible 1280×820 window.
  The launch/hash report is retained in the worktree at
  `.tmp/installed-launch-verification.json`.

## Work Log image merge (2026-09-16)

- Integrated `e3a684e` into the current main history, restoring saved Work Log
  images and queued bilingual image summaries alongside the latest Task UI.
- Preserved both the current layout styles and the image-summary styles when
  resolving the stylesheet conflict. Uncommitted changes in the original
  `fix/work-log-images` worktree remain there and are not part of this merge.
- At the user's request, no tests were run for this integration. No release
  build, packaged desktop E2E, or installation was performed; the installed
  application still needs an update to receive this fix.

## Modal Escape and focus follow-up (2026-09-16)

- Task detail and refinement now share document-level Escape handling and focus recovery.
  Refinement takes priority over Task detail; native dialogs retain their own handling.
  Background inert state is restored only after the last custom modal closes.
- Regression cases were added for escaped focus, nested modal priority, disabled inputs,
  repeated Escape, IME composition, native dialog ownership, and background restoration.
- At the user's request, tests, type checking, lint, release build, and packaged desktop
  E2E were not run for this change. Run the combined verification after the pending work
  is collected. Manually verify macOS fullscreen Escape, Korean IME cancellation,
  loading/error states, unsaved-change prompts, and saving failures.
- This change does not update the installed application described below.

## Historical local installation verification (2026-09-15)

- Installed the application built from `7eac07c` (Capture/refinement images) at
  `/Applications/LLM Wiki.app` on 2026-09-15. The guarded installer retained the
  previous bundle at `/Applications/LLM Wiki.app.previous-1789513091476-58810`.
- Strict/deep signature checks passed with the existing local signing identity and
  unchanged designated requirement. A normal Launch Services launch remained alive
  and created a 1280 × 821 window. The installed executable matched the tested
  candidate (SHA-256 `af987f8c3722fc21e20261886f42a306be875d218687c0a9c4e8996aa3f59d50`).
- A read-only check confirmed the existing database reached schema 11 and contains
  `input_images`. No screen interaction or feature E2E was performed in this run.
- **Packaging follow-up:** the default Intel macOS signed build still references an
  unsigned Homebrew ONNX runtime and aborts before database initialization. For this
  installation, 86 runtime libraries were copied into the candidate's Frameworks
  directory, their dependencies changed to bundle-relative paths, and all libraries
  and the app signed. Local signing has no Team ID, so the candidate also required
  `com.apple.security.cs.disable-library-validation`; system security settings were
  unchanged. These are artifact repairs, not changes to the build scripts. Automate
  dependency bundling and the local-signing policy before relying on a fresh build.
- Local diagnostic scripts and results remain under
  `.worktrees/capture-refinement-images/.tmp/` (`repair-package.py`,
  `package-smoke.py`, and `installed-launch-verification.json`).

## Runtime

- React owns the shell, primary screens, dialogs, navigation, and shared UI.
- Twelve bounded controllers in `frontend/public/runtime/` preserve complex presentation behavior
  through the centralized Tauri application client; none contains an HTTP fallback.
- Thin domain commands delegate to Rust workflow, jobs, completion, Lineage, localization, Vault,
  semantic, settings, and provider modules.
- SQLite stores workflow, indexes, and jobs. Application settings, including the API key, live in
  the atomic `~/.llm-workbench/settings.json`; the key is omitted from UI and MCP responses.
  Markdown Vault writes use source hashes and atomic replace.
- SQLite schema changes are ordered in `native/migrations.rs`, recorded with `PRAGMA user_version`,
  and committed atomically per version. Version-zero Python/native databases migrate on startup;
  newer unsupported versions fail closed.
- New installations require an explicit native folder choice; existing installations retain their
  former Documents Vault without a migration prompt.
- The multilingual embedding model and all required fonts are bundled; runtime downloads are not
  required.
- No internal port, Python process, sidecar, web server, or browser product remains.
- Local stdio MCP gives Codex and ChatGPT desktop a second Chat input over the same Workbench state.
  The stdio child is persistence-free and forwards over a protected Unix domain socket or Windows
  named pipe to the running GUI process, which is the sole owner of the application service,
  SQLite/Vault adapters, and projector. Connection scopes, immutable work events, a leased
  projector, current/topic/overview resources, Vault evidence tools, and separate Knowledge
  draft/publication gates are implemented. ChatGPT web and remote MCP remain intentionally outside
  the first release.

## Task-centered verification record

### Work Log images and bilingual summaries (2026-09-15)

Task detail renders saved and migrated image attachments inline. Image saves enqueue
Korean and English summaries automatically; existing images can request them manually.
Queue results open the owning entry, and summaries follow the interface language.
Both languages and completion commit atomically with source, cancellation, and retry
attempt guards. Automatic submission is idempotent for a saved entry.

Verification is recorded in the task handoff. This environment requires the existing
workspace ONNX library via `ORT_LIB_LOCATION` and `ORT_PREFER_DYNAMIC_LINK=1` for
native builds because upstream has no prebuilt `x86_64-apple-darwin` binary.

**UI/UX follow-up final package:** the signed `9b2b81082a43de0637bedd33a8cc670709ff2b21` bundle passed 32/32 scenarios. The generated inventory recorded 188 scanned source controls and 175 rendered/exercised/asserted controls; all six coverage-gap arrays were empty. Manual Korean IME, VoiceOver, OS reduced motion, Windows, zoom, and native quit/crash draft durability remain unverified or out of scope.

Nine final native 1198×768 Korean-light captures from isolated `Dlnhjg` state were visually reviewed, including the Workbench, Task detail, Refinement, Task review, completion/Knowledge, Queue recovery, Vault search, Compass, and AI setup. Task Escape returned focus to its original trigger. This does not verify real Korean IME, VoiceOver, or OS reduced motion.

- React/adapter/runtime: 34 Vitest files and 176 tests passed, with six desktop-helper, two fake-provider,
  and four signing/runtime/native-only/application-boundary checks. Typecheck and scoped changed-code
  ESLint pass. Full lint retains 23 pre-existing `no-explicit-any` findings in untouched completed- and remaining-runtime tests.
- Rust: `cargo test --manifest-path src-tauri/Cargo.toml` passed 106 tests.
- macOS: the signed release bundle built 2026-09-12 11:44:31 AM by `LLM Wiki Local Signing` has CDHash
  `9b2b81082a43de0637bedd33a8cc670709ff2b21`. It was built from dirty `fix/ui-ux-improvements` worktree
  state based on `de01ff47398871902a765d43b5a4060161316f92`; its full packaged E2E passed 32/32.
  Manual visual/accessibility claims remain limited as stated above.
- Windows/Linux/macOS lint, typecheck, Rust tests, and unbundled Tauri builds remain configured in
  CI. Windows MSI/NSIS packaging is automated by `scripts/package_windows.ps1`; installed Windows
  named-pipe and Unicode-path acceptance remains an external gate on this macOS host.
- Exact Task-centered evidence is retained in
  [acceptance-verification.md](../specs/012-task-centered-workbench/acceptance-verification.md).
- Current interactive coverage recorded 175 rendered/exercised/asserted controls across 188 scanned source controls; all six evidence-gap arrays were empty. OS reduced-motion review, real-provider quality/latency, and Windows/Linux installed testing remain unverified.

## MCP Workbench Bridge convergence

- Capture preview is non-persistent until exact acceptance. In-app cards and MCP Elicitation share
  the same application service for Problem, Task, checkpoints, conflict resolution, completion,
  private Knowledge draft, exact publication, and recoverable withdrawal decisions.
- The GUI is the sole SQLite/Vault/application-service owner. A scoped stdio child forwards over a
  user-only Unix socket or Windows named pipe and fails closed when the GUI, connection, or grant is
  unavailable. Session handles are not credentials.
- Workbench and Chat read and update the same session head. Refresh preserves in-progress input;
  non-overlapping stale saves retry once, while governed stale actions require a renewed preview.
- Conflict review uses lexical and semantic evidence in the current Chat AI. The server rejects a
  hidden conflict-model job, fabricated/stale citations, and out-of-scope evidence.
- Topic membership is explicit, whole-Workbench access is a separate scope, and stable overview
  paging never silently mixes revisions. The representative Skill asks one decision at a time and
  keeps completion separate from publication.
- Remaining external evidence: installed Windows packaged E2E and a new Codex task loading the
  freshly installed plugin. ChatGPT web and remote MCP remain intentionally outside Phase 1.

## Post-migration product backlog

These are product enhancements rather than migration work:

1. Add a visible three-way merge for non-overlapping external Markdown edits.
2. Ratify and automate multilingual search, startup, command-latency, and memory budgets.
3. Add richer Conflict Review progress/deduplication and Lineage inference-failure presentation.
4. Decide the durable Queue history TTL and accessibility acceptance scope.

## Deferred Task-centered architecture work

The behavior release intentionally defers the internal follow-up items recorded in
[Task-centered architecture backlog](backlog/task-centered-architecture.md): duplicated Task
mutation/service/refinement SQL, handwritten DTO and operation registries, legacy DOM retirement,
Task assistance responsibilities, and migration recovery scope. A plugin backlog capture was
attempted but requires connector elicitation; no remote record was saved.

## Future work rules

Run `npm test`, lint, typecheck, production build, Rust format/clippy/tests, Tauri build, and packaged
desktop E2E for native changes. Preserve the application-client boundary and keep command handlers
thin. Do not restore the browser server as a compatibility shim; its retirement record is
[here](migrations/python-browser-retirement.md).

## Refinement dialog follow-up

The focused Refinement dialog keeps the proposed result and the conversation visible together, and
preserves Task proposal `body` values as Task detail when accepted. The signed release build,
202 frontend tests, typecheck, lint, and whitespace checks passed. Rendered review covered Korean
at 1280×800 and 800×600, English at 600×650, and keyboard focus containment.
The final packaged E2E passed 29 of 33 scenarios, including refinement and modal geometry at
1200px/900px. `task-controls` (waiting for a Knowledge draft), `task-legacy-chat-controls`,
`task-mcp-continuation`, and `global-queue-notifications` timed out. Three also failed before the
dialog change; `task-controls` is newly observed and its cause is not established. The earlier
`task-chat-controls` failure passed in the final run. The complete release gate remains unmet.
Results are retained in the task worktree at `.tmp/desktop-e2e-artifacts-PUCV85/results.json`.

Integration with the database-lock recovery change passed 206 npm tests, typecheck, lint, a signed
release build, and six focused packaged scenarios: refinement, localization, provider recovery,
close retry, close pending, and relaunch. Results: `.tmp/desktop-e2e-artifacts-YF1Gtd/results.json`.
This focused verification does not replace the full release gate noted above.

## Knowledge draft Queue handoff — 2026-09-14

- Branch `fix/knowledge-draft-feedback` queues private Knowledge generation and opens the exact
  completed result in Workbench Review, focused and scrolled to its Markdown preview. The Queue
  result survives closing the Task panel and restarting the app; publication stays explicit.
- Cancelled jobs cannot save a draft. Draft storage, source validation, and Queue completion are
  atomic. Older, corrected, or published result snapshots are read-only.
- Previous feature validation: 213 frontend tests, full Cargo tests, typecheck, lint, whitespace
  check, signed release build, and packaged `task-controls`, `task-publication`, and `task-review`
  passed. English and Korean wide/narrow windows and opening a persisted result from Search were
  visually reviewed.
- The additional `global-queue-notifications` packaged scenario still times out waiting for the
  visible click target of an existing completed-job result. It did not pass; do not report the entire
  desktop suite green. Diagnostics: `.tmp/desktop-e2e-artifacts-BcWWpu/results.json` in this worktree.
- The primary checkout's existing user edits are excluded from this feature commit.
- Post-rebase integration validation passed 217 frontend tests, full Cargo tests, typecheck, lint,
  whitespace checks, and the signed release build. Packaged `task-publication` and `task-review`
  passed on that build. `task-controls` timed out after lineage loading, so the full desktop gate
  remains unmet. Diagnostics: `.tmp/desktop-e2e-artifacts-919VQg/results.json` in this worktree.


## Refinement continuity (2026-09-16)

Capture refinement now promotes the same ID, removes the original from Inbox,
shows its text on the Task card and in details, and resumes the Capture session
from the Task. Existing differently identified Tasks retain their origin links.
Subtask acceptance requires the explicit split action (`intent: "split"`).
Regression coverage was updated for promotion, legacy continuity, and split
intent. At the user's request, tests, builds, and rendered UI verification were
not run before merging to main. Run the combined verification after the pending
work is integrated. The live application database was not edited.

### Image-only Capture initial refinement (2026-09-17)

Image-only Captures automatically start the existing conversation and preview
flow after saving. Older Captures without conversation history start on opening
Refine. Reopening does not repeat the initial request; proposal application
remains a user decision. English and Korean feature guides are updated.

`npm test` passed, including 247 frontend tests and the supporting runtime,
provider, and signing checks. Native tests and release packaging were stopped
at the user's request; packaged desktop E2E was skipped. Native verification
remains outstanding for this change.


## Active Knowledge workflow integration — 2026-09-26

User authorized full execution of all 17 packages, Spec Kit, isolated worktrees and final main merge.
The current program ledger is `specs/knowledge-workflow-program.md` in
`.worktrees/knowledge-workflow-integration` (`feat/knowledge-workflow-integration`).
Main has not yet been merged. A/B/C/D and F native contracts are integrated; E consumer rescue,
F review UI, G journaled archive completion and final cross-feature/packaged verification remain.
Keep every existing worktree and uncommitted edit. Cargo builds share a target and require one
explicit lane owner. Do not interpret checkpoint commits or old frontend test passes as completion.

### Vault navigation and note reader (separate worktree)

Approved mockups were implemented on `design/vault-navigation-mockups` in `.worktrees/vault-navigation-design`; no merge/push. See `design/vault-navigation/IMPLEMENTATION.md` for actual rendered comparisons, verified Library screenshot IDs, scope and validation. Frontend checks pass; 14 native command tests and the 3 focused Vault checks pass. The full native library run has an unresolved unchanged structural-index timing budget failure (about 4.03s against 3s); two other timing failures passed isolated reruns. Packaged E2E was assessed and skipped in favor of native boundary integration and real Mermaid/image rendering under production CSP. No personal Vault or installed-app capture was used; merge/push remains a separate user decision.

The Vault follow-up fixed confirmed stale selected backgrounds in WKWebView by removing scoped selection transitions and adding search-result aria-current. All three tabs now pass actual computed-color/content assertions before screenshots are saved. Frontend verification is now 496 tests; the screenshot manifest includes new Library versions and a Search capture.

User explicitly authorized a follow-up raising the common Vault note-read cap to 10 MiB (10,485,760 bytes), shared with navigation metadata reads. Image size policy remains a separate unchanged 10 MiB cap, and automatic translation behavior is unchanged. Native boundary checks cover below/exact/over sizes; large-Markdown UI rendering performance remains unmeasured. The previous 2 MiB value was a pre-read data-budget guard, not a web-standard or benchmark-derived limit.

Vault main integration and normal push were explicitly approved after the 10 MiB follow-up. The dedicated Vault branch rebased onto `9940d91` without conflicts, preserving Compass and excluding pending notification/session work. Integration verification passed 498 frontend tests, runtime checks, lint, typecheck, build and 15 native command tests. Rust formatting still fails on both the integration tree and archived remote-main baseline; no broad unrelated cleanup was made. The existing native index timing limitation remains documented in the Vault handoff.


## Session implementation checkpoint (2026-10-23)

Dedicated branch `design/task-session-design` follows approved session/environment/reasoning/Queue mockups on `f3ede45`. Existing execution services are reused; durable queue snapshots/atomic claim, stop-confirmation gating, consent revision guards and a compact session UI are implemented. See `design/task-sessions/IMPLEMENTATION.md` and `COMPATIBILITY.md` in that branch for scope, tests and release gates. The user subsequently authorized session-only main merge/push with the disclosed visual limitation; notifications remain on hold.

Actual implementation screenshot comparison is pending: Computer Use reported that the Mac is locked and requires manual unlock. Do not substitute mockup screenshots for implementation evidence or claim pixel acceptance. Resume the prepared synthetic shipping-component review after unlock. Packaged E2E remains skipped based on the recorded cheaper lifecycle/transaction/bridge/UI test coverage; assess a targeted scenario only if rendered review reveals a concrete gap. Known v9 migration fixtures remain separate baseline failures.

Session migration follow-up: exact `f3ede45` reproduces both v9 fixture failures. Only the new queue-table teardown omission is fixed; preservation assertions and unrelated baseline assumptions remain unchanged. `design/task-sessions/MIGRATION-COMPATIBILITY.md` records official versus diagnostic results and the future global migration-26 collision with held notifications `406b106b`. No notification merge/reconciliation is authorized.

Session preparation regression follow-up: `resolveReasoning=true` bypassed the verified live-thread cache and resumed before the first rollout existed. Local fake reproduces the exact missing-rollout error. The correction reuses normalized provider metadata only for identical thread/generation/settings revision, still resolves reasoning, and preserves explicit missing-conversation errors after restart with retry/new-session guidance. Existing IDs/history are never cleared or replaced. See the separate local session follow-up commit; main integration requires Capture-owner coordination. Visual mockup parity remains incomplete/unverified.

Fresh-session preparation follow-up: native fixture explicitly verifies no initial thread ID, exactly one thread/start and zero thread/resume calls on first and repeated preparation. The reported first-prepare failure is not reproduced by that fixture and must not be called resolved. Added start/resume stage prefixes for provider errors. Model-setting changes intentionally invalidate cached preparation; missing-rollout errors preserve the original thread and invalidate readiness rather than executing under stale settings. Provider-restart recovery preserves the same boundary.

Approved Sessions UX gap correction is recorded in `design/task-sessions/UX-COMPLETION.md`: setup and current decision priority, navigation-scoped creation, running request wording/empty queue destination, completed-action removal, local settings cancellation, and scoped spacing/overflow fixes. Actual wide/narrow pixel acceptance remains pending manual Mac unlock; first-provider-prepare incident is still unresolved. Keep this UI correction separate from functionality hotfix `6333386`; coordinate with parent before main writes.

User authorized session follow-up integration after disclosure of unresolved fresh first-Prepare and unverified pixels. Dedicated `.worktrees/session-followup-integration` applies only `6333386` and `2a9617b` on `cbbc892`; held notifications, sidebar and separate Capture formatting correction are excluded. Original separately reviewable commits remain on the session design branch.

## Open-modal session Work Log refresh correction (2026-10-23)

Dedicated `.worktrees/session-worklog-recording-audit` on `a9b3f35` reproduces saved session logs missing from an already-open Task snapshot until reopen. Parent TaskDetail refresh/wiring and native terminal/sync functions are identical to pre-redesign `f3ede45`; this is an existing gap, now explicitly authorized for a narrow fix. Persisted Run revision changes invalidate the parent Task, related reads coalesce with a trailing pass for intervening changes, and exact log-open waits for reread before focus. Pointer/keyboard Work-tab entry rereads data; active execution/runnable Queue/distillation job metadata drives scoped polling, preserving old image/translation behavior. A read-only Task refresh hint covers the between-Run Queue gap and excludes paused Queue/uncertain terminal execution; it is separate from transition guards. Native storage/dispatch/Queue logic is unchanged; Task projection adds read-only distillation job status. Regression coverage includes no per-token reads, coalescing, stale Task identity, retained drafts, summary completion and stopping refresh. Native fake verifies three linked logs and terminal reports in Task projection. Initial first-Prepare incident and locked-Mac visual acceptance remain unresolved. No merge/push is authorized for this correction.

## Navigation visual consistency — 2026-10-23

Task branch `task/navigation-consistency` in `.worktrees/task-navigation-consistency`
reuses the existing Vault rail appearance in the shared sidebar stylesheet:
cream surface, flat 15px menu buttons, regular inactive labels, bold pink current
view, smaller brand and separated settings entry. Existing menu markup, order,
labels, current-view semantics, locale controls and focus treatment are preserved.
Workbench/Compass/AI setup retain their 248px desktop rail and narrow horizontal
navigation; Vault retains its 176px/76px rail and existing mobile reader layout.
The existing guidance card remains available outside Vault.

Rendered comparison remains pending: computer-use inventory reported that the
Mac is locked. No unlocking, security-setting change, installed-app mutation or
production-data access was attempted. After manual unlock, compare Workbench,
Compass, AI setup and Vault at wide/narrow widths in English and Korean, including
hover, current selection and keyboard focus. No screenshots from this task have
been verified. Existing screenshots describe their earlier revisions.

Packaged E2E is skipped: this is a shared CSS-only appearance change, with no
routing, native boundary, persistence, lifecycle or packaging changes. Existing
Sidebar/App/Workbench/Vault navigation tests cover destination and current-view
behavior; lint/typecheck and the normal test command cover regressions. Pixel
layout, wrapping and real keyboard focus visibility remain the visual-review gap.

Verification: all 65 Vitest files / 498 tests passed, including Sidebar, App,
WorkbenchNavigation and SearchView navigation. Desktop helper tests (6), provider
fixture tests (4), macOS signing/runtime/icon tests (7), runtime/native-only/work
tracking boundary checks, lint, typecheck, production build/font verification and
whitespace validation passed. The initial `npm test` wrapper stalled at the
provider fixture under restricted loopback access; that stage passed separately
with loopback permission, and all remaining wrapper checks passed separately.
No Rust tests or release build were needed for the CSS-only change.


Approved navigation integration: the user authorized main merge and normal push.
The sidebar-only commit was rebased onto `9534092`; the only conflict was this
handoff file, resolved by preserving both sections. Integration verification
passed all 66 frontend files / 513 tests, the complete `npm test` wrapper, lint,
typecheck, production build/fonts and whitespace checks. Existing unrelated
Rustfmt/performance baselines were not altered. The Mac remained locked when
checked again, so wide/narrow/focus pixel review remains unverified. Local
packaged E2E remains skipped for the CSS-only risk assessment above.


## Shared vertical navigation correction — local only, 2026-10-23

The preceding `57822d4` / integrated `c8e6a43` change only shared appearance.
It left `Sidebar`'s active-Vault branch reordering menus and swapping hardcoded
English labels/icons for a separate `vaultText` translation path. Vault-only CSS
also changed rail width, compact placement and visibility; other views switched
to top navigation. The user explicitly rejected those remaining differences.

Follow-up `.worktrees/task-shared-navigation`, branch `fix/shared-navigation`,
starts at `c8e6a43`. One unconditional menu definition in the existing Sidebar
uses the Vault icon/order treatment (Workbench, Compass, Search vault, AI setup)
and existing app `nav.*` keys from both public i18n dictionaries. The separate
Vault shell translations are removed. The component remains mounted once before
`main`; active view affects only current class/ARIA, never menu content/order.
Global locale initialization, persistence and rollback still use the existing
runtime. A translated navigation landmark names the shared menu.

One shared shell layout uses a 176px vertical rail or a 76px compact rail at
<=900px across all routes. Page CSS no longer styles or hides the shell rail.
The compact rail keeps visually hidden accessible labels, real native buttons,
the same focus styling, and a reachable language selector. Vault's <=650px
list/reader back behavior remains within page content. This intentionally
replaces the previous mixed 248px/top-navigation and Vault-only rail layouts.
Within-page tabs, route IDs, native operations and user data are unchanged.

Tests cover stable DOM node identity/placement before main, order/icons/labels,
current-view exclusivity and focus retention for all four actual App routes;
English/Korean round trips also use production locale bootstrap/change/API code
with isolated bridge fixtures. These are DOM/runtime assertions, not pixel or
real keyboard Tab/Enter validation. Computer Use again reported a locked Mac;
no unlock/bypass was attempted and no new screenshot is verified. After manual
unlock, compare wide/compact/<=650px views, keyboard focus, language switcher,
and real text wrapping before claiming visual acceptance.

No merge or push is authorized for this follow-up. Documentation guide reviewed;
English/Korean READMEs and the affected Vault guide describe the common rail.
Packaged E2E is skipped: existing route and locale runtime integration tests
cover behavior; source/CSS review covers removal of per-view layout overrides.
Actual pixel layout remains pending, and packaged E2E cannot substitute for the
locked visual review. No native boundary, persistence or packaging changes need
a release build, Cargo tests or unrelated Rustfmt/performance-baseline edits.

Verification: 66 frontend files / 516 tests and the complete `npm test` command
passed, including the shared-menu and production locale-runtime regressions.
Lint, typecheck, production build/font verification and whitespace checks passed.
The first test fixture omitted production `api` and was corrected; no runtime
logic change was needed. Screenshots are explicitly marked as earlier revisions.
