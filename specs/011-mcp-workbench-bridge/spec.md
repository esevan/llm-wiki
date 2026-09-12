> **Partially superseded for schema 8+**: Append-only events, scopes, cursors, expiring reviews, and
> replay remain. Problem/Solution proposal and gate semantics are replaced by the shared Task aggregate
> and Task events in [012](../012-task-centered-workbench/spec.md).

# Feature Specification: Dual-Chat Work Tracking

**Feature Branch**: `feat/mcp-support`

**Created**: 2026-09-04

**Status**: Draft

**Input**: User description: "Workbench에 가지 않고 현재 Chat session 안에서 Capture, Problem, Solution, Completed Work까지 자연스럽게 추적하고 결정하고 싶다. 인앱 Chat과 MCP 외부 Chat은 서로 다른 두 입력 인터페이스로 모두 지원한다."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start Tracking Without Leaving the Current Chat (Priority: P1)

A user works with an AI in an MCP-capable Chat application and asks it to track the work in LLM
Wiki. Without opening Workbench, the assistant creates a private Capture and shows the proposed
Problem and Solution for review inside the current Chat. The user can confirm, edit, or reject each
proposal there.

**Why this priority**: The external Chat is the user's active workspace. LLM Wiki should supply a
durable work model in the background, not force a context switch to another interface.

**Independent Test**: In one supported Chat session, start tracking, confirm a Capture, edit and
adopt a Problem draft, and adopt a Solution draft. Verify through the MCP session resource and
stored records that one continuous lineage exists even though Workbench was never opened.

**Acceptance Scenarios**:

1. **Given** an active connection and a user who asks to track the conversation, **When** LLM Wiki requests confirmation, **Then** the current Chat shows the exact Capture preview and records it only after the user accepts it.
2. **Given** the conversation clarifies a Problem, **When** the assistant proposes a structured draft, **Then** the user can accept, edit, or reject the exact draft inside the current Chat and the accepted payload becomes the draft Problem.
3. **Given** a user-adopted Problem, **When** the assistant proposes a Solution, **Then** conflict information and the exact Solution are reviewed and approved inside the current Chat before workflow state changes.
4. **Given** the same external request is retried, **When** it carries the same request identity and content, **Then** LLM Wiki returns the existing result and creates no duplicate session, Capture, proposal, decision, or workflow entity.

---

### User Story 2 - Track and Resume Work in Chat (Priority: P1)

As the external AI performs work, it records concise checkpoints, decisions, evidence, artifacts,
and validation outcomes against the Solution. The user can later resume in the same or a restored
Chat session and receive the current Problem, Solution, progress, and next decision without opening
Workbench or recovering a full transcript.

**Why this priority**: The durable record must support continuity across context windows, host
restarts, and long-running work while keeping the Chat as the sole interaction surface.

**Independent Test**: Append several checkpoints, restart both applications, resume by the stable
external session identity, and verify that the Chat can summarize the ordered, source-labeled Work
Log and continue from the correct revision.

**Acceptance Scenarios**:

1. **Given** an external session linked to an adopted Solution, **When** the assistant records a checkpoint, **Then** it is added to the Solution Work Log with summary, evidence, validation, origin, and timestamp.
2. **Given** an external session has no adopted Solution, **When** the assistant records progress, **Then** it stays pending on that session and cannot mutate an unrelated Solution.
3. **Given** the user resumes later, **When** the assistant reads the session resource, **Then** the current Chat can explain the Problem, Solution, latest accepted progress, pending proposals, and next user decision from bounded state.
4. **Given** a checkpoint contains an assistant inference or ambiguous evidence, **When** it would affect completion evidence, **Then** the current Chat asks the user to accept, edit, or reject that exact checkpoint before projection.

---

### User Story 3 - Complete, Then Choose Whether to Publish (Priority: P1)

When the current-session AI believes the work is done, it submits a completion proposal containing
the claimed outcome, evidence, validation, risks, and follow-ups. LLM Wiki presents the exact
proposal and completion checks inside the current Chat. Completion produces Completed Work only.
After that succeeds, the workflow Skill asks whether the user wants to publish it as portable
Knowledge; publication is a separate, explicitly reviewed decision.

