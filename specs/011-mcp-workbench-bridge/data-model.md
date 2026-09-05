# Data Model: Dual-Chat Work Tracking

## Existing workflow records

The feature reuses `captures`, `problems`, `features` (Solutions),
`solution_progress_entries`, completion reviews/decisions, Lineage snapshots/evidence, and
`completion_playbooks`. Workbench, in-app Chat, and MCP enter through one application service and
persistence port set. MCP proposal events never directly write workflow, SQLite, search-index, or
Vault state. After an authenticated decision, the application service invokes the same domain
rules and records links back to the source event and decision.

The existing `ai_runs`/`workflow_chat` path remains active for in-app Chat. Conversation messages
are not automatically copied into structured events or published. Only an explicit milestone card
action creates a structured event/decision, linked to but distinct from chat history.

## New durable records

### MCP Connection

Table: `mcp_connections`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Non-secret local connection identity |
| `name` | trimmed 1–80 characters | User-visible external host label |
| `profile` | `work_tracking` | Fixed MVP capability bundle |
| `scopes_json` | closed application scope set | Independently granted session/topic/context/search/evidence/draft/publish access |
| `checkpoint_policy` | `confirm_each` or `allowed_for_started_sessions` | Explicit local recording policy |
| `state` | `active` or `revoked` | Current authorization |
| `created_at`, `updated_at` | UTC | Grant history |
| `last_used_at`, `revoked_at` | nullable UTC | Activity/revocation evidence |

No row exists on first install. Connection IDs and session handles are not credentials; the local
OS account is the trust boundary. A revoked connection cannot be reactivated.

### Tracked Work Session

Table: `work_tracking_sessions`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Opaque server-issued session handle |
| `connection_id` | nullable active connection FK | Owning external host; null for in-app Chat |
| `source_interface` | `external_mcp_chat` or `in_app_chat` | Originating input adapter |
| `conversation_ref_hash` | keyed digest | External lineage key or internal chat identity |
| `capture_id` | unique Capture FK | Initial private Capture created at session open |
| `head_event_id` | event FK | Current immutable event head |
| `head_revision` | integer starting at 1 | Compare-and-swap version |
| `state` | `active`, `completion_proposed`, or `completed` | Integration status, not a workflow stage |
| `publication_state` | `not_requested`, `offered`, `deferred`, `draft`, `publishing`, `published`, `conflicted`, or `failed` | Separate Knowledge lifecycle |
| `publication_offer_revision` | nullable completion revision | Ensures one unsolicited offer per completion revision |
| `parent_session_id` | nullable same-owner FK | Explicit follow-up/fork only |
| `created_at`, `updated_at` | UTC | Session lifetime |

Partial unique constraints bind `conversation_ref_hash` to its source: external sessions use
`(connection_id, conversation_ref_hash)` and in-app sessions use
`(source_interface, conversation_ref_hash)`. Raw external or internal Chat identifiers are not
stored in this table.

Invariants:

- `create` makes one Capture and first event in one transaction.
- `resume` returns the same session only to its owning active connection or authenticated in-app
  Chat context.
- One session owns one lineage branch; topic changes create a new session.
- Similarity never attaches a session to existing work.
- `closed` is derived only after existing user-controlled completion succeeds and rejects new
  appends. Further work uses a new session linked as a follow-up.

### Work Tracking Event

