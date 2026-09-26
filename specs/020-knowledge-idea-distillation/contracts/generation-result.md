# Contract: Knowledge generation input and result

This contract is nested inside feature 015's versioned prompt/reference/job/result envelope. Exact foundation field names may be adapted after integration; these semantic rules remain feature 020 ownership.

## Input

```json
{
  "task": {"id":"task-id","revision":7,"definition":{},"completion":{}},
  "distillations": [{"id":"distillation-id","revision":3,"primaryFinalReportRef":{},"claims":[]}],
  "journey": {"projectionRevision":9,"sourceSetHash":"sha256","topicStates":[],"completionSnapshot":{}},
  "actualReferenceUsages": [{"documentId":"doc","documentVersion":"sha256","section":"heading","disposition":"used|adopted|counterevidence"}],
  "assumptions": [{"id":"assumption-id","statement":"bounded","sources":[]}],
  "sourceManifest": [{"type":"...","id":"...","revision":"...","locator":"...","contentHash":"sha256","role":"..."}],
  "generationSnapshotHash": "sha256",
  "locale": "en|ko"
}
```

Candidate, viewed-only, or merely mentioned references are excluded. The primary Run final report supplies the Distillation center when available; raw completed evidence supplies material omissions, corroboration, contradictions, failures, and verification limits.

## Result

```json
{
  "article": {
    "type": "concept|guide|comparison_decision|research_result",
    "title": "standalone title",
    "finalOutcomes": [{"topicKey":"stable-topic","statement":"supported current outcome","claimIds":["claim-id"]}],
    "bodyMarkdown": "final article only",
    "applicability": {"summary":"supported use situation","representativeQuestions":[],"helpsWith":[],"conditions":[],"exclusions":[],"sourceRefs":[{"type":"...","id":"...","revision":"...","locator":"...","quote":"exact quote"}]},
    "claimBindings": [{"claimId":"claim-id","statement":"bounded claim","epistemicState":"reported|suggested|decided|attempted|performed|observed|verified","sourceRefs":[{"type":"...","id":"...","revision":"...","locator":"...","quote":"exact quote"}]}],
    "assumptionBindings": [{"assumptionId":"assumption-id","bodyLocator":"section","sourceRefs":[{"type":"...","id":"...","revision":"...","locator":"...","quote":"exact quote"}]}]
  },
  "ideas": [{
    "id":"stable-idea-id",
    "title":"idea title",
    "bodyMarkdown":"separate portable idea",
    "disposition":"unverified|deferred|out_of_scope|rejected",
    "reconsiderationConditions":[],
    "sourceRefs":[{"type":"...","id":"...","revision":"...","locator":"...","quote":"exact quote"}],
    "relatedTopicKeys":[]
  }],
  "qualityFindings": [{"kind":"unsupported_addition|material_omission|idea_leak|invalid_finality|invalid_verification|applicability_overreach|missing_reference","severity":"blocking|warning","locator":"...","sourceRefs":[{"type":"...","id":"...","revision":"...","locator":"...","quote":"exact quote"}]}]
}
```

## Required deterministic validation

1. Every factual or decision-bearing statement binds to source(s) present in the exact input manifest with an exact quote. Decided or verified bindings preserve a validated Distillation claim ID, statement, epistemic state, and citations; synthesized prose remains `reported` and cannot supply a final outcome.
2. `verified`, successful, effective, approved, or completed language requires corresponding observed evidence. A final report cannot override contradictory completed evidence.
3. Each final outcome comes from its own feature 017 stable topic final/completion state. Suggested, attempted, adopted-without-final-confirmation, superseded, contradicted, or unresolved content cannot become final; one final topic never promotes sibling claims or the whole document.
4. The article body contains no returned idea body or source-only exploration. Repeated normalized passages and idea-only source bindings are blocking `idea_leak` findings.
5. Conditions, exceptions, failures, counterevidence, verification limits, and unresolved matters marked material in Distillation/journey appear in the article or generate `material_omission`.
6. Applicability fields are required and cannot promise help beyond the supported body and bindings.
7. A comparison/decision structure is valid only when exact final decisions exist. Other tasks use concept, guide, or research-result structure without an invented choice.
8. Every idea has a valid disposition, at least one exact source, Task/final-Knowledge linkage supplied by the application, and meaningful reconsideration condition(s). Archival occurrence is not support.
9. Any blocking finding rejects the entire candidate; existing current/published versions remain unchanged.
10. Before append, the application verifies the current canonical source manifest still hashes to `generationSnapshotHash`.

## Provider failure fallback

The last valid current private and published revisions remain readable. If sources change, last-good stays visible as stale. If no valid revision exists, the UI may show bounded availability and exact source links, but it must not fabricate an article, final outcome, idea, or applicability claim.
