# Work tracking from Codex and ChatGPT desktop

[한국어](mcp-workbench-bridge.ko.md) | **English**

LLM Wiki can track work from its in-app Chat or a local MCP-capable Chat without requiring the
Workbench screen to be open. Codex and ChatGPT desktop are supported in the first release;
ChatGPT web and remote MCP transports are not.

## One workflow, two Chat inputs

Both inputs use the same application service, durable event log, workflow records, Vault, and
background projector. Starting tracking first creates a server-issued preview; only accepting its
exact content creates the private Capture and tracked session. Reviewed
Problem and Solution proposals update the normal Workbench records, so opening Workbench later
shows the same state and changes made there are visible on the next fresh Chat read.

Meaningful checkpoints are committed to the event log before the Chat receives confirmation.
Workbench materialization happens in the background and may briefly appear as queued. Per-stream
watermarks use application timestamps and deterministic ordering; late events remain auditable but
cannot rewind current state.

## Connect a desktop Chat

Open **AI setup → Chat connections**, create a named connection, review its scopes, and configure
the local host with the displayed connection ID. The packaged MCP command is:

```text
llm-wiki-desktop --mcp --connection <connection-id>
```

The bundled plugin can instead read `LLM_WIKI_CONNECTION_ID`. The desktop host talks MCP over
stdio to a thin bridge. That bridge forwards through a user-only Unix domain socket on macOS/Linux
or named pipe on Windows to the running LLM Wiki GUI process. The GUI process is the sole owner of
the application service, SQLite, Vault adapter, and projector. Open LLM Wiki before invoking MCP.
A connection can be revoked immediately from settings; session IDs are not credentials.

Scopes independently control session reads/writes, topic or current/whole-Workbench summaries,
lexical search, semantic search, evidence reads, Knowledge drafts, and Knowledge publication.
Topic access is additionally limited to the topic IDs selected when the connection is created.
Topic membership is managed explicitly in AI setup: matching words in a title or document never
add an item implicitly, and removing membership revokes evidence handles issued for it.
Ordinary continuation uses the current session; the whole Workbench is read only when explicitly
requested.

## Human review and conflict checks

Problem, Solution, conflict, completion, and publication transitions require an exact review.
Modern MCP hosts use multi-round Elicitation, bound to the connection, session, source event,
payload, revision, expiry, and a single-use challenge. Caller fields such as `confirmed` never
grant authority.

The AI already serving the current Chat performs conflict reasoning. LLM Wiki supplies bounded
lexical and semantic search plus revision-checked evidence passages; the MCP server does not run a
hidden model. If semantic coverage is unavailable, lexical search remains usable and the AI must
describe the evidence as incomplete. Search results issue opaque, connection- and scope-bound
evidence handles that expire after ten minutes; evidence reads recheck ownership and source revision.

## Completion is not publication

Completing a tracked session creates private Completed Work and no Knowledge file. The workflow
skill then asks once: “Publish this as Knowledge?” A yes first creates a private, versioned draft.
Publishing is a second explicit action that accepts only the reviewed draft ID, revision, and
content hash. External Markdown changes cause a conflict and are never overwritten.
Withdrawing a publication is another exact review: the file moves to a non-indexed local recovery
copy while Completed Work and its decision history remain intact.

See the [feature specification](../../specs/011-mcp-workbench-bridge/spec.md) and
[MCP contract](../../specs/011-mcp-workbench-bridge/contracts/mcp-server.md) for the exact protocol.
