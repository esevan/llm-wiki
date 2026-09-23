# Feature Specification: Task Work Sessions

**Feature Branch**: `feature/task-session-mvp`

**Created**: 2026-09-21

**Status**: Draft

**Input**: Add a durable work-session space inside each Task detail so users can create, select, reopen, configure, and continue multiple isolated work conversations without running AI.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resume Task Work (Priority: P1)

A user creates a work session inside a Task, records one or more chat-style notes, closes the screen or application, and later reopens the exact session with its records intact.

**Why this priority**: Durable resumption is the central user value and directly serves the product principle “Resume Where You Left Off.”

**Independent Test**: Create a session under Task A, save a note, restart the application, reopen Task A and the session, and confirm the same note and attachment remain available without a new session appearing.

**Acceptance Scenarios**:

1. **Given** a Task with no work sessions, **When** the user opens its detail, **Then** no session is created until the user explicitly chooses New session.
2. **Given** a session with saved records, **When** the application is reopened, **Then** the session appears and its records can be reopened.
3. **Given** a save failure, **When** the user remains in the composer, **Then** the text and attachment remain available for retry and the retry does not create duplicate records.

---

### User Story 2 - Keep Sessions and Tasks Separate (Priority: P1)

A user maintains several sessions for one Task and moves between different Tasks without records, drafts, settings, or attachments leaking across boundaries.

**Why this priority**: Incorrect cross-Task or cross-session disclosure would break trust in the private local workspace.

**Independent Test**: Create records in two sessions under Task A and one session under Task B, switch among all three, and verify that each shows only its own saved and unsaved content.

**Acceptance Scenarios**:

1. **Given** two sessions under the same Task, **When** the user switches between them, **Then** each session shows only its records and pending draft.
2. **Given** sessions under Task A, **When** the user opens Task B, **Then** Task A sessions and records are not listed or readable.
3. **Given** a session identifier from Task A, **When** it is used through Task B, **Then** reads and writes are rejected.

---

### User Story 3 - Prepare Future Execution Context (Priority: P2)

A user chooses a Codex model, an approval preference, and a project folder path for a session, and can attach files or paste an image while clearly seeing that the MVP only saves these values and does not execute AI or read project files.

**Why this priority**: These settings establish a useful extension boundary without introducing an execution system prematurely.

**Independent Test**: Save settings and a file in a session, reopen it, and verify the exact settings, preview/download behavior, and inactive-execution explanation.

**Acceptance Scenarios**:

1. **Given** a session, **When** the user selects Codex and a supported model and saves settings, **Then** those settings are restored on reopen.
2. **Given** an image pasted into the composer, **When** it is selected for the next record, **Then** a preview appears and the saved image can be reopened or downloaded.
3. **Given** a project folder path, **When** it is saved, **Then** it is retained as context without any automatic file access.
4. **Given** any use of the workspace, **When** records or settings are saved, **Then** the Task state and completion state do not change.

### Edge Cases

- Empty Task workspaces stay empty across repeated opens.
- Empty records cannot be saved unless they include an attachment.
- Files larger than 10 MB are rejected while the existing text draft remains.
- Long Task titles, session titles, notes, and folder paths wrap without hiding primary actions.
- A removed or invalid Task/session produces an explicit error and never falls back to another Task’s data.
- Korean and English labels remain usable at narrow and wide window sizes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The Task detail MUST expose a work-session space that follows the existing visual and keyboard interaction patterns.
- **FR-002**: Users MUST explicitly create, select, rename, and reopen multiple sessions belonging to one Task.
- **FR-003**: Opening a Task or session list MUST NOT create a session.
- **FR-004**: The system MUST durably preserve each session’s Task ownership, settings, ordered records, and attachments across application restarts.
- **FR-005**: Each record MUST distinguish author and kind so future AI output and execution results can be represented without changing user notes.
- **FR-006**: Failed record saves MUST retain composer text and attachment and support an idempotent retry.
- **FR-007**: Reads and writes MUST enforce both Task and session identity; no record, setting, draft, or attachment may cross Task or session boundaries.
- **FR-008**: The session space MUST show current canonical Task content and existing references without creating a separate Task-content copy.
- **FR-009**: The MVP MUST provide a Codex provider choice and a fixed catalog of five supported model choices, defaulting new sessions to GPT-5.6-Sol.
- **FR-010**: The MVP MUST save an approval preference and project folder path while explaining that they are inactive until a future execution feature exists.
- **FR-011**: The system MUST NOT read files from the saved project path or invoke AI, a CLI, an execution engine, run management, or multi-agent orchestration.
- **FR-012**: Users MUST be able to attach a file or paste an image up to 10 MB, preview a pending image, and reopen or download a saved attachment.
- **FR-013**: Saving sessions, settings, or records MUST NOT change Task workflow state, complete a Task, resolve a Problem, publish Knowledge, or create a Work Log entry.
- **FR-014**: The experience MUST support keyboard operation and Korean and English presentation at wide and narrow desktop widths.

### Key Entities

- **Task Work Session**: A private resumable workspace owned by exactly one Task, with title, provider, model, approval preference, project folder path, and timestamps.
- **Session Record**: An ordered durable item owned by one session, with an author, kind, text, optional attachment, and timestamp.
- **Session Attachment**: App-managed file content with its original name and media type, limited to 10 MB.
- **Pending Composer Draft**: Unsaved text and attachment isolated to a selected session and retained after a failed save while the screen remains open.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of restart checks, an explicitly created session and its successfully saved records, settings, and attachments are restored.
- **SC-002**: In cross-boundary tests covering two Tasks and two sessions per Task, zero records, drafts, settings, or attachments appear outside their owner.
- **SC-003**: Reopening an empty Task detail five consecutive times creates zero sessions.
- **SC-004**: A failed save followed by retry creates exactly one saved record and retains the draft until success.
- **SC-005**: All primary session actions are reachable by keyboard in both Korean and English at 1280 px and 640 px viewport widths without horizontal page overflow.
- **SC-006**: Across all workspace scenarios, the Task workflow state remains exactly as it was before the workspace was used.
- **SC-007**: After stored session data becomes available, the work-session view updates within 100 ms at the 95th percentile under normal local use.

## Out of Scope

- Actual AI or CLI calls, automatic assistant replies, execution engines, run management, and multi-agent orchestration.
- Automatic Work Log creation, Task completion, Problem resolution, Knowledge synchronization or publication, frontmatter search, and conflict review.
- Project-folder traversal, file indexing, attachment indexing, and unsaved-draft restoration after application restart.

## Assumptions

- This is a single-user local desktop workspace; collaboration and sync remain outside scope.
- Codex is the only provider shown in this MVP. The catalog contains GPT-6-Astra, GPT-5.6-Sol, GPT-5.6-Terra, GPT-5.6-Luna, and GPT-5.5, but selection does not guarantee future runtime availability.
- “Approval for me” is stored as future execution intent and has no enforcement effect in this MVP.
- The project folder path is entered as text and is never traversed or validated in this MVP.
- Successfully saved records persist across restart; unsaved composer drafts are guaranteed through save failure and in-screen switching, not application restart.
- Existing Task detail, canonical Task references, local persistence, localization, and focus patterns are dependencies to reuse.
