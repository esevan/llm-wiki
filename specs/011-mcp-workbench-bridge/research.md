# Research: Dual-Chat Work Tracking

## Decision 1: Each opted-in conversation is a Tracked Work Session

**Decision**: Treat one explicitly tracked in-app or external AI conversation as one private Tracked Work Session. Native Chat actions, MCP, and Workbench are adapter-specific views/actions over the same record and revision. Workbench is optional but fully active.

**Rationale**: The user can remain where the work is happening while LLM Wiki supplies durable work state in the background. A stable session preserves continuity without importing the entire chat product or requiring navigation to Workbench.

**Alternatives considered**: Storing each message as a Capture creates noise and loses a coherent branch. Mirroring a full transcript recreates Chat storage and increases privacy risk. Attaching directly to arbitrary existing Solutions makes lineage and authority ambiguous.

## Decision 2: Separate work, retrieval, and Knowledge capabilities

**Decision**: Phase 1 advertises work-session open/append/advance, separate lexical and semantic Vault search, bounded evidence read, Knowledge draft save, and Knowledge publish. Resources provide bounded session, topic, current-Workbench, and whole-Workbench context. Every mutation and read calls the application service; the MCP adapter has no repository, SQL, database-path, index, or Vault-file access.

**Rationale**: Resource reads are application-driven context, while search and mutations are explicit model-controlled operations with materially different latency and scope. Separate lexical and semantic tools expose fallback and index freshness instead of hiding them behind one score. Session/evidence handles remain identifiers, not authorization ([stateful tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools#stateful-tools)).

**Alternatives considered**: One generic tool obscures authority and freshness semantics. Always loading the whole Workbench overexposes context. Direct database access from MCP creates a second persistence boundary and bypasses invariants.

## Decision 3: Require explicit start consent, not passive transcript capture

**Decision**: Starting through either interface requires an exact in-chat Capture confirmation or a user-configured start policy. Routine checkpoints may auto-append only inside that opted-in session, but any checkpoint used as authoritative evidence and every workflow transition requires exact in-chat review.

**Rationale**: MCP tools are model-controlled. Tool annotations are usability hints, not proof of authorization or consent ([Tools security model](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)). A model-supplied `confirmed: true` field is therefore never accepted as consent.

**Alternatives considered**: Recording every conversation after connection overcaptures unrelated work. Requiring Workbench confirmation defeats the no-context-switch goal. Treating the model's statement that the user agreed as sufficient fails the human-authority boundary.

## Decision 4: Use MCP Elicitation as the in-chat decision boundary

**Decision**: `inbound_work_advance` uses the MCP 2026-07-28 multi-round tool-result pattern. Its first call returns `InputRequiredResult` with a bounded form and opaque `requestState`. The host presents the exact action and editable payload in the current Chat. On retry, the server validates the response and request state, rechecks domain gates, consumes the challenge once, and commits atomically. Hosts without the required capability receive `elicitation_required` and cannot perform governed transitions.

**Rationale**: The current protocol supports user input during tool calls through Elicitation and MRTR ([Elicitation](https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation), [multi-round tool results](https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr)). `requestState` is returned through the client and is therefore protected with authenticated encryption or an HMAC-backed server record binding principal, connection, workspace, session, action, payload hash, revision, expiry, and nonce. This lets the decision happen in Chat without trusting the model as the approver.

**Alternatives considered**: Requiring Workbench review for external Chat violates the no-context-switch workflow, though optional Workbench action remains valid. A plain confirmation field is forgeable by the caller. Tool annotations do not establish consent. Falling back on older hosts would silently weaken authority, so compatibility is record-only.

## Decision 5: Store distilled checkpoints, not chain-of-thought or complete chat

**Decision**: The initial Capture contains a short title/summary and optional user-stated excerpt. Later immutable checkpoints record bounded summaries, decisions, changes, observed results, evidence labels, validation, blockers, and next steps. Each claim is marked `user_stated`, `observed`, or `assistant_inferred`.

**Rationale**: Resume value comes from decisions and evidence, not conversational filler. Provenance prevents assistant inference from becoming a user decision or observed fact. Raw chain-of-thought, credentials, provider prompts, and full transcript are prohibited.

**Alternatives considered**: Full transcripts maximize recall but increase storage, disclosure, and review burden. Summary-only records without provenance make fluent inference indistinguishable from evidence.

## Decision 6: Durably append first, then project in the background

**Decision**: Each append supplies `operationId`, `sessionId`, and `expectedHeadRevision`. The foreground transaction commits canonical input hash, immutable event, new head, projection outbox, content-free activity, and logical response before acknowledging `accepted/queued`. A supervised background projector materializes workflow/Workbench/search state idempotently and recovers leased work after restart. Same key/hash returns the same response; same key/different hash fails.

**Rationale**: JSON-RPC request IDs are transport correlation, not business idempotency. MCP hosts retry after uncertain timeouts, and concurrent desktop/MCP processes require deterministic conflict behavior. The current MCP protocol is stateless per request, so no authorization or head state is inferred from the process ([versioning model](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)).

**Alternatives considered**: Last-write-wins loses user edits. An in-memory queue can lose acknowledged checkpoints. Immediate direct writes from each adapter create different concurrency rules. Mutable event rows erase the historical explanation of a changed proposal.

## Decision 7: Proposals advance only after a bound in-chat decision

**Decision**: `problem_draft`, `solution_draft`, `work_log_checkpoint`, and `completion_proposal` events carry no authority. A valid decision may adopt or approve structure, resolve a conflict, accept evidence, or verify and complete work by invoking existing domain rules. Completion produces Completed Work only. Knowledge draft save and publication are later, separate governed actions. Progress before Solution adoption stays pending; accepted progress is projected into the existing Work Log with provenance.

**Rationale**: AI may organize and propose, while the user owns workflow state through the current Chat. Tracked Work Session remains provenance metadata, not another stage below Solution. Accepted reports can enter deterministic Lineage; rejected/inferred material remains private history.

**Alternatives considered**: A standing write grant for generic transitions violates human authority. Workbench-only materialization adds a required context switch. Treating every checkpoint as completion evidence lets unreviewed claims influence publication.

## Decision 8: Keep two Chat inputs and active Workbench over one domain service

**Decision**: Retain LLM Wiki's chat composer/history, streaming Tauri command, conversation context builder, Channel/cancellation adapter, and completed-work Chat. Add explicit milestone cards to in-app Chat. MCP maps the same intents from external Chat, while Workbench exposes direct structured view/edit/review actions. All three adapters use one work-tracking service and compare-and-swap session head; the differences are UX, transport, consent proof, and provenance.

**Rationale**: In-app Chat and external Chat serve different conversational contexts, while Workbench is better for overview and direct structured editing. Sharing typed domain intents prevents divergent lifecycle rules. A Workbench commit advances the same session head; stale Chat actions reload and reconcile instead of overwriting. Old messages remain conversation history and are not automatically reinterpreted as structured work.

**Alternatives considered**: Removing in-app Chat discards a useful native interface. A read-only Workbench blocks users who prefer structured continuation. A separately synchronized Workbench copy risks drift and duplicate decisions. Auto-converting old chats changes their meaning without consent.

## Decision 9: Bridge local stdio into the active GUI process

**Decision**: The host launches `<executable> --mcp --connection <id>`. `main.rs` selects a thin
stdio bridge before Tauri startup. The bridge connects to the already-running GUI over a
user-protected Unix domain socket on macOS/Linux or named pipe on Windows. The GUI constructs the
MCP server with its existing application service. Use the official `rmcp` 3.x SDK, MCP
`2026-07-28` baseline, and `2025-11-25` compatibility. stdout is protocol-only; logging uses
content-free stderr; EOF exits promptly. If the GUI is absent, the bridge fails closed.

**Rationale**: stdio remains the host-facing MCP transport
([stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio)).
The private local IPC hop makes the active GUI process the single persistence owner, rather than
letting every MCP child create another SQLite/Vault adapter and projector. No TCP/HTTP listener,
Node/Python runtime, or second persistence topology is introduced. The Rust SDK continues to avoid
hand-written dual-era protocol handling
([official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)).

**Alternatives considered**: Loopback HTTP adds authentication, Origin, port, daemon, and firewall concerns. A separate runtime duplicates packaging and application boundaries.

## Decision 10: Scope identity and permissions per connection/session

**Decision**: `(connection_id, lineage_key_hash)` uniquely identifies a session. Session IDs and lineage keys are opaque handles, never bearer credentials. Every call reloads the active connection grant and verifies session ownership. No similarity matching or automatic attachment to existing workflow records is allowed.

**Rationale**: Two AI hosts may reuse the same conversation ID. Explicit connection scoping prevents collision, while a valid in-chat decision is the safe place to link an existing or newly created Problem/Solution.

**Alternatives considered**: Global external IDs collide. Model-selected entity IDs permit cross-session mutation. Semantic auto-linking can silently merge unrelated work.

## Decision 11: Add bounded Vault retrieval for current-session AI

**Decision**: Phase 1 includes independently scoped `vault_search_lexical`, `vault_search_semantic`, and `vault_evidence_read`. Searches return bounded snippets and opaque evidence IDs; evidence reads enforce exact revision, authorization, and total response limits. The AI in the current Chat—not the MCP server—uses these results for conflict review, refinement, completion review, and other synthesis.

**Rationale**: Conflict review constitutionally requires portable Knowledge, and the current-session AI has the live conversation. Lexical search remains available when semantic indexing is unavailable; semantic results expose source/index revision so incomplete coverage cannot be mistaken for clearance. Server-generated citations make evidence inspectable.

**Alternatives considered**: Whole-Vault dumps risk private-note disclosure and waste context. A hidden server-side model splits judgment from the current conversation. One hybrid endpoint conceals fallback and freshness differences.

## Decision 12: Ship one representative workflow Skill

**Decision**: Provide `.agents/skills/llm-wiki-workflow` as canonical guidance for ChatGPT desktop and Codex, then package it with the local MCP connection. Ordinary resume uses session/current scope; topic and whole-Workbench expansion are explicit. Conflict review follows a dedicated lexical-plus-semantic evidence procedure and is synthesized by the current-session AI. After completion, the Skill asks once whether to publish Knowledge and invokes separate draft/publish actions only after user review.

**Rationale**: MCP schemas describe valid calls but do not teach an agent when to retrieve current work, how to turn Capture/Problem/Solution/Work Log into natural dialogue, or how to avoid guessing among lineages. A concise Skill makes the intended experience portable and testable. OpenAI's product model treats Skills as reusable guidance for both ChatGPT and Codex and plugins as the installable bundle for Skills plus MCP connectors.

**Alternatives considered**: Embedding all guidance in tool descriptions consumes context on every call and weakens discovery. A prompt resource would add another protocol surface and remain host-dependent. Allowing semantic auto-selection risks attaching work to the wrong lineage.

## Decision 13: Order events by a trusted per-stream watermark

**Decision**: The application boundary assigns authoritative `occurred_at`; caller/model timestamps remain descriptive `observed_at`. Projection order is deterministic within each server-issued `(session, stream)` using `(occurred_at, source_sequence, event_id)`. An event at or below the applied watermark is retained for audit but marked `ignored_late` and excluded from current state, Lineage, and completion evidence.

**Rationale**: A global watermark lets one source suppress another, while trusting model time permits state regression. Per-stream ordering and immutable late-event retention produce deterministic current state without erasing provenance.

**Alternatives considered**: Arrival order cannot honor delayed trusted events. Global timestamps are vulnerable to clock skew and unrelated-source interference. Deleting late events damages auditability.

## Decision 14: Use fine-grained application scopes and minimal context

**Decision**: Local connection grants distinguish session read/write, topic read, current/overview Workbench read, lexical/semantic search, evidence read, Knowledge draft, and Knowledge publish. Default Chat context is its owned session; topic scope is selected explicitly; whole-Workbench scope is one-request and explicit. All context is private and bounded to eight passages/6,000 tokens for evidence-backed AI work.

**Rationale**: The context cost and disclosure risk differ sharply between continuing one conversation and summarizing an entire workspace. Tool arguments cannot grant their own permissions.

**Alternatives considered**: A single `workbench_read` grant is too coarse. Always retrieving everything increases latency, token use, and private-data exposure.

## Decision 15: Refresh reads naturally and retry only unchanged intent

**Decision**: Workbench refreshes on focus/visibility, after writes, and on a bounded interval. Chat refreshes before a contextual read, milestone preview, or mutation and applies incremental UI updates without resetting input, IME, focus, selection, scroll, or streaming. On CAS conflict, the client refreshes and automatically retries at most once only when the exact intent is idempotent or provably non-overlapping; changed governed content returns to review.

**Rationale**: This keeps multi-surface state fresh without interrupting conversation or silently rebasing a decision the user did not review.

**Alternatives considered**: Push subscriptions add Phase 1 complexity. Blind polling can disrupt Chat. Blind revision replacement turns stale approval into authority over changed content.

## Decision 16: Limit Phase 1 to local desktop hosts

**Decision**: Phase 1 supports Codex and ChatGPT desktop through local stdio. ChatGPT web, remote HTTP transport, remote OAuth, and server deployment are deferred.

**Rationale**: Local stdio shares the supervised application boundary and avoids introducing a second authentication, lifecycle, and persistence topology before the core workflow is proven.

**Alternatives considered**: Shipping web simultaneously multiplies transport and authorization risk and is not required by the clarified release goal.
