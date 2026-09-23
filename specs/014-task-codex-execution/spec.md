# Feature Specification: Task Codex Execution

**Feature Branch**: `feature/task-session-execution`

**Created**: 2026-09-21

**Status**: Draft

**Input**: Extend existing Task work sessions with real Codex execution, durable continuation, formal approval handling, and automatic projection into the existing Task Work Log.

## Product Spirit

This feature advances **Resume Where You Left Off** and **Tasks Carry Work** by letting a user continue real work from the Task that owns it. Execution remains private process, and Codex may perform authorized work without completing the Task, resolving a Problem, or publishing Knowledge on the user's behalf.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run Codex from a Task session (Priority: P1)

A user opens an existing Task work session, enters a work instruction, and explicitly starts Codex. The instruction is saved before execution begins. The same session shows real progress, the final result, and errors, while the Task's existing Work Log immediately gains one linked item for that execution.

**Why this priority**: Real execution followed by a saved result and canonical Work Log record is the core value and mandatory completion boundary.

**Independent Test**: In an isolated project folder, execute one harmless instruction through a Task session and observe one real Codex turn, durable progress and result records, and exactly one linked Work Log item without changing Task completion state.

**Acceptance Scenarios**:

1. **Given** a Task with an existing work session and valid project folder, **When** the user explicitly starts an instruction, **Then** the instruction and execution record are saved, a real Codex turn starts, and the same session displays observed progress and the terminal result.
2. **Given** an execution has started, **When** the Task Work Log is viewed, **Then** it contains exactly one linked item showing the request, Codex as the tool, current execution state, and a route back to the source session and execution.
3. **Given** Codex finishes normally, **When** the result is saved, **Then** the session and Work Log show the model-authored final report and separately label event-derived command, file-change, artifact, and check evidence.
4. **Given** a session has no configured valid project folder, **When** the user attempts execution, **Then** no Codex turn starts and the instruction remains available while the user is asked to correct the folder.
5. **Given** a successful Codex turn, **When** its result is saved, **Then** the Task remains in its existing workflow state until the user uses the existing completion action.

---

### User Story 2 - Continue and control the same Codex conversation (Priority: P1)

A user gives follow-up instructions in the same work session. Each instruction becomes a separate execution attempt in the one Codex conversation bound to that session, so Codex retains its own conversation and tool context without the application replaying the full history.

**Why this priority**: Durable continuity is the reason to use a work session and avoids repeated cold starts or duplicated context.

**Independent Test**: Run two related instructions in one session, verify both use the exact bound Codex conversation and distinct execution records, then confirm another Task cannot read or resume that conversation.

**Acceptance Scenarios**:

1. **Given** a session with a completed Codex execution, **When** the user sends a follow-up instruction, **Then** a new execution starts in the exact same bound Codex conversation and the earlier transcript is not replayed as a second copy.
2. **Given** a session is already executing or awaiting a formal response, **When** another start is attempted, **Then** it is blocked without creating a second execution.
3. **Given** Codex asks for command, file-change, permission, or other structured approval or a structured question, **When** the request appears, **Then** the session durably shows a waiting state and offers only the response choices supported by that request.
4. **Given** the user writes ordinary prose that resembles approval, **When** it is submitted as an instruction, **Then** it is treated as work content and does not answer a pending formal request.
5. **Given** an execution is running, **When** the user explicitly stops it, **Then** the session shows that interruption was requested and uses the actual terminal provider event to decide whether completion or cancellation won the race; a retry is always a new execution.
6. **Given** the user closes the Task or changes tabs while execution continues, **When** they return, **Then** the session reconnects to the saved execution rather than cancelling or duplicating it.
7. **Given** Codex sends a structured user-input request, **When** it is displayed, **Then** each offered option is a keyboard-operable labeled button, questions without options accept free text, optional other text follows the request, and one explicit Submit sends all question answers without a default confirmation.
8. **Given** a structured answer submission fails, **When** the request returns to an actionable state, **Then** the selected or entered answer remains available for retry and is not mistaken for an accepted response.

---

### User Story 3 - Recover and audit execution safely (Priority: P2)

A user can reopen a Task after an application or execution-service interruption and understand what is known, what is uncertain, and what action is available. Historical attempts and manual Work Log content remain intact.

**Why this priority**: Process interruptions and partial persistence must never be presented as success or silently cause repeated work.

**Independent Test**: Interrupt an active execution, reopen the application, and verify the attempt is marked as interrupted or requiring confirmation, is never automatically rerun, and remains linked to its original Work Log item.

