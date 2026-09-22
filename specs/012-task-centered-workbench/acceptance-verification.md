# Task-centered Workbench acceptance verification

## Lineage flowchart and Knowledge follow-up (2026-09-21)

This follow-up applies to the uncommitted `fix/lineage-visual-polish` worktree. It does not update
any signed release artifact or historical packaged-test result below.

- `npm test` passed 38 Vitest files / 269 tests plus the Node/runtime/boundary checks. After final
  routing edits, focused journey and interactive-source checks passed (8 tests). TaskDetail
  coverage verifies that Review uses a saved draft journey while Details uses current lineage.
- Focused Rust checks cover relationship validation, locale-bound/versioned caches, recorded
  fallback, deleted/stale source protection, journey preparation before Knowledge, exact snapshot
  persistence, and cancelled/stale draft saves. The application-service roundtrip test verifies
  that reopening a Task returns the stored draft lineage. These are focused checks, not a claim
  that the complete native suite was rerun for this follow-up.
- Typecheck, scoped graph lint, and `git diff --check` passed. The documentation synchronization
  that follows changes no application code and requires document/link checks only.
- Actual browser review exercised the graph fixture at 1100px and 420px container settings,
  English/Korean interface labels and date formatting, mixed-language long titles, keyboard
  selection/focus, chronological row transitions, gap markers, and all three semantic link styles.
  The fixture's node titles and relation explanations were fixed mixed-language sample data.
  **This does not verify Korean-translated node titles, localized provider output, or a fully
  localized Korean graph.** The earlier conversational claim of complete bilingual visual
  verification was too broad.
- The reviewed fixture uses the actual React graph component, not the packaged native application.
  Native Review-tab rendering, real-provider relation quality, and end-to-end provider-to-Knowledge
  locale behavior remain unverified by that capture. Unit/component coverage establishes the
  checked preparation, persistence, and tab-selection contracts.
- Packaged desktop E2E and a release build were skipped. Focused Rust/component checks cover the
  changed data/state contracts and browser inspection covers graph geometry; no packaging change
  required a new packaged scenario. The native/provider limitations above remain explicit.

