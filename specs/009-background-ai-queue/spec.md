# Feature Specification: Background AI Queue

**Feature Branch**: `[009-background-ai-queue]`

**Created**: 2026-09-01

**Status**: Draft

**Input**: User description: "Route AI work through purpose-specific background queues, show actionable work in a bottom-right Queue interface, provide task-specific completion destinations and notifications, translate Capture, Work Log, and Knowledge content in the background, and keep responsive interaction work in a hidden Fast Queue."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See meaningful AI work without blocking (Priority: P1)

As a user, I can continue working while longer AI work runs and can inspect its state from a compact Queue interface at the bottom right of the screen.

**Why this priority**: The primary value is removing AI latency from the foreground without making work disappear.

**Independent Test**: Start two long-running AI tasks, continue navigating the application, and verify that the Queue exposes each task's current state and appropriate completion action without blocking other work.

**Acceptance Scenarios**:

1. **Given** no visible work is active, **When** a regular background task is submitted, **Then** the bottom-right Queue indicates that work is active without taking focus or changing the current scroll position.
2. **Given** multiple regular tasks exist, **When** the user opens Queue, **Then** each task shows a recognizable type, related item, state, progress when measurable, and available action.
3. **Given** a regular task succeeds, fails, or is cancelled, **When** its state changes, **Then** Queue reflects the terminal state without requiring a page reload.
4. **Given** a Chat or another responsive interaction task uses Fast Queue, **When** it runs, **Then** one shared request throttler serializes it separately from background work without persisting status, appearing in Queue, or creating a notification.

---

### User Story 2 - Receive each result in the right context (Priority: P1)

As a user, I receive completed AI output where it is useful rather than being sent to a generic result page.

**Why this priority**: A single result experience would interrupt Draft, Work Log, review, and translation workflows in different ways.

**Independent Test**: Complete one task of every defined result type and verify that each uses the destination and interruption policy in the task interface matrix.

**Acceptance Scenarios**:

1. **Given** a Draft or Refine surface remains open, **When** its task completes, **Then** the proposal appears in that surface for user review without navigation.
2. **Given** the originating Draft or Refine surface is closed or superseded while work is running, **When** cancellation is processed, **Then** the work stops and no detached result page or completion notification is created.
3. **Given** an Image Summary task completes, **When** the user remains on the current screen, **Then** the summary is added without changing the current tab or scroll position.
4. **Given** a completed Image Summary Queue entry is selected, **When** its result action opens, **Then** the related Solution opens on the Work tab and scrolls to the exact summary.
5. **Given** a Completion Review finishes, **When** the result becomes available, **Then** the user receives a temporary bottom-right notification and can open the review result for the required follow-up decision.

---

### User Story 3 - Recover background translations (Priority: P1)

As a user, Capture, Work Log, and Knowledge translations continue in the background, preserve original content, and resume from completed work after interruption.

**Why this priority**: Translation consistency must not reintroduce foreground latency or lose progress on long documents.

**Independent Test**: Interrupt translations after several paragraphs, restart processing, and verify that completed checkpoints are reused, original text is unchanged, and the completed Knowledge translation becomes the durable reading version.

**Acceptance Scenarios**:

1. **Given** a Capture or translatable Work Log entry is created or changed, **When** the save succeeds, **Then** missing or stale derived translations are enqueued immediately without blocking the save.
2. **Given** a managed Knowledge document has a missing or stale reading version for a supported non-canonical language, **When** the document becomes eligible for translation, **Then** paragraph translation proceeds in the background even when its reader is not open.
3. **Given** translation stops after some paragraphs, **When** it resumes, **Then** valid completed paragraph checkpoints are reused and only remaining or stale paragraphs are processed.
4. **Given** Knowledge translation completes, **When** the durable reading version is handed off, **Then** the completed queue working data is removed and the reading version remains available from Knowledge.
5. **Given** source text changes during or after translation, **When** a checkpoint or completed translation is considered, **Then** output from an older source revision is not presented as current.

---

### User Story 4 - Notice work requiring a decision (Priority: P2)

