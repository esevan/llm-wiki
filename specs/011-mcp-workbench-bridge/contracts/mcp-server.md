# MCP Server Contract: In-Chat Work Tracking

## Launch, trust, and protocol

```text
<llm-wiki-executable> --mcp --connection <connection-uuid>
```

- The external host uses local stdio; stdout contains MCP JSON-RPC only and content-free
  diagnostics use stderr. The stdio process is a persistence-free bridge to the active GUI process
  over a user-only Unix domain socket or Windows named pipe.
- Phase 1 clients are Codex and ChatGPT desktop. ChatGPT web, remote HTTP, remote OAuth, and remote
  deployment are not supported.
- Full lifecycle requires MCP `2026-07-28` with multi-round Elicitation for tool calls.
- A `2025-11-25` host may open/read sessions and append private proposals/checkpoints, but governed
  transitions return `elicitation_required`; compatibility never weakens user authority.
- Advertised server capabilities are tools and connection-scoped resources. Prompts, Roots,
  Sampling, Completion, Tasks, subscriptions, remote transport, and generic workflow operations are
  absent.
- The connection grant is reloaded on every request. Connection/session IDs are handles, not bearer
  credentials or evidence of authorization.
- The MCP adapter performs protocol/schema conversion only and calls the work-tracking application
  service. It must not import or invoke SQLite, `rusqlite`, SQL/schema modules, database paths,
  search indexes, repositories, or Markdown Vault adapters directly.
- The bridge must fail closed when the GUI process is absent and must never construct an
  application service, persistence adapter, projector, database, Vault, or semantic model.

Tool annotations are descriptive hints only. The server never treats them, model text, or an input
field such as `confirmed:true` as user consent.

## Tool: `inbound_work_open`

Create or resume the External Work Session for one external conversation.

```json
{
  "operationId":"logical-retry-key",
  "lineageKey":"host-scoped-opaque-conversation-id",
  "mode":"create",
  "capture":{
    "title":"Concise title",
    "summary":"Distilled intent",
    "userIntent":"Optional explicit intent",
    "sourceExcerpt":"Optional bounded user excerpt",
    "sourceTurnKey":"optional-turn-key"
  },
  "parentSessionId":"optional-follow-up-session-uuid"
}
```

The closed JSON Schema requires `operationId`, `lineageKey`, and `mode`. `create` requires
`capture`; `resume` forbids it. Before create, the server either uses the connection's explicit
session-start policy or performs the same Elicitation round-trip described below with action
`start_tracking`. The raw lineage key is never stored.

Create atomically commits one private Capture, session, revision-1 event, idempotency response, and
safe activity event. Resume returns the owned session's current head without changing state.

```json
{
  "sessionId":"session-uuid",
  "captureId":"capture-uuid",
  "headEventId":"event-uuid",
  "headRevision":1,
  "state":"active",
  "created":true,
  "deduplicated":false,
  "persistenceStatus":"durable",
  "projectionStatus":"queued",
  "resourceUri":"llm-wiki://work-session/session-uuid"
}
```

## Tool: `inbound_work_append`

Append one immutable, bounded event to an owned session. This tool records proposals and progress;
it never advances workflow state.

```json
{
  "operationId":"logical-retry-key",
  "sessionId":"session-uuid",
  "expectedHeadRevision":4,
  "sourceTurnKey":"optional-turn-key",
  "supersedesEventId":"optional-prior-event-uuid",
  "event":{}
}
```

`event` is exactly one closed variant:

- `problem_draft`: statement, detail, assumptions, open questions, provenance-labeled claims.
- `solution_draft`: title, outcome, non-goals, validation criteria, provenance-labeled claims.
- `work_log_checkpoint`: summary, decisions, changes, verification, blockers, and next steps.
- `conflict_proposal`: detected conflict, affected records, proposed resolution, and rationale.
- `completion_proposal`: outcomes, verification, residual risks, follow-ups, and selected evidence.

