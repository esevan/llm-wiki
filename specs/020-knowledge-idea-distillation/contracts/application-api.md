# Application Contract: Knowledge distillation and version review

All operations validate Task ownership. Read operations never enqueue AI. Mutation replay with the same operation ID and payload is idempotent; a changed payload conflicts.

## Read current review projection

`GET /tasks/{taskId}/knowledge`

Returns current private and published revisions independently, immutable version summaries, the selected body and applicability, exact source/assumption summaries, separate idea summaries, quality warnings, and publication references. The client may request one or two exact revisions for compare. Comparison reads stored versions and provenance differences without a model call or synthesized merge.

## Request generation

`POST /tasks/{taskId}/knowledge/generations`

```json
{"operationId":"uuid","expectedTaskRevision":7,"locale":"en"}
```

The operation first reconciles feature 017. It queues only after journey, completion snapshot, and source bundle are `current`. Shared policy may perform bounded transient retries. `pending`, `stale`, `retryable_failure`, `repair_required`, or `unavailable` keeps the last-good projection visible with its true freshness and never generates from a guessed snapshot.

Completed output identifies the appended private revision and exact generation snapshot. A save-time mismatch returns `source_snapshot_stale` and appends nothing.

## Create an edited revision

`POST /tasks/{taskId}/knowledge/versions/{revision}/edits`

Input includes `operationId`, `expectedCurrentPrivateRevision`, `expectedContentHash`, `bodyMarkdown`, and structured applicability/assumption edits. The source revision stays immutable. A valid edit appends the next revision with `derivationKind=edited`.

Edits cannot remove source bindings from factual/decision claims or broaden applicability beyond the body. A blocked edit returns separate unsupported-addition and material-omission findings without changing the current pointer.

## Restore as new

`POST /tasks/{taskId}/knowledge/versions/{revision}/restore`

Input includes `operationId` and `expectedCurrentPrivateRevision`. The response is a newly appended revision derived from the selected older revision. It preserves exact source/assumption provenance and does not alter Task logs, decisions, Distillation, journey history, earlier drafts, or the published pointer.

## Publish exact reviewed revision

`POST /tasks/{taskId}/knowledge/versions/{revision}/publish`

Input includes `operationId`, `expectedContentHash`, `expectedGenerationSnapshotHash`, and explicitly selected idea revision IDs (default empty). Feature 020 passes the exact stored body and metadata to feature 021. No provider or regeneration call is allowed in the publication path.

Success records feature 021's stable document ID/path and authoritative content hash, then updates the published pointer. External-change, write, organization, or index failure leaves both pointers unchanged.

## Idea boundary

Idea drafts are returned only in their separate collection. Each includes exact disposition, reconsideration conditions, source/Task/final-Knowledge links, and publication state. The UI always renders the exact `idea.disposition`, including `out_of_scope`; it never substitutes the compatibility top-level search status.

Feature 020 prepares idea units; feature 021 owns `idea/` path choice, portable file creation, organization, MOC/link repair, withdrawal, and index update. Publishing Knowledge does not implicitly publish ideas.

## Compatibility routes

Existing `/tasks/{taskId}/knowledge/drafts` routes may adapt to this service during migration, but they must follow immutable edit and exact publication semantics. Compatibility responses expose the appended revision and cannot update a stored body in place.

## Errors

- `task_not_completed`
- `task_revision_conflict`
- `journey_not_current`
- `source_snapshot_stale`
- `version_not_found`
- `current_private_conflict`
- `content_hash_conflict`
- `validation_unsupported_addition`
- `validation_material_omission`
- `publication_handoff_failed`
- bounded shared provider/job failures

Raw prompts, secrets, provider payloads, and unredacted stderr never cross this boundary.
