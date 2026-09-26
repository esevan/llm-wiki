# Knowledge workflow execution program

Status: active implementation program, not a shipped-feature declaration.
User authorization: 2026-09-26, following the agreed 17 work packages.

## Outcome and authority

Improve Knowledge quality, archival organization, and later retrieval. Preview structures work;
Knowledge structures reusable outcomes. Exploration may diverge; published conclusions remain
evidence-grounded. AI may prepare a useful evidence-backed preview without repeated questions;
draft preparation does not imply user adoption, execution, verification, or finality. Completion
and publication remain separate user actions. External personal Vault connection is excluded.

## Requirements ledger

| Work | Contract and acceptance boundary | Owner |
|---|---|---|
| 1 | Prompt registry objects: id/version, input/output, builder/validation; retrieval needed/reason/queries/aspects/filters/requery; execution policy separate | A, feature consumers |
| 2 | Immediate raw Capture persistence, one automatic background text/image cleanup; no workflow-stage/count/version-list inflation; retry retains source | B |
| 3 | Shared content-application transition for cleanup, preview adoption/auto apply, refinement, restore; erase title right-to-left then reveal left-to-right; stable order/focus/input; reduced motion | B, E, F |
| 4 | Workflow/job/content/UI state separation, execution vs application status, CAS/coalescing/recovery; durable essential jobs vs latest-only optional investigations | A, consumers |
| 5 | Work description/background/goals/scope/exclusions/constraints/completion/approach; retrieve first when needed, then complete preview; show assumptions and provenance, do not demand routine clarification | E |
| 6 | Preview and Knowledge draft version picker, meaningful revisions, read vs apply, compare, restore as new, source snapshots, draft vs published; no rollback of actual logs/decisions | A storage, E/F UI |
| 7 | Applicability/decision/content/exploration vectors; model/version/dimension/hash and exact document section/version; independent candidate sets; incremental indexing | D |
| 8 | Applicability-weighted hybrid ranking, lower idea priority, dedupe, superseded history redirects to final decision with reason; validate current applicability/conflicts | D |
| 9 | Conversation continues during background follow-up retrieval/conflict comparison; freshness check; critical findings surfaced, supporting evidence in next answer, ancillary results in references; no endless optional retry | E |
| 10 | Many-document searchable/filterable/sortable reference list; no sudden reordering; hyperlinks to Markdown modal with section, history/back/version; @ and multi-document/section mentions without auto-send; viewed/used/adopted distinct | E |
| 11 | Meaningful incremental journey; omit routine retry/continue requests; add/enrich/merge/omit/supersede based on changes, stable IDs, affected subgraph invalidation; rich precomputed detail; final decisions per topic | C |
| 12 | Standalone evidence-grounded Knowledge with final conclusions, context, conditions/limits, applicability, sources; type-appropriate structure, no invented decision; exact current snapshot; reviewed body published unchanged | F |
| 13 | Separate idea/ Markdown, not exploration in final article; valuable idea units with deferred/unverified/rejected distinctions, reconsideration conditions/source links; promotion only with later evidence | F/G |
| 14 | Propose new/update/merge/conflict/supersede, existing taxonomy first, one canonical path, tags/aliases/MOCs; reviewed writes, actual-use wikilinks and rationale, managed MOC regions; repair links on move/withdraw | G |
| 15 | Stable IDs/versioned sources, source-preserving migration, external-edit guards, recoverable file/index partial failure; portable Markdown/YAML, rebuildable index; mixed-content metadata independent of folder | A/D/G |
| 16 | Evidence fidelity and search relevance fixtures, timing measurements, stale/concurrency/restart/migration checks, rendered UI/accessibility/EN-KO/reduced motion, targeted E2E, paired docs | All |
| 17 | Run final report as main Distillation input; raw evidence corroborates omissions/conflicts/checks; preserve material conditions/failures/IDs; no invented facts/reasons; citations, absent-report handling, readable default with inspectable originals | C |

## Specs and dependencies

Actual directories are explicitly allocated to prevent parallel numbering collisions.

| Key | Directory | Prerequisites | Status |
|---|---|---|---|
| A | 015-workflow-foundation | none | integrated; registry/policy checks passed |
| B | 016-capture-auto-distillation | A implementation | integrated; once-only cleanup and modal/card state verified; final visual/lifecycle assessment pending |
| C | 017-worklog-decision-lineage | A implementation | integrated; grounded incremental native and readable UI checks passed; final coverage reconciliation pending |
| D | 018-multiaspect-knowledge-retrieval | A and C decision contracts | integrated; persistence, ranking and measured indexing checks passed |
| E | 019-reference-aware-workbench | B, D, version/reference contracts | integrated; high-volume list, batch mentions and exact provenance checks passed |
| F | 020-knowledge-idea-distillation | C, reference provenance contract | integrated; grounded manual completion, immutable review and adoption transitions verified |
| G | 021-organized-knowledge-archive | D, F, references | native journal/index/recovery and review UI integrated; focused archive 17 tests passed; selected final lifecycle check pending |

