# API Contract: Lineage Knowledge Layer

All paths are under `/api`. Error bodies follow the application's existing HTTP error convention.

## GET `/features/{feature_id}/lineage`

Return the current completed-Solution lineage snapshot. This is a read-only, provider-free path.

### Response `200`

```json
{
  "snapshot_id": "uuid",
  "feature_id": "uuid",
  "version": 1,
  "status": "ready",
  "stale": false,
  "generation": {
    "schema_version": 1,
    "created_at": "2026-08-21 16:10:00",
    "inference_error": ""
  },
  "detail": {
    "title": "Refinement Preview status",
    "outcome": "Users can see preview generation progress and errors.",
    "non_goals": "Do not add Preview to Capture.",
    "validation_criteria": ["Preview generation state is visible"]
  },
  "lineage": {
    "stages": [
      {"kind": "capture", "record_id": "uuid", "title": "Capture", "summary": "Original user feedback excerpt", "claims": ["claim-capture"]},
      {"kind": "problem", "record_id": "uuid", "title": "Problem statement", "summary": "Refined problem and desired outcome", "claims": ["claim-problem"]},
      {"kind": "solution", "record_id": "uuid", "title": "Final Solution", "summary": "Final direction", "claims": ["claim-solution"]},
      {"kind": "complete", "record_id": "uuid", "title": "Complete", "summary": "Human completion decision", "claims": ["claim-complete"]}
    ],
    "transitions": [
      {"from": "problem", "to": "solution", "decision_claim_id": "claim-transition", "material_conflict": {"status": "addressed", "disposition": "modified"}}
    ]
  },
  "claims": {
    "claim-transition": {
      "classification": "decided",
      "confidence": null,
      "text": "Preview context replaced the always-open modal context.",
      "evidence_ids": ["evidence-1"],
      "revisions": 1
    }
  },
  "decision_changes": [],
  "conflicts": [],
  "completion_evidence": []
}
```

### Errors

- `404`: completed Solution or lineage snapshot does not exist.
- `409`: snapshot is in a failed state and must be regenerated.

## GET `/features/{feature_id}/lineage/evidence/{evidence_id}`

Return a completion-time evidence excerpt and optional live-record navigation metadata.

### Response `200`

```json
{
  "id": "evidence-1",
  "source_type": "solution_decision",
  "source_id": "uuid",
  "field_name": "reason",
  "excerpt": "Existing UX should change minimally.",
  "captured_at": "2026-08-21 16:10:00",
  "live_record": {"available": true, "entity_type": "problems", "entity_id": "uuid"}
}
```

The endpoint never returns raw image bytes or private, unpromoted chat history.

## POST `/features/{feature_id}/lineage/regenerate`

Create a new version from current retained source records and attempt optional AI enrichment. This is an explicit user action and does not change workflow state.

### Request

```json
{"include_inference": true}
```

### Response `201`

Returns the same representation as `GET /features/{feature_id}/lineage` with an incremented version.

### Errors

- `400`: Solution is not completed.
- `409`: a prior generated Markdown document has external changes that require user resolution before replacement.
- `502`: deterministic generation failed. Provider failure alone returns `201` with `ready_without_inference` and an `inference_error`.

## POST `/features/{feature_id}/lineage/claims/{claim_id}/corrections`

Append a user correction to an interpretation claim.

### Request

```json
{
  "text": "The context was moved into Preview rather than removed.",
  "reason": "Clarifies the preserved requirement."
}
```

### Response `201`

```json
{
  "claim_id": "claim-id",
  "revision_id": "uuid",
  "supersedes_id": "prior-revision-id",
  "author_type": "user",
  "text": "The context was moved into Preview rather than removed.",
  "reason": "Clarifies the preserved requirement.",
  "is_current": true,
  "created_at": "2026-08-21 16:20:00"
}
```

### Validation and errors

- `400`: blank correction, attempt to edit immutable source/evidence, or attempt to upgrade provenance without evidence.
- `404`: completed Solution, snapshot, or claim does not exist.
- `409`: claim is no longer current; reload before correcting.

After correction, the API regenerates the final Markdown projection using existing external-change protection. If the Markdown write conflicts, the correction remains in SQLite and the response reports that document synchronization needs explicit retry.

## Completion integration

Existing completion endpoints keep their request shape. Successful responses add lineage metadata:

```json
{
  "path": "2026/90. Archive/Completed Work/example.md",
  "raw_path": "2026/90. Archive/Completed Work/assets/example.raw.md",
  "lineage": {"snapshot_id": "uuid", "status": "ready_without_inference", "version": 1, "retryable": true},
  "report_generation": {"status": "generated", "lineage_snapshot_id": "uuid", "lineage_version": 1}
}
```

Workflow completion is not rolled back if optional inference or vault projection fails. The error is explicit and regeneration remains available.

Before the response is produced, final-report generation receives the selected Lineage projection and only its referenced evidence excerpts. It does not receive an arbitrary prefix of Raw Data. The generated report is downstream presentation and cannot be registered as evidence in that same snapshot.

Normal completed-work Playbook regeneration reuses its recorded Lineage snapshot. A request that explicitly regenerates Lineage creates the new snapshot first and then regenerates the report and document from the new version.

## Conflict address input

When a human clears a previously detected conflict, the existing conflict-decision request is extended with:

```json
{
  "state": "clear",
  "citation": "Decision record or implementation evidence",
  "address": {
    "basis": "explicit_decision",
    "disposition": "modified",
    "summary": "The always-open context requirement became Preview context with status feedback.",
    "evidence_source_type": "solution_decision",
    "evidence_source_id": "uuid"
  }
}
```

If a prior conflict exists and `address` is absent or unsupported, lineage reports the prior conflict as `unclear`; a new clear evaluation does not silently imply resolution.