As a user, I am notified about completed work that requires my follow-up without being notified about routine or conversational AI work.

**Why this priority**: Completion Review can be missed after asynchronous execution, while notifications for every AI task would create noise.

**Independent Test**: Complete a Completion Review while looking elsewhere, allow its temporary notification to disappear, and verify that the unread alert remains discoverable and opens the review.

**Acceptance Scenarios**:

1. **Given** a Completion Review becomes ready, **When** the user is elsewhere, **Then** a temporary bottom-right notification appears without taking focus.
2. **Given** that notification disappears before it is opened, **When** the user views the top-right bell, **Then** its badge includes the unread review notification.
3. **Given** the user opens or dismisses the notification, **When** unread status changes, **Then** the badge count updates consistently.
4. **Given** routine translation, Image Summary, Draft, Refine, or Fast Queue work completes, **When** it reaches a terminal state, **Then** it does not add a bell notification.

### Task and Completion Interface Matrix

| Work category | Queue | Completion interface | Notification | Lifetime and application rule |
| --- | --- | --- | --- | --- |
| Draft next-stage Problem or Solution | Visible regular Queue | Inline proposal in the still-open originating surface; no generic result page | None | Cancel when the originating surface closes or is superseded; user explicitly applies the proposal |
| Refine Capture, Problem, or Solution | Visible regular Queue | Inline refinement preview in the still-open originating surface | None | Cancel when the originating surface closes or is superseded; user explicitly applies the proposal |
| Work Log Image Summary | Visible regular Queue | Add summary in place without navigation; completed entry deep-links to the Solution Work tab and exact summary | None | Preserve active tab and scroll; attach only to the explicitly requested Work Log entry |
| Completion Review | Visible regular Queue | Dedicated review result with actions for the user's follow-up decision | Temporary notification plus persistent unread bell entry | Review output never completes or changes workflow state by itself |
| Conflict Review | Visible regular Queue | Existing conflict-review result in the related Solution context | Persistent unread notification only when a user decision is required | Preserve cancellation, evidence, and stale-source safeguards |
| Knowledge translation | Visible while active | The related Knowledge reading is the result; no separate generic result page | None | Process supported derived languages by paragraph, resume from checkpoints, then remove completed working records after durable handoff |
| Capture and Work Log derived translation | Visible regular Queue | Completed entry opens the related item and translated reading | None | Enqueue immediately after source save; preserve authored source and invalidate stale derived output |
| Workbench organization | Visible regular Queue | Preview of proposed organization in Workbench context | Persistent unread notification only if user approval is required | Never reorganize durable work without explicit approval |
| Lineage inference | Visible regular Queue | Related Solution Lineage view | None unless a recoverable user decision is required | Add only marked inference; do not change recorded evidence |
| Completion report generation | Visible regular Queue | Related completed Problem or Knowledge preview | Persistent unread notification if publication or conflict review is required | Publication remains an explicit user action |
| Embedding generation, refresh, and cleanup | Visible regular Queue while active | No result page; search readiness reflects valid completed document units | None | Resume by unchanged document checkpoint and keep lexical search available throughout |
| Problem enrichment | Fast Queue when used as an interactive aid; otherwise regular Queue | Inline in its originating context | None | Any durable change remains a proposal |
| Capture, Problem, Solution, and completed-Solution Chat; next-stage conversation | Hidden Fast Queue | Existing inline streamed conversation | None | Use separate capacity; closing the conversation cancels its active request |
| Provider setup and model discovery checks | Not an AI content task | Existing setup feedback | None | Excluded from AI Queue metrics and result history |

### Edge Cases