The canonical event is capped at 16 KiB; arrays contain at most 20 items; list strings are at most
1,000 characters. Claim provenance is `user_stated`, `observed`, or `assistant_inferred`. Absolute
filesystem paths, credentials, raw chain-of-thought, and full transcripts are rejected or safely
redacted. The caller cannot provide server time, author, decision, approval, workflow state,
priority, completion, or publication fields.

```json
{
  "sessionId":"session-uuid",
  "eventId":"event-uuid",
  "kind":"work_log_checkpoint",
  "revision":5,
  "headEventId":"event-uuid",
  "headRevision":5,
  "deduplicated":false,
  "persistenceStatus":"durable",
  "projectionStatus":"queued",
  "workflowState":"unchanged",
  "reviewState":"pending",
  "resourceUri":"llm-wiki://work-session/session-uuid",
  "nextAction":"accept_checkpoint"
}
```

Append validates active connection ownership, operation idempotency, and the durable head revision.
Its foreground transaction commits the immutable event, new head, idempotency response, and
projection outbox before returning. A background projector later reports `applied`, `ignored_late`,
`conflict`, or `failed`. Events append only; corrections use `supersedesEventId`. A pre-Solution
checkpoint stays private until a valid advancement decision makes it eligible. Completed sessions
reject append except for explicit follow-up creation.

## Tool: `inbound_work_advance`

Request one governed action. The tool never accepts a caller assertion that the user approved it;
it completes only through the MCP multi-round user-input flow.

Initial input:

```json
{
  "operationId":"logical-retry-key",
  "sessionId":"session-uuid",
  "expectedHeadRevision":7,
  "sourceEventId":"proposal-or-checkpoint-uuid",
  "action":"adopt_problem",
  "proposedPayload":{}
}
```

`action` is a closed discriminated union:

- `adopt_problem`: create/edit a draft Problem from the Capture and source proposal.
- `approve_problem`: apply the existing Problem approval gate.
- `adopt_solution`: create/edit a proposed Solution under the linked Problem.
- `resolve_conflict`: accept/edit/reject one explicit conflict resolution and rationale.
- `approve_solution`: apply conflict and Solution approval gates.
- `accept_checkpoint`: project exact selected progress/evidence into the Work Log.
- `accept_completion_proposal`: create/update the existing completion review from exact evidence.
- `verify_and_complete`: run final verification and create Completed Work through existing domain
  rules. This action never saves a Knowledge draft or publishes Knowledge.
- `link_current_work`: link the caller-owned tracked session to one lineage returned by the current
  Workbench resource, after exact user confirmation and a matching workspace/entity revision.

The schema for each action permits only fields accepted by its existing domain operation. One call
cannot combine actions or target an arbitrary entity. Required sequencing is exposed by the session
resource's `nextActions`.

### Multi-round Elicitation

If no valid response accompanies the first call, the server returns an `InputRequiredResult`:

```json
{
  "resultType":"input_required",
  "message":"Review the exact LLM Wiki change in this chat.",
  "requestedSchema":{
    "type":"object",
    "additionalProperties":false,
    "required":["decision"],
    "properties":{
      "decision":{"type":"string","enum":["accept","edit","reject"]},
      "editedPayload":{"type":"object"},
      "note":{"type":"string","maxLength":1000}
    }
  },
  "requestState":"opaque-authenticated-single-use-state"
}
```

The host renders the action name, exact proposed values, affected record, evidence/provenance,
validation results, conflicts, and consequences in the current Chat. Secret fields are never
requested. The retry uses a new JSON-RPC ID and returns `requestState` plus the user's structured
response according to the MCP 2026-07-28 MRTR/Elicitation contract.

Before commit, the server:

1. authenticates or resolves `requestState` and verifies protocol version, local principal,
   workspace, connection, session, action, source event, proposed payload hash, expected revision,
   expiry, and nonce;