Each feature follows specify, clarification assessment, checklist, plan, tasks, analyze, implement,
and convergence. Clarifications already settled in the conversation must not be asked again.
Plans must resolve current code paths rather than treating old Solution lineage as current Task code.
No task is complete merely because a contract or unused helper exists: consumers and observable
acceptance behavior must be wired and tested in the owning feature.

## Execution waves and integration

1. Amend conflicting governance to reflect authorized initial cleanup, preview preparation, deliberate
   idea publication, Distillation, and incremental history while preserving human decision authority.
2. A: registry, operation policy, freshness/version/reference/decision contracts. Reuse canonical
   storage and queue rather than creating duplicate sources of truth.
3. B and C independently implement after A integration. Shared migration numbers assigned centrally.
4. D starts with stable decision/reference contracts; search uses fixtures/old documents without
   waiting for the new generation pipeline.
5. E joins Capture, search, preview/version UI, background investigation, references/mentions.
6. F generates final Knowledge and separate idea drafts with version UI and exact evidence snapshots.
7. G connects organization, reviewed publication, MOC/wikilinks, index update, future reuse.
8. Main integration and selected remaining heavy validation. Fixes use main-derived worktrees.

The primary repository started clean on main at 0a7da6a. Integration worktree is
`.worktrees/knowledge-workflow-integration` on `feat/knowledge-workflow-integration`.
Feature worktrees use the required creation script and shared cache rules. Three child slots maximum
at current capacity. Every child has a bounded scope, explicit model/effort, isolated branch/worktree,
and no redundant investigation. Model escalation is evidence-driven, up to ultra if needed.
No primary application/test edits. Shared schema/core modules have an assigned owner. Integrate
finished branches before starting dependent implementations; do not use partially written files as
a released contract. Keep shared Cargo target builds serialized when concurrent compilation would
cause lock contention or benchmark noise.

## Runtime policy and invariants

- Durable important work: initial Capture cleanup, Run Distillation, incremental journey, publication
  indexing. Limited transient retries and crash recovery; bundle successive changes.
- User-requested generation: visible status/result/failure and explicit retry; necessary retrieval
  belongs to the requested job. Do not silently drop a direct investigation request.
- Optional speculative retrieval: latest-wins, cancel stale work, no required durable retry; revisit
  on the next relevant request. Relevant prior results may be cached with source/version bounds.
- A job success is not necessarily application success. Exact input versions guard every write;
  raw sources and user edits win over late generated results. UI subscribes to canonical status.
- Initial cleanup's internal revision is not a user-visible draft history item. Modal opening never
  creates another cleanup operation. A completed title transition does not determine job correctness.
- Published Knowledge prioritizes final decisions. AI suggestions, adopted choices, performed actions,
  and verified results remain separate. Search matching a rejected choice returns its final successor.
- Idea weighting uses type/state, not only `idea/` path. Future mixed Vault sections may carry different
  types and explicit-vs-inferred provenance; actual external Vault integration remains out of scope.
- References preserve exact ID/version/section and why used; a candidate, click, or mention is not
  adoption. Only actual reasoning/reference use feeds published links (including useful counterevidence).
- Journey details are prepared on update, not generated on click. Actual decision changes append and
  supersede; same-work facts enrich. Raw corrections propagate only through affected dependencies.

## Acceptance scenario and quality checks

Contract-change request: immediate Capture; evidence-backed preview proposes explicit approval rather
than receipt, with a stated assumption. Existing comparison-table guidance becomes a source link.
While conversation continues, customer count/approver changes trigger focused fresh investigation.
The user can open a reference in a Markdown modal, mention its section, compare and restore drafts.
Run records say sent/receipt-only/pending approval/direct follow-up; Distillation must not invent
approval or claim direct contact is universally better. Routine retry messages are not decision
nodes. A final approval snapshot produces final Knowledge, while untested reminder automation becomes
a separate idea. Reviewed publication updates MOC and source wikilinks. A future similar initial
choice finds the final approval decision; reminder ideas appear as lower-priority unverified advice.

Validate important-information omissions and unsupported additions separately. Benchmark capture,
projection, indexing, retrieval, and model stages without making unmeasured latency promises.
Focused unit/integration/type/whitespace checks run before merges; known defects are not deferred to
main. Heavy testing concentrates on final integrated main per user preference. E2E remains selected
by concrete residual risk, not automatically full-suite. Packaged E2E requires final release build
only when necessary. Installing/releasing is not implied by merging main. Update EN/KO feature docs
as behavior ships and record checks not available. Use commit-convention for every commit and the
single-commit rule when preparing any authorized remote PR push.

## Known current-code entry points (reconfirm narrowly)