- Closing and reopening the global Queue does not cancel work; only task-specific cancellation rules do.
- Closing an originating Draft or Refine surface just as the task completes must not attach a late inline result to a different item or newer request.
- Repeated clicks are idempotent: they reuse an equivalent active task or clearly create a new attempt without applying duplicate results.
- A failed task preserves source content and any prior valid result, exposes a useful failure state, and offers retry only where retry remains safe.
- Loss of connectivity or application restart does not cause durable regular-Queue tasks to remain permanently running; recoverable work resumes and unsafe work becomes explicitly retryable.
- Fast Queue saturation does not consume the execution capacity reserved for regular background work, and regular long work does not prevent responsive interaction work from starting.
- Multiple application processes do not multiply Fast Queue concurrency or permit more than one Fast request to reach the provider at a time.
- An asynchronous worker that stops after claiming work does not leave that work permanently active; the work is safely resumed, made retryable, or failed according to its task policy.
- A repeated delivery of the same asynchronous work does not duplicate a durable result, notification, Vault file, summary, or embedding.
- A result targeting a deleted or externally changed item is retained as unapplied or marked stale and never attached to another item.
- Knowledge checkpoint reuse requires both the same source revision and the same paragraph identity; changed paragraphs are processed again.
- An unread alert whose target is later removed becomes dismissible and explains that the original result is no longer available.
- Queue and notification overlays remain keyboard accessible and do not cover the focused control or trap focus.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST classify every AI content task and embedding task as either regular background work or responsive Fast Queue work according to the task matrix.
- **FR-002**: Regular and Fast Queue work MUST have independent execution capacity so long-running work in one class does not starve the other.
- **FR-002a**: Fast Queue MUST act as one shared first-in-first-out throttler with exactly one active provider request across all application processes; it MUST NOT persist job status, expose global progress, retain result history, retry automatically, or survive application restart.
- **FR-003**: The bottom-right Queue MUST show all active regular tasks and MUST NOT show Fast Queue tasks.
- **FR-004**: Each visible task MUST expose its recognizable task type, related work item, current state, and measurable progress when the task has countable units.
- **FR-005**: The Queue MUST support at least queued, running, awaiting-user-review, completed, failed, cancelling, and cancelled outcomes, while presenting user-facing wording appropriate to the task.
- **FR-006**: Each completed visible task MUST use the completion interface defined in the task matrix rather than a mandatory generic result page.
- **FR-007**: Queue interactions and automatic result placement MUST preserve the current page, active tab, focus, and scroll unless the user explicitly chooses a result destination.
- **FR-008**: Draft and Refine work MUST appear in Queue while its originating surface remains open and MUST be cancelled when that surface closes or is superseded.
- **FR-009**: A successful Draft or Refine result MUST remain an unapplied proposal until the user explicitly accepts it.
- **FR-010**: A successful Image Summary MUST attach naturally to its requested Work Log entry without changing the current scroll position; its completed Queue action MUST open the related Solution Work tab at that summary.
- **FR-011**: Completion Review MUST run as regular background work and MUST expose a result that the user can inspect before taking any completion-related action.
- **FR-012**: Completion Review readiness MUST create both a temporary bottom-right notification and a persistent unread alert accessible from a top-right bell with an accurate unread-count badge.
- **FR-013**: Opening or dismissing an alert MUST update its read state and the visible unread count; routine and Fast Queue completions MUST NOT create alerts unless the task matrix explicitly requires one.
- **FR-014**: Knowledge MUST maintain background reading-version work for every supported language that differs from the current canonical language whenever that derived version is missing or stale.
- **FR-015**: Knowledge translation MUST track progress by stable paragraph units and MUST resume using only checkpoints that still match the current source revision and paragraph content.
- **FR-016**: After a Knowledge translation is durably available in the Vault, completed working records and paragraph checkpoints MUST be removed from the active work store without removing the durable result.
- **FR-017**: Capture saves and Work Log body, comment, and checklist-item saves MUST immediately enqueue missing or stale derived translations of their natural-language text without delaying the source save; code, URLs, paths, identifiers, and quoted evidence MUST remain unchanged inside derived readings.
- **FR-018**: Derived translations MUST preserve authored source text and MUST never replace the original Capture or Work Log evidence.
- **FR-019**: Chat and other responsive interaction tasks assigned to Fast Queue MUST remain inline, hidden from Queue, free of completion notifications, and cancellable with their interaction surface.
- **FR-020**: The system MUST prevent late, duplicate, cancelled, or source-stale results from being attached or applied to current content.
- **FR-021**: Recoverable regular work MUST survive application interruption; work that cannot safely resume MUST become explicitly failed or retryable instead of appearing indefinitely active.
- **FR-022**: Cancelling work MUST retain existing source content and prior valid output and MUST communicate whether provider execution actually stopped or only its result was discarded.
- **FR-023**: AI results that can change workflow organization, status, publication, or durable authored fields MUST require explicit user review and approval.
- **FR-024**: Queue, result links, temporary notifications, bell alerts, failure actions, and unread state MUST be operable by keyboard and understandable without relying on color alone.
- **FR-025**: Korean user-facing text MUST refer to the person using the product as `사용자` and MUST NOT use `인간` as the translation of "user" or "human."
- **FR-026**: Regular work ownership MUST expire when its worker becomes unavailable so that no task remains permanently active, and recovery MUST prevent two workers from publishing the same durable result.
- **FR-027**: Translation, embedding, and other restartable work MUST persist only validated checkpoints tied to the exact source revision and generation model.
- **FR-028**: Embedding work MUST run as regular background work, report document-level progress, resume from valid document checkpoints, reject stale document output, and leave lexical search usable before semantic coverage is complete.
- **FR-029**: The number of regular background workers MUST be configurable without changing Fast Queue's single-request throttling rule or interrupting work already running.
- **FR-030**: During migration from foreground execution, the system MUST preserve each existing API's successful result, failure behavior, source-protection rules, and user-visible side effects until that API's asynchronous replacement contract is explicitly introduced and validated.
- **FR-031**: The completed implementation MUST have one authoritative implementation for each execution path and MUST remove superseded queue, provider, background-thread, translation, embedding, and routing code rather than retaining parallel legacy behavior.
- **FR-032**: The completed implementation MUST separate web composition, HTTP controllers, application services, asynchronous task handlers, synchronous operations, and persistence repositories with dependencies directed inward and no HTTP concerns in services or persistence logic in controllers.

