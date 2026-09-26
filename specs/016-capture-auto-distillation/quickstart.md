# Quickstart: Validate Automatic Capture Distillation

## Prerequisites

- Workflow foundation merged into this branch.
- Local test database isolated from user data.
- Deterministic provider fixtures for success, delay, malformed output, transient failure, and permanent failure.
- Dependencies prepared by `scripts/create_task_worktree.sh`; do not run `npm ci` against shared `node_modules`.

## Focused verification

Run stable commands separately after focused changed-module tests:

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Implementation verification on macOS arm64:

- `capture_distillation::tests`: 4 passed, including runtime registry validation,
  exact-head atomic apply, stale proposal retention, and batch/single projection equivalence.
- `application_commands`: immediate canonical Capture projection and modal read/no-enqueue
  checks passed independently.
- Workbench, Refinement, transition, and interactive-control tests: 57 passed.
- TypeScript typecheck and `git diff --check`: passed.
- The feature branch did not rerun the broad `npm test` or complete native suite. Those checks are
  reserved for the final integrated program branch so the shared build lane is not repeated for
  every package.

## Scenario 1: source and one operation

Delay provider execution. Save text-only, mixed, and image-only new Captures. Confirm exact source is immediately readable and survives restart; one logical operation exists per source revision. Open/close modal, rerender, reconnect, and restart without creating another. Existing Captures receive none.

Expected: source persistence remains under 50 ms p95 and provider delay does not block another input.

## Scenario 2: grounded apply and shared state

Complete deterministic outputs. Confirm every derived claim maps to fixture source, unsupported fields stay empty, and card/modal show the same current status/content. Confirm same ID, Inbox position, category, activity time, and separately inspectable raw source.

Expected: one atomic applied result; no Task, Problem, workflow state, recency change, or visible version count.

## Scenario 3: active context preservation

While running, open modal; type composing/non-composing input, select text, choose tab, and scroll page/modal. Publish success and repeat with failure.

Expected: value, selection/composition, focus, tab, and scroll stay stable; input remains enabled.

## Scenario 4: conflict, deletion, and retry

Delay completion, then directly edit one Capture and add explanatory chat to another. Confirm the direct edit gets zero overwrite plus optional proposal, while chat supersedes the old attempt and coalesces one latest-context successor inside the same logical operation. Delete another Capture before completion and confirm superseded output cannot recreate it. Produce permanent failure, restart, and retry twice while first retry is active.

Expected: exact currentness wins, one proposal/publication per job, one active equivalent retry, and safe failure remains visible.

## Scenario 5: motion and accessibility

Trigger eligible automatic apply on mounted card. Inspect frames: old title clips right-to-left, new title reveals left-to-right, text stays stationary, body change is subtle. Repeat navigation, remount, locale change, restart hydration, same revision, historical reading, and reduced motion. Exercise keyboard modal/retry and long Korean/English content at wide/narrow widths.

Expected: only eligible live replacement animates; focus, labels, wrapping, contrast, and announcements remain usable.

## Benchmarks

- Representative Capture persistence p95 under 50 ms and no >15% regression.
- 1,000-item Workbench projection under 100 ms p95 and no >15% regression.
- One provider request per logical operation, exact source only, no Vault retrieval, bounded output.

The representative native fixture contained 900 Captures, 300 Tasks, 20 Problems, and 20 Work
Logs, with 100 measured samples per operation. On macOS arm64, p95 was 2.255 ms for Capture
persistence, 2.490 ms for Task persistence, and 32.453 ms for Workbench projection. All absolute
gates passed. Capture distillation metadata is loaded once per Workbench projection; an earlier
per-Capture query implementation measured 253.674 ms and was replaced before completion.

## Rendered review

The shared transition has automated coverage for normal and reduced motion, rapid revisions,
focus, stationary layers, and replay guards. Workbench and Refinement tests cover long-lived card
identity, active input, failure, and retry. A rendered review of the final native UI was not run on
this feature worktree because no isolated packaged build containing this branch was available;
opening the installed application would have used the user's live data and stale application code.
Wide/narrow Korean/English visual inspection remains an integration review item.

## E2E decision

Packaged E2E was skipped on this feature branch. Focused Rust transaction/read tests and React
interaction tests cover the implemented boundaries, while the remaining packaged risk is restart
recovery and WebKit event/focus behavior. The final integrated program branch should decide whether
that concrete gap warrants the single `capture-distillation-restart` scenario after its one release
build; the full suite is not justified by this package.

Run one targeted `capture-distillation-restart` packaged scenario only if final checks leave a material gap in cross-process recovery, native event delivery, or WebKit focus/reduced-motion behavior. Build release once after final code/docs and do not run the full suite.