- `native/task_assistance.rs`: work preview proposals and Knowledge draft/publication; keep distinct.
- `application/task_execution_service.rs`: final-answer extraction into per-Run final_report and log
  sync. `native/task_execution_runtime.rs` already requests a concise final report.
- `application/task_service.rs`: Work Log projections include report excerpt and raw evidence;
  display truncation must not become the only Distillation source.
- `TaskWorkSessions.tsx`: existing report Markdown display/deduplication.
- Current Task journey already supports exact draft snapshots and evidence-quoted interpreted links.
  Legacy `native/lineage.rs` must not be assumed to own the current Task journey.
- Existing queue/search/publication paths have evolved since earlier planning. Inspect current
  canonical adapters and latest migration versions before extending them.

## Progress ledger — 2026-09-26, main acceptance

Implementation is merged into local main through db4ada7. Work packages 1–15 and
17 are integrated; work 16 automated acceptance has passed the selected packaged checks; the
rendered state matrix and extended profiling/permutation coverage remain open. No release installation or push occurred.

| Work packages | Current status | Remaining acceptance |
|---|---|---|
| 1–8 | Registry, Capture cleanup, shared application transitions, jobs, automatic previews, versions and multiaspect search integrated | Selected native Capture/refinement lifecycle checks passed |
| 9–10 | Background investigation, grouped/paged references, batch/atomic mentions and exact Markdown sources integrated | Latest rendered list/accessibility matrix unavailable on locked workstation |
| 11, 17 | Grounded incremental decisions and final-report-first Work Log Distillation integrated | Full transient rendered matrix not claimed |
| 12–15 | Grounded Knowledge, separate ideas, exact reviewed artifacts, taxonomy/MOC and recoverable archive integrated | Selected packaged publication check passed |
| 16 | Frontend, native, migration, performance and documentation acceptance active | Locked-Mac visual matrix and extended profiling/permutation coverage |

Confirmed results:

- Main frontend suite: 375 tests plus provider, desktop helper, signing and boundary
  checks passed. Subsequent session-effect fix passed 33 focused tests, lint and
  typecheck. Latest reference/Knowledge changes passed 87 focused tests.
- Sequential native unit suite: 252 passed, one authenticated live test intentionally
  ignored. Latest exact reference provenance passed 17 focused tests. Capture-adopted
  references reach Knowledge and archive only with exact actual-use/counterevidence
  authority; candidate/viewed/excluded and unrelated Task references stay excluded.
- Main application command integration: 13 passed, including real HTTP generation,
  immutable edits, exact reviewed file bytes, index receipt and external-edit guard.
  Legacy recovery, stdio/context MCP, projection, Task v8, concurrency and all six
  remaining work-tracking suites passed. Historical
  migration fixtures now accurately replay through schema head 22; both affected
  binaries passed (3 and 2 tests).
- Independent 1,000-reference UI fixture, 25 samples: initial page p95 6.84 ms;
  filter/sort commit p95 12.59 ms. This is mounted jsdom measurement, not visual QA.
  Isolated 1,000-note index measured 1.699 s; warm lexical search 11 ms. Existing
  performance budgets were not relaxed to hide shared-load failures.
- Earlier English wide/Korean narrow visual evidence exists for portions of the UI.
  Latest list/artifact and transient accessibility/reduced-motion review remains
  unavailable while the Mac is locked. Mounted tests do not substitute for it.

Run only task-capture, task-refinement and task-publication packaged scenarios after
one final release build: native background application, modal/version/adoption and
file/index boundaries warrant this coverage. Full E2E is not warranted. Follow-up
fixes remain in dedicated worktrees and are integrated to main before rechecking.

Final automated receipts:

- Archive overlap is explicitly capped at eight exact passages and 6,000 estimated
  tokens, without full viewer bodies. Changed archive suite passed 16 tests.
- Knowledge validator passed 14 tests including all four idea dispositions. The
  missing-report factual fallback was already covered; no duplicate test was added.
- Knowledge Review passed 13 tests, with 25 independent 40-revision profiling
  cycles: projection p95 18.67 ms and selection p95 17.31 ms.
- Signed release build succeeded. Selected packaged scenarios passed: Capture
  (`.tmp/desktop-e2e-artifacts-2ZohfQ`), publication (`qZOCVD` under the same
  artifact prefix), refinement (`i2GYOb`). Publication verifies corrected immutable
  bytes, exact index receipt, reviewed withdrawal and private history; refinement
  verifies images/chat, editor/save and adoption by stable Task ID.
- Harness corrections removed obsolete Problem setup/text matching and await
  actual editor renders. The first refinement timeout was not reproduced; later
  failures were diagnosed harness errors and all retained assertions now pass.
- No full-suite E2E: the three selected flows address the identified native risks.
  No install or push. Mac remains locked; manual latest visual acceptance is not
  claimed. Broader generation/archive stage timings and expanded retry/selected
  idea publication permutations remain unverified, as recorded in Spec Kit tasks.
