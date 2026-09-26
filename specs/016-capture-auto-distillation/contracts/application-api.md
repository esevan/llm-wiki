# Application Contract: Automatic Capture Distillation

## Shared foundation dependency

Implementation begins after workflow foundation is merged. This feature consumes:

- registered durable `capture_distillation` policy for `captures`, explicit retry, default `applied`, and registered prompt ID/version; Capture performs its domain-specific active-equivalent coalescing before retry or successor submission;
- consumer submission fields `taskKind`, `entityType`, `entityId`, and exact opaque `sourceRevision` plus domain payload;
- queue reads exposing `prompt`, `source_revision`, `execution_outcome`, and `application_disposition` while preserving status/result/retry;
- `record_job_application_disposition(tx, job_id, expected_source_revision, actual_source_revision, disposition)`, which records `superseded` on mismatch;
- `record_version_provenance(tx, provenance)` for the applied Capture display revision.

Capture owns its revision/content schema and domain projection; no generic content head is introduced.

## Create Capture

`POST /captures` continues to accept `operationId`, `text`, and existing validated `images`.

The command transaction persists raw source, assigns source/current revision, and submits one initial operation identity. Provider execution is outside the transaction and response path.

The submitted text and every ordered image's name, media type, byte hash, and bytes are also stored as immutable internal source revision 1 before editable current Capture content can change.

Successful response includes an immediate Capture projection:

```json
{
  "id":"capture-1",
  "kind":"capture",
  "sourceRevision":"capture-1:source:1:sha256:...",
  "currentRevision":1,
  "source":{"text":"rough source","images":[{"name":"shot.png","mediaType":"image/png"}]},
  "display":{"title":"rough source","content":"rough source","revision":0,"placeholder":true},
  "distillation":{"jobId":"job-1","status":"queued","executionOutcome":"pending","applicationDisposition":"pending","retryAllowed":false}
}
```

Image-only display title is localized (`Image capture` / `이미지 캡처`). A workflow submission failure after source persistence returns the saved Capture with a visible failed/retryable state; it does not reinterpret source persistence as failed or submit from a later read. Repeating the same `operationId` returns the same Capture and logical operation.

## Read projections

`GET /workbench` includes source/current revision, display, and safe distillation summary for each Capture card.

`GET /captures/{captureId}` or the existing refinement context operation extended with the same object returns identical state plus raw image access metadata, derived context/requests, and optional proposal. The modal never has an independent state interpretation.

Reads never submit or retry work.

For new distillation-eligible image-only Captures, refinement open also bypasses the legacy `prepare_image_capture` synthetic message/job path. The creation-time `capture_distillation` is the sole initial interpretation. Pre-feature compatibility behavior does not make an older Capture eligible for distillation.

Existing `item.update` for `captures` appends a new internal source snapshot and advances source/current revision in the same transaction as the text change. Existing expected-revision conflict behavior remains. It does not reschedule initial cleanup; useful late output may become a proposal.

Explanatory refinement chat input advances a distinct context revision without rewriting or deleting source snapshots. If initial cleanup is pending/running, the domain logical operation marks the old exact-revision attempt superseded and submits or coalesces at most one successor for the latest source-plus-chat context. This is not a second logical initial operation. Modal open alone never takes this path.

## Retry

Existing `POST /jobs/{jobId}/retry` is used. Foundation retains original prompt version and source revision.

Retry is valid only when the job permits it, Capture still exists, source/current identity matches, and no equivalent attempt is active. Equivalent repeated requests return current work. A mismatch records/returns `superseded` and cannot bind old output to new source.

## Domain payload and validated output

The submission's domain payload contains schema version, Capture ID, numeric current revision, canonical source hash, locale, exact text, and ordered image identities/content hashes. Binary images resolve locally for provider input and never appear in list/events/error payloads.

Validated output:

```json
{
  "schemaVersion":1,
  "title":"Readable title",
  "content":"Organized source content",
  "context":"Only context supported by the source",
  "explicitRequests":["A request stated in the source"],
  "grounding":[
    {"field":"title","sourceRefs":["text:0-12"]},
    {"field":"content","sourceRefs":["image:0"]}
  ]
}
```

Every nonempty semantic field requires source references. Unsupported context/requests remain empty. Unexpected fields, invalid references, over-limit values, or empty all-field results fail validation.

## Atomic publication

The result publisher opens one transaction and compares job Capture ID, opaque source revision, numeric current revision, and source hash with current state.

- **Exact current head**: write current distillation, record provenance/references, call foundation helper with `applied`, and finish successfully.
- **Useful conflicting result**: the helper records `superseded` because revisions differ; in the same transaction store one immutable proposal, provenance/references, and a proposal destination.
- **Deleted/missing/incompatible result**: the helper records `superseded` and no current/proposal content is written.

The concrete helper order must preserve one transaction: domain writes and disposition either commit together or not at all. Every branch leaves raw source, Inbox membership, and `last_user_activity_at` unchanged.

## Event and reconciliation

Existing job events are invalidation hints containing entity/job identity and safe lifecycle metadata. The client reloads the durable Workbench/Capture projection after event, reconnect, or sequence gap. It never applies result content directly from an event.

## Safe errors

Safe codes include `provider_unavailable`, `invalid_result`, `source_stale`, `target_missing`, `retry_exhausted`, and `interrupted`. Text is localized. Prompts, credentials, raw provider output, and binary data never appear.
