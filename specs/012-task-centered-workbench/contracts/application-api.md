# Application Contract: Task-Centered Workbench

These paths are the shared logical contract for the Tauri request bridge and tests. JSON uses
camelCase. Every mutation accepts `operationId`; replay with the same operation and canonical payload
returns the original result. A changed payload returns `operation_conflict`.

Revision-bound mutation errors use `409`:

```json
{"error":"head_conflict","currentRevision":4,"conflictingFields":["outcome","scope"]}
```

## Workbench and Creation

The MCP bridge exposes the same logical operations through closed `inbound_work_open` and reviewed
`inbound_work_advance` actions; it does not provide arbitrary native passthrough. A direct Task
continuation uses `mode:"continue_task"` with `taskId`, exact session/head/source-event binding,
and user Elicitation. Acceptance may create a connection-owned captureless session; cancellation
creates none. It never fabricates a Capture or claims another connection's session.

`GET /workbench` returns:

```json
{
  "revision": 42,
  "activeShortcuts": [{"kind":"task","id":"t1","taskRevision":3}],
  "refiningShortcuts": [{"kind":"capture","id":"c1","draftRevision":2}],
  "categories": [{"id":"general","label":"General","items":[
    {"kind":"capture","id":"c1","text":"...","lastUserActivityAt":"..."},
    {"kind":"task","id":"t1","taskRevision":3,"state":"in_progress","title":"...","readiness":{"resolved":2,"missing":1,"notApplicable":0},"lastUserActivityAt":"..."}
  ]}]
}
```

Active shortcuts are first and limited to three; refining shortcuts exclude Task IDs already active.
Categories keep existing canonical category identity and ordering. Shortcuts do not remove cards.

`POST /captures` body `{operationId,text,category?}` creates canonical Capture.

`POST /tasks` body `{operationId,inputText?,originCaptureId?,title,detail?,outcome?,scope?,nonGoals?,validationCriteria?,category?}` atomically creates Task revision 1 and, for direct input, hidden provenance. At most one origin field is supplied.

## Task Aggregate

- `GET /tasks/{taskId}` returns current revision, state, fields, readiness, links, relationships,
  Work Log summary, reviews, completion/publication states, and provenance.
- `POST /tasks/{taskId}/revisions` accepts `{operationId,expectedTaskRevision,patch}`; patch allows
  title/detail/outcome/scope/nonGoals/validationCriteria only.
- `POST /tasks/{taskId}/transitions` accepts `{operationId,expectedTaskRevision,to,reason?,continueDecision?}` where `to` is `in_progress | completed | reopen`. Completion data uses the completion endpoint.
- `POST /tasks/{taskId}/problem-links` accepts exact `problemId`, `problemRevision`, relationship and note.
- `DELETE /tasks/{taskId}/problem-links/{linkId}` records unlink history and accepts operation/revision in body.
- `POST /tasks/{taskId}/relationships` accepts target Task, `prerequisite | split_from | related`, note and expected revision. Invalid self, duplicate or cycle returns `relationship_invalid` with a reason.
- `DELETE /tasks/{taskId}/relationships/{relationshipId}` records an unlink decision.
- `GET /tasks/{taskId}/readiness` returns field entries `{key,status,reason,evidenceRefs,sourceRevision,applicableReason,provenance}` and no aggregate score.
- `POST /tasks/{taskId}/readiness-decisions` records revision-bound `not_applicable` or restored-calculated decision with reason.

## Work Log and Decisions

- `GET|POST /tasks/{taskId}/work-log`; create accepts body, optional attachment metadata/data and expected Task revision.
- `POST /work-log/{entryId}/comments`, `POST /tasks/{taskId}/checklist`, and
  `POST /tasks/{taskId}/decisions` create append-only or revision-safe records.
- `PUT /tasks/{taskId}/checklist/{itemId}` changes checked/body with operation ID.
- `POST /tasks/{taskId}/completions` accepts `{operationId,expectedTaskRevision,evidence,report?}`,
  creates completion and moves only this Task to completed atomically.
- `POST /problems/{problemId}/resolutions` accepts exact Problem revision, rationale, and evidence refs;
  it never changes a Task.

## Refinement

- `GET|POST /captures/{id}/refinement` and `GET|POST /tasks/{id}/refinement` read/create the single
  subject session.
- `POST /refinement/{sessionId}/messages` appends a message and schedules optional provider work.
- `PUT /refinement/{sessionId}/workspace` saves `{inputDraft,activeTab,scrollAnchor,baseDraftRevision}`;
  callers debounce 500 ms and flush on navigation/close.
- `GET /refinement/{sessionId}/proposals` returns proposal ID/type/payload and exact draft revision.
- `POST /refinement/{sessionId}/proposal-decisions` accepts `{operationId,proposalId,draftRevision,decision,editedPayload?}`. Selected Task, Problem revisions and links commit atomically. Stale draft returns `draft_conflict`.

## Conflict Review

`POST /conflict-reviews` accepts one subject:

```json
{"operationId":"...","subject":{"kind":"capture_draft","captureId":"c1","sessionId":"r1","draftRevision":3,"materialHash":"..."},"triggerKind":"explicit"}
```

