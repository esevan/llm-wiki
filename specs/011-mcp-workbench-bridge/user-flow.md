# User Flow: Two Chat Inputs, One Work Lifecycle

## Product rule

The user may stay in the Chat where the work started, or open Workbench whenever structured
overview/editing is more useful. LLM Wiki turns explicit conversational milestones into durable
work records in the background. Workbench is available but never required.

```text
In-app Chat ── native inline cards ──┐
                                    ├─ Tracked Work Session
External Chat ── MCP Elicitation ───┘   Capture → Problem → Solution → Work Log → Completed Work
                                              ▲
                                              └── Workbench direct view/edit/review
```

All three surfaces use the same application service, durable event log, projections, revision
checks, completion gates, and Lineage. The
two Chat paths differ only in connection setup, transport, proof of user action, and provenance;
Workbench is the structured direct-manipulation surface over the same session.

## First use

### In-app Chat

1. The user opens or continues an LLM Wiki Chat.
2. The user says, for example, “이 작업을 추적해줘,” or chooses **Track this work**.
3. Chat shows an inline Capture card with the distilled title and intent.
4. The user accepts, edits, or cancels the card. Acceptance creates the Tracked Work Session.

No connection setup is needed because the authenticated desktop Chat is the local input adapter.

### External Chat through MCP

1. The user creates a revocable LLM Wiki connection once and adds its stdio configuration to the
   external Chat host.
2. In that Chat, the user says, for example, “LLM Wiki에서 이 작업을 추적해줘.”
3. LLM Wiki returns an MCP Elicitation request containing the exact Capture preview.
4. The host renders accept/edit/reject controls in the current Chat. Acceptance creates the same
   kind of Tracked Work Session.

If the host lacks the required Elicitation capability, governed actions stop with
`elicitation_required`. The flow does not send the user to Workbench.

## Common lifecycle in the chosen Chat

When the external user asks “현재 작업을 이어서 해줘” without a known session, the bundled
`llm-wiki-workflow` Skill reads the permission-scoped current Workbench projection. It prefers the
explicit active selection; if several items are plausible, it asks the user to choose rather than
guessing. It then turns the selected lineage into a short conversational catch-up—current intent,
latest meaningful progress, one blocker or pending decision, and the next useful move—rather than
dumping the Workbench columns or internal identifiers into Chat.

If the user explicitly asks for the entire Workbench, the Skill switches to overview mode. It reads
one complete paginated snapshot and presents: overall stage counts, everything needing attention,
all active Solutions, compact coverage of Captures/Problems/proposed Solutions, recent completions,
and one useful question about what to open next. An ordinary resume does not pay this larger context
cost.

Context is scope-aware. Ordinary continuation reads only the owned session. The user may expand to
one topic for related sessions/evidence, or explicitly request the whole Workbench. The Skill never
silently upgrades a smaller scope or treats a returned handle as authorization.

### 1. Capture intent

The assistant distills the current goal, scope, and optional user excerpt. The user reviews the
Capture once. The full transcript is not copied into the work record.

Result: private Capture + tracked session + source-interface/session provenance.

### 2. Shape the Problem

When the need becomes clear, the assistant presents a Problem milestone card containing the exact
statement, context, assumptions, open questions, and provenance-labeled claims.

The user chooses:

- **Accept**: create the draft Problem, followed by a separate approval card when required.
- **Edit**: revise the fields in Chat, preview the materially changed payload, then accept.
- **Reject**: retain the proposal in the trace but create no Problem.

### 3. Shape and approve the Solution

The assistant proposes the outcome, non-goals, validation criteria, and evidence. LLM Wiki runs the
existing conflict checks and shows any conflict plus its proposed resolution inside Chat. The user
resolves the conflict and approves the Solution through explicit, separate actions.

Before approval, the AI in the current Chat runs both bounded lexical and semantic Vault searches.
It cites returned passages, identifies agreement, contradiction, and missing evidence, and proposes
a resolution without deciding it. Semantic index lag is shown as insufficient evidence; lexical
search remains available. Result: Capture→Problem→Solution lineage with the same rules through
either input interface.

### 4. Track work continuously

During execution, the assistant appends concise checkpoints: changes, decisions, observed results,
artifacts, blockers, and next steps. Routine checkpoints may be recorded automatically only after
the session has opted in. A checkpoint that will count as authoritative completion evidence is
shown for accept/edit/reject first.

Accepted checkpoint logging first commits the immutable event and projection outbox, then returns
quickly as queued. A background projector materializes Work Log/current views. If an older event
arrives after the stream watermark, it remains visible in audit history as late but cannot roll the
current state backward or become completion evidence.

The user may ask “지금 어디까지 했어?” at any time. Chat reads the bounded session projection and
returns the current Problem, Solution, recent checkpoints, blockers, and next decision.

### 5. Complete the work

The assistant presents a completion card with claimed outcomes, validations, selected evidence,
residual risks, and follow-ups. Rejecting it keeps the Solution active. Accepting it enters final
verification; **Verify and complete** runs completion gates and produces private Completed Work.
It does not write Knowledge.

After completion, the Skill naturally asks once, “Knowledge로 발행할까요?” If the user defers,
Completed Work remains intact and no file is written. If the user agrees, Chat first saves/previews
an exact Knowledge draft and then performs a separate publication action against that draft's
revision and content hash. External Vault changes return to patch review rather than overwrite.

Result: completion and publication have separate state, decision, and audit records. No model-only
message can complete or publish work.

### 6. Resume or follow up

The same Chat resumes from its tracked session. A restored external host uses the stable session
identity and MCP resource; in-app Chat uses its local Chat identity. Closed work is immutable.
Additional work starts an explicit follow-up session linked to the Completed Work.

## Switch to Workbench at any time

Opening Workbench requires no import or sync action. It reads the current session head and shows
the Capture, linked Problem/Solution, Work Log, evidence, pending proposals, and next valid actions.
The user can edit structure, decide proposals, add progress, resolve conflicts, or complete the work
there through the same domain service.

Each Workbench commit advances the shared session revision. Workbench refreshes on focus,
visibility, writes, and a bounded interval; Chat refreshes naturally before context reads,
milestones, and saves without resetting input, IME, focus, scroll, or streaming. If Chat and
Workbench started edits from the same revision, the first valid commit wins. The other surface
preserves unsaved input, reloads the current head, and retries once only when the original intent is
unchanged and non-overlapping; otherwise it asks the user to reconcile. A pending MCP Elicitation
challenge tied to the previous revision becomes stale.

## Interface differences

| Concern | In-app Chat | External Chat |
| --- | --- | --- |
| Setup | Already available in LLM Wiki | One-time revocable MCP connection |
| Conversation | Existing streaming/history UI | Host's own Chat UI |
| User decision proof | Explicit native inline-card action | MCP Elicitation + signed single-use request state |
| Resume identity | Local Chat identity | Connection-scoped host conversation identity |
| Provenance | `in_app_chat` | `external_mcp_chat` + connection |
| Unsupported capability | Not applicable | Governed action returns `elicitation_required` |
| Domain behavior | Shared work-tracking service | Shared work-tracking service |

## Workbench's role

Workbench is an optional active workspace, not a read-only mirror. It reflects both Chat sources and
supports direct structured work, review, completion, audit, and recovery. The happy path does not
require opening it, but entering it never breaks or forks the tracked session.
