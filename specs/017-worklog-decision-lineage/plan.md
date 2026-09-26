# Implementation Plan: Evidence-Grounded Work Distillation

**Branch**: `017-worklog-decision-lineage` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/017-worklog-decision-lineage/spec.md`

**Status**: Design complete; implementation and schema allocation are blocked until feature 015 is integrated.

## Summary

Generate one evidence-grounded structured Distillation from exact Task and Run sources. For Run-linked Work Log, treat the captured `final_report` as primary and consult completed evidence only for corroboration, material omissions, and contradictions. Feed the same typed claims into the active Task journey, where stable semantic nodes and evidence-backed links preserve decision evolution, epistemic state, completion snapshots, and current topic status. The shared feature-015 prompt/reference/job foundation owns common persistence and execution semantics; feature 017 owns source selection, semantic actions, Task projections, and user interfaces.

## Technical Context

**Language/Version**: TypeScript 5.9/React 19; Rust 2021/Tauri 2

**Primary Dependencies**: Existing React/Tauri/rusqlite/serde stack plus feature 015 `src-tauri/src/workflow_foundation.rs` prompt registry and `src-tauri/src/native/workflow_foundation.rs` immutable-reference, version-provenance, application-disposition, and durable-job helpers; no new dependency

**Storage**: Existing local SQLite. Feature-specific schema uses reserved migration 18 after capture migration 17. Its standalone SQL payload can be developed independently, while migration-chain integration waits for migration 17.

**Testing**: Focused Rust unit/integration tests for source selection, validation, projection, invalidation, batching, and preservation; Vitest/Testing Library for Work Log and Task journey states; `git diff --check`; targeted performance profile. Packaged E2E is deferred unless implementation leaves a concrete native lifecycle risk.

**Target Platform**: macOS and Windows desktop

**Project Type**: Local-first Tauri desktop application with React UI and one native application boundary

**Performance Goals**: Keep normal collapsed Task and Work Log projections within the constitutional 100 ms p95 after local data arrives; open saved node detail without a model call; coalesce related changes and bound model context to material candidate sources

**Constraints**: Final report is primary; exact sources per claim; no invented facts or causality; original records remain unchanged; stable semantic node IDs; targeted subgraph regeneration; durable retries and stale-result rejection; no automatic workflow transition; no legacy Solution-lineage authority

**Scale/Scope**: One Distillation projection per Run-linked Work Log entry and one current Task journey projection with retained revisions; batches may cover multiple changed sources for one Task

## Constitution Check

*GATE: Passed before research and after design, against constitution 3.1.0.*

- **You Talk. The Work Organizes Itself**: Distillation organizes recorded work without requiring the user to classify each event. It never creates or applies a user decision.
- **Reduce Cognitive Load**: The readable projection is primary; original detail remains secondary and inspectable. Only meaningful nodes appear.
- **Resume Where You Left Off**: reasons, failed approaches, exact conditions, results, verification state, unresolved topics, and completion snapshots remain available.
- **Tasks Carry Work; Problems Carry Context**: the active Task journey and Task Work Log are authoritative; legacy Solution lineage is not extended.
- **Private Process, Portable Knowledge**: results remain local process and do not publish Knowledge.
- **Human Authority over AI**: AI suggestions, user decisions, performed work, and verified results remain distinct; no Task state changes are automated.
- **Evidence and Logical Consistency**: every material claim and relationship requires exact source references. Stale or failed jobs cannot appear current.
- **Measured Performance**: normal projection and model-context budgets have explicit checks; no AI work runs on Task read or node click.
- **Independent Adapters**: feature code uses the feature-015 workflow foundation rather than provider-specific calls or a new job stack.
- **Local and Cross-Platform**: durable SQLite jobs and projections remain local, with platform-independent semantics.
- **Minimal Complexity**: existing Work Log, `capture_final_report`, Task journey, React components, and shared foundation are extended. No new dependency or duplicate lineage system is introduced.

## Product Spirit Assessment

The feature primarily serves “Reduce Cognitive Load” and “Resume Where You Left Off.” It replaces a raw-heavy default with a traceable account while preserving every original record needed for audit. Current decisions become easy to find without erasing old choices or turning AI inference into user intent. It does not weaken Capture, Task, Problem, completion, privacy, publication, or human-control boundaries.

## Source and Token Budgets

- A deterministic pre-pass selects the final report, completed evidence that corroborates or conflicts with it, and exact affected Task sources. It excludes streamed fragments and routine control events.
- Each claim carries a bounded evidence set. Large reports or evidence are chunked with stable locators; truncation is recorded and cannot silently imply completeness.
- Batch context contains only changed sources plus the current affected nodes and their direct links. Unrelated Task history is not resent.
- Opening a Work Log section or node detail is a local read. Model work runs only in durable background jobs.
- Profile the normal collapsed Task and Work Log reads. A regression above the constitutional 15% gate or beyond 100 ms p95 blocks completion.

## Architecture and Ownership

### Feature 015 owns

- Prompt registry identity and immutable prompt versions.
- Exact-revision source reference and versioned result-envelope contracts.
- Durable job enqueue, lease, retry, cancellation, stale completion, and idempotency semantics in `workflow_foundation.rs` and its shared persistence.
- Shared migration numbering and common job/reference/result metadata.

### Feature 017 owns

- Run and Task source selection, bounded evidence context, and contradiction flags.
- Structured Distillation schema beyond the shared result envelope.
- Claim validation and add/enrich/merge/omit/supersede semantics.
- Stable semantic node identity, topic current-state resolution, completion snapshots, and targeted subgraph invalidation.
- Work Log and Task journey projections, source navigation, freshness/error UI, and prepared detail.
- Any feature-specific tables/indexes after foundation integration and migration allocation.

### Processing flow

1. A Run terminal transition schedules `run_report_distillation`, the sole Run-linked Work Log Distillation operation. The same Run MUST NOT also enqueue `work_log_distillation`; that operation is reserved for non-Run/manual Work Log scope outside this feature.
2. The feature-017 source selector builds an immutable input manifest. For a Run, it selects `final_report` first and then relevant completed evidence.
3. The prompt registry resolves the named prompt version; the provider returns the structured feature result inside the shared versioned envelope.
4. A strict validator rejects uncited claims, missing sources, illegal status upgrades, unknown action targets, and malformed relationships.
5. The projector compares the valid candidate result to the expected prior revision, applies the Work Log view, then submits the same validated structured claims to `task_journey_increment` with the affected Task source bundle. The journey operation updates only the affected subgraph and does not redistill the Run.
6. Reads return the last-good projection plus freshness state. Original sources are fetched separately on demand.

The foundation leaves the application disposition pending until the feature consumer performs its compare-and-publish transaction. A validated projection may appear immediately after that transaction records `applied`; this means the exact-source projection was persisted, not that a person adopted any suggestion or source decision. Publishing the projection does not mutate source records or gate ordinary Task work.

## Incremental Invalidation

- Maintain a reverse dependency from exact source revision to claims and relationships.
- An edit or deletion invalidates only dependent claims and links; downstream topic/current-state summaries are recomputed from surviving nodes.
- Candidate changes receive one of `add`, `enrich`, `merge`, `omit`, or `supersede`. Only `add` and `supersede` create a new semantic node; `enrich` adds supported detail without changing identity; `merge` creates an explicit redirect and combines nonconflicting evidence; `omit` changes no published semantic content.
- A meaningful new decision always uses a new stable node and an evidence-backed relationship. Returning to an older option for a new reason is also a new decision node.
- Full regeneration is reserved for repair, incompatible schema/result version, or rules-version change and still attempts semantic ID reconciliation.

## UI Design Direction

- **Primary task**: understand the current supported result or decision and resume work.
- **Hierarchy**: readable Distillation/current decision first; unresolved and freshness status second; exact sources and raw/original records in expandable secondary areas.
- **Visual characteristic**: a quiet document-like reading surface using existing cream/white tokens, with compact provenance/status chips and a clear supersession path rather than a dense card wall.
- Work Log shows sections for outcome, decisions/reasons, performed work, checks/results, failed approaches, and unresolved items only when populated. “Original report and evidence” expands separately.
- Task journey emphasizes adopted and unresolved topic nodes. Superseded/withdrawn nodes remain on the path with lower visual weight. Node detail is a keyboard-safe modal/drawer populated entirely from saved detail.
- All status meaning is textual as well as colored; long Korean/English text, numbers, and paths wrap safely at wide and 640 px widths. Focus returns to the invoking node when detail closes.

Rendered review is required during implementation for wide/narrow, Korean/English, long content, empty, pending, stale, failed, contradicted, superseded, withdrawn, unresolved, completion-snapshot, keyboard, and source-navigation states. No rendered review is claimed in this design-only turn.

## Project Structure

```text
specs/017-worklog-decision-lineage/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── tasks.md
├── contracts/{distillation-result.md,application-api.md}
└── checklists/{requirements.md,distillation.md}