or `{kind:"task_revision",taskId,taskRevision,materialHash}`. The service adds current Vault revision
and evidence scope/grant revision. Response is `202` with run ID/status.

`GET /conflict-reviews/{runId}` returns identity, status (`queued | running | clear | findings |
insufficient_evidence | failed | cancelled | stale`), timings, current flag, findings with source ID,
path and excerpt, and safe error. `POST /conflict-reviews/{runId}/cancel` requests cancellation.
`POST /conflict-findings/{findingId}/decisions` records immutable user disposition; it cannot rewrite
the finding. Material changes supersede/cancel older runs. Non-material category/layout/timestamp
changes do not.

## Knowledge and Lineage

- `GET /tasks/{taskId}/lineage` returns typed graph snapshot and exact source revisions.
- `POST /tasks/{taskId}/knowledge/drafts` requires completed Task revision/completion and creates an
  unpublished draft.
- `POST /tasks/{taskId}/knowledge/drafts/{draftRevision}/correction` accepts
  `{operationId, expectedContentHash, bodyMarkdown}` only while that exact draft is unpublished.
  It returns the corrected draft with a replacement `contentHash`; a stale hash or non-draft state
  returns `draft_conflict`.
- `POST /tasks/{taskId}/knowledge/drafts/{draftRevision}/publish` requires exact draft hash and user
  review operation; external hash mismatch returns `source_changed`.
- regenerate and withdraw are explicit revisioned operations and preserve earlier records.

### Recorded journey and inferred relationships

The lineage response retains the top-level provenance `nodes`, `edges`, and `sourceHash`, and
adds the Task event journey separately:

- `recordedJourney`: deterministic `events` (`id`, `type`, `occurredAt`, `detail`) and recorded
  `edges` of kind `followed_by` or `refinement_context`. Chronology does not establish causality.
- `journeySourceHash`: hash of the full recorded journey and inference schema version, distinct
  from the top-level provenance hash. Changing source text invalidates this cache even if the
  current Task revision is unchanged.
- `journey`: the current locale/source-matched cached journey, or `null` while unavailable.
  It preserves recorded events/edges and adds `titles: [{id,title}]` (at most 48 characters) and
  a separate `relationships` array. Cache identity includes Task, locale, and journey source hash.
- `journeyStatus`: latest matching job's `{jobId,status,error}`, or `null`; `modelStatus` and
  `modelError` describe the saved interpretation. A provider fallback retains recorded events
  and local labels and contains no inferred relationships.

Each inferred relationship has this shape:

```json
{
  "from": "later-event-id",
  "to": "earlier-event-id",
  "kind": "supersedes",
  "provenance": "inferred",
  "rationale": "Why these recorded events may be related",
  "evidence": [
    {"eventId": "later-event-id", "quote": "Verbatim text from the later event"},
    {"eventId": "earlier-event-id", "quote": "Verbatim text from the earlier event"}
  ]
}
```

Allowed kinds are `supersedes` (replacement/reversal), `derived_from` (derivation), and
`depends_on` (prerequisite). Direction is the later referring event to the earlier referenced
event. Endpoints must exist, differ, and follow that order; evidence must quote both endpoint
narratives. Invalid/ungrounded candidates are discarded and valid duplicates are collapsed.
These interpretations neither mutate recorded decisions nor create Task-to-Task relationships.
No accept/reject/edit operation for these inferred journey links is introduced here.

### Journey preparation and exact Knowledge snapshots

Generated Knowledge drafts and regeneration first reuse or prepare a current journey before
generating the article. A usable recorded fallback permits drafting when AI interpretation is
unavailable. Preparation rejects deleted/stale sources and reuses a concurrently completed current
cache instead of overwriting it.

The draft's `lineage.journey` contains the exact preparation result
`{sourceHash,journey,modelStatus,modelError}`. Both direct and queued generated-draft saves recheck
the Knowledge source/completion and journey source hash; changed evidence cannot be committed as
a current generated draft. The saved interpretation is not replaced by a later cache entry.
`GET /tasks/{taskId}` includes the stored `publication.lineage` so reopening a draft retains its
generation evidence. Review shows this draft snapshot when present, otherwise current lineage;
Details shows current lineage. Supplied-body/MCP draft operations retain their existing contract;
the new prerequisite describes application-generated drafts.

Knowledge synthesis can use recorded decision evolution and the inferred relationship explanations
and endpoint quotations. It must distinguish historical/replaced choices from current conclusions
and keep inferred relationships qualified. Publication remains a separate user decision.

## Removed Contract

`/board`, `/captures/{id}/promote`, Problem approval, `/problems/{id}/features`, feature approval/stage,
feature Work Log, feature conflict/completion/lineage and Problem completion-playbook write routes are
removed in the Task-centered release. There is no legacy write alias.

New legacy Problem/Solution adoption or approval, mandatory conflict gates, and
`verify_and_complete` are rejected. Historical completed events and legacy Knowledge draft
lineages remain readable. Unresolved legacy mutation proposals require a fresh canonical Task,
Problem-link, completion, or Knowledge review. Canonical Knowledge operations use
`task-knowledge.{draft,correction,regenerate,publish,withdraw}` semantics with immutable lineage
JSON and separate body/source/lineage hashes.
