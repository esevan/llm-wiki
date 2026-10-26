# Knowledge revision chat verification

This feature extends SpecKit020 with explicit diagnostic chat proposals while
preserving immutable versions and feature 021's publication authority. It does
not reinterpret private review notes as conversation or source evidence.

## Coverage

| Boundary | Verification |
| --- | --- |
| Generation input | Local HTTP provider fixture receives selected article, diagnostics, exact catalog/snapshot and conversation; private notes are absent. |
| Diagnosis | Stored findings and deterministic failures identify the target statement, short original evidence, reason and corrective direction. Unauthenticated quotes are not exposed as authenticated source excerpts. |
| Full correction | Replacing a quote, revising a sentence, removing a nonmaterial claim and adding a bounded interpretation update complete structured candidates. |
| Safety | Fabricated/outside-snapshot citations, invented verification, required unresolved omissions, unchanged quality blockers and invalid proposals cannot apply. |
| Apply | Real SQLite transactions append one immutable private revision, recheck source freshness/private head, preserve previous versions and published pointers, and replay application idempotently. |
| Cancellation/error | Shared registry cancellation and delayed replies cannot apply after cancellation; request text and persisted failures remain available. Missing evidence produces a request with no applicable candidate. |
| Publication | Actual native dispatch applies a published-version correction; temporary-vault integration keeps the old file until explicit archive approval, then finishes index and moves the published pointer. External edits both before and after prepare are preserved and block publication. |
| App/adapter | Mounted full App with production taskClient/Tauri adapter exercises blocked publication → chat → compare → apply → exact file preview → explicit publication. Native responses are mocked in this test; Rust integration separately verifies the backend. |
| UI | Actual App in an isolated browser fixture: Korean 1440×900 diagnosis, comparison and private v2 apply; English 900×900 two columns and 780×900 one column, textarea/button keyboard focus and no horizontal panel overflow. |

## Commands

- `cargo test --manifest-path src-tauri/Cargo.toml knowledge`
- `cargo test --manifest-path src-tauri/Cargo.toml workflow_foundation`
- `npm test`
- `npm run typecheck`
- `npm run lint`
- `npm run build`
- `git diff --check`

Logs are task-local under `.tmp/knowledge-chat-*.log`. Early tests exposed the
nullable candidate contract, controls inventory/read-count expectations, and
apply-selection timing; these were corrected. One earlier native inventory
benchmark exceeded its unchanged time budget during concurrent activity; the
subsequent Knowledge run passed. Temporary fixture HTML/server were removed
or stopped after screenshots, and browser viewport overrides were reset.

Final verification passed: Knowledge selection has 55 native unit tests plus
4 command/search/service integration tests (one existing timing test ignored);
workflow-foundation selection has 26 passing tests and one existing ignored test.
`npm test` passes 449 React tests and all helper/provider/macOS/runtime/boundary
checks. Typecheck, lint, frontend build and whitespace validation pass.

The conversation uses existing Task/Knowledge identity as its session boundary.
Existing refinement sessions own Task previews and adoption; modifying those
sessions would mix publication correction with Task mutation. One small turn
ledger supplies durability without adding a separate session head or scheduler.

## Limits and E2E decision

No real model experiment or original-user-data reproduction was performed.
Natural-language semantic truth is not proven by exact citation/structure checks:
source-linked interpretations and human review remain explicit. Body-only manual
editing still preserves existing bindings; coordinated repairs use chat proposals.

Native Mac app inventory/window capture was blocked by the locked host. Therefore
packaged desktop E2E/native screenshots could not be run. Do not substitute the
browser fixture captures for packaged Tauri evidence. Provider HTTP tests, mounted
App adapter tests and actual native SQLite/Vault/index integration cover the
implemented boundaries; native window focus/scroll/IME behavior remains a material
unverified check. No release build or installation was performed for unavailable
E2E, and no user database/file publication, merge or push occurred.

Captured images are local deliverables named `knowledge-revision-ko-wide.jpg`,
`knowledge-revision-ko-compare.jpg`, `knowledge-revision-ko-applied.jpg`,
`knowledge-revision-en-900.jpg`, and `knowledge-revision-en-780.jpg` in the calling
Codex task directory. They contain synthetic records only.

## Integrated responsive Task workspace verification

The integrated App keeps Task refinement in its existing dialog and preserves
all four Work/Sessions/Refine/Review tabs. Generated previews replace the
duplicate saved-definition panel; legacy definitions and unfinished edits
remain accessible in the preview position. Preview and Knowledge chat share
one 340–720 px responsive support-column token. Full App/shared-CSS fixtures
measure matching chat widths of about 429/583/720/720 px in
1440/1920/2560/3440 px windows, plus narrow stacked layouts.

The integration passes 457 React tests, 11 browser layout/interaction tests,
55 Knowledge native unit tests and four integration tests, 29 migration tests
and 26 workflow-foundation tests. Schema 25 and the 29-entry prompt registry
are preserved. The browser flow covers unsent-chat retention, comparison,
explicit application, cancelled publication and separate final approval;
native fixtures check external edits before and after archive preparation.
All model responses and UI records are synthetic. Native window behavior,
real-model semantics and the original user's database remain unverified.

## Same-app modal sessions (2026-10-21)

- Enter sends once; Shift+Enter inserts a newline. Composing Enter and IME
  keyCode 229 never send. The synchronous session request guard prevents repeat
  submission before React renders.
