# Application Contract: Reference-aware workbench

All operations validate the current refinement subject and return user-safe errors. Mutation replay with the same operation ID and payload is idempotent; reuse with different content conflicts. Raw prompts, secrets, provider payloads, and unredacted errors never cross the boundary.

## Read workspace

`GET /refinement/{sessionId}/reference-workspace`

Returns the existing conversation/session identity plus:

```json
{
  "preview":{"id":"p1","currentVersion":3,"selectedVersion":3,"versions":[]},
  "generation":{"status":"completed","executionOutcome":"succeeded","applicationDisposition":"review_needed"},
  "investigation":{"status":"searching|no_suitable_result|failed|changed_context|completed"},
  "references":{"items":[],"query":"","filters":{},"sort":"relevance","orderEpoch":4},
  "mentions":[],
  "viewState":{}
}
```

Reads never enqueue generation, retrieval, cleanup, or investigation. Requested generation failure and optional investigation state remain separate.

## Request preview generation

`POST /refinement/{sessionId}/preview-generations`

```json
{"operationId":"uuid","expectedContextRevision":"refinement:12","expectedTaskRevision":7,"locale":"en"}
```

Returns a visible foundation job. The job runs planner → necessary retrieval → finalizer and appends only when exact heads remain current. An active equivalent request coalesces. Explicit retry addresses the same logical request against the current heads and never overwrites a newer preview.

## Continue conversation and optional investigation

The existing message route accepts exact mention bindings in addition to text/images:

```json
{
  "operationId":"uuid",
  "message":"Compare @approval with @rollout",
  "mentions":[
    {"mentionId":"m1","documentId":"d1","documentVersion":"v4","section":"Approval"},
    {"mentionId":"m2","documentId":"d2","documentVersion":"v2","section":"Rollout"}
  ]
}
```

Insertion never invokes this route. Sending advances context revision and may schedule one latest-context optional investigation. Older optional work becomes cancelled or `changed_context`; its result cannot enter the current reply/preview.

`POST /refinement/{sessionId}/investigations` permits an explicit direct investigation. It remains visible and retryable by explicit action, while automatically suggested ancillary investigation uses latest-wins speculative policy.

## Read/filter references

`GET /refinement/{sessionId}/references?query=&aspect=&status=&usage=&sort=&cursor=`

Returns deduplicated document rows with exact passages, source versions, feature 018 metadata, usage state, and a stable `orderEpoch`. Pagination appends without changing existing row order. A changed explicit sort/filter creates a new epoch. The live desktop consumer performs grouping, filters, deterministic sorting and paging
in `ReferenceList.tsx`; the native list supplies stored exact bindings or query results.
The query URL is a logical contract, not a separately implemented HTTP paging route.

## Open exact reference

`POST /refinement/{sessionId}/references/open`

```json
{"documentId":"d1","documentVersion":"v4","section":"Approval","version":3}
```

The local desktop command validates the visible refinement subject and an exact binding
already stored in the selected preview or its investigation context. D's index resolves
the stable document/version/section; the Vault read must match the exact source hash.
Arbitrary caller paths are not accepted. It records `viewed` only and does not record
`used` or `adopted`, enqueue work, or mutate the composer.

MCP evidence opening is a separate authority surface: `evidence_read` validates the
connection-bound evidence grant, expiry and granted/current/expected revisions in
`adapters/vault/mod.rs`. A desktop session is not an MCP grant.

## Save mention draft bindings

`PUT /refinement/{sessionId}/mention-draft`

Input includes `operationId`, expected draft/context revision, plain composer text, and an ordered list of exact mention tokens. Multiple document/section mentions are allowed. Save does not send the message. A stale draft returns `draft_conflict` and preserves the local text/tokens for reconciliation.

## Compare preview versions

`GET /refinement/{sessionId}/preview-versions/{left}/compare/{right}`

Returns stored field values, per-field change kinds, assumptions, exact reference snapshots, derivation metadata, and current/historical labels. No model/retrieval call occurs. Reading/comparing never changes a pointer or Task.

## Append edited preview version

`POST /refinement/{sessionId}/preview-versions/{version}/edits`

Input includes `operationId`, `expectedCurrentPreviewVersion`, `expectedContentHash`, structured fields, assumptions, and unchanged exact reference bindings. Success appends a new immutable `edited` version. It does not apply to the Task.

## Restore as new