**Why this priority**: Finishing private work and publishing reusable Knowledge have different
consequences. Both must remain possible in the current Chat without collapsing them into one action
or turning a model-authored confirmation flag into human approval.

**Independent Test**: Submit, reject, revise, and finally approve a completion proposal entirely in
one Chat. Verify that the accepted action produces Completed Work but no Knowledge, that the Skill
then asks once whether to publish, and that only a second exact confirmation publishes Knowledge.

**Acceptance Scenarios**:

1. **Given** a completion proposal, **When** it is first submitted, **Then** the Solution remains incomplete and the current Chat displays the exact claimed outcomes, evidence, unmet checks, risks, and source session.
2. **Given** the user rejects or edits the proposal in Chat, **When** that response is processed, **Then** the decision is preserved, the Solution stays active, and follow-up work can continue.
3. **Given** all completion gates pass, **When** the user confirms the exact final action in Chat, **Then** LLM Wiki creates Completed Work with Capture→Problem→Solution→completion lineage and cited accepted evidence, while Knowledge remains unpublished.
4. **Given** completion just succeeded, **When** the Skill continues the conversation, **Then** it summarizes the publish target and asks one natural question equivalent to “Knowledge로 발행할까요?” without publishing automatically.
5. **Given** the user accepts the separate publication preview, **When** publication gates pass, **Then** LLM Wiki writes the reviewed Knowledge artifact and records a distinct publication decision.
6. **Given** a tool/model sends `confirmed`, `approved`, or equivalent fields without a server-issued in-chat challenge, **When** the request is evaluated, **Then** the request is rejected and no workflow or publication state changes.

---

### User Story 4 - Support Two Chat Input Interfaces (Priority: P1)

LLM Wiki accepts work through two first-class input interfaces: its in-app Chat and an external Chat
connected through MCP. Both interfaces drive the same Capture→Problem→Solution→Completed Work
domain model and expose the same user decisions without requiring a visit to Workbench. Workbench
remains a fully active structured workspace that the user may enter at any time.

**Why this priority**: The two interfaces serve different contexts. In-app Chat is available when
the user is already in LLM Wiki; MCP lets any supported external Chat contribute the same durable
work without a context switch.

**Independent Test**: Complete equivalent tracked work once through in-app Chat and once through an
external MCP Chat. Verify that both use the same domain gates, provenance model, and final lineage,
and that neither path requires opening Workbench.

**Acceptance Scenarios**:

1. **Given** a supported external Chat host, **When** the user tracks and completes work, **Then** every required review and decision is available in that Chat and no instruction says to open Workbench.
2. **Given** the user works in LLM Wiki's in-app Chat, **When** the conversation reaches the same milestones, **Then** inline review cards let the user create, approve, revise, and complete the same records without opening Workbench.
3. **Given** equivalent accepted input through either Chat interface, **When** domain operations commit, **Then** record semantics and completion gates are identical while provenance identifies the originating interface and session.
4. **Given** the user later opens Workbench optionally, **When** they inspect or modify the work, **Then** it shows the same current state and supports the same valid edits, reviews, approvals, Work Log additions, and completion actions without adding duplicate decisions.

---

### User Story 5 - Move Between Chat and Workbench (Priority: P1)

A user can open Workbench midway through work started in either Chat, immediately see the current
Capture, Problem, Solution, Work Log, evidence, and pending decision, and continue working there.
When the user returns to Chat, the conversation resumes from the Workbench-updated state.

**Why this priority**: Chat is best for conversational input, while Workbench is useful for direct
structured editing and overview. Choosing one interface must not strand or fork the work.

**Independent Test**: Start in external Chat, adopt a Problem, open Workbench and edit/approve the
Solution plus add progress, then return to Chat and complete the work. Verify one session, one
ordered revision history, and no duplicated or overwritten records.

**Acceptance Scenarios**:

