# Data Model: Lineage Knowledge Layer

## Overview

The model separates immutable source/audit records from the current presentation snapshot. Existing Capture, Problem, Solution, approval, conflict, Work Log, checklist, completion review, and human completion tables remain authoritative. Internal snapshot rows preserve audit safety and correction history but are not a user-facing document Version Control feature.

## Existing source entities

- **Capture**: `id`, original `text`, `created_at`.
- **Problem**: `id`, `capture_id`, refined `statement`, structured `detail`, `state`, `created_at`.
- **Solution (`features`)**: `id`, `problem_id`, `title`, `outcome`, `non_goals`, `validation_criteria`, `conflict_state`, `state`, `created_at`.
- **Completion sources**: Work Log entries/comments, checklist items, completions, completion reviews, Problem completion decisions, approvals, conflict reports/review runs, and AI runs. AI runs are private audit input and are never authoritative by themselves.

## New entities

### Solution Decision Event

Append-only record of a material change to a Solution.

| Field | Meaning |
| --- | --- |
| `id` | Stable event identifier |
| `feature_id` | Changed Solution |
| `event_type` | `created`, `refinement_applied`, `manual_edit`, `conflict_addressed`, `approved`, `completed` |
| `before_json` | Changed fields before the decision; empty for creation |
| `after_json` | Changed fields after the decision |
| `reason` | Explicit human reason or `Not explicitly recorded` |
| `provenance` | `observed` or `decided`; AI output alone cannot create a decided event |
| `source_type`, `source_id` | Approval, AI run, conflict address, progress entry, or direct human action |
| `created_at` | Event time |

Validation:

- Only changed Solution fields appear in before/after JSON.
- `decided` requires a human action or persisted workflow transition source.
- Updates append events in the same transaction as the live-field projection.

### Conflict Address

Structured treatment of a detected conflict.

| Field | Meaning |
| --- | --- |
| `id` | Address identifier |
| `feature_id` | Affected Solution |
| `conflict_report_id` | Earlier conflict report being addressed |
| `status` | `detected`, `addressed`, `unaddressed`, `unclear` |
| `basis` | `explicit_decision`, `implementation_evidence`, `ai_inferred` |
| `disposition` | `preserved`, `modified`, `superseded`, `rejected`, or null when not addressed |
| `summary` | Concise conflict context and treatment |
| `evidence_source_type`, `evidence_source_id` | Supporting pre-snapshot source record; required for Addressed |
| `created_at` | Address time |

Validation:

- `addressed` requires `explicit_decision` or `implementation_evidence`, a disposition, and evidence.
- `ai_inferred` cannot produce `addressed`; it remains `unclear` unless later confirmed.
- A later `clear` conflict report does not automatically address an earlier conflict.

### Lineage Snapshot

Immutable completion-time version of the lineage projection.

| Field | Meaning |
| --- | --- |
| `id` | Snapshot identifier |
| `feature_id` | Completed Solution |
| `version` | Monotonic version per Solution |
| `schema_version` | Projection schema version |
| `source_hash` | Digest of ordered source IDs, field hashes, and event versions |
| `status` | `building`, `ready`, `ready_without_inference`, `failed` |
| `document_json` | Ordered four-stage graph and section projection |
| `inference_error` | Provider/validation failure, if any |
| `created_at` | Snapshot time |

Rules:

- `(feature_id, version)` is unique.
- Same `feature_id + source_hash + schema_version` is idempotent.
- A ready snapshot is never overwritten internally; regeneration selects the newly built snapshot as current alongside the regenerated document. This internal history is not presented as document Version Control.
- `ready_without_inference` remains a valid complete lineage.

### Lineage Claim

A statement shown in a stage, transition, Decision Changes, Conflicts & Addresses, or Completion Evidence.

| Field | Meaning |
| --- | --- |
| `id` | Stable claim identifier |
| `snapshot_id` | Owning snapshot |
| `claim_key` | Stable semantic key used to carry corrections across regeneration |
| `section` | `stage`, `transition`, `decision_change`, `conflict`, `completion_evidence` |
| `subject_type`, `subject_id` | Related workflow subject |
| `classification` | `observed`, `decided`, `inferred` |
| `confidence` | `high`, `medium`, `low`, or null |
| `material` | Whether it appears in the primary graph |
| `created_at` | Claim creation time |

