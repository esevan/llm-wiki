# Continue Task work from Codex and ChatGPT desktop

**English** | [한국어](mcp-workbench-bridge.ko.md)

An approved local MCP connection can read and propose changes to the same Task-centered work records as the desktop Workbench. It works through explicit connection scopes and topic membership; it does not expose the Vault, app database, or credentials as unrestricted files.

## Task proposals and review

A chat can capture a thought, propose a Task action, add a checkpoint, or propose a completion or Knowledge action. The proposal identifies its Task and expected revision. When Problem context is needed, it carries an exact Problem revision as optional provenance; a Task does not need a Problem parent.

The host presents the exact proposal for review. The user can accept, reject, edit, defer, or request another review where the available action permits it. A proposal never changes the Task merely because an AI generated it. Stale, cancelled, expired, and rejected proposals remain auditable without replacing current state.

## Direct Task continuation and synchronization

Both inputs use the same application service, durable event log, workflow records, Vault, and
background projector. Starting tracking first creates a server-issued preview; only accepting its
exact content creates the private Capture and tracked session. A user may also continue an existing
Task directly: reviewed `inbound_work_open` with `mode:"continue_task"` and `taskId` creates a
connection-owned captureless session without inventing Capture provenance; cancellation creates no session. Reviewed
Task proposals and exact Problem-link decisions update the normal Workbench records, so opening Workbench later
shows the same state and changes made there are visible on the next fresh Chat read.

Accepted Task mutations and their tracking event, session head, and applied projection result commit
atomically. Desktop Task changes also advance linked sessions once per operation; a retry adds no
duplicate event. Other event projections can run in the background and briefly appear as queued.
Meaningful checkpoints are committed to the event log before the Chat receives confirmation. Per-stream
watermarks use application timestamps and deterministic ordering; late events remain auditable but
cannot rewind current state.

## Limits and continuity

Workbench, in-app chat, and local MCP use the same application boundary, so accepted actions receive the same revision checks and persistence rules. Completing tracked chat work does not publish Knowledge. Knowledge drafting and publication remain separate explicit decisions.

The current bridge is local, stdio-based integration for supported desktop hosts. It does not promise ChatGPT web, remote MCP, automatic task selection, unrestricted cross-topic access, or automatic approval of AI proposals.

## Set up a local connection

Open **AI setup** and use the **Chat connections** section. Enter a connection name, choose only the needed grants, and optionally list allowed topic IDs; create the connection. Expand its **Granted access** details to review the exact grants and the local command shown there: `llm-wiki-desktop --mcp --connection <id>`. Use that command in a supported local desktop host's stdio MCP configuration. If the executable is not on PATH, use the installed application's full executable path. Keep the GUI running: the stdio bridge forwards to it and fails closed when it is unavailable. Manage topic membership from the same section, and use **Revoke** to disable a connection immediately.

See [Task-centered Workbench](conflict-gated-workflow.md) and the current historical context in [specification 012](../../specs/012-task-centered-workbench/spec.md).

## Human review and conflict checks

Task transitions, exact Problem links/resolution, conflict review, completion, and publication require an exact review.
Modern MCP hosts use multi-round Elicitation, bound to the connection, session, source event,
payload, revision, expiry, and a single-use challenge. Caller fields such as `confirmed` never
grant authority.

The AI already serving the current Chat performs conflict reasoning. LLM Wiki supplies bounded
lexical and semantic search plus revision-checked evidence passages; the MCP server does not run a
hidden model. If semantic coverage is unavailable, lexical search remains usable and the AI must
describe the evidence as incomplete. Search results issue opaque, connection- and scope-bound
evidence handles that expire after ten minutes; evidence reads recheck ownership and source revision.

## Completion is not publication

Completing a Task creates private Completed Work and no Knowledge file. The workflow
skill then asks once: “Publish this as Knowledge?” A yes first creates a private, versioned draft.
Publishing is a second explicit action that accepts only the reviewed draft ID, revision, and
content hash. External Markdown changes cause a conflict and are never overwritten.
Withdrawing a publication is another exact review: the file moves to a non-indexed local recovery
copy while Completed Work and its decision history remain intact.

Task Work Log, refinement, relationships, and Problem links remain separate decisions; conflict
review is advisory and does not block Task work. New Knowledge uses the canonical Task draft flow
with an immutable lineage snapshot; historical `knowledge_drafts` records remain readable as legacy
history. Retained Knowledge tool aliases use the same canonical Task DTOs and review wrappers.
Canonical refinement, advisory, lineage, and Knowledge tools use the `task_refinement_*`,
`task_advisory_*`, `task_context_read`, `task_lineage_read`, and `task_knowledge_*` names. Body/content hashes remain
separate from source/lineage hashes.
Knowledge correction, publication, and withdrawal require both the exact draft `expectedContentHash`
and the exact lineage `expectedSourceHash`; a missing source hash is rejected rather than guessed.
Historical drafts retain their recorded source hash. `task_context_read` is bounded to 4,000 characters
and 100 array items per field, and binary attachments remain local-only rather than crossing MCP.

See the [Task MCP contract](../../specs/012-task-centered-workbench/contracts/mcp-task-contract.md),
the retained [transport contract](../../specs/011-mcp-workbench-bridge/contracts/mcp-server.md), and
the [remediation verification record](../testing/mcp-task-workflow-verification.md).