1. **Given** active tracked work from either Chat, **When** the user opens Workbench, **Then** the current workflow state, provenance, recent checkpoints, and pending actions are visible without an import or sync command.
2. **Given** the user edits or advances work in Workbench, **When** the transaction commits, **Then** the tracked session head advances and both Chat interfaces read the updated state on their next action or resume.
3. **Given** Chat and Workbench attempt changes from the same old revision, **When** one commits first, **Then** the second receives a stale-state response, preserves its unsaved input, and offers review against the new head instead of overwriting it.
4. **Given** a proposal was already decided in Chat, **When** Workbench opens it, **Then** the decision appears as completed and cannot be applied a second time.

---

### User Story 6 - Resume Work Through a Representative Skill (Priority: P1)

A user can ask an AI to load their current LLM Wiki work without knowing MCP resource names or the
workflow schema. The bundled `llm-wiki-workflow` Skill retrieves the bounded current Workbench
projection, orients the user, selects only an unambiguous session, and continues through the normal
MCP decision flow. When explicitly asked, it can also retrieve a complete Workbench snapshot and
present a clean portfolio-style overview without dumping the frontend structure into Chat.

**Why this priority**: Tools provide capability, but a representative Skill teaches AI clients the
intended retrieval order, workflow semantics, evidence boundaries, and safe transition behavior.

**Independent Test**: Invoke the Skill with “현재 작업을 이어서 해줘” while one Workbench item is
selected, then with multiple active items and no selection. Verify correct retrieval in the first
case and an explicit user choice in the second, with no whole-Vault read or guessed attachment.

**Acceptance Scenarios**:

1. **Given** a connection with current-Workbench read scope, **When** the Skill is asked to resume current work, **Then** it reads the bounded current projection before proposing an action.
2. **Given** one explicit active selection, **When** the projection is returned, **Then** the Skill summarizes Capture, Problem, Solution, progress, blockers, and next decision.
3. **Given** multiple plausible active items and no selection, **When** the Skill orients the user, **Then** it presents a concise choice and does not guess by title or semantic similarity.
4. **Given** the user chooses work originating from another interface, **When** the Skill continues it, **Then** linking requires an exact user-confirmed action and matching revisions.
5. **Given** the user asks for the entire Workbench, **When** the Skill loads every snapshot page, **Then** it shows complete stage counts, every attention item, all active Solutions, compact coverage of remaining non-archived work, and recent completions without exposing raw records or internal identifiers.

---

### User Story 7 - Review Conflicts with the Current Session's AI (Priority: P1)

When a Problem or Solution needs conflict review, the AI already active in the user's current Chat
retrieves bounded lexical and semantic Vault evidence and performs the comparison in that session.
LLM Wiki supplies search evidence and controlled draft/publication operations; it does not invoke a
second hidden model to make the judgment.

**Why this priority**: The current AI has the live conversational context and should remain the one
reasoning partner. Separating retrieval from synthesis also keeps model choice outside the hot
persistence path and makes cited evidence inspectable.

**Independent Test**: Trigger conflict review from both in-app Chat and an MCP Chat. Grant only one
topic, run lexical and semantic searches, and verify that results stay in scope, cite source
passages, and lead to the same user-reviewed workflow action without a server-side AI invocation.

**Acceptance Scenarios**:

1. **Given** a conflict review request, **When** the current-session AI begins analysis, **Then** it refreshes the relevant work state and performs both lexical and semantic Vault searches within the granted scope.
2. **Given** search results disagree or are incomplete, **When** the AI explains the conflict, **Then** it distinguishes agreement, contradiction, missing evidence, and inference with source citations and leaves the final resolution to the user.
3. **Given** a topic-scoped grant, **When** either search runs, **Then** results outside the allowed topic are neither returned nor revealed through counts or errors.
4. **Given** semantic search is unavailable, **When** conflict review continues, **Then** lexical search remains usable and the AI states the evidence limitation instead of fabricating semantic coverage.

### Edge Cases

