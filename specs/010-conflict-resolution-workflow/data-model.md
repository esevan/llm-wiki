# Data Model: Conflict Resolution Workflow

## Conflict Review Run (existing)

- `id`: unique review run identifier
- `feature_id`: owning Solution
- `status`: running, ready, failed, or cancelled
- `query`: hashes of the reviewed Solution context and Vault manifest
- `report_json`: backward-compatible structured result snapshot
- `created_at`, `updated_at`
- Relationship: owns zero or more Conflicts

## Conflict

- `id`: stable identifier unique within the review run
- `run_id`: owning Conflict Review Run
- `feature_id`: owning Solution
- `target_id`, `target_title`: source document or decision identity and readable title
- `severity`: `high`, `medium`, or `low`
- `category`, `summary`
- `current_claim`, `existing_claim`
- `impact`, `recommendation`
- `evidence_json`: ordered trusted citations, excerpts, hashes, and line ranges
- `created_at`

Validation:

- `(run_id, id)` is unique.
- Severity is normalized before insertion.
- IDs are deterministic per run when model output omits or duplicates them.
- Evidence citation metadata comes from retrieval, not free-form model output.

## Conflict Resolution

- `id`: unique resolution identifier
- `run_id`: owning Conflict Review Run
- `conflict_id`: resolved Conflict
- `feature_id`: owning Solution
- `action`: `apply_recommendation` or `accept_conflict`
- `rationale`: optional for apply; required and non-empty for accept
- `resolved_at`: resolution timestamp

Validation:

- `(run_id, conflict_id)` is unique so one conflict has one active resolution.
- Every submitted conflict ID exactly matches a persisted Conflict in the run.
- A complete submission contains exactly one resolution for every conflict.
- The entire set and review-level evaluation are committed atomically.

## Review Decision mapping

If all actions are `accept_conflict`, persist the resolutions, record a clear current conflict report referencing the run, add an addressed conflict record with explicit human-decision basis and preserved disposition, and set the Solution gate clear.

If any action is `apply_recommendation`, persist the resolutions, record a conflicted report referencing the run, add an unaddressed status describing required revision, and keep the Solution gate conflicted.

## Lifecycle

```text
review ready
    ↓ persisted conflicts
unresolved ── select action/comment ──> locally valid
    ↓ all conflicts valid and submit
resolved/accepted ──> clear gate + explicit address
resolved/revise  ──> conflicted gate + fresh review required
```

Changing the Solution, Problem, or Vault source invalidates the reviewed query and prevents submission against current state.