### Key Entities

- **AI Work Item**: One requested AI operation, including its class, task type, related item, lifecycle state, progress, source identity, attempts, cancellation intent, and result destination.
- **Fast Request**: An ephemeral responsive AI interaction serialized by one shared throttler, with no durable identity, global status, retry, recovery, history, or completion notification.
- **Task Result**: Output associated with exactly one work item and source revision, together with whether it is inline, navigable, awaiting review, safely attached, stale, or discarded.
- **Notification**: A follow-up signal associated with a result, including temporary display state, read state, target, and whether a user decision is still outstanding.
- **Translation Unit Checkpoint**: Completed work for one stable paragraph and source revision that permits safe resumption without treating stale content as current.
- **Embedding Unit Checkpoint**: Validated embedding work for one document source revision and model identity that permits safe resumption without indexing stale content.
- **Work Lease**: Time-limited ownership of a regular work item that allows another worker to recover it after its owner becomes unavailable without permitting duplicate publication.
- **Derived Translation**: A supported-language reading version linked to preserved authored or canonical source content and invalidated when that source changes.
- **Originating Surface**: The specific item, panel, dialog, tab, or conversation that owns an ephemeral Draft, Refine, or Chat request and controls its cancellation lifetime.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance coverage, 100% of AI content task types are assigned to regular Queue or Fast Queue and exhibit the visibility, result, notification, and cancellation behavior in the task matrix.
- **SC-002**: A user can submit regular AI work and resume non-AI interaction within one second in at least 95% of representative local acceptance runs.
- **SC-003**: Queue state changes become visible without manual refresh within two seconds in at least 95% of acceptance observations.
- **SC-004**: Draft, Refine, and Image Summary completion changes the user's active page, tab, focus, or scroll position in 0% of acceptance cases unless the user selects a result link.
- **SC-005**: Closing or superseding a Draft, Refine, or Chat surface prevents its late result from appearing or being applied in 100% of cancellation race acceptance cases.
- **SC-006**: Completion Review generates exactly one unread alert, remains discoverable after its temporary notification disappears, and opens the correct review in 100% of acceptance cases.
- **SC-007**: A Knowledge translation interrupted after at least one completed paragraph resumes without reprocessing unchanged checkpointed paragraphs in 100% of controlled interruption cases.
- **SC-008**: Source edits cause stale paragraph checkpoints and derived translations to be rejected in 100% of acceptance cases.
- **SC-009**: Capture and eligible Work Log source saves complete without waiting for translation in 100% of acceptance cases, while translation work is enqueued immediately after successful persistence.
- **SC-010**: Fast Queue tasks remain absent from Queue and notification history and can begin while regular work is saturated in 100% of capacity-isolation acceptance cases.
- **SC-011**: Provider failure, cancellation, application restart, and target deletion preserve all pre-existing user content in 100% of recovery acceptance cases.
- **SC-012**: All Queue and notification journeys can be completed with keyboard-only operation and expose status through text or accessible semantics in 100% of accessibility acceptance cases.
- **SC-013**: With at least two web processes active, no more than one Fast Queue provider request runs concurrently in 100% of throttling acceptance observations.
- **SC-014**: After terminating a regular worker at every persisted transition boundary, 100% of controlled recovery cases end in one valid completion, an explicit retryable or failed state, or safe cancellation, with no permanently active work.
- **SC-015**: Re-delivering the same regular work produces no duplicate durable result or notification in 100% of idempotency acceptance cases.
- **SC-016**: An interrupted embedding run resumes without recomputing unchanged validated document units and leaves lexical search usable in 100% of controlled interruption cases.
- **SC-017**: Before each foreground API is replaced, its characterization suite passes against both the original execution path and its synchronous job-recording compatibility path with no unapproved observable behavior changes.
- **SC-018**: Static analysis and repository inventory find zero superseded direct provider calls, unmanaged background threads, duplicate task handlers, unreferenced feature modules, or unreachable compatibility branches after migration.