Table: `work_tracking_events`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Immutable event identity |
| `session_id` | session FK | Owning work session |
| `revision` | positive integer, unique per session | Durable append/head version; not source-time ordering |
| `previous_event_id` | nullable event FK | Append-only chain |
| `stream_id` | server-issued ID scoped to session and source | Independent ordering/watermark stream |
| `source_sequence` | positive monotonic integer within stream | Deterministic same-time ordering key |
| `kind` | `capture`, `problem_draft`, `solution_draft`, `work_log_checkpoint`, `completion_proposal`, `work_completed`, `knowledge_draft_saved`, `knowledge_publish_requested`, `knowledge_published`, `conflict_proposal`, or `workflow_link` | Strict event payload type |
| `payload_json` | validated bounded JSON | Distilled work; never raw chain-of-thought or a complete transcript |
| `payload_hash` | SHA-256 | Idempotency/evidence integrity |
| `source_turn_key_hash` | nullable digest | Chat turn correlation without raw host identifiers |
| `supersedes_event_id` | nullable same-session FK | Explicit correction without mutation |
| `occurred_at` | trusted application-clock UTC | Authoritative state-machine event time assigned on acceptance |
| `observed_at` | nullable caller observation time | Untrusted descriptive provenance only |
| `ingested_at` | server UTC | Durable append/latency evidence |

Review disposition is derived from immutable decision events/records rather than updated on the
event row. Payload claims use a required provenance enum: `user_stated`, `observed`, or
`assistant_inferred`. The caller cannot provide author, authoritative occurrence time, approval,
completion, or publication fields. Ordering is `(occurred_at, source_sequence, event_id)` within a
server-issued stream; an adapter-supplied time is authoritative only when the application marks
that adapter as trusted.

State rules:

- Capture event is accepted only after the configured start policy or explicit inline confirmation;
  it is private and reversible.
- Draft and completion proposal events start `pending`.
- Work Log checkpoints remain pending until linked to/adopted into a Solution.
- A newer event may make a proposal stale but never deletes it.
- `assistant_inferred` claims can remain context but cannot become accepted evidence without user
  review.

### Projection Outbox and Result

Tables: `work_tracking_projection_jobs`, `work_tracking_projection_results`

| Field | Rule | Meaning |
| --- | --- | --- |
| `event_id`, `projection_name` | unique pair | Exactly-once logical materialization key |
| `state` | `pending`, `claimed`, `applied`, `ignored_late`, `conflict`, or `failed` | Background status kept outside immutable events |
| `lease_owner`, `lease_expires_at` | nullable | Multi-process crash-safe claim |
| `attempts`, `next_attempt_at` | bounded retry metadata | Backoff and restart recovery |
| `result_revision`, `result_entity_id` | nullable | Applied projection outcome |
| `safe_error_code` | nullable bounded enum | Diagnostic without content leakage |
| `created_at`, `updated_at` | server UTC | Queue and projection lag evidence |

The foreground transaction validates authorization, idempotency, and expected head, then commits
the immutable event, session head, idempotency response, and one outbox job. Only then may it return
`accepted` with `projectionStatus: queued`. A supervised projector claims expired/pending work,
materializes eligible domain state through the same persistence ports, and records a terminal or
retryable result. Unique destination provenance keys prevent duplicate Work Log rows if a worker
crashes after materialization but before marking the job applied.

### Stream Projection Watermark

Table: `work_tracking_stream_watermarks`

| Field | Rule | Meaning |
| --- | --- | --- |
| `session_id`, `stream_id`, `projection_name` | composite primary key | Independent ordered projection stream |
| `last_occurred_at`, `last_source_sequence`, `last_event_id` | deterministic order key | Greatest applied event |
| `version` | positive CAS integer | Concurrent projector protection |
| `updated_at` | server UTC | Projection freshness |

An event whose ordering key is less than or equal to the applied watermark is retained in the
immutable log and receives `ignored_late`; it does not change current workflow state, Work Log,
Lineage, completion evidence, or the watermark. A correction is a new event with a new trusted
occurrence time and an explicit `supersedes_event_id`.

### Knowledge Draft and Publication

Knowledge draft versions remain private and append-versioned. Each version records the source
Completed Work, draft revision, content hash, bounded Markdown, exact evidence references, and
expected Vault source revisions. Publication is a separate decision/event bound to one reviewed
draft revision and content hash. Saving a draft cannot publish it; completing work cannot create a
Knowledge file. `MarkdownVaultAdapter` performs reviewed atomic writes and returns an external
change conflict rather than overwriting a changed file.

### Work Tracking Idempotency Record

Table: `work_tracking_idempotency_records`