**Acceptance Scenarios**:

1. **Given** an execution was active when its process connection was lost, **When** the application restarts and cannot establish its exact terminal state, **Then** the execution is shown as interrupted or requiring confirmation, never as running or successful.
2. **Given** saved provider events are delivered again after reconnection, **When** they are ingested, **Then** output, terminal results, and Work Log links remain single and ordered.
3. **Given** Work Log projection is temporarily unavailable after the execution result is saved, **When** synchronization is retried, **Then** only the existing linked record is repaired and Codex is not run again.
4. **Given** Task A and Task B each have sessions, **When** either Task is opened, **Then** the other Task's conversations, executions, outputs, approvals, and Work Log links are unavailable.
5. **Given** pre-existing sessions and manual Work Log records, **When** this feature is installed and used, **Then** their identifiers, text, attachments, comments, and ordering remain unchanged.

### Edge Cases

- Codex is not installed, cannot be launched, or reports that authentication is required.
- The configured project folder is missing, not a directory, inaccessible, or changes between turns.
- Execution fails before a provider conversation or turn identifier is assigned.
- The connection ends after Codex performs work but before a final report arrives.
- Codex reports success without observed command, file-change, artifact, or check evidence.
- A formal approval or structured question remains unanswered across a UI detach or application restart.
- A nonblocking structured question is present while the Codex turn continues producing events.
- The installed Codex version or active model does not advertise structured user-input requests.
- The user retries after failure or cancellation while delayed events from the earlier attempt arrive.
- Output contains a secret or content that resembles an instruction, approval, or permission change.
- A Task revision or linked Problem changes while its Codex conversation remains active.
- The final report is very long, missing, malformed, or contradicts observed execution events.
- The Work Log item has user-added comments or other manual content when execution metadata changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST extend the existing Task work-session surface and MUST NOT create a second execution workspace.
- **FR-002**: The system MUST offer separate explicit actions for saving a note and starting an execution; opening a Task or session MUST NOT start execution.
- **FR-003**: Starting execution MUST durably save the user's instruction, an immutable execution identity, and one linked item in the Task's existing Work Log before external work begins.
- **FR-004**: Each Task work session MUST bind to at most one exact Codex conversation, and a conversation MUST NOT be selected through a global-latest or cross-Task lookup.
- **FR-005**: The first execution in a session MUST establish its Codex conversation with the chosen working folder, model, current Task context, linked Problem and Task references, and applicable work constraints.
- **FR-006**: Each later instruction in that session MUST start a distinct turn in the same Codex conversation without also replaying the complete stored transcript.
- **FR-007**: When the current Task definition or links materially change, later execution context MUST identify the bounded change without duplicating the prior conversation.
- **FR-008**: Each execution MUST retain its Task, session, Codex conversation and turn identities, instruction, provider, model, working folder, lifecycle state, start and finish times, final result or error, and links to retained evidence.
- **FR-009**: Execution states MUST distinguish queued, running, awaiting a formal response, succeeded, failed, user-cancelled, interrupted, and requiring confirmation.
- **FR-010**: A retry MUST create a new execution identity linked to the earlier attempt and MUST NOT overwrite it. Use instruction again restores an instruction without replacing an unrelated draft; only explicit Run starts the new attempt. An uncertain outcome MUST warn that work may already have happened and distinguish checking/reconnecting from rerunning.
- **FR-011**: Only one nonterminal execution MAY exist in a session; duplicate starts and uncertain request retries MUST resolve to the same execution.
- **FR-012**: The session MUST show real provider progress, output, formal requests, terminal status, and errors; it MUST NOT generate simulated progress.
- **FR-013**: Closing the Task, switching sessions, or detaching the UI MUST NOT cancel an execution. Explicit Stop MUST request interruption of the exact active turn.
- **FR-013a**: Acceptance of an interruption request MUST NOT itself be treated as terminal cancellation; success, failure, and cancellation MUST follow the actual terminal provider event so completion and Stop races are represented accurately.
- **FR-014**: Provider events and reconnect replays MUST be stored idempotently and in provider order, without one durable record per streamed text fragment.
- **FR-015**: The system MUST resume only the exact Codex conversation bound to the selected Task work session after reconnecting to the execution service.
- **FR-016**: If the exact state of a previously active turn cannot be recovered, the system MUST mark it interrupted or requiring confirmation and MUST NOT automatically execute the instruction again.
- **FR-017**: Formal command approvals, file-change approvals, permission approvals, and structured questions MUST remain associated with the same execution and Work Log item, survive UI detachment, and accept responses only through their structured response control.
- **FR-017a**: Approval controls MUST be derived from the decisions supported by the actual provider request and MUST NOT invent broader privilege or sandbox choices.
- **FR-017b**: A structured user-input request MUST dynamically present its actual answer options as clickable buttons with labels and descriptions. A question with null or empty options MUST accept free text; a question with options MAY accept additional text only when `isOther` permits it. Selection is per question and one explicit Submit answers the request; no option is selected or submitted by default.
- **FR-017c**: Formal requests MUST distinguish pending, submitting, answered, stale, and error states; failed submission MUST retain the user's proposed answer, stale or duplicate responses MUST be rejected, and focus MUST return to the remaining actionable control. Nonsecret answer drafts MUST be keyed by Task/session/Run/request/question and survive view detachment and locale changes; stale drafts remain read-only and are not transferred to another request.
- **FR-017d**: A nonblocking structured request MUST remain answerable without incorrectly blocking the rest of the turn; a blocking request MUST clearly explain that execution is waiting.
- **FR-017e**: When structured user-input is unsupported by the installed provider capability or active model, the interface MUST NOT simulate it; an ordinary question in final prose is answered through a later turn in the same conversation.
- **FR-018**: Ordinary session prose, referenced content, and provider output MUST be treated as work data and MUST NOT grant approval, alter permissions, or answer a formal request.
- **FR-019**: Existing Codex authentication, approval policy, and sandbox boundaries MUST be preserved by default; a saved convenience preference MUST NOT silently select an unrestricted or approval-bypassing mode.
- **FR-019a**: An explicit Ask setting MUST route supported approvals to the user. Automatic review MUST require a new current opt-in and MUST NOT be activated merely because an older session contains the legacy automatic value.
- **FR-019b**: Before execution, the session MUST show the authoritative model, working folder, approval behavior, and sandbox behavior beside the composer. Explicit Prepare conversation / Check settings MAY create or resume the exact bound thread but MUST NOT start a turn or create a Run or Work Log. Opening a session MUST NOT prepare it automatically. Run requires current ready settings and their revision; a mismatch preserves the draft without dispatch. A bound-folder mismatch requires a new session, never silent thread replacement.
- **FR-019c**: Secret-valued structured inputs MUST NOT be collected in the Workbench in this increment; the interface MUST direct the user to the existing Codex authentication or secret-entry flow without persisting the secret.
- **FR-020**: The system MUST distinguish executable-not-found, authentication-required, invalid-folder, start failure, turn failure, abnormal disconnect, user cancellation, and normal completion in saved state and user-facing status.
- **FR-021**: User instructions MUST be transmitted as structured provider input rather than interpolated into a shell command.
- **FR-022**: Secrets and authentication material MUST NOT be intentionally copied into session records, durable execution evidence, or Work Log summaries.
- **FR-023**: Every execution MUST use one existing Task Work Log item as its canonical summary surface; streamed output MUST remain in execution details rather than creating additional Work Log items.
- **FR-024**: The Work Log item MUST expose the request, tool, lifecycle status, model-authored final report, factual event evidence, source session and execution, and any unresolved limitation.
- **FR-025**: The same Codex turn MUST produce the final report describing performed work, changes, checks, artifacts, and unresolved issues; the system MUST NOT make a separate AI summarization call.
- **FR-026**: Model-authored claims MUST be labeled separately from observed provider events. A normal process exit MUST NOT be presented as proof that the requested work is correct, and claimed checks MUST NOT be presented as observed unless corresponding evidence exists.
- **FR-027**: When failure or cancellation prevents a final report, the Work Log MUST say that the report is unavailable and still show factual status and retained event evidence.
- **FR-028**: Updating execution metadata or repairing its Work Log projection MUST NOT overwrite the Work Log item's body, attachments, comments, or any manual Work Log record.
- **FR-029**: Replayed events, repeated terminal handling, UI reopening, and Work Log synchronization retries MUST NOT create duplicate execution results or Work Log items.
- **FR-030**: A Work Log synchronization failure MUST be visible and retryable without rerunning Codex.
- **FR-030a**: If the application durably records dispatch but cannot prove whether the provider accepted the turn, the execution MUST require attention and MUST NOT be automatically sent again.
- **FR-031**: Execution success MUST NOT complete or reopen the Task, resolve a Problem, publish Knowledge, or apply an unrelated workflow transition.
- **FR-032**: All reads, subscriptions, approvals, stops, retries, and Work Log links MUST enforce the complete Task-session-execution ownership chain.
- **FR-033**: Existing Task work sessions and session entries MUST remain readable with their stable identifiers, and existing manual Work Log content and evidence MUST survive migration unchanged.
- **FR-034**: The execution surface, including dynamic answer-option buttons and optional free-text responses, MUST support keyboard operation, visible focus, predictable focus movement, non-color status labels, English and Korean copy, narrow-window use, and accessible live progress in a small status region that does not steal focus or scroll position. Session/Run and Work Log links MUST switch the actual Task tab and focus the exact destination. Historical Runs MUST remain accessible without hiding an active execution or response-needed indicator.
- **FR-035**: The feature MUST be verified with focused automated tests, at least one isolated real Codex execution through Work Log projection, and the smallest packaged desktop scenario needed for the remaining native lifecycle risk.
- **FR-036**: Actual-provider evidence and substitute-based test evidence MUST be reported separately, including any unavailable UI, platform, authentication, or interruption checks.
- **FR-037**: The execution UI/UX design MUST receive an Astra High review before substantive UI implementation continues. Findings MUST be reflected in the design and implementation; actual rendered UI verification remains a separate completion check.
- **FR-038**: Existing saved attachments MUST remain valid session records, but the execution action MUST include an attachment only when its type is explicitly supported and shown as execution input; unsupported files MUST NOT be presented as if Codex received them.

