# Data Model: Automatic Capture Distillation

## Capture source/current identity

The existing Capture remains the canonical record. A new immutable source snapshot preserves what was submitted before the existing editable `captures.text` can change. Snapshot-owned image rows preserve every original image's name, media type, byte hash, and bytes even if current Capture attachments later change.

| Field | Meaning | Rule |
| --- | --- | --- |
| capture_id | Stable Capture identity | Existing immutable ID |
| source_revision | Internal revision of user-authored Capture text/image membership | Starts at 1 for new records; advances only when source changes |
| current_revision | Currentness token for user-authored mutation that invalidates automatic apply | Monotonic; refinement input or Capture edit advances it even when raw source is unchanged |
| source_hash | Canonical hash of raw text plus ordered image identities/content hashes | Deterministic for one source revision |
| created_at | Source creation time | Existing value, unchanged by background work |
| last_user_activity_at | Last allowlisted user action | Never updated by distillation status/result |

Each source revision snapshot stores authored text, ordered owned image bytes/metadata/hashes, timestamp, and provenance. Revision 1 is the separately inspectable initial raw Capture. Later item edits append an internal snapshot before changing the current head; this provenance is not exposed as a user version-history/count feature.

Old Capture rows receive a baseline snapshot and neutral currentness during migration but are not enqueued. Eligibility for initial distillation is recorded at creation; revision 1 alone never causes backfill.

## Capture distillation

At most one current applied derived presentation exists per Capture.

| Field | Meaning | Rule |
| --- | --- | --- |
| capture_id | Owner | Unique current record; foreign key to Capture |
| result_revision | Derived presentation revision | Monotonic internal token; not shown as user version history |
| source_revision | Raw source used | Required exact provenance |
| bound_current_revision | Currentness at submission | Required exact provenance |
| source_hash | Source identity | Must match job input/publication |
| title | Human-readable short title | Trimmed, at most 120 characters |
| content | Readable organization of source | At most 4,000 characters |
| context | Grounded situational context | Empty when unsupported; at most 2,000 characters |
| explicit_requests | Ordered requests stated by the source | Zero to eight entries, each at most 500 characters |
| locale | Generation locale | `ko` or `en`; source remains unchanged |
| job_id | Publishing job | Unique publication provenance |
| applied_at | Successful current publication time | Background timestamp, not user activity |

Validation rejects malformed types, unexpected fields, over-limit content, an empty all-field output, and invalid source references. Application is all-or-nothing.

## Distillation proposal

A validated useful result that cannot apply because currentness advanced.

| Field | Meaning | Rule |
| --- | --- | --- |
| proposal_id | Stable proposal identity | Immutable |
| capture_id | Owner | Required |
| job_id | Producing job | One proposal per job/publication kind |
| source_revision / bound_current_revision / source_hash | Exact old source identity | Immutable |
| title / content / context / explicit_requests | Proposed derived fields | Same validation limits as current result |
| state | Review state | Initial `available`; acceptance/rejection is outside automatic apply |
| created_at | Proposal creation time | Does not affect recency |

Foundation exposes a mismatched job as `application_disposition: superseded`; the Capture projection separately supplies the optional proposal. It does not introduce automatic merge or a visible version count.

## Distillation job projection

The shared durable job retains execution lifecycle. Capture views receive only safe fields.

| Field | Meaning |
| --- | --- |
| job_id | Logical operation/attempt anchor |
| logical_operation_id | Stable once-only initial cleanup identity across latest-context successor jobs |
| prompt | Registered prompt ID/version |
| source_revision | Foundation's opaque exact source revision |
| status | Existing queue lifecycle status |
| execution_outcome | `pending`, `succeeded`, `failed`, or `cancelled` |
| application_disposition | `pending`, `review_needed`, `applied`, `superseded`, `rejected`, or `not_applicable` |
| safe_error | Localized-safe code/message, absent for success |
| retry_allowed | Whether explicit retry is valid for current source |
| attempt | Current bounded attempt number |

## Capture distillation summary

Read model shared by Workbench card and Capture modal:

```text
CaptureDistillationSummary
├── captureId, sourceRevision, currentRevision
├── source: raw text, image metadata/access references
├── display: placeholder or current derived result
├── job: latest safe foundation job projection
└── proposal: optional content from superseded work
```

The card uses title/content/status. The modal uses the same object and additionally renders raw source, context, requests, error/retry, and proposal. No consumer reconstructs currentness from timestamps.

## Content transition event

Presentation-only identity, not persisted workflow state:

| Field | Meaning | Rule |
| --- | --- | --- |
| entity_key | Stable rendered entity | Same key before/after |
| previous_revision | Content revision currently displayed | Must be known and mounted |
| next_revision | New current content revision | Must differ and be explicitly reconciled |
| cause | Why content changed | `preview_adopted`, `automatic_apply`, `refinement_apply`, `version_restore` |
| historical | Historical reading mode | `true` suppresses motion |
| reduced_motion | User/system preference | `true` suppresses motion |

Locale and ordinary render cycles are not revision causes.

## State transitions

```text
new Capture transaction
  ├─ persist raw source + revision 1 in Inbox
  └─ submit/coalesce capture_distillation → queued → running

running + exact currentness + valid result
  → write distillation + disposition applied

running + advanced currentness + useful valid result
  → write proposal + disposition superseded

running + deleted/incompatible source
  → disposition superseded

running + transient failure
  → bounded recovery → retryable or running

running + permanent/exhausted failure
  → failed → explicit retry → queued/running
```

Capture remains in Inbox throughout. Opening, listing, rerendering, and event reconnect only read/reconcile.
Explanatory chat input advances context currentness and the logical operation's latest context revision. An older attempt becomes superseded; at most one successor for the latest context may be queued/running. Direct Capture content edits do not reschedule initial cleanup and retain useful old output only as a proposal.
For a new eligible image-only Capture, opening refinement must not invoke the older synthetic initial-image refinement path. Older records may retain that compatibility behavior without receiving a distillation operation.
