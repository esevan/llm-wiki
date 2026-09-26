# Contract: Archive proposal

An archive proposal is an immutable, reviewable plan. Preparation may use the registered model prompt, but it never changes files, published pointers, Task state, or index state.

## Input

```json
{
  "operationId": "uuid",
  "taskId": "task-id",
  "knowledgeRevision": 4,
  "expectedKnowledgeContentHash": "sha256",
  "expectedGenerationSnapshotHash": "sha256",
  "selectedIdeaRevisionIds": ["idea-revision-id"],
  "locale": "en"
}
```

Idea selection defaults to empty. IDs must belong to the exact Knowledge revision/task lineage and preserve their feature 020 disposition.

## Preparation inputs

The service binds the proposal to:

- exact feature 020 Knowledge/idea revisions and hashes;
- eligible feature 019 `used`/counterevidence facts and feature 015 exact reference bindings;
- feature 018 overlap candidates with stable identity, current file revision, type/status/applicability, path, and exact sections;
- normalized relevant taxonomy: current explicit metadata, canonical paths, tags, aliases, and MOC managed regions;
- actual hashes of every target/MOC/link-repair file.

The canonical hash of those inputs is `taxonomySnapshotHash`. Candidate excerpts are bounded and source-addressed. Raw secrets, unrestricted paths, and unrelated private process do not enter the prompt.

## Output

```json
{
  "proposalId": "uuid",
  "proposalVersion": 1,
  "state": "review_needed",
  "outcome": "update",
  "target": {
    "documentId": "stable-id",
    "revision": "actual-file-sha256",
    "path": "Knowledge/Area/Topic.md"
  },
  "rationale": "The existing article covers the same decision and conditions.",
  "taxonomySnapshotHash": "sha256",
  "newCategoryRationale": null,
  "artifacts": [],
  "mocPatches": [],
  "referenceLinks": [],
  "linkRepairs": [],
  "unresolvedConflicts": [],
  "proposalHash": "sha256"
}
```

`outcome` is exactly one of:

- `new`: allocate a stable document ID and create one canonical artifact;
- `update`: append a revision to the same logical document;
- `merge`: publish reviewed combined bytes into one existing logical document and preserve merged lineage;
- `conflict`: present unresolved incompatible claims/paths; no apply is permitted;
- `supersede`: publish a successor and mark exact predecessor decision/document relationships without deleting history.

Every artifact contains exact final bytes, artifact hash, body hash, explicit type/status, stable document ID, path, expected pre-hash, and source draft revision. Final Knowledge and each selected idea are separate artifacts. No unselected idea is included.

## Validation

Registry validation rejects:

- unknown outcomes, status, information type, or patch kind;
- a `new` target that collides by stable ID/path or duplicates an overlap without rationale;
- update/merge/supersede without an exact current target revision;
- a conflict proposal with applicable artifacts;
- a new taxonomy category without rationale;
- idea content within final Knowledge or an idea artifact without exact disposition;
- source links not backed by an actual-use fact with `support` or `counterevidence` role;
- missing reference rationale or exact document revision;
- absolute, parent-traversing, non-Markdown, reserved recovery, or case-colliding paths;
- malformed/duplicate managed markers or changes outside a managed region;
- bytes whose recomputed hashes differ from the proposal.

Apply recomputes all exact heads. Any mismatch marks the proposal stale or conflict and performs no new write. A changed user choice appends a proposal version; it does not mutate the reviewed version.

## Prompt ownership

The central registry owns a dedicated `knowledge_archive_proposal` definition with the complete organization instruction, allowed outcome schema, taxonomy-first rule, idea/final separation, eligible-reference rule, and no-broad-rewrite constraints. It uses the durable visible `user_generation` operation policy. `publication_index` remains a separate post-write indexing operation and is never used to classify archive outcomes. Callers provide only structured source context and bounded user choices. Changing any semantic instruction or schema increments the prompt version. Deterministic validation remains authoritative when model output is incomplete or unsafe.