2. rejects replay, cancellation, expiry, payload substitution, model-generated extra fields,
   revocation, cross-session access, and stale workflow state;
3. validates any edited payload against the action-specific closed schema and, when its meaning
   materially changes, issues a new exact preview instead of committing silently;
4. runs the existing domain authorization, conflict, approval, evidence, and completion
   gates; and
5. atomically consumes the challenge and commits the decision, workflow mutation, provenance link,
   new session revision, activity event, and idempotency result.

An accepted result returns affected record IDs, new session head, decision ID, workflow state,
resource URI, and next actions. Reject/cancel records the disposition but produces no authoritative
workflow mutation.

After `verify_and_complete`, the resource exposes `offer_knowledge_publication` as a conversational
next action. The Skill asks whether to publish; that question is UX guidance, not authorization.
Only the separate Knowledge tools and exact user-reviewed draft can publish.

## Tools: Vault Search and Evidence

These tools require independent application scopes and return only content visible inside the
selected `session`, `topic`, or `workbench` target. The default is the owned session; a tool
argument cannot expand the connection grant.

### `vault_search_lexical`

Input contains `query` (maximum 512 characters), a scope selector, bounded filters, `limit` (1–20),
and an optional opaque cursor. Output includes `searchRevision`, bounded hits with `evidenceId`,
title, kind, URI, snippet, matched terms, content revision, and modification time. It never returns
whole documents or counts from invisible scopes.

### `vault_search_semantic`

Input contains the same query/scope shape, `limit` (1–10), optional `minSourceRevision`, and cursor.
Output includes `sourceRevision`, `indexRevision`, `indexState` (`current` or `lagging`), and ranked
hits with the same source metadata. If the index is behind `minSourceRevision`, return
`semantic_index_not_ready` rather than presenting stale coverage as complete. Lexical search must
remain usable when semantic search is unavailable.

### `vault_evidence_read`

Input accepts at most eight `{evidenceId, expectedRevision}` entries returned by context/search and
an optional page cursor. It accepts no path, URL, or database key. The server caps one response at
24,000 characters. Every item returns a server-generated citation, exact revision, content hash,
and truncation/cursor metadata. Authorization is rechecked per item; changed content returns
`evidence_revision_changed`. Cursors bind principal, scope, query/evidence, snapshot revision, and
expiry and cannot be reused for another request.

The current-session AI calls both search tools for conflict review, distinguishes agreement,
contradiction, and insufficient evidence, and cites only returned passages. Search tools return
evidence; they never invoke a model or decide workflow state.

## Tools: Knowledge Draft and Publication

### `knowledge_draft_save`

Creates or appends a private draft version from one accessible Completed Work record. Input binds
`operationId`, work session/completion source, optional draft and expected draft revision, bounded
title/summary/Markdown, topic IDs, and exact `{evidenceId, revision, contentHash}` references. The
service revalidates every citation and source revision. Output returns draft ID/revision/content
hash and `status:"draft"`. Saving never publishes and arbitrary Vault paths are not accepted.

### `knowledge_publish`

Publishes exactly one reviewed draft using `operationId`, `draftId`, `expectedDraftRevision`, and
`expectedContentHash`; it accepts no content edits. It requires a scope distinct from draft save,
reruns publication/Vault conflict gates, and returns the same result for an exact idempotent retry.
Mismatch or external file change returns `publish_conflict` without overwrite. A caller field such
as `confirmed:true`, a Skill utterance, or a tool annotation is never consent.

## Resource: External Work Session

URI template:

```text
llm-wiki://work-session/{sessionId}{?sinceRevision}
```

`resources/list` returns only sessions owned by the configured connection. `resources/read` returns
bounded resume state:

```json
{
  "sessionId":"session-uuid",
  "capture":{"id":"capture-uuid","title":"...","summary":"..."},
  "headRevision":8,
  "state":"active",
  "linkedWorkflow":{
    "problem":{"id":"problem-uuid","title":"...","state":"approved"},
    "solution":{"id":"solution-uuid","title":"...","state":"approved"}
  },
  "recentCheckpoints":[],
  "pendingProposals":[],
  "recentDecisions":[],
  "nextActions":["accept_checkpoint","accept_completion_proposal"]
}
```

The projection always reflects committed in-app Chat and Workbench changes to the shared session
head. The projection contains at most eight cited passages and 6,000 retrieved-context tokens. It never
contains a full transcript, chain-of-thought, credentials, arbitrary private records, raw evidence
bodies, or unrelated sessions. Current status is always read fresh; the URI is not authorization.

## Resource: Current Workbench

URI:

```text
llm-wiki://workbench/current
```

This resource is advertised and readable only when the connection has the explicit
`workbench:current:read` scope. It gives workflow-oriented clients a bounded orientation view, not the
whole database or Vault:

```json
{
  "workspaceRevision":42,
  "activeSelection":{
    "entityType":"features",
    "entityId":"solution-uuid",
    "trackedSessionId":"optional-session-uuid"
  },
  "activeWork":[
    {
      "trackedSessionId":"session-uuid",
      "capture":"Original intent",
      "problem":{"title":"...","state":"approved"},
      "solution":{"title":"...","state":"approved"},
      "latestProgress":"...",
      "blockers":[],
      "pendingActions":["accept_checkpoint"],
      "sourceInterface":"in_app_chat",
      "headRevision":8,
      "updatedAt":"server-time"
    }
  ]
}
```

`activeWork` contains at most ten active tracked lineages, ordered by explicit active selection and
then recent user activity. Each item has at most one latest progress excerpt and no raw evidence
body or Chat message. If there is no active selection, the field is null and a client must not guess
among multiple candidates. Reading this resource grants no mutation or arbitrary session access.
To continue a lineage created by another interface, the external Chat opens its own session and
uses an exact user-confirmed link action through `inbound_work_advance`.

## Resource: Topic Context

URI template:

```text
llm-wiki://topic/{topicId}/current
```

This resource requires `topic:read` for the exact allowlisted topic. It returns bounded related
sessions, Problem/Solution decisions, latest checkpoints, open questions, and evidence references;
it never returns complete transcripts or all Workbench items. The current-session AI uses it when
the user explicitly selects related topic context. An opaque topic ID grants no access by itself.

## Resource: Whole Workbench Overview

URI template:

```text
llm-wiki://workbench/overview{?snapshotRevision,cursor,limit}
```

This read also requires `workbench:overview:read`. It is separate from the fast current-work resource so an
ordinary Chat resume does not load the whole board. The first page returns a stable snapshot
revision, complete aggregate counts, attention summaries, bounded recent completions, and the first
page of non-archived items:

```json
{
  "snapshotRevision":42,
  "summary":{
    "captures":3,
    "problems":2,
    "proposedSolutions":1,
    "inProgressSolutions":2,
    "blockedOrConflicted":1,
    "pendingDecisions":1
  },
  "attention":[{"entityRef":"opaque-ref","title":"...","reason":"conflict_review"}],
  "items":[{
    "entityRef":"opaque-ref",
    "kind":"solution",
    "title":"...",
    "category":"...",
    "state":"approved",
    "outcomeOrContext":"bounded text",
    "latestProgress":"bounded text",
    "nextAction":"...",
    "updatedAt":"server-time"
  }],
  "recentlyCompleted":[{"title":"...","outcome":"...","completedAt":"server-time"}],
  "nextCursor":"opaque-snapshot-cursor"
}
```

The client passes the same `snapshotRevision` with each cursor. Pages contain at most 50 items and
the snapshot cursor expires after a bounded period. A changed/expired snapshot returns
`snapshot_stale` rather than mixing revisions. Aggregate counts cover the complete snapshot, not
only the current page. `attention` includes every blocker, conflict, stale item, and pending user
decision in the snapshot, subject to a documented response-size ceiling; overflow is explicitly
reported and paginated rather than omitted.