| Field | Rule | Meaning |
| --- | --- | --- |
| `source_interface`, `source_owner_hash`, `operation_name`, `operation_id` | composite primary key | Logical mutation identity across both adapters |
| `request_hash` | SHA-256 of canonical validated input | Conflicting retry detection |
| `session_id`, `event_id` | nullable IDs | Created logical result |
| `response_json` | exact bounded logical response | Deterministic retry response |
| `committed_at` | UTC | Commit evidence |

The record commits in the same transaction as its Capture/session/event/activity. Same key and hash
returns the stored response with `deduplicated=true`; a different hash returns
`idempotency_conflict`. Validation, authorization, and transient failures do not consume a key.
Records live for the session lifetime and at least 30 days after user deletion/tombstoning.

### Workflow Link

Table: `work_tracking_links`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Link identity |
| `session_id`, `source_event_id` | FKs | Chat-source event |
| `entity_type` | `captures`, `problems`, `features`, `solution_progress_entries`, completion decision, or lineage evidence | Existing destination kind |
| `entity_id` | existing record ID | Destination |
| `relationship` | bounded enum | `origin`, `adopted_problem`, `adopted_solution`, `accepted_progress`, `completion_source`, or `follow_up` |
| `decision_id` | in-chat decision/approval reference | Proof of user materialization |
| `created_at` | UTC | Link time |

Only existing domain operations invoked after a valid in-chat decision create these links. The MCP
connection gains access only to the session resource and its linked summaries; it never gains
arbitrary entity access from an ID.

### Elicitation Challenge

Table: `mcp_elicitation_challenges`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Server-side challenge identity |
| `connection_id`, `session_id` | owning FKs | Authorization scope |
| `principal_hash`, `workspace_hash` | keyed digests | Local user/workspace binding without raw identifiers |
| `action` | closed advancement enum | Exact requested transition |
| `source_event_id` | same-session FK | Proposal/checkpoint being reviewed |
| `proposed_payload_hash` | SHA-256 | Exact previewed content |
| `expected_head_revision` | positive integer | State reviewed by the user |
| `nonce_hash` | unique keyed digest | Replay protection |
| `expires_at` | short server UTC lifetime | Stale-response boundary |
| `consumed_at` | nullable UTC | Single-use marker |
| `status` | `pending`, `accepted`, `edited`, `rejected`, `cancelled`, `expired`, `stale`, or `revoked` | Terminal disposition |
| `created_at` | server UTC | Challenge history |

The opaque `requestState` returned through the Chat host is either authenticated-encrypted state or
a random nonce referencing this row plus a server MAC. It binds the fields above and protocol
version. It contains no secret, conversation text, raw user response, or authorization grant. A
valid response is still denied if the connection, session revision, domain preconditions, or
workspace changed after issuance.

### Work Tracking Decision

Table: `work_tracking_decisions`

| Field | Rule | Meaning |
| --- | --- | --- |
| `id` | UUID primary key | Review decision |
| `session_id`, `event_id` | unique proposal/event FK | Reviewed source |
| `decision` | `accepted`, `accepted_with_edits`, or `rejected` | User action |
| `accepted_payload_hash` | nullable SHA-256 | Exact adopted content revision |
| `result_entity_type`, `result_entity_id` | nullable | Existing workflow result |
| `challenge_id` | nullable unique challenge FK | Exact external in-chat prompt/response binding |
| `decision_channel` | `mcp_elicitation`, `in_app_chat`, or `workbench` | Where the user acted |
| `client_capability` | bounded protocol/capability label | Evidence that in-chat input was available |
| `created_at` | UTC | Decision time |

For in-app Chat and Workbench, proof is the explicit local UI action plus the authenticated Tauri
window/session context rather than an MCP challenge. The accepted payload hash is calculated after
user edits and before the transaction. Draft adoption, approval, conflict resolution, Work Log
projection, and completion all use existing workflow functions transactionally.

### Work Tracking Activity Event

