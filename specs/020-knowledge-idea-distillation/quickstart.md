# Quickstart: Validate final Knowledge and separate idea drafts

## Prerequisites

- Feature 015 prompt/job/reference/version contracts are integrated.
- Feature 017 Distillation and incremental journey are integrated and expose current freshness plus exact completion snapshots.
- Feature 018 metadata compatibility and feature 021 exact-body handoff are agreed.
- Migration 21 (`F21`) is assigned after E20; feature 017 result schema v2 is present on the final integration branch.

## Focused commands

Run each command separately after implementation:

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Focused native verification on 2026-09-26:

- `cargo test --manifest-path src-tauri/Cargo.toml knowledge_distillation -- --nocapture`: 9 passed after typed-authority hardening.
- `cargo test --manifest-path src-tauri/Cargo.toml knowledge_provider_dispatch_keeps_full_sources_and_nested_contract -- --nocapture`: 1 passed through the actual loopback provider dispatch.
- `cargo test --manifest-path src-tauri/Cargo.toml knowledge_migration -- --nocapture`: 2 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml knowledge_prompt_exposes_the_complete_nested_contract_and_full_sources -- --nocapture`: 1 passed.
- `git diff --check`: passed for the migration and native runtime checkpoints.
- `npx vitest run frontend/src/components/ContentRevisionTransition.test.tsx frontend/src/features/workbench/KnowledgeReviewPanel.test.tsx frontend/src/features/workbench/TaskDetail.test.tsx`: 66 passed after the review hardening, covering exact Markdown artifact display, byte measurement, readable MOC/reference changes, conflict and dirty publication locks, newest recoverable-operation hydration, retry/status completion, regeneration, unsaved-close focus return, and semantic new-revision transitions.
- The earlier focused application boundary set passed 91 tests covering the mounted review, guarded edit, archive organization/recovery, application routing, and shell binding.
- `npm run typecheck`: passed with the integrated feature 019 shared reference/version component contract.

Task Review UI timing and full provider-stage profiling remain integration work. The rendered review evidence below is separate from native and component checks.

## Scenario 1: final approval leads, history remains traceable

Use `receipt-is-not-approval` plus `historical-choice-to-current-decision` from `specs/knowledge-workflow-acceptance.json`.

- Final Knowledge leads with explicit approval as the current condition/outcome for its topic.
- Receipt-only remains historical and traceable, not current advice.
- Sent, receipt, missing approval, follow-up, conditions, and limits retain exact sources.
- No claim says approval was obtained or direct contact was effective without supporting evidence.
- Applicability includes use situations, representative questions, help, conditions, and exclusions without exceeding the body.

## Scenario 2: useful exploration stays separate

Use `deferred-idea-not-general-lesson`.

- The final body contains no reminder-automation exploration.
- A separate optional idea exists only if reusable.
- Its exact disposition and reconsideration conditions are visible; `out_of_scope` is never displayed merely as `deferred`.
- It links to the Task, exact sources, and related final Knowledge.
- It does not claim direct contact is better or automation works.

## Scenario 3: report does not override evidence

Use `report-is-not-verification` and `missing-final-report`.

- Contradictory failed command evidence blocks verified/successful language.
- A missing final report remains explicit and known artifact creation is retained.
- Unsupported-addition and material-omission findings remain distinguishable.
- Provider failure or blocked validation leaves existing versions usable.

## Scenario 4: stale generation cannot replace current evidence

Start generation from snapshot S1, then append evidence that advances Distillation/journey to S2 before save.

- Generation initially waits until feature 017 reports current; bounded transient retries may occur under shared policy.
- The save-time manifest check rejects S1.
- No Knowledge or idea revision is appended from S1.
- Last-good remains visible as stale; current private/published pointers stay unchanged.
- Retry from S2 records its exact snapshot.

## Scenario 5: versions, compare, and restore-as-new

Use `restore-draft-not-work-history` with versions 1–3 and restore version 1.

- Version selector identifies current private and published revisions separately.
- Comparison reads exact selected versions without generation.
- Restoration appends version 4 derived from version 1.
- Versions 1–3, published pointer, Task logs, decisions, Distillation, and journey remain unchanged.
- Source and assumption references for version 4 match restored provenance.

## Scenario 6: reviewed body is the published body

Edit a private draft to create a new immutable revision, review it, and publish that exact revision through a feature 021 substitute.

- Publication performs no provider call.
- Handoff bytes/hash match the reviewed revision.
- Successful return records feature 021's stable ID and actual-content hash.
- A simulated feature 021 failure changes neither private nor published pointer.
- Declared `source_revision` remains provenance; actual file content hash is authoritative.

## UI review

After implementation, inspect wide and narrow Task Review views in English and Korean with long real-level content. Verify primary article/state hierarchy, side-by-side then stacked comparison, disclosure content, empty/error/loading/success states, exact idea disposition labels, visible focus, keyboard order, and reduced-motion behavior. Run a selected packaged scenario only if a concrete Tauri publication/restore risk remains after component and Rust integration coverage; otherwise record the E2E skip and residual risk.

The focused component and native boundary tests cover immutable edit/restore requests, exact proposal hashes and Markdown bytes, source guards, index-pending polling, restart recovery, organization review, and explicit recovery callbacks. A local fixture was rendered at wide English and 620 px Korean layouts with long final outcomes, applicability limits, separate ideas, and a repair-required publication. Keyboard Tab/Enter opened the exact source modal, focus landed on its close control, and no clipping was observed. The later exact publication-artifact modal and regenerated-title/body transition are covered by mounted component tests; a second native visual pass could not run because the macOS session locked.

Packaged E2E remains skipped in this feature worktree because the shared reference components and feature 021 native transaction are integrated by the parent branch. The final integrated main build owns the smallest publication/restart packaged scenario. Windows, real assistive technology, OS reduced-motion settings, and the remaining empty/loading/conflict/error visual fixtures were not manually rendered here.


### Integrated native contract reconciliation

The first full main run exposed obsolete expectations for schema heads, legacy
fallback journey creation, prepared Knowledge shape, prompt version and semantic
health fields. Tests now exercise the current typed source manifest and current
unit embeddings. The queued finalizer additionally rejects a prepared Task or
revision different from its requested subject, without writing a version or
completing the job.

Focused rechecks passed: 38 Knowledge/archive-related tests, 27 migration tests,
registry semantics and bundled offline semantic search. The prior parallel-run
3-second index and 5-second child-burst deadlines failed under shared load, then
passed unchanged in isolated runs: structural indexing 1.699 s, warm lexical search
11.000 ms; child burst test 2.21 s. Final broad native checks use a single test
thread so resource-sensitive checks measure the feature rather than competing
benchmark/test processes. No timing threshold was relaxed.

### Integrated review timing receipt

The mounted Knowledge Review fixture exercises 40 immutable revisions with a
40-paragraph article, across 25 independent mounts and historical selections.
Measured nearest-rank p95: projection 18.67 ms and version selection 17.31 ms.
`KnowledgeReviewPanel.test.tsx` passed 13 tests. Timing uses real performance.now
around React commits and validates the selected exact historical body. This is
jsdom UI timing with a resolved local projection, not provider latency, native
IPC measurement, visual acceptance, or a new promised latency budget. Full
generation-stage profiling remains separately unverified.

### Final selected packaged receipt

`task-publication` passed on signed main `3b07837`; artifacts: `.tmp/desktop-e2e-artifacts-qZOCVD`. Generated content was corrected as a new immutable version, reviewed and published with exact artifact hash/index receipt, withdrawn through reviewed controls, and reopened with private history preserved.

The three selected flows address concrete native background, modal/adoption and
file/index risks; the full suite was not needed. Later source changes were
refinement-harness-only. The installed app and personal Vault were not changed.
Latest direct visual review remains unavailable on the locked Mac; mounted or
packaged assertions do not substitute for that visual state matrix.