## Assumptions

- The currently supported locales are Korean and English. English-canonical Knowledge therefore needs a Korean derived reading; the rule automatically covers additional supported non-canonical languages if they are added later.
- Fast Queue is a single application-wide request throttler rather than a durable queue; only request-scoped cancellation information exists while an interaction is active.
- Regular work may be executed by multiple independent worker processes, but durable state rather than process memory determines ownership, recovery, progress, and completion.
- Migration uses an expand-migrate-contract sequence: characterize existing APIs, record synchronous executions as completed regular work without changing their public result contract, move shared task behavior behind background handlers, and only then introduce accepted asynchronous API contracts.
- "Close the current window" means closing, navigating away from, or superseding the item-specific Draft, Refine, or conversation surface, not collapsing the global Queue or minimizing the application window.
- A requested Image Summary is safe to attach to its existing Work Log entry because the user explicitly initiated that summary request; it does not change workflow state.
- Capture and Work Log translation creates a derived reading and never rewrites authored evidence, so making the derived result available does not constitute approval of a workflow-state change.
- Completed Knowledge working records are removed only after the durable derived file is successfully available and validated; failed and resumable records remain until retried or explicitly cancelled.
- Regular Queue shows active work and a bounded recent result history. Durable results remain discoverable from their owning Capture, Solution, review, Lineage, or Knowledge view.
- The existing single-user, local privacy boundary and configured AI provider remain unchanged.
- Provider configuration tests and model discovery are operational checks rather than AI content tasks and are excluded from Queue classification.

## Product Spirit Alignment

- **Reduce Cognitive Load**: Queue makes long-running work observable in one compact location, while Fast Queue and notification filtering prevent routine AI activity from creating visual noise.
- **Resume Where You Left Off**: Persistent regular work, paragraph checkpoints, deep-linked results, and unread review alerts preserve the exact place and decision required to continue.
- **Private Process, Portable Knowledge**: Source Capture and Work Log evidence remain unchanged, while completed Knowledge readings move to durable portable storage only after successful validation.
- **User Authority over AI**: Drafts, refinements, reviews, organization proposals, publication, and workflow-state changes remain subject to explicit user action; automatic derived translations and requested summaries do not alter their sources or workflow state.
- The feature does not create a workflow stage below Solution, expose Fast Queue process unnecessarily, or score a worker.