- The Chat host does not support MCP Elicitation or cannot return a server-issued request state.
- A user cancels or ignores an in-chat review, or responds after its expiry.
- A response is replayed, edited by the model, bound to a different user/workspace, or applied after the session revision changed.
- One external conversation changes topic and needs a new work session rather than mutating the first.
- Drafts arrive out of order, or progress arrives before the user adopts a Solution.
- Two connections claim the same external session identity.
- A completion proposal cites missing, stale, inaccessible, or unverifiable evidence.
- Revocation occurs between an elicitation response and the transactional write.
- The database is unavailable while external work is being recorded.
- A checkpoint event is received after a newer event for the same session even though its claimed source time is older.
- A checkpoint is durably accepted but its background projection has not yet reached Workbench.
- An in-app Chat and an external Chat try to update the same linked work at the same revision.
- Workbench is opened while an MCP Elicitation challenge is still pending.
- An external host or its model provider retains conversation content after LLM Wiki access is revoked.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: MCP work tracking MUST be disabled by default and require an explicit user-created connection.
- **FR-002**: A new external session MUST start only after the current Chat obtains user consent for the exact Capture or applies a user-configured consent policy for that connection; passive recording of all conversations is forbidden.
- **FR-003**: The system MUST identify an external work session by connection plus host-supplied session identity so two hosts cannot collide or take over each other's work.
- **FR-004**: The first accepted checkpoint MUST create exactly one private Capture containing concise intent and origin metadata, not an entire transcript by default.
- **FR-005**: An external assistant MUST be able to submit bounded Problem drafts, Solution drafts, checkpoints, conflict resolutions, and completion proposals without possessing workflow authority.
- **FR-006**: Every adoption, approval, conflict-resolution, evidence-acceptance, completion, and publication action MUST be explicitly reviewed in the user's active interface with the exact proposed payload and accept/edit/reject choices: MCP Elicitation for external Chat, a native inline card for in-app Chat, or the structured Workbench editor.
- **FR-007**: An external state-changing response MUST be bound to the authenticated local principal, connection, workspace, session, action, exact payload hash, expected revision, expiry, and one-time nonce. An in-app response MUST be bound to its authenticated window, Chat session, source event, payload hash, and revision. A caller-supplied confirmation boolean MUST never count as user authority.
- **FR-008**: After a valid decision from any interface, the server MUST recheck authorization, applicable revocation, revision, and existing domain gates before committing the decision and resulting workflow records atomically.
- **FR-009**: If the current Chat host cannot perform the required MCP user-input exchange, LLM Wiki MUST return `elicitation_required` for governed state transitions and MUST NOT direct the user to Workbench as part of the normal flow.
- **FR-010**: The assistant MUST be able to append ordered progress summaries, decisions, evidence references, artifact references, and validation outcomes to the linked session.
- **FR-011**: Progress received before Solution adoption MUST remain pending on the session and be projectable only after an explicit, conflict-checked decision in Chat or Workbench.
- **FR-012**: Accepted external progress MUST appear in the existing Solution Work Log with connection, session, event, decision, and provenance links.
- **FR-013**: A completion proposal MUST leave the Solution incomplete until the user confirms the exact final action in Chat or Workbench and every existing completion gate passes.
- **FR-014**: Rejected, cancelled, expired, stale, and superseded proposals MUST remain traceable without changing authoritative workflow state.
- **FR-015**: A user-confirmed completion MUST include the tracked session and selected accepted evidence in the existing Lineage and Completed Work output.
- **FR-016**: Every mutating request MUST be idempotent; reuse of one request identity with different content MUST fail without partial changes.
- **FR-017**: Every update MUST validate the current session/entity revision and reject stale writes with enough bounded metadata to recover safely.
- **FR-018**: A connection MUST be limited to sessions it created and records linked through valid in-chat decisions; arbitrary entity identifiers MUST NOT grant write access.
- **FR-019**: Revocation MUST block all requests beginning after revocation commits, including a retry carrying a previously issued but unconsumed review challenge.
- **FR-020**: Activity history MUST record connection, session, operation, time, outcome, and affected record identities without conversation text, work summaries, evidence bodies, prompts, credentials, raw review responses, or raw errors.
- **FR-021**: Setup guidance MUST explain external-provider data handling, required Chat-host capabilities, and that revocation cannot erase copies outside LLM Wiki.
- **FR-022**: LLM Wiki MUST continue to support its in-app Chat as a first-class input interface alongside external MCP Chat hosts.
- **FR-023**: In-app Chat MUST offer inline Capture, Problem, Solution, checkpoint, conflict, and completion review actions backed by explicit user interaction and the same domain services used by MCP.
- **FR-024**: The two Chat interfaces MUST produce equivalent workflow semantics, gates, idempotency, and lineage while preserving distinct interface/session provenance.
- **FR-025**: Workbench MUST be optional but fully functional: it MUST show the same current tracked state and allow direct structured editing, review, approval, progress, conflict, completion, and follow-up actions through the shared domain service.
- **FR-026**: A committed Workbench action MUST advance the same session revision and become visible to both Chat interfaces on their next read; no explicit import, export, or synchronization action may be required.
- **FR-027**: Concurrent Chat and Workbench mutations MUST use compare-and-swap revisions. A stale action MUST preserve user-authored input, expose the new current state, and never overwrite or duplicate a newer decision.
- **FR-028**: A pending in-chat challenge MUST become stale when an intervening Workbench action changes its bound revision or payload; its later response MUST create no workflow mutation.
- **FR-029**: MCP failures, malformed inputs, unsupported capabilities, stale or replayed challenges, denied access, cancellation, and concurrent desktop use MUST produce bounded errors without corrupting workflow or Knowledge state.
- **FR-030**: MCP tracking MUST preserve startup, capture, Workbench, indexing, and search performance budgets and MUST not load AI code on hot write paths.
- **FR-031**: MCP connection, session, proposal, in-chat decision, error, provenance, in-app Chat, and Workbench workflow copy MUST be available in English and Korean, using `사용자` for the person using the product in Korean copy.
- **FR-032**: A connection MUST use independently grantable scopes for owned-session read/write, topic read, current-Workbench read, whole-Workbench overview, lexical Vault search, semantic Vault search, bounded evidence read, Knowledge draft save, and Knowledge publication; a broader read or write capability MUST NOT be implied by a narrower scope.
- **FR-033**: The current-work projection MUST contain at most ten active tracked lineages, prioritize an explicit active selection, and expose only bounded workflow summaries, revisions, provenance, blockers, and pending actions—not Chat history, raw evidence, completed documents, arbitrary Vault content, credentials, or settings.
- **FR-034**: The project MUST provide a representative `llm-wiki-workflow` Skill that translates Workbench structure into concise, natural conversation for ChatGPT and Codex; reads current work before acting; distinguishes authoritative state from proposals/inferences; asks at most one workflow question at a time; avoids ambiguous automatic selection; and preserves all user-decision boundaries.
- **FR-035**: Linking an external Chat session to work originating from another interface MUST require an exact user-confirmed action and matching workspace/entity revisions; a resource-returned identifier MUST NOT itself grant mutation authority.
- **FR-036**: Normal Skill responses MUST hide MCP/resource/revision terminology, avoid board-shaped dumps, summarize only current intent/progress/blocker/next action, and reveal technical details only when requested or required for recovery.
- **FR-037**: An explicit whole-Workbench request MUST use a stable paginated snapshot covering every visible non-archived Capture, Problem, and Solution, complete aggregate counts, all attention conditions, and bounded recent completions.
- **FR-038**: The Skill MUST present a whole-Workbench snapshot in a clean conversational hierarchy: at-a-glance counts, needs-attention items, every active Solution, compact coverage of work still being shaped, recent completions, and one useful next question.
- **FR-039**: A whole-Workbench response MUST identify partial/expired snapshots, MUST NOT combine pages from different revisions, and MUST NOT infer priority from recency, rank the user, or silently omit overflow.
- **FR-040**: Phase 1 MUST support local Codex and ChatGPT desktop MCP hosts and MUST exclude ChatGPT web, remote HTTP transport, and remote OAuth deployment.
- **FR-041**: Completion and Knowledge publication MUST be separate governed actions and decision records; successful completion MUST never publish Knowledge as a side effect.
- **FR-042**: After successful completion, the representative Skill MUST ask once whether to publish the reviewed result as Knowledge and MUST call the publication action only after an explicit separate user decision.
- **FR-043**: Conflict review and other AI synthesis MUST be performed by the AI in the current Chat session using returned evidence; the work-tracking service and MCP server MUST NOT invoke a hidden model for these judgments.
- **FR-044**: The system MUST expose separate bounded lexical-search and semantic-search capabilities over the Vault, plus bounded evidence retrieval, with every result carrying source identity, passage location, score/match metadata, and content revision.
- **FR-045**: Vault mutations exposed to Chat MUST be limited to reviewed Knowledge draft save and Knowledge publication operations; arbitrary path or file write is forbidden.
- **FR-046**: Workbench, in-app Chat, and MCP MUST use one authoritative workflow and persistence boundary so equivalent accepted intents have the same validation, ordering, idempotency, projection, and resulting state.
- **FR-047**: A checkpoint acknowledgment MUST follow a durable append to an immutable event log; Workbench, session, and search projections MUST be updated asynchronously and idempotently from that log.
- **FR-048**: The system MUST assign the authoritative event occurrence time at the trusted local application boundary and order same-session events deterministically. An event older than or equal to the applied stream watermark MUST remain auditable but MUST NOT replace newer current state.
- **FR-049**: Every projection MUST expose whether an accepted event is queued, applied, ignored as late, or failed, and MUST recover safely across restart without applying one event more than once.
- **FR-050**: Workbench MUST refresh on focus, visibility changes, successful writes, and a bounded periodic interval; both Chat interfaces MUST refresh relevant state naturally before reads, milestone decisions, and mutations without interrupting ordinary conversation.
- **FR-051**: On a concurrent-save conflict, the active interface MUST preserve the user's uncommitted input, refresh the latest state, and retry automatically only when the original mutation has identical meaning and remains valid; otherwise it MUST show the material change and request renewed review.
- **FR-052**: Ordinary Chat continuation MUST load owned-session context by default, topic context only when selected or requested, current-Workbench context for orientation, and whole-Workbench context only after an explicit whole-overview request.