The overview returns enough metadata to represent every visible non-archived item without reading
full bodies. It excludes Chat messages, raw evidence, archived document contents, deleted records,
credentials, settings, and arbitrary Vault files. Opaque references allow a later user-selected
detail/link flow but grant no mutation authority.

## Idempotency and transaction rules

- Mutation scope is `(connectionId, toolName, operationId)`.
- Canonical request hash and logical response commit with all created records.
- Same key/hash returns the original result; same key/different hash returns
  `idempotency_conflict` without a write.
- Challenges are independent single-use objects. Replaying a consumed response returns the original
  logical result only when the operation and request hashes match.
- A Workbench or in-app Chat mutation that advances the bound session/entity revision makes an
  outstanding external challenge stale; its retry returns the current head and writes nothing.
- Validation, authorization, cancellation, stale revision, expiry, rate limit, and transient
  storage errors do not consume an operation ID.
- Concurrent identical calls produce exactly one workflow mutation.
- Checkpoint append may acknowledge `queued`; consumers refresh the affected current/session
  resource to observe projection status. `ignored_late` events remain auditable and cannot update
  current state or become completion evidence.

## Application scopes and context selection

Scopes are launcher-granted application permissions, never caller-provided tool arguments:

- `session:read`, `session:write`
- `topic:read` with an allowlisted topic target
- `workbench:current:read`, `workbench:overview:read`
- `vault:search:lexical`, `vault:search:semantic`, `vault:evidence:read`
- `knowledge:draft:write`, `knowledge:publish`

Ordinary continuation reads the owned session. Topic context is read only after explicit selection
or request. Whole Workbench is read only for an explicit overview request and does not persist as a
silent broader default. Resource responses use private cache scope; current state is immediately
stale for caching (`ttlMs:0`). Phase 1 advertises neither resource subscriptions nor list-change
notifications, so clients refresh with bounded reads before decisions/writes and after conflicts.

## Errors

Recoverable failures return `isError:true`, a stable code, safe text, and only bounded recovery data:

| Code | Safe recovery data |
| --- | --- |
| `invalid_input` / `content_too_large` | bounded field errors |
| `elicitation_required` | required protocol capability and action |
| `elicitation_cancelled` / `elicitation_expired` | whether a new review may be requested |
| `elicitation_invalid` / `elicitation_replayed` | correlation ID only |
| `session_not_found_or_not_visible` | none; absence and denial are indistinguishable |
| `session_closed` | follow-up-session guidance |
| `head_conflict` | current head event/revision |
| `projection_pending` / `projection_conflict` | projection status and affected resource URI |
| `idempotency_conflict` | operation ID only |
| `semantic_index_not_ready` | source/index revision and bounded retry delay |
| `evidence_revision_changed` | current permitted revision |
| `draft_revision_conflict` / `publish_conflict` | current permitted revision and resource URI |
| `domain_gate_failed` | bounded unmet checks and allowed next actions |
| `rate_limited` | `retryAfterMs` |
| `storage_unavailable` | `retryable:true` and safe recovery guidance |
| `cancelled` | none |
| `internal_error` | correlation ID only |

Errors and logs never include conversation/checkpoint content, credentials, SQL, internal paths,
raw user responses, request-state plaintext, or inaccessible record existence.

## Explicitly unavailable MCP actions

There is no generic workflow/native-operation tunnel and no tool for arbitrary priority/stage
changes, combined completion/publication, unchecked publication, path-based patch apply, archive,
delete, provider/model access, job control, arbitrary entity read/write, settings mutation, or
arbitrary Vault file access. Allowlisted work and Knowledge actions require the relevant scope,
exact revisions/hashes, application-service validation, and user decision boundary.