`POST /refinement/{sessionId}/preview-versions/{version}/restore`

Input includes `operationId` and `expectedCurrentPreviewVersion`. Success appends a new version derived from the selected version, preserving its exact assumptions/reference snapshot. It changes only the preview head and does not roll back Task revisions, Work Logs, decisions, journey, or messages.

## Explicitly apply preview

`POST /refinement/{sessionId}/preview-versions/{version}/apply`

```json
{
  "operationId":"uuid",
  "expectedCurrentPreviewVersion":3,
  "expectedContentHash":"sha256",
  "expectedTaskRevision":7,
  "editedFieldHashes":{}
}
```

One immediate transaction verifies exact preview, Task, subject, and edit heads; updates canonical Task fields through the existing Task application path; records actual `used`/`adopted` references and foundation application disposition; and commits. Any mismatch changes nothing. The response includes the new Task revision and a `ContentRevisionTransition` cause. Capture promotion continues to use existing explicit proposal semantics.

## Mark exclusion/use

`POST /refinement/{sessionId}/references/{referenceId}/usage`

Only explicit `used` or `excluded` facts are accepted with a scope and reason. `adopted` cannot be set through this route. Used/counterevidence facts remain addressable by stable document identity/version after archive or path change and are the only reference facts eligible for later published links.

## Error codes

- `preview_generation_failed`
- `necessary_retrieval_failed`
- `no_suitable_result`
- `changed_context`
- `context_revision_conflict`
- `task_revision_conflict`
- `preview_head_conflict`
- `content_hash_conflict`
- `draft_conflict`
- `reference_version_unavailable`
- `evidence_grant_expired`
- `invalid_reference_binding`
- `optional_investigation_failed`


## Implemented native adapter names (2026-09-26)

All calls use the existing `taskOperation` adapter and include `sessionId`.

| Operation | Native name | Additional input |
| --- | --- | --- |
| Workspace/status/list | `task-refinement.reference-workspace` | None |
| Exact stored version | `task-refinement.reference-version` | `version` |
| Compare | `task-refinement.reference-compare` | `left`, `right` |
| Open exact reference | `task-refinement.reference-open` | `documentId`, `documentVersion`, `section`, optional `version` |
| Append edit | `task-refinement.reference-edit` | `operationId`, `version`, `expectedCurrentPreviewVersion`, `expectedContentHash`, `fields` |
| Restore as new | `task-refinement.reference-restore` | `operationId`, `version`, `expectedCurrentPreviewVersion` |
| Explicit apply | `task-refinement.reference-apply` | `operationId`, `version`, `expectedCurrentPreviewVersion`, `expectedContentHash`, `expectedTaskRevision`, optional `editedFieldHashes` |

Workspace returns `generation`, `retrievalOutcome`, `stageTimings`, `preview`,
`investigations`, and `findings`. `preview.current` contains exact structured `fields`,
`references`, `contentHash`, `derivationKind`, `taskRevision`, and `contextRevision`;
`preview.versions` is an ordered summary list. A missing first preview is null, while
requested generation status remains available. Comparison returns complete `left` and
`right` versions plus a field comparison. Apply returns `{task,transitionCause,version}`.
Hashes are `sha256:` followed by the digest of canonical JSON serialization.

An edit preserves exact assumption bindings. A restored version copies assumptions and
reference snapshots without writing Task or work history. Exact local source reads reject
missing or changed versions; no arbitrary caller-provided path is accepted. Generation is
automatic through the existing message command; retry uses the existing AI queue operation.
Standalone generation and investigation use `task-refinement.reference-generate` and
`task-refinement.reference-investigate`. Draft text and exact mentions persist through
`task-refinement.mention-draft`; reference lookup uses `task-refinement.reference-list`.
Explicit usage uses `task-refinement.reference-usage`. Desktop filtering/paging is a
projection of these results in the shared reference rail.

F exports Task-owned actual use and Capture use bound by a successful recorded
acceptance/application decision for that Task. It coalesces exact document/version/section
records, retains counterevidence and recorded claim statements, and excludes facts that
only describe discovery, viewing or mentioning. Explicit exclusions remain effective
until a later explicit use; automatic use/adoption does not undo them. Unrecoverable
legacy rationale fails with `reference_provenance_unresolved`. Source manifests include
this normalized evidence, so usage changes invalidate F append and G prepare guards.
