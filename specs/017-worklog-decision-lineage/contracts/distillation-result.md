# Contract: Distillation Input and Result

This feature contract is nested inside the versioned prompt/reference/job/result envelope owned by feature 015. Foundation field names may be adapted after integration; the semantic requirements below are feature 017 ownership.

## Foundation operation mapping

- `run_report_distillation`: the only Run-linked Work Log Distillation operation in feature 017. It coalesces by exact final-report hash, automatically retries transient failures up to the foundation limit, and permits an explicit retry after exhaustion.
- `task_journey_increment`: applies the validated same structured result to the affected Task source bundle and incrementally updates the active Task journey.
- `work_log_distillation`: not enqueued for Run-linked entries in feature 017, avoiding duplicate Distillation passes. It remains available to later non-Run/manual Work Log scope.

Both selected operations are durable and use the foundation registry's versioned prompt identity and retry policy. Their application disposition remains pending until the feature consumer persists the exact-source projection and records `applied` in the same transaction. Existing `lineage_inference` maps to journey policy only for compatibility and is not the new projection authority. Implementations must not create ad hoc prompt text or a second job table when a registry/foundation equivalent exists.

`applied` describes publication of the derived projection only. The UI shows it as an explicitly AI-generated working projection with sources and freshness. It is not user adoption and cannot change a source decision or workflow state; the consumer records the disposition only through the foundation helper in the same transaction as projection publication.

## Input contract

```json
{
  "projectionKind": "work_log|task_journey",
  "owner": {
    "taskId": "stable-id",
    "taskRevision": 7,
    "runId": "optional-stable-id",
    "runRevision": 12,
    "workLogEntryId": "optional-stable-id"
  },
  "primarySource": {
    "type": "run_final_report",
    "id": "run-id",
    "revision": "12",
    "locator": "final_report",
    "contentHash": "sha256"
  },
  "missingPrimaryReason": null,
  "sources": [
    {
      "type": "run_completed_item|task_revision|task_decision|task_completion|work_log_entry|refinement_input",
      "id": "stable-source-id",
      "revision": "exact-revision",
      "locator": "bounded-field-or-item",
      "contentHash": "sha256",
      "role": "corroborates|missing_material_fact|contradicts|task_context|deletion_tombstone"
    }
  ],
  "affectedSemanticIds": ["node-or-claim-id"],
  "expectedProjectionRevision": 4,
  "sourceSetHash": "sha256",
  "rulesVersion": "immutable-version",
  "locale": "en|ko",
  "repairReason": null
}
```

### Selection rules

1. If `final_report` exists, it is `primarySource`.
2. Completed evidence is included only when it corroborates an important report claim, supplies a material missing fact, or contradicts the report.
3. Raw streaming deltas, routine retry/continue/reconnect/approval controls, and chronology-only context are excluded.
4. Without a final report, `missingPrimaryReason` is required and every candidate fact must cite known saved evidence.
5. Every source is exact and immutable. A changed record is a different revision, never a mutable pointer.

## Result contract

```json
{
  "claims": [
    {
      "id": "stable-claim-id",
      "kind": "decision|performed|verified|failed_approach|unresolved",
      "statement": "bounded supported statement",
      "actor": "user|ai|tool|system|unknown",
      "epistemicState": "suggested|decided|attempted|performed|observed|verified|unresolved",
      "status": "current|historical|contradicted|unknown",
      "topicKey": "optional-stable-topic",
      "sources": [{"type":"...","id":"...","revision":"...","locator":"..."}],
      "contradictedBy": []
    }
  ],
  "nodeChanges": [
    {
      "action": "add|enrich|merge|omit|supersede",
      "candidateId": "stable-semantic-id",
      "targetIds": [],
      "expectedTargetRevisions": {},
      "claimIds": [],
      "detail": {
        "before": null,
        "after": null,
        "reason": null,
        "evidenceClaimIds": [],
        "result": null,
        "currentStatus": "adopted|superseded|withdrawn|unresolved|not_applicable",
        "links": []
      },
      "sources": []
    }
  ],
  "relationshipChanges": [
    {
      "kind": "derived_from|supports|verifies|contradicts|merged_into|supersedes",
      "from": "node-id",
      "to": "node-id",
      "reason": null,
      "sources": []
    }
  ],
  "topicStates": [],
  "completionSnapshots": [],
  "workLogView": {
    "sections": [
      {"kind":"outcome|decisions|performed|checks|failed_approaches|unresolved","claimIds":[]}
    ]
  },
  "warnings": [],
  "dependencies": [{"source":{},"claimIds":[],"relationshipIds":[]}]
}
```

## Required validation

- Reject the entire candidate publication if any material claim or semantic relationship lacks a supplied exact source.
- Reject sources absent from the input manifest or whose hash/revision changed.
- Reject `verified` without an observed completed check/evidence source.
- Reject `decided` for AI-only suggestions or ordinary prose without an explicit user/saved decision source.
- Reject a reason or causal link supported only by order or proximity.
- Permit an evidence-backed but nonexplicit relationship only as an AI suggestion with exact sources; it cannot set topic state or be presented as a proven reason, user decision, verification, or supersession.
- Reject `supersede` that overwrites a node, targets no earlier node, or lacks explicit replacement evidence.
- Reject `merge` with conflicting topic meaning; use contradiction or separate nodes instead.
- Reject node mutations with stale expected target revisions.
- Preserve unknown reason/result/status as null or unresolved; do not fill it with general knowledge.

Each feature source maps to the foundation attribution shape as document ID, exact version, optional section, bounded excerpt, and claim ID. The feature payload retains its source kind and locator for semantic validation and UI navigation; these fields do not replace the shared attribution record.

## Deterministic fallback

If the model is unavailable or the candidate is invalid, the last-good Distillation remains current. When no last-good result exists, show a bounded factual fallback assembled from saved Run status, final-report availability, and exact evidence labels without claiming semantic decisions or causal links.

After any source revision changes, that retained last-good result is exposed with `stale` freshness until a replacement publishes. A failed automatic retry sequence does not erase it; the exhausted job remains explicitly retryable.

## Compatibility

An incompatible prompt, rules, or result-schema version schedules explicit full repair. A compatible source revision change uses targeted invalidation. Locale-only regeneration must preserve semantic node IDs.