### Key Entities

- **MCP Connection**: A named, revocable grant allowing one local external Chat host to track work.
- **Tracked Work Session**: The durable link between one in-app or external Chat conversation and its Capture, workflow records, checkpoints, proposals, and decisions.
- **Work Checkpoint**: An immutable, idempotent record of progress, decisions, evidence, artifacts, or validation.
- **Structure Proposal**: Assistant-supplied Problem or Solution content awaiting an exact in-chat decision.
- **Completion Proposal**: Assistant-supplied outcome and evidence awaiting the existing completion gates and final in-chat decision.
- **Elicitation Challenge**: A short-lived, single-use, authenticated binding between one proposed action/payload/revision and the user's response in the current Chat.
- **Work Decision**: An accepted, edited, rejected, or cancelled user response tied to the active interface, exact payload, revision, and resulting workflow operation.
- **Source Provenance**: Input interface, optional connection/host, Chat and tracked-session identity, request, event, decision, time, and content revision associated with a work record.
- **Work Tracking Activity Event**: Content-free local evidence that an operation was accepted, denied, conflicted, cancelled, expired, replayed, or failed.
- **Current Workbench Projection**: A permission-scoped, bounded orientation view of the active selection and recent tracked lineages for use by workflow-aware clients.
- **Workflow Skill**: The bundled agent guidance that translates requests such as “resume current work” into safe projection reads, session selection, summaries, and allowlisted MCP actions.
- **Projection Watermark**: The greatest deterministic event-order key applied to one session stream, used to prevent late older events from replacing newer current state.
- **Knowledge Publication Decision**: A distinct user-reviewed action that converts eligible Completed Work into a portable Knowledge artifact after completion.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can start tracking and create the first confirmed Capture inside a supported Chat in under 3 minutes without opening Workbench or copying text manually.
- **SC-002**: One Chat session can produce Capture→Problem→Solution, three accepted Work Log checkpoints, and Completed Work with 100% continuous provenance while Workbench remains unopened.
- **SC-003**: 100% of adoption, approval, conflict, evidence, completion, and publication transitions have a valid interface-bound decision record tied to the exact committed payload; model-only confirmations produce zero transitions.
- **SC-004**: Replaying any accepted request or challenge up to 10 times creates exactly one result; conflicting reuse produces zero partial records.
- **SC-005**: Permission and isolation tests produce zero unauthorized writes across connections, sessions, workflow entities, Knowledge, settings, credentials, or Vault files.
- **SC-006**: At least 9 of 10 usability participants can resume in the external Chat and explain the current Problem, Solution, progress, and pending decision without opening Workbench or the original transcript.
- **SC-007**: A host without required user-input capability receives `elicitation_required` for 100% of governed transitions and never receives a fallback path that weakens user authority.
- **SC-008**: Equivalent scripted scenarios through in-app Chat and external MCP Chat produce the same workflow states, required decisions, completion gates, and lineage, with only their source provenance differing.
- **SC-009**: Revocation blocks 100% of requests beginning after confirmation, including responses to challenges issued before revocation.
- **SC-010**: At least 95% of local checkpoint writes become readable through the session resource within 500 ms, and idle performance regresses by no more than 15%.
- **SC-011**: macOS and Windows packaged acceptance tests complete equivalent in-app and external Chat flows, including Capture, draft review, checkpoints, completion, restart, and external-connection revocation.
- **SC-012**: In 100% of parity scenarios, work can move Chat→Workbench→Chat with one ordered session history, no duplicate decisions, and no lost accepted or unsaved user input.
- **SC-013**: In representative Skill tests, 100% of unambiguous active selections resume the correct lineage, 100% of ambiguous projections ask the user to choose, and ordinary catch-up responses contain no raw IDs, resource URIs, revisions, or empty workflow-stage dumps.
- **SC-014**: Whole-Workbench tests represent 100% of visible non-archived items through an item line or explicit aggregate count, surface 100% of blockers/conflicts/pending decisions, and combine zero pages from different snapshot revisions.
- **SC-015**: In 100% of completion tests, Completed Work exists before any Knowledge artifact and declining or ignoring the publish question leaves publication state unchanged.
- **SC-016**: In conflict-review tests, 100% of cited conclusions link to returned lexical or semantic passages, and zero server-side model calls occur.
- **SC-017**: In 10,000 randomized out-of-order and replayed checkpoint events, the projected state is deterministic, each event is applied at most once, and every late ignored event remains in audit history.
- **SC-018**: At least 95% of durable checkpoint appends acknowledge within 50 ms locally and become visible in current projections within 500 ms, with an explicit queued state during the gap.
- **SC-019**: In 100% of concurrent Chat/Workbench save tests, no accepted update or unsaved user input is lost; unchanged-intent retries either commit once or return an actionable refreshed conflict.
- **SC-020**: Representative context tests return no records outside the requested session/topic scope and load the whole Workbench only for explicit overview requests.

## Assumptions

- The supported external Chat host implements the MCP 2026-07-28 multi-round user-input flow for tool calls; this capability is required for the complete no-Workbench workflow.
- Each Chat surface owns its conversational UX; LLM Wiki owns durable state, domain validation, provenance, decision verification, and the optional structured Workbench.
- The integration stores concise checkpoints and selected evidence, not a full transcript unless the user deliberately includes a bounded excerpt.
- Routine checkpoint capture may follow an explicit per-session consent policy, but any checkpoint that becomes authoritative evidence requires a bound user decision.
- Workbench remains an optional active structured workspace, not a mandatory step in the primary user workflow.
- In-app Chat keeps its conversation history and streaming behavior; structured milestones are stored separately as workflow events rather than inferred from old history automatically.
- Both Chat interfaces are first-class input surfaces. Neither requires Workbench for the normal lifecycle.
- Phase 1 supports Codex and ChatGPT desktop through local stdio. ChatGPT web, remote hosting,
  remote HTTP/OAuth, team permissions, autonomous workflow advancement, arbitrary Vault writes,
  attachment transfer, and full transcript synchronization are out of scope.
- A durable event-log append completes before an accepted checkpoint is acknowledged; background
  work refers to projection/materialization rather than an in-memory-only write queue.
