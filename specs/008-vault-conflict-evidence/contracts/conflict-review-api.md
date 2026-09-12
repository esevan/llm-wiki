> **Retired for schema 8+**: The feature-only route is replaced by exact Capture-draft/Task subjects,
> eight statuses, and nonblocking behavior in [012](../../012-task-centered-workbench/contracts/application-api.md).

# Conflict Review API Contract

- `POST /api/features/{feature_id}/conflict-review` returns `202` with a run snapshot; unchanged completed inputs may return a reused ready snapshot.
- `GET /api/conflict-reviews/{run_id}` returns state meanings, scope/coverage, counts, phase/progress, partial findings, timings, and cache provenance.
- `DELETE /api/conflict-reviews/{run_id}` records cancellation and returns the cancelled snapshot; no later provider call may begin.

States: `reviewing` means candidates remain; `potential_conflict` means evidence-backed findings exist; `no_conflict_found` means none found yet but clear is unjustified; `clear` means complete adequate review; `insufficient_evidence` means coverage/candidates/output/citations cannot support clear; `cancelled` and `failed` are terminal without a recommendation.
