> **Partially superseded for schema 8+**: Connection security and exact review remain; Task snapshots
> and decisions in [012](../../012-task-centered-workbench/contracts/mcp-task-contract.md) replace
> Problem/Solution payloads and actions.

# Native Chat, Workbench, and Connection Contract

These allowlisted Tauri operations are available only to the LLM Wiki desktop UI. They support the
first-class in-app Chat input, active Workbench use, connection administration, and recovery. They
are not MCP tools; external Chat uses the equivalent MCP contract. Every query and mutation maps to
the same application service, event log, persistence ports, and projection rules. Native handlers
must not implement a second workflow path.

## In-app Chat work tracking

### `chat.work-session.open`

Starts or resumes structured tracking for the current in-app Chat after an explicit inline Capture
preview. It maps to the same domain intent as `inbound_work_open`, uses the local Chat identity, and
returns the same session/head shape plus `sourceInterface=in_app_chat`.

### `chat.work-session.append`

Records one bounded Problem/Solution/checkpoint/conflict/completion proposal associated with the
current Chat turn. It maps to the same validation, revision, idempotency, durable append, and
background-projection rules as `inbound_work_append`. A successful response may report
`projectionStatus=queued`; Chat message text is not copied wholesale.

### `chat.work-session.advance`

Consumes only an explicit accept/edit/reject action from a rendered inline milestone card. The card
shows the exact action, payload, affected record, evidence, conflicts, and consequences. The native
adapter binds the action to the authenticated window, Chat session, source event, payload hash, and
current revision, then invokes the same typed domain intent and gates as `inbound_work_advance`.
Model output, a hidden auto-action, or a caller-provided `confirmed` field cannot invoke it.

The in-app response shape matches MCP advancement: decision ID, affected records, session head,
workflow state, and next actions. This keeps both input interfaces behaviorally equivalent while
recording `decision_channel=in_app_chat` instead of `mcp_elicitation`.

## Connection operations

### `mcp.connections.list`

Returns active/revoked connections, checkpoint policy, supported protocol/capability requirements,
last-used time, and whether MCP is enabled. It returns no credentials or raw external identifiers.

### `mcp.connections.create`

```json
{
  "name":"External AI Chat",
  "profile":"work_tracking",
  "scopes":["session:read","session:write","workbench:current:read"],
  "checkpointPolicy":"allowed_for_started_sessions"
}
```

Response `201` includes the connection and display-only host configuration:

```json
{
  "connection":{"id":"connection-uuid","name":"External AI Chat","state":"active"},
  "configuration":{
    "command":"/absolute/installed/path/to/llm-wiki-desktop",
    "args":["--mcp","--connection","connection-uuid"]
  },
  "requiredHostCapability":"MCP 2026-07-28 tool-call Elicitation/MRTR",
  "scopes":["session:read","session:write","workbench:current:read"],
  "allowedRecords":["capture","drafts","work checkpoints","completion proposals","in-chat decisions"],
  "warning":"The external host may send conversation content to its own model provider."
}
```

The setup view grants session, topic, current/overview Workbench, lexical/semantic search, bounded
evidence, Knowledge draft, and Knowledge publication scopes independently and previews their
disclosure. It states that Capture→Completed Work finishes in the external Chat, publication is a
separate action, and Workbench is optional. It shows the exact command, transcript-minimization
policy, capability requirement, and unavailable actions. The connection ID is not a security
secret. Phase 1 setup is provided for Codex and ChatGPT desktop only, not ChatGPT web.

## Current-session AI context and evidence

### `work-tracking.context.read`

Reads one bounded `session`, `topic`, `workbench_current`, or `workbench_overview` projection using
the same application query as MCP Resources. Session is the default. Topic and whole-Workbench
scope require explicit user selection/request and their own permission.

### `vault.search.lexical`, `vault.search.semantic`, `vault.evidence.read`

These native commands map to the same scoped application queries and limits as their MCP tools.
In-app Chat passes returned evidence to its currently selected AI provider for conflict review,
Draft/Refine, and completion review. The server does not run a second AI. Citations must bind exact
evidence ID, revision, hash, and source passage; semantic lag or absence is represented as
insufficient evidence while lexical search remains available.

