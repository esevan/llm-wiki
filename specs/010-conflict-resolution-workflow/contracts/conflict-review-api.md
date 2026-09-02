# Contract: Conflict Review API

## Structured job result

The existing conflict-review job result retains its metadata and adds `status` plus `conflicts`. `findings` remains during compatibility transition.

```json
{
  "run_id": "review-run-id",
  "feature_id": "solution-id",
  "status": "conflicts_found",
  "recommended_state": "potential_conflict",
  "conflicts": [
    {
      "id": "conflict-1",
      "target_id": "decisions/adr-008.md",
      "target_title": "ADR-008",
      "severity": "high",
      "category": "Storage ownership",
      "summary": "The sources assign authority to different stores.",
      "current_claim": "Client-side state is authoritative.",
      "existing_claim": "Server state is authoritative.",
      "impact": "Concurrent clients can diverge.",
      "recommendation": "Keep server authority and revise the Solution.",
      "evidence": [{"citation": "decisions/adr-008.md:12-18", "excerpt": "...", "source_hash": "sha256", "start_line": 12, "end_line": 18}],
      "resolution": null
    }
  ]
}
```

Status values are `conflicts_found`, `clear`, and `insufficient_evidence`. Legacy reports with `findings` only are normalized to equivalent cards for display.

## Persist resolutions

`PUT /api/conflict-reviews/{run_id}/resolutions`

Request:

```json
{
  "resolutions": [{"conflict_id": "conflict-1", "action": "accept_conflict", "rationale": "Offline-first operation is an intentional exception."}]
}
```

Success `200` includes `run_id`, `feature_id`, aggregate `state`, resolved/unresolved counts, `requires_revision`, and saved resolution objects with timestamps.

Validation failures use `400` and do not partially persist or change the Solution gate. They cover non-ready or stale reviews; missing, duplicate, or unknown conflict IDs; invalid actions; blank acceptance rationale; and incomplete submissions. `404` indicates an unknown run.

## Read resolved review

Existing job-result and `GET /api/conflict-reviews/{run_id}` responses include saved `resolution` objects on each structured conflict so reopening after restart restores history.