### Key Entities

- **Task Work Session**: Existing Task-owned private work space, extended with the exact bound Codex conversation identity and connection state.
- **Execution**: One immutable user attempt corresponding to one Codex turn, with its own status, timing, instruction, result, and evidence.
- **Execution Event**: Ordered, deduplicated provider evidence used for progress, recovery, and factual reporting.
- **Formal Request**: A structured approval or question bound to one execution and answered only through an explicit structured response.
- **Work Log Execution Link**: One-to-one association between an execution and one canonical existing Task Work Log item, exposing execution metadata without replacing manual content.
- **Final Report**: The model-authored end-of-turn account of performed work, changes, checks, artifacts, and unresolved issues.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the isolated acceptance flow, one explicit instruction produces one real Codex turn, one saved execution result, and exactly one linked existing Work Log item.
- **SC-002**: Two follow-up instructions in one session use the same exact Codex conversation and produce two distinct execution attempts with no duplicated transcript or Work Log item.
- **SC-003**: Within a known dispatch outcome, duplicate submission and replay tests produce zero duplicate provider turns, result records, or Work Log items across at least 100 repeated deliveries; an uncertain dispatch is surfaced for attention rather than claimed as exactly-once.
- **SC-004**: A user returning to a running or finished execution sees its latest saved state and existing output within one second after local data is available.
- **SC-005**: Stop, failure, abnormal disconnect, and restart-recovery scenarios are each distinguishable from success in both the session and Work Log.
- **SC-006**: All tested attempts to access or control an execution through the wrong Task or session are rejected and reveal no execution content.
- **SC-007**: Migration validation preserves 100% of sampled pre-existing session identifiers, entries, manual Work Log bodies, attachments, and comments.
- **SC-008**: No acceptance scenario changes Task completion, Problem resolution, or Knowledge publication state without the user's existing explicit workflow action.
- **SC-009**: A final Work Log summary lets a reviewer identify the request, result, observed evidence, model-only claims, artifacts, and unresolved issues without reading the full streamed output.
- **SC-010**: Normal Task and Work Log projections remain within the existing 100 ms p95 local-render budget when no execution detail is expanded.

## Assumptions

- Codex is the only executable provider in this increment; saved but unsupported provider choices are not presented as runnable.
- The desktop application owns one supervised local connection to the installed Codex execution service and may reconnect it without starting a user turn.
- Codex conversation history and harness state are authoritative for conversational continuity; local records provide audit, display, recovery, and Work Log projection.
- A valid working folder is required before execution and remains an explicit, visible session setting.
- Existing note attachments remain note data unless the execution surface explicitly identifies a provider-supported input type.
- Execution records and raw details remain private local process. This feature does not publish them as Knowledge.
- Existing unfinished visual verification from feature 013 remains a separate follow-up unless this feature changes the same states.

## Out of Scope

- Autonomous Task selection, scheduling, or multi-agent orchestration.
- More executable providers or a general provider plug-in framework.
- Automatic Task completion, Problem resolution, Knowledge publication, or proposal application.
- New Knowledge search, frontmatter synchronization, or conflict-review behavior.
- Treating free-form text as an approval response.
