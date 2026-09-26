# Contract: Aspect-aware Retrieval

## Candidate collection

Lexical and semantic retrieval each scan their eligible index independently. Semantic candidates are
not restricted to lexical hits. Both candidate sets must validate current source revision; semantic
rows additionally validate input hash, model ID/version, dimensions, and byte length.
Private recovery/withdrawal storage and explicitly internal LLM Wiki paths are ineligible candidates;
ordinary dot paths remain eligible unless a separate visibility policy excludes them.

## Ranking and output

Candidate ranks are fused before pagination. Applicability is weighted above other aspects;
exploration is downweighted when explicit type/status marks it as unverified, deferred, or rejected.
Folder names alone never establish epistemic status. Results deduplicate repeated passages from the
same document and return at most eight passages / 6,000 estimated tokens.

Existing public fields remain stable. Additive evidence metadata may include:

```json
{
  "documentId": "stable-id",
  "revision": "computed-content-sha256",
  "section": "Applicability > Conditions",
  "chunkIndex": 0,
  "chunkCount": 1,
  "aspect": "applicability|decision|content|exploration",
  "informationType": "knowledge|idea",
  "status": "current|historical|unverified|deferred|rejected",
  "conditions": ["saved condition"],
  "historicalMatch": {"decisionId": "old-id", "reason": "saved rationale"},
  "successor": {"decisionId": "current-id", "documentId": "stable-id"},
  "warnings": []
}
```

## Lineage and fallback

A superseded decision may redirect to a confirmed-final adopted successor across documents. Missing
targets, cycles, condition mismatch, or unresolved/withdrawn targets stop resolution and remain
qualified. Conflicting applicable conclusions remain visible with their different conditions.

If embeddings are unavailable, incomplete, stale, or incompatible, lexical candidates still return
with `semantic_available: false` or an equivalent current public indicator. The response must not
claim semantic completeness. Adapter scope checks happen before ranking, and evidence grants remain
connection-bound, revision-bound, expiring capabilities.