Validation:

- `inferred` requires confidence and one or more evidence references.
- `observed` and `decided` cannot have AI confidence.
- Claims without evidence become the literal absence marker rather than an assertion.

### Evidence Reference

Stable citation from a claim to a retained source.

| Field | Meaning |
| --- | --- |
| `id` | Evidence identifier |
| `claim_id` | Claim it supports |
| `source_type` | Capture, Problem, Solution field, decision event, approval, conflict report/address, Work Log, checklist, completion review, or completion decision |
| `source_id` | Original record identifier |
| `field_name` | Specific field where relevant |
| `excerpt` | Completion-time source excerpt |
| `source_hash` | Digest of the excerpt/source value |
| `captured_at` | Snapshot time |

Rules:

- The excerpt is immutable and remains readable if the live source disappears.
- Live navigation is offered only when the source still exists and is safe to expose.
- Private chat/AI-run text is excluded unless separately retained as an approved decision/evidence record.

### Interpretation Revision

Append-only current text for a claim.

| Field | Meaning |
| --- | --- |
| `id` | Revision identifier |
| `claim_id` | Revised claim |
| `supersedes_id` | Previous revision, null for first revision |
| `author_type` | `deterministic`, `ai`, `user` |
| `text` | Rendered interpretation |
| `reason` | Optional correction note |
| `is_current` | Exactly one current revision per claim |
| `created_at` | Revision time |

Rules:

- Saving a correction clears the previous `is_current` and inserts a new user revision atomically.
- Old revisions and evidence references are never deleted by correction.
- A correction cannot change immutable source excerpts or fabricate a stronger provenance class.

## Relationships

```text
Capture 1 ── 0..1 Problem 1 ── * Solution 1 ── 0..1 Completion
                            │
                            ├── * Solution Decision Event
                            ├── * Conflict Report ── * Conflict Address
                            └── * Lineage Snapshot ── * Lineage Claim
                                                       ├── * Evidence Reference
                                                       └── * Interpretation Revision
```

## Snapshot state transitions

```text
building ── deterministic assembly succeeds ──> ready_without_inference
    ├── deterministic assembly + valid requested inference ──> ready
    └── deterministic assembly fails ───────────────────────> failed
```

- Provider failure during inference-enabled regeneration produces a new `ready_without_inference` version and records `inference_error`.
- Reading never triggers generation.
- Regeneration preserves prior internal audit snapshots and never destroys source records, while the UI and completed document show only current Knowledge.

## Conflict lifecycle

```text
detected ── insufficient basis ──> unclear/unaddressed
    └── explicit decision or implementation evidence + disposition ──> addressed
```

`resolved` may be a display label only when an Addressed conflict's evidence shows the conflict no longer applies; persistence uses the stricter `addressed` state.

## Document projection

`document_json` contains:

1. `detail`: final Solution fields.
2. `lineage`: four stages and three transitions.
3. `decision_changes`: ordered material decision events and inferred interpretations.
4. `conflicts`: conflict events and addresses.
5. `completion_evidence`: criteria, Work Log evidence, completion review, and human decision.
6. `generation`: schema/source hash, status, inference error, and version.

The same projection feeds the browser and Markdown renderer to prevent semantic drift.

## Completed-work projection metadata

Extend the existing completed-work Playbook metadata with:

| Field | Meaning |
| --- | --- |
| `lineage_snapshot_id` | Exact snapshot used as the report input boundary |
| `lineage_version` | Snapshot version rendered into the document |
| `lineage_schema_version` | Projection schema understood by the renderer |
| `report_input_hash` | Digest of the ordered Lineage projection and referenced evidence excerpts sent to the report generator |
| `report_generation_status` | `generated`, `deterministic_fallback`, or `failed` |

The report input is derived only after current Lineage is ready. Final-report output is stored as projection/audit material but is never inserted as evidence into that snapshot. Regeneration rebuilds Lineage and the final document in the same lifecycle after external-file checks, while prior internal audit snapshots and interpretation revisions remain intact.
