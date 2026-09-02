# Conflict Review API Contract

- `POST /api/features/{feature_id}/conflict-review` returns `202` with a run snapshot; unchanged completed inputs may return a reused ready snapshot.
- `GET /api/conflict-reviews/{run_id}` returns state meanings, scope/coverage, counts, phase/progress, partial findings, timings, and cache provenance.
- `DELETE /api/conflict-reviews/{run_id}` records cancellation and returns the cancelled snapshot; no later provider call may begin.

States: `reviewing` means candidates remain; `potential_conflict` means evidence-backed findings exist; `no_conflict_found` means none found yet but clear is unjustified; `clear` means complete adequate review; `insufficient_evidence` means coverage/candidates/output/citations cannot support clear; `cancelled` and `failed` are terminal without a recommendation.
