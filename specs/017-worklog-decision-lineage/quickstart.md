# Quickstart: Evidence-Grounded Work Distillation

This is a future implementation validation guide. The design turn does not claim these checks have run.

## Prerequisite gate

1. Feature 015 is merged and its prompt/reference/result/job contracts are mapped in the plan.
2. The feature-specific migration allocation is recorded.
3. Focused fixtures include exact source revisions and expected semantic results.

## Focused validation commands

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

During implementation, prefer the smallest focused test filters while iterating, then run the stable commands once after final code and documentation changes.

## Scenario A: final report is primary

Create a successful Run fixture whose final report states the goal, changed path, decision reason, one failed approach, and an unresolved item. Add completed evidence that corroborates the changed path and check, plus routine control events.

Expected:

- The readable Work Log follows the final report's meaningful structure.
- Completed evidence supplies or corroborates checks/paths without becoming an activity feed.
- Routine controls create no claims or nodes.
- Every displayed claim links to an exact source revision.
- Original report and evidence remain byte-for-byte unchanged and inspectable.

## Scenario B: report contradiction and verification boundary

State “all checks passed” in the final report but provide a completed check item with nonzero exit and a normal provider exit.

Expected:

- The claim is contradicted, not verified.
- Normal exit is not proof of correctness.
- Both sources are accessible.
- No unsupported reconciliation reason is invented.

## Scenario C: failed/interrupted Run without report

Create failed and interrupted Runs with no final report, one observed file change, and uncertain terminal outcome.

Expected:

- Only the saved file-change and status facts appear.
- Missing report and uncertain outcome are explicit.
- No success, completion, decision reason, or verification claim appears.

## Scenario D: decision lineage

Supply an AI suggestion, explicit user adoption, performed work, observed verification, a later explicit replacement for a supported reason, an explicit withdrawal on another topic, and one unresolved choice.

Expected:

- Suggestion, decision, performed, observed, and verified remain distinct.
- The replacement is a new node linked to the historical node.
- Topic states are adopted, superseded, withdrawn, and unresolved as recorded.
- The last chronological message cannot override those states by itself.
- Detail opens locally with before/after/reason/evidence/result/status/links.

## Scenario E: completion snapshots

Complete, reopen, and complete the same Task with different evidence.

Expected:

- Two immutable snapshots remain.
- The first is historical after reopen; the second is current.
- Neither completion is inferred from a Run success.

## Scenario F: targeted edit/delete propagation

Generate a Task projection with two unrelated decision topics. Edit one exact source, then delete a source that only supports one relationship.

Expected:

- Only dependent claims, links, prepared detail, and topic state are reconsidered.
- Unrelated node IDs and content are identical.
- Prior projection revisions remain inspectable.

## Scenario G: durable batch, replay, and stale completion

Queue several related changes, deliver the same batch 100 times, and complete an older attempt after a newer source revision is current.

Expected:

- One current projection with no duplicates.
- Atomic publication; no partially changed decision state.
- Older completion is diagnostic/stale and cannot replace current.
- Failed retry leaves last-good content visible and never reruns Codex.

## Scenario H: UI and accessibility

Inspect wide and 640 px windows in English and Korean with long paths, numbers, empty, pending, stale, retryable failure, contradiction, superseded, withdrawn, unresolved, and completion states. Traverse Work Log, source links, journey nodes, detail, and close/return focus with keyboard only.

Expected:

- One clear reading focus, readable hierarchy, no overflow, and no dense equal-weight card wall.
- Status is conveyed with text and accessible names, not color alone.
- Focus is visible and returns to the invoking node.
- Opening detail makes no model request.

## Performance check

Profile collapsed Task and Work Log reads with representative long Distillation results. Record p50/p95 and compare with the baseline. Block completion if the projection exceeds 100 ms p95 after local data arrives or regresses a binding budget by more than 15% without governance change.

## E2E decision

No E2E is required for the specification-only turn. During implementation, focused native and component tests cover most semantic and UI behavior. Run one smallest packaged scenario only if a concrete remaining risk crosses the native boundary, such as restart persistence of a leased batch or deep-link focus after native reconnect. Do not run the full suite by default.

## Implementation verification — 2026-09-26

- Grounding regression slice: `cargo test --manifest-path src-tauri/Cargo.toml --lib knowledge_distillation -- --test-threads=1` passed 14 tests, including failed/interrupted no-report factual fallback and all four Knowledge idea dispositions with metadata-preservation and final-article leakage checks.
- Native `cargo test --manifest-path src-tauri/Cargo.toml --lib distillation`: 32 passed, including source authority, exact quotes, live source/worker CAS, enrichment, completion status and exact Run repair.
- Focused TaskDetail, TaskJourneyGraph, WorkLogDistillation and source-control inventory: 70 passed, including pending and explicit repair state coverage. TypeScript check passed.
- An early broad frontend run exposed stale expectations for collapsed originals, localized statuses and missing control mappings; these were corrected and the affected suite rerun.
- Native E2E is not needed for deterministic authority/transaction checks. Combined UI/native lifecycle and rendered English/Korean wide/narrow review remain scheduled for final integrated validation; no release build or visual acceptance is claimed here.

### Performance and rendered review

The focused frontend suite now profiles 20 independent representative long collapsed Work Log and
Task journey render/open cycles, including opening prepared node detail from saved data. Both p95 assertions remain below
the 100 ms local projection budget; the tests do not make a model request. The existing session
projection profile also remains below the same budget. These checks are deterministic render
budgets rather than a packaged-device benchmark, so no cross-platform performance claim is made.

Reviewed the shipping components through an isolated Vite fixture in the in-app browser: English wide detail and Korean 640 px overview/detail, long statements, stale view, adopted/superseded states, exact source text, and Escape focus return. Screenshots are in `.tmp/ui-review/work-distillation/`. This is representative evidence only; the full T038 state matrix remains outstanding. Review exposed equal visual weight for replaced and adopted decisions; explicit status badges and differentiated borders/surfaces now distinguish them. Native packaged lifecycle remains a separate final integration risk for T039.