See the current [lineage contract](contracts/application-api.md#recorded-journey-and-inferred-relationships)
and [feature guide](../../docs/features/lineage-knowledge-layer.md). Historical acceptance figures
below remain attached to their original artifacts.

**Verified:** 2026-09-12

> **Historical Task-centered acceptance record:** The artifact identities, nine-case records, and
> 32/32/169 figures below predate the UI/UX follow-up. The follow-up package passed 32/32 scenarios;
> nine current captures were reviewed; real IME/VoiceOver/OS reduced-motion remain unverified;
> do not treat this file's earlier totals as its release result.

**Current UI/UX artifact:** CDHash `9b2b81082a43de0637bedd33a8cc670709ff2b21`, signed 2026-09-12 11:44:31 AM by `LLM Wiki Local Signing`, from dirty `fix/ui-ux-improvements` state based on `de01ff47398871902a765d43b5a4060161316f92`. Focused native geometry passed: requested native 900×640, 904×768, and 1280×820 yielded WebKit viewports 900×608, 904×736, and 1280×788; long-URL Task header/close and title-input geometry passed. The requested native sizes are not viewport claims. Full E2E passed 32/32, with 188 scanned source controls and 175 rendered/exercised/asserted controls; nine current captures were reviewed; real IME/VoiceOver/OS reduced-motion remain unverified.

## Release artifact

### UI/UX follow-up final package result

Nine final native 1198×768 Korean-light captures from isolated `Dlnhjg` state were visually reviewed: Workbench, Task detail, Refinement, Task review, completion/Knowledge, Queue recovery, Vault search, Compass, and AI setup. Manual Task Escape returned focus to its original trigger. These captures establish the represented visible states; real Korean IME, VoiceOver, and OS reduced-motion remain unverified.

The signed UI/UX package with CDHash `9b2b81082a43de0637bedd33a8cc670709ff2b21` passed the full suite: 32/32 scenarios. Its inventory recorded 188 scanned source controls and 175 rendered, exercised, and asserted controls; `unknownEnabled`, `missingSourceEvidence`, `undocumentedDisabled`, `unexercised`, `effectMissing`, and `notRendered` were all empty. Manual Korean IME, VoiceOver, OS reduced-motion, Windows, zoom, and native quit/crash draft durability remain unverified or out of scope.

Current checks: `npm test` passed 34 Vitest files / 176 tests, six desktop-helper tests,
two local-provider tests, four signing tests, and runtime/native-only/application-boundary checks.
`cargo test --manifest-path src-tauri/Cargo.toml` passed all 106 tests. Typecheck, changed-code
ESLint, and whitespace checks pass. Full lint retains 23 pre-existing `no-explicit-any` findings
in untouched `completedButtonsRuntime.test.ts` and `remainingButtonsRuntime.test.ts`.
The portable [evidence record](../../docs/testing/evidence/ui-ux-improvements.json) records the
signed package, source snapshot, per-scenario results, geometry, coverage, and image hashes.
Local raw artifacts are `.tmp/ui-ux-improvements/e2e-complete/` and
`.tmp/ui-ux-improvements/geometry-intrinsic/`.

## Historical baseline artifact and verification (before the UI/UX follow-up)

The historical baseline macOS artifact was `src-tauri/target/release/bundle/macos/LLM Wiki.app`. Its code-directory
hash is `1988d962cfc113ca58786875eb7553c0cad5630b` and the complete recorded artifact identity is
`1988d962cfc113ca58786875eb7553c0cad5630bcf1dc3ac5fd955d0f2fff6c3`.

## Verification evidence

- `npm test`: 29 Vitest files and 117 tests passed; five desktop helper tests, the provider fake,
  macOS signing tests, native-runtime checks, and work-tracking boundary checks also passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: all unit and integration suites passed after
  the final Rust changes, including application commands, stdio MCP, Task tracking, concurrency,
  migrations, parity, the 10,000-event projector, release acceptance, review, search, and service.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and strict all-target clippy
  passed after the final Rust changes. `npm run typecheck` and the touched Task UI ESLint scope
  passed after the final frontend changes. Full `npm run lint` retains 21 pre-existing `any`
  errors in old runtime tests; no new touched-scope lint error was introduced. `git diff --check`
  passed.
- The focused Task-client contract suite (nine tests) and `npm run typecheck` passed after the
  final frontend correction. The client now sends `expectedProblemRevision` and presents backend
  `detail` errors for stale resolutions.
- All nine packaged scenarios passed together against the artifact above. Results are in
  `.tmp/desktop-e2e-artifacts-cJf2wU/results.json`: capture, Work Log, refinement A→B→A restore,
  exact relationships, cancelled/stale/retried review, correction/publish/regenerate/withdraw,
  explicit Problem resolution, relaunch persistence, and English/Korean narrow localization. The
  Problem-resolution scenario proves Task completion does not resolve the Problem, the rendered
  exact-resolution control succeeds, evidence remains on the explicit decision, and stale revision
  resolution returns `409` with conflict detail.
- `npm run review:ui -- --reuse-build` launched the same artifact with isolated state at
  `.tmp/ui-review-1duIbO`. Final wide/narrow English/Korean captures, the populated Task header,
  collapsed connection details, refinement panel, cited current review result, and
  findings/stale/cancelled attempt history are in `.tmp/ui-review-final/`.
- A deterministic rich Knowledge document with Work Log, checklist, decision, exact Problem
  revision, Task completion, and source provenance is retained at
  `.tmp/desktop-e2e-task-publication-pOy8a1/vault/.llm-wiki-withdrawn/4162ec5c-d505-470b-9226-4f164d3e029a-r2-f7cc4f39-9875-4788-a8ea-406e83cd4a34.md`.

## Final verification update (2026-09-12)

Final signed artifact CDHash: `3be85b685e04f76596c914fafa4bf9639081958d`; full identity:
`3be85b685e04f76596c914fafa4bf9639081958d250fa8475eff84ecf9c35921`; signed 08:58:37. The
packaged runner exited 0 with 32/32 scenarios passed in `.tmp/desktop-e2e-artifacts-FcAbm6/results.json`.
The manifest recorded 169 rendered/exercised/asserted controls across 177 source controls; all
evidence-gap counters were zero. F48, F51, and F60–F66 are unreachable; F67 is reachable and
covered. Earlier nine-case and 68-row baseline records are historical. Functional manual review
is complete; OS reduced-motion review remains pending. Real-provider quality/latency and
Windows/Linux installed testing remain unverified.

### Final supporting checks and manual review

- Frontend: 149 tests passed. Rust: 106 tests passed. Typecheck, scoped ESLint, Rust formatting,
  strict all-target Clippy, and 19 final coverage/source metadata tests passed.
- Full-repository ESLint still has 21 pre-existing `no-explicit-any` findings; scoped success does
  not mean that baseline debt disappeared.
- Root independently read `FcAbm6/results.json` and `interactive-coverage.json`, and rechecked the
  signed artifact identity after manual review. All six missing/unknown/disabled-reason arrays are empty.
- CUA manual review used the same release app and isolated `.tmp/ui-review-jPAdBV`: Korean Task
  creation/start/Work Log save; Capture original-Solution display, note edit, close, new shortcut,
  and exact note restoration; English/Korean switching with retained content. Wide 1198×768 and
  narrow 911×752 native-window screenshots were directly inspected. These images are conversation
  evidence, not retained screenshot files. Only the isolated review app was closed.
- Reduced-motion CSS rules in `refresh.css` and `legacy-components.css` were inspected statically.
  Actual OS preference execution was not performed; user-wide preferences were not changed.
- Final generated Markdown was inspected at
  `.tmp/desktop-e2e-task-publication-qLchGx/vault/.llm-wiki-withdrawn/98dd3780-4ff0-4d20-90c8-93efbaf9326b-r2-0bd0a9ab-7cb0-4b1f-9b86-2de820e526fd.md`.
  Authored Task fields, Work Log/checklist/decision evidence IDs, exact Problem revision 1,
  Task revision 2, completion identity, and lineage were retained.
- Implementation remains uncommitted in the dedicated task worktree. The installed application and
  user Vault were not replaced or migrated by this verification.

## External gates

Provider readiness was rechecked without revealing settings on 2026-09-08 and remained false.
The fixed 12-document/24-conflict real-model corpus is therefore blocked by the absence of a
configured provider key; deterministic provider coverage passed. The live plugin attempt required
elicitation and saved no record. Installed Windows named-pipe and Unicode-path packaging remains an
external-platform gate. These gates do not leave an unconstrained application implementation task.

## Historical implementation notes (2026-09-08; superseded by final verification)

Source and component/native checks on 2026-09-08 cover the new task-chat scenarios, but they do
not update the release-artifact result above. The deterministic fake records complete request
history and the scenario requires three ordered Capture-refinement turns whose final local-provider
request contains the saved Capture, all prior user turns, and an earlier assistant reply. React
checks cover panel controls and in-app tracking controls; those mocked component checks are distinct
from packaged UI execution.

The migrated-Problem native regression revises the Problem to revision 2, accepts an explicit
current-Task proposal, and reads back the exact `problemId`/revision link. A second regression
rejects nonexistent revision 99 with `not_found_or_not_visible`. No real-provider credential or user
database is involved. The previously installed `/Applications/LLM Wiki.app` was only inspected
read-only; its reported Refine no-op was not reproduced and no cause is claimed. The new
`task-chat-controls` and `task-legacy-chat-controls` scenarios, their F31–F35/F44–F49 manifest
effects, and the final executable identity are **pending** the one final artifact build and packaged
run; they are not marked passed by this source evidence.

The isolated E2E fixture for conditional Knowledge cards creates an accepted, completed tracking
session without touching a user Vault; the scenario then defers, reviews, accepts, expands, publishes,
and edits cards through the rendered UI. The Preview-warning scenario arms an E2E-only one-shot native
refinement-context failure, then uses its rendered retry control to reload the same migrated Problem
preview. The old draft modal remains explicitly unexercised: it is entered only by the legacy
`openNextChat`/`draftWithAI` route and has no rendered entrypoint from the current migrated **Refine**
card. It is not reported as packaged coverage.

## Documentation and operational scope

`docs/DOCUMENTATION_GUIDE.md` was reviewed. Product, bilingual feature, API contract, migration,
packaging, quickstart, continuation, and characterization records were updated with the Task as the
canonical unit. The next architecture pass is recorded in
[`docs/backlog/task-centered-architecture.md`](../../docs/backlog/task-centered-architecture.md);
the attempted plugin capture requires connector elicitation, so no remote backlog record was saved.
No commit, deployment, live-database migration, or primary-checkout rewrite was performed.