Table: `work_tracking_activity_events`

Stores source interface, optional connection, session ID, operation, time, duration, outcome, event/record IDs,
challenge ID, and a bounded error category. It stores no lineage key, conversation text, checkpoint
payload, evidence body, prompt, raw response, credential, absolute path, or raw error. Retention is
the newest 500 events and at most 30 days.

## Session resource projection

`llm-wiki://work-session/{sessionId}` is derived from:

- Capture title/summary;
- current head revision, projection freshness/status, and last eight bounded events;
- pending Problem/Solution/completion proposals;
- accepted links and current workflow title/state;
- next human action, such as `review_problem_draft`, `approve_problem`, `review_solution_draft`,
  `review_progress`, `resolve_conflict`, or `review_completion`;
- at most eight excerpts and 6,000 retrieved-context tokens.

Rejected/superseded history is summarized but not replayed in full. The resource is authorized by
the current connection on every read and is not a capability token.

## Current Workbench projection

`llm-wiki://workbench/current` is available only to connections with the corresponding current-view scope. It derives a
workspace revision, the explicit UI selection when available, and at most ten active tracked
lineages ordered by selection then recent user activity. Each entry includes bounded
Capture/Problem/Solution state, one latest progress excerpt, blockers, pending actions, source
interface, tracked-session reference when permitted, and head revision.

The projection contains no full Chat message, raw evidence body, completed archive content,
credential, settings, deleted record, unrelated Vault file, or arbitrary private note. A returned
record ID remains a handle, not authority. Linking an external session to current Workbench work
requires a bound `link_current_work` decision and matching workspace/entity revisions.

## Whole Workbench overview projection

`llm-wiki://workbench/overview` derives a stable, paginated snapshot of every visible non-archived
Capture, Problem, and Solution plus bounded recent completions. Its first page includes complete
counts by lifecycle state and category, every attention-condition count, and a snapshot revision.
Each item contains only kind, title, category, state, bounded outcome/context, latest progress, next
action, update time, and an opaque reference.

Subsequent pages use the same snapshot revision. A concurrent mutation expires the snapshot or is
isolated from it; pages from different revisions are never combined. The projection has no durable
table of its own and grants no mutation authority. Chat history, raw evidence, full item bodies,
archived documents, deleted records, credentials, settings, and arbitrary Vault content are
excluded.

## Concurrency and transitions

```text
session absent
  -> active (Capture + revision 1)
  -> completion_proposed (completion proposal at head)
  -> active (proposal rejected or follow-up event appended)
  -> completed (existing user completion succeeds; no Knowledge side effect)
  -> publication offered or deferred
  -> Knowledge draft
  -> published (separate exact-draft decision)
```

- Append requires `expected_head_revision`; mismatch writes nothing and returns current head.
- Checkpoint append and projection are deliberately split: durable event/head/outbox/idempotency
  commit together, while eligible Work Log materialization runs in the background.
- In-chat advancement checks the challenge, event, session, and destination entity revisions so user edits are never overwritten.
- Challenge validation and challenge consumption occur in the same transaction as the decision and domain mutation.
- A cancelled, rejected, expired, replayed, stale, or revoked challenge creates no authoritative workflow change.
- Workbench and both Chat adapters read the same projections and append through the same application
  service; none has a shadow truth or an adapter-owned synchronization queue.
- A Workbench commit invalidates any pending challenge or inline card bound to an older revision.
- A stale surface keeps its uncommitted user input and reloads the new head. It retries once only
  for an idempotent or provably non-overlapping unchanged intent; a changed governed payload returns
  to exact review.
- Revocation and append serialize through SQLite transactions: an append either commits before
  revocation or makes no change.
- Accepted checkpoint projection creates or links one existing Work Log entry once; retries cannot
  duplicate it. Late ignored, pending, rejected, conflicted, or failed events cannot become
  completion evidence.
- Lineage source hashes include accepted event IDs, payload hashes, source-interface provenance, and
  decision links. Rejected and pending events are not completion evidence.
