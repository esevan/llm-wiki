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