- A Task/version session preserves unsent input and an in-flight promise outside
  the modal. X/Escape close immediately during chat; reopening reads the saved
  conversation and reconnects to the existing request. Closing does not cancel.
  Cancellation remains explicit. A late cancelled reply cannot clear new input.
- Only body edits and private notes participate in the modal's unsaved guard;
  chat drafts have their own retained session. Atomic Task mutations retain the
  existing guard. App restart/resume scheduling is outside this change.
- The transcript scrolls independently, diagnostics are expandable, and the
  composer stays outside transcript scrolling. The inherited support-rail
  max-height is overridden because it otherwise compresses this nested chat.
- Focused chat/modal tests: 10; TaskDetail regression tests: 44. TypeScript and
  ESLint passed. The full React App with production CSS was visually exercised
  using synthetic long conversations and delayed provider responses. This is
  browser fixture verification, not a packaged-native or real-model run.

## Follow-up: proposal readiness and main-column review
The previous candidate-only UI condition could expose Apply when preparation
failed and no prepared JSON existed. The server now returns explicit readiness;
legacy proposed rows rebuild from their saved candidate/snapshot. Unknown source
identities are quarantined with advisory findings, like exact-quote mismatches,
rather than leaving a misleading applicable candidate.

Focused verification: 8 native revision tests and 20 React tests (15 chat, 5 App)
pass. These include native dispatch, persisted legacy-null preparation, rejected
citations, duplicate apply, structural failure, cancel/head guards; UI covers
0/1/6 diagnostics, legacy malformed rows, main-column portal and update requests.
The actual full App browser fixture uses mocked transport, not a real model or
user DB. Korean long-document comparison and private v2 application were checked;
native SQLite/provider fixtures separately verify backend behavior.

Final follow-up checks: full React suite 466 passed with one unchanged
ReferenceList performance threshold failure (mount p95 127.65ms while native
work ran concurrently). The isolated performance test and production control
inventory rerun passed, 3 tests total. Typecheck, lint and frontend build passed.
Native library suite passed 328 tests with 3 existing ignores; Knowledge selection
passed 69 plus 4 integration tests. Later archive-only additions passed 28 tests.
No packaged native-window or real-model experiment was performed.

Browser evidence (full production App, mocked transport): Korean long document
at 1440x1000 and 780x900; comparison is in the main reading column, absent from the
chat transcript; narrow comparison is one column with no horizontal overflow.
Diagnostics 0/1/6 match rendered rows, expanded notes have no nested height limit.
Apply produces private v2 in the fixture and leaves published v1 selected separately.
Captures: knowledge-followup-long-compare-ko.png,
knowledge-followup-long-narrow-ko.png and knowledge-followup-applied-narrow-ko.png
in the task output directory. The supplied JPEG was materialized locally and
viewed; the exact original user's database state was not inspected.

An additional native regression passes after the broad run: source changes reject
the old proposal without losing its candidate; an explicit update request includes
that candidate and the latest snapshot, then successfully applies private v2.

Main integration verification: the unchanged two-commit application tree was
squashed on current origin/main cd32449 without conflicts. `npm test` passed
all 467 React tests and helper/provider/runtime checks. Knowledge selection
passed 71 native tests (one existing ignore) plus four integrations. Typecheck,
lint, build and whitespace checks passed. Packaged desktop E2E was skipped: the
App fixture and native persistence/index tests cover this integration, with no
new packaging changes; native window behavior remains outside that evidence.

CI follow-up: the main comparison target now uses a stable parent ref. Only chat
resolves the mounted target, avoiding a second full document render per mount.
The portal regression exercises the ref route. The exact first integration CI
also exposed unrelated ReferenceList, RefinementPanel and TaskWorkSessions timing
failures; no global timeout or unrelated test behavior is changed here.

The ref follow-up passed 40 focused review/chat tests and all 467 React tests
plus runtime helpers, typecheck, lint and build. Local timing samples report
projection p95 19.14ms and selection p95 13.81ms, versus 26.44/20.58ms in
the previous integration run; this is local timing evidence, not a CI guarantee.

## Idea classification and proposal readiness (2026-10-26)

The generation contract is version 7 and the revision contract is version 3.
Both provider schemas enumerate the same four canonical idea dispositions.
Revision response parsing retains an unknown string only as proposal material:
domain preparation rejects it before readiness or canonical application.

Regression coverage includes the exact invalid-disposition error, retained
candidate, unchanged saved draft, legacy proposal revalidation on reopen,
Korean idea-specific diagnostics, and updated proposal application to private
revision 2 without changing publication. Mounted UI fixtures distinguish
structural failure, stale sources, and applicable review notes while keeping
the diff inspectable and the refresh action available.

Packaged E2E is skipped for this bounded contract/copy fix. Native loopback
provider/SQLite tests exercise the complete failed-proposal → refresh → private
apply path, and mounted production components exercise the diff, readiness
message and button conditions. No new native bridge, persistence schema,
layout or packaging behavior is introduced. These fixtures do not verify a real
provider, the affected host, or its saved data.

Follow-up recovery uses revision contract version 4. Mounted component tests click the actual update control against a saved failed v1 proposal while reviewing v2, verify one POST across repeated clicks and remount, preserve unsent text, and apply only the newly returned proposal. A clarification response no longer automatically reopens the historical error. Native fixtures assert that the rejected candidate and its validation diagnostics reach the provider request. Synthetic provider replies do not establish real-model recovery on the affected host.