src-tauri/src/application/
└── task_distillation_service.rs           # feature 017
src-tauri/src/native/
├── workflow_foundation.rs                 # feature 015 persistence helpers
├── task_execution_runtime.rs              # terminal scheduling hook
├── task_journey.rs                        # active Task projection integration
├── task_distillation.rs                   # feature projection persistence
└── migrations.rs                          # allocation only after 015 merge
frontend/src/features/workbench/
├── TaskDetail.tsx
├── TaskJourneyGraph.tsx
├── WorkLogDistillation.tsx
└── task-workbench.css
frontend/src/services/{taskClient.ts,taskOperation.ts}
frontend/src/types/taskWorkbench.ts
```

**Structure Decision**: Keep the feature in the existing Task vertical slice. A new application service owns semantic validation and projection; shared job infrastructure stays in feature 015. UI components render read models and never call a model directly.

## Complexity Tracking

No constitution violation requires justification. The separate current projection and retained projection revisions are necessary for last-good reads, stale-result rejection, and inspectable repair; they do not duplicate source records.

## Integration Gate

Before implementation starts:

1. Merge feature 015.
2. Map the shared `run_report_distillation` and `task_journey_increment` policies, `sourceRevision`, prompt identity/version, source attribution, execution outcome, application disposition, and stale helper against [contracts/distillation-result.md](contracts/distillation-result.md).
3. Update file paths and data-model mappings if foundation naming differs.
4. Re-run `$speckit-analyze`; do not author migrations while this gate is open.