### `knowledge.draft.save`, `knowledge.publish`

These commands share the MCP Knowledge application commands. Draft save is private and
append-versioned. Publish accepts one exact reviewed draft revision/hash and uses a distinct user
action and permission. Neither arbitrary paths nor combined complete-and-publish actions exist.

### `mcp.connections.revoke`

Input: `{"connectionId":"connection-uuid"}`. Response `204` after commit. Repeated revocation is
idempotent. Existing private sessions remain until handled; revocation cannot erase external copies.

## Workbench session operations

### `work-tracking.sessions.list`

Returns bounded cards from both Chat sources with source interface, optional connection name,
Capture, linked Problem/Solution, head revision, last update, pending decision count, and next
actions. No external transcript is available.

### `work-tracking.sessions.get`

Returns the current session head, safe provenance, linked records, Work Log, accepted evidence,
pending proposals, decision history, and valid actions. Opening it does not change the session.

### `workbench.work-session.apply`

```json
{
  "sessionId":"session-uuid",
  "expectedHeadRevision":7,
  "operationId":"local-action-uuid",
  "action":"approve_solution",
  "sourceEventId":"optional-event-uuid",
  "payload":{}
}
```

`action` is a closed union of the same governed actions used by Chat plus direct structured actions:
`edit_capture`, `adopt_problem`, `edit_problem`, `approve_problem`, `adopt_solution`,
`edit_solution`, `resolve_conflict`, `approve_solution`, `append_checkpoint`,
`accept_checkpoint`, `accept_completion_proposal`, `verify_and_complete`, and
`create_follow_up`. `verify_and_complete` creates Completed Work only. Knowledge draft save and
publication use separate commands. Each variant has a closed, action-specific payload schema.

The Workbench form shows current source provenance, affected records, evidence, conflicts, and
consequences. An explicit user submit records `decision_channel=workbench`, invokes the same domain
gate as Chat, advances the same session head, and invalidates challenges/cards bound to an older
revision. It never creates a synthetic MCP challenge.

A stale request returns the current head and a field-level comparison while preserving unsaved
input in the UI. The client refreshes and retries at most once only for an identical idempotent or
provably non-overlapping intent. Overlapping or governed payload changes require renewed review;
there is no last-write-wins merge. Synchronous governed decisions commit through one unit of work;
checkpoint Work Log projection follows its durable outbox asynchronously. Repeating the same
operation ID and input returns the original result.

## Activity operations

### `mcp.activity.list`

Returns bounded content-free events with operation, outcome, duration, record/challenge IDs, time,
and safe error category. It never returns checkpoint content, prompts, raw user responses,
credentials, raw paths, request-state plaintext, or raw errors.

## UI behavior

- Workbench shows the current state from either Chat source and supports direct editing, review,
  Work Log updates, conflict resolution, completion, and follow-up work.
- Workbench is optional: its absence never blocks either Chat, and entering it never creates a copy
  or requires import/synchronization.
- The MCP session resource is the primary resume interface for the external Chat.
- Returning to either Chat reads the Workbench-updated head before presenting its next action.
- Workbench refreshes on focus, visibility, successful writes, and a bounded periodic interval.
  Chat refreshes before context reads, milestone previews, and mutations. Incremental updates must
  preserve unsent input, IME composition, caret/selection, scroll anchor, open disclosures, and a
  streaming response.
- In-app Chat retains its composer, history, streaming, cancellation, context routing, and
  completed-work conversations, and adds inline work milestone cards.
- One-shot `Draft with AI` and `Refine with AI` retain their preview/apply workflows.
- Conflict review and other AI work use the current Chat session's AI plus scoped lexical and
  semantic evidence. The application service supplies and validates evidence but does not choose a
  separate hidden model.
- After completion, Chat offers Knowledge publication once for that completion revision. Deferring
  leaves Completed Work intact and prevents unsolicited repeat prompts; later publication remains
  available from Chat or Workbench.
- Historical Chat remains visible in its existing UI but is never automatically converted into a
  structured session, Work Log, or Knowledge.
- All copy and accessibility labels are available in English and Korean, using `사용자` in Korean.
