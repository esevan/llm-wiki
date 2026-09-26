# Quickstart: Validate the reference-aware workbench

## Dependency gate

Before implementation, confirm integrated feature 015 contracts, feature 016 full Capture wiring/shared transition, feature 018 retrieval persistence/API, and the serialized migration head through D19. Do not add E20 while the repository head is 16. Record final dependency hashes and migration assignment in `plan.md`.

## Focused validation commands

Run separately after implementation:

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml reference_aware_workbench
cargo test --manifest-path src-tauri/Cargo.toml --test work_tracking_search
git diff --check
```

Use the stable repository commands if test filtering syntax changes. Do not run a release build or packaged E2E unless the final risk assessment identifies a native dialog/restart behavior that focused component/application tests cannot cover.

## Scenario 1: Necessary retrieval before first preview

Create the contract-change fixture with explicit approval language and an existing comparison-table source. Submit once without answering a clarification question. Verify the visible requested job moves through planning/searching/generation, no useful preview appears before required retrieval resolves, and the resulting preview includes every structured field, a labeled assumption, and exact source links. Confirm no Task field changes until Apply.

## Scenario 2: Conversation during optional work

Start an optional investigation for context revision 1, then change the customer count and approver in revision 2 while it runs. Verify the composer remains usable, revision-1 output becomes `changed_context`, revision-2 critical evidence surfaces promptly, supporting evidence appears in the next reply, ancillary evidence appears only in references, and no stale advice overwrites the preview.

## Scenario 3: References, modal, and mentions

Load duplicate titles, multiple versions, multiple sections, redirects, and at least 1,000 rows. Filter/sort, let new results arrive, and confirm existing rows do not move until the user accepts a new order. Open an exact section, navigate links/back/forward/previous/next, close with Escape, and verify focus/input/selection/scroll restoration. Insert multiple document/section mentions and confirm no message is sent until Send.

## Scenario 4: Usage provenance

View, mention, use, exclude, and explicitly adopt different references. Verify each state remains distinct. Move/archive a used source and confirm its stable ID/version/section provenance remains available. Verify later publication input includes actual-used sources and useful counterevidence only, never view/mention-only rows.

## Scenario 5: Versions and restore

Create generated, edited, and restored preview versions. Select and compare versions 1 and 3, including assumptions/reference snapshots. Restore version 1 as version 4. Verify versions 1–3 remain byte-equivalent, version 4 identifies its source, and Task revisions, Work Logs, decisions, journey, and messages are unchanged. Apply explicitly and verify one canonical Task revision plus the correct transition cause.

## Scenario 6: Failure and continuity

Exercise requested generation failure, necessary retrieval failure, no suitable result, optional investigation failure, changed context, expired grant, stale apply, and restore conflict. The last good preview and unsent edits remain visible. Optional failures never block conversation. No async event steals focus, scrolls unexpectedly, sends, applies, or produces an endless retry loop.

## Rendered UI review

Inspect wide/narrow English/Korean layouts and keyboard order with long content, empty/loading/error states, reference arrivals, viewer history, comparison, mention lookup, and reduced motion. Record screenshots or explicit limitations in the implementation handoff. Planning alone does not claim this review has run.

## Native consumer verification (2026-09-26)

- `cargo test --manifest-path src-tauri/Cargo.toml reference_aware_workbench --lib`: 10 passed.
  These include real HTTP fake-provider calls through the existing message command and
  native durable job. They prove planner → persisted retrieval → finalizer ordering,
  no automatic Task adoption, same-transaction apply/replay, user edits winning against
  delayed finalizers, an externally changed unindexed source superseding output,
  non-blocking conversation during optional investigation, stale optional findings being
  discarded, once-only reply evidence and exclusion of ancillary discovery, and actual
  aspect/filter/bounded-requery behavior.
- `cargo test --manifest-path src-tauri/Cargo.toml native::task_assistance --lib`: 28 passed
  initially; the remaining test had a pre-existing assumption about the registry prompt's
  data wrapper. Its parser was updated, and its exact targeted rerun passed.
- `cargo test --manifest-path src-tauri/Cargo.toml workflow_foundation --lib`: 7 passed
  after replacing the stale hardcoded schema-16 assertion with an actual database-head check.
- `git diff --check`: passed.

The native data flow uses deterministic provider fixtures and an indexed local Vault.
It does not assert production model output quality or a new latency promise. Stored
preview runs expose planner/retrieval/finalizer durations for later measurement.

Packaged E2E was skipped for this native consumer slice: deterministic command/job/provider,
SQLite and indexed-Vault tests cover the concrete ordering/concurrency/persistence risks.
No Tauri packaging or modal lifecycle was changed here. Shared React controls, mention
flows, live wide/narrow EN/KO rendering, keyboard/focus/IME/reduced-motion review and the
1,000-reference projection measurement remain for the UI integration checkpoint. No
release build or installation was performed.


## Live consumer rescue verification (2026-09-26)

The mounted `RefinementPanel.references.test.tsx` suite calls the production
`TauriApplicationClient` with deterministic native command responses. It covers initial
load and automatic generation, live polling, exact edit/apply heads, dirty-edit protection,
historical fetch/compare/restore, nested source modal focus, query forwarding, persisted
unsent mentions, IME/keyboard insertion, background investigation and locale switching.
The legacy panel tests remain in place with explicit handling of the new read routes.

- Native reference tests: 13 passed, including standalone registered job/provider flow,
  saved mention bindings, and opening investigation-only sources after an indexed move.
- `npm test`: 345 React tests passed, followed by all desktop helper, fake-provider,
  signing, runtime and boundary checks. After pinning the open viewer version, 16 focused
  mounted/component/inventory tests passed; the relative-link viewer addition passed all
  3 viewer tests. The 1,000-reference filter/order test took 477 ms in a focused jsdom run;
  this is fixture evidence, not a production latency guarantee.
- The final exact-source native regression passed again after extending historical
  investigation-context reads. No broader native changes followed the 13-test pass.
- `npm run typecheck`, scoped ESLint and `git diff --check` passed. Global `npm run lint`
  still reports four pre-existing React hook dependency warnings in the untouched
  `TaskDetail.tsx` and `TaskWorkSessions.tsx` files.
- Rendered production component harness: English at approximately 1,160 and 620 pixels;
  Korean at 1,080, 760 and 620 pixels. The exact source modal was reviewed at 620 pixels;
  its stretched close control and default browser control styling were corrected and
  inspected again. Native browser keyboard traversal reached the next source, Enter
  changed the exact source binding, and Escape returned to Refinement. Mocked component
  tests separately assert opener focus restoration and late-response cancellation.
- The harness used an isolated Vite page with fixture responses. It did not write real
  user data. No release build, packaged desktop run or installation was performed.

Packaged verification is reserved for the final integrated application: this checkpoint
covers the command adapter, persistence/order races in native tests and mounted modal
behavior separately. OS reduced-motion settings and a physical Korean IME were not
changed; IME composition guards are covered in mounted tests and reduced-motion behavior
uses the existing shared transition component and CSS rules. Real provider output quality
and final integrated packaged lifecycle remain outside this isolated fixture review.


## Reference and mention completion verification (2026-09-26)

- `cargo test --manifest-path src-tauri/Cargo.toml reference_aware_workbench --lib`:
  14 passed. The added real provider/workspace test verifies information type, exact
  used-source facts and persisted assumption status. An existing unused-function
  warning remains; the native Cargo lane was released after this focused suite.
- New rail/mention/preview tests passed (3 + 3 + 6). Mounted Refinement, source inventory
  and command adapter tests passed (32 total). These cover the 1,000-document/page
  boundary, sections, combined filters, held arrivals, focus, batch insertion, atomic
  deletion, paste fallback and exact canonical adoption event. The new extracted
  control source is registered; discovery-based inventory validation replaces the
  stale fixed source count.
- An attempted filtered `npm test` was expanded by the chained npm script into a full
  React run: 354 passed and two source-inventory failures identified the extracted
  component registration. After fixing the inventory, focused reruns passed. No full
  suite rerun or follow-on npm success is claimed for this checkpoint.
- TypeScript, scoped ESLint and `git diff --check` passed. No release or install ran.
- A new production Refinement fixture was prepared for visual review, but CUA reported
  no available browser and an explicit in-app browser attempt was unavailable. No local
  Chrome/Edge or Playwright runtime was installed. The new controls therefore retain a
  rendered wide/narrow EN/KO review limitation. The temporary fixture and Vite server
  were removed after the attempt; no user data was touched.

The final integrated program owns packaged lifecycle verification and the full visual
matrix. No packaged E2E was run for this bounded follow-up; mounted native-adapter and
real SQLite/provider tests cover its data, event and interaction changes separately.


## Reference rail profiling (2026-09-26)

`ReferenceList.performance.test.tsx` measured the production `ReferenceList` component
with 1,000 exact reference bindings across 25 independent mount, filter and sort cycles.
Each mount projected the initial 20-document page. Each update applied the `Source 09`
search and title order in one React `act`, then verified the resulting 20-document page
from 100 matches. The test uses real `performance.now()` wall-clock samples and the
nearest-rank 95th percentile; it does not mock timers.

- Initial 20-document page mount: 25 samples, p95 6.84 ms.
- Filter and sort state commit: 25 samples, p95 12.59 ms.
- Both measured p95 values passed the 100 ms budget in the isolated Vitest/jsdom run.

These measurements cover component computation and DOM commits in jsdom. They are not
rendered visual review, packaged desktop E2E, or a production-device latency guarantee.
## Capture-to-publication provenance regression (2026-09-26)

`cargo test --manifest-path src-tauri/Cargo.toml reference_provenance --lib` passed
both tests. `cargo test --manifest-path src-tauri/Cargo.toml reference_aware_workbench
--lib` passed all 14 tests. The Knowledge filter passed 36 of 39 on this branch; three
pre-existing prompt/schema and Task-assistance fixture failures have separate main
repairs and are not reported as passing here.

The new integration fixture uses an indexed Vault, actual native refinement commands,
a deterministic HTTP provider and real job persistence. It applies a Capture preview,
completes its Task, generates a Knowledge draft from its exact snapshot and prepares
an immutable archive proposal. Assertions cover exact counterevidence section/version
and recorded statement, non-export of viewed/candidate/excluded rows, two-Task isolation,
and F/G stale-source guards after exclusion. A second test checks Task-owned legacy
role normalization and visible failure when no stored claim can explain the use.

Packaged E2E is skipped for this bounded native provenance change: these actual
command/job/provider/DB/index/application/archive tests exercise its remaining data
integrity risk without a packaging or UI change. Final integrated packaging checks
remain owned by the program integration run.


## Packaged refinement timeout diagnosis (2026-09-26)

The integrated `task-refinement` run exceeded its outer 180-second timeout without
a result. Its retained database contains both Tasks, both preview owners, all three
completed preview jobs, the two-image user message and assistant response. No edited
preview or adopted Task revision was stored. These facts locate progress after send
and before adoption; they do not establish a native generation deadlock or a screen
lock cause.

The scenario now persists diagnostic progress at restore, image send, terminal polling,
scroll, close/reopen and edit/save/apply boundaries. The post-send checkpoint includes
scroll dimensions. All existing assertions and their 10-second wait limits are unchanged.
This instrumentation is preparation for the next selected packaged run, not evidence
that the timeout is resolved.

### Final selected packaged receipt

`task-refinement` passed on signed main `db4ada7`; artifacts: `.tmp/desktop-e2e-artifacts-i2GYOb`. Note/image restoration, chat completion, overflow scroll, immutable preview edit/save and exact adoption by the original Task ID all passed. Harness waits now follow actual editor render/save; final verification uses stable ID because adoption may change the title.

The three selected flows address concrete native background, modal/adoption and
file/index risks; the full suite was not needed. Later source changes were
refinement-harness-only. The installed app and personal Vault were not changed.
Latest direct visual review remains unavailable on the locked Mac; mounted or
packaged assertions do not substitute for that visual state matrix.
