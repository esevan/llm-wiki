# Contract: Reference-aware preview prompt

## Registry ownership

E adds a versioned prompt definition under the central feature 015 registry. The definition owns all semantic instructions and schemas. Callers pass structured context only; source text, messages, and retrieved documents are labeled as data and cannot select a prompt by substring.

The operation is user-requested generation: visible status, explicit retry after terminal failure, and no automatic canonical Task apply. Necessary retrieval is part of that requested operation. Optional follow-up search uses the existing `speculative_search` policy and is not substituted for requested generation.

## Planner input

```json
{
  "subject": {"type":"task","id":"task-id","revision":7},
  "contextRevision":"refinement:12",
  "locale":"en",
  "messages":[{"id":"m1","role":"user","body":"..."}],
  "savedWork":{"title":"...","detail":"..."},
  "hierarchyContext":{},
  "knownReferences":[]
}
```

Input is bounded to the current Task/Capture, relevant hierarchy boundaries, current conversation, and exact known references. B’s raw/derived Capture cleanup may be input context but is not a WorkPreviewVersion.

## Planner output

```json
{
  "retrieval": {
    "needed": true,
    "reason": "Validate the current approval rule",
    "queries": ["approval rule contract change"],
    "aspects": ["applicability", "decision", "content"],
    "filters": {"informationTypes":["knowledge"]},
    "requery": null
  },
  "preliminaryAssumptions": [
    {"id":"a1","text":"Approval is required before sending","basis":"user_context"}
  ]
}
```

The retrieval object must pass feature 015 `RetrievalIntent` validation. `needed:false` requires no queries or requery. A planner response is not a useful preview and is never rendered as one.

## Finalizer input

Finalization receives the same exact subject/context heads plus the validated retrieval outcome:

```json
{
  "planner": {},
  "retrievalOutcome": "results|no_suitable_result|not_needed",
  "references": [
    {
      "documentId":"doc-id",
      "documentVersion":"sha256",
      "section":"Approval > Conditions",
      "aspect":"applicability",
      "status":"current",
      "excerpt":"bounded exact passage",
      "grant":"opaque-current-evidence-grant"
    }
  ]
}
```

A validated `needed:false` plan uses `not_needed`; it is not an empty search result.
Provider/retrieval failure is an operation failure, not `no_suitable_result`. Exact evidence grants are validated before source content enters the finalizer.

## Finalizer output

```json
{
  "preview": {
    "description":"...",
    "background":"...",
    "goal":"...",
    "scope":"...",
    "nonGoals":"...",
    "constraints":["..."],
    "completionCriteria":["..."],
    "initialApproach":["..."],
    "assumptions":[
      {"id":"a1","text":"...","basis":"model_inference","sourceClaimIds":[]}
    ]
  },
  "claimSources":[
    {
      "claimId":"background:approval",
      "documentId":"doc-id",
      "documentVersion":"sha256",
      "section":"Approval > Conditions",
      "role":"constraint"
    }
  ],
  "optionalInvestigation": {
    "needed":true,
    "reason":"Look for a newer exception",
    "queries":["approval exception"],
    "aspects":["decision"],
    "filters":{},
    "requery":null
  }
}
```

All nine preview areas are present; absent evidence uses an empty value plus a labeled assumption rather than an invented fact. `claimSources` must reference only supplied exact bindings. Constraints and completion criteria distinguish intended conditions from performed or verified results. Output never says the user adopted, executed, approved, completed, or published anything unless that fact is in supplied canonical evidence.

## Validation and freshness

- Reject unknown fields, invalid types, empty description/goal, unsupported claim references, duplicate assumption IDs, and unbounded arrays/text.
- Recompute the exact subject revision, context revision, and source bundle hash at save time.
- A mismatch records successful execution with `application_disposition=superseded`; it appends no current version.
- A current valid result appends an immutable preview version with `application_disposition=review_needed`.
- Final output never writes canonical Task fields. Explicit apply is a later domain transaction.
