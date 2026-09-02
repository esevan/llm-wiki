# Feature Specification: Background AI Queue

**Feature Branch**: `009-background-ai-queue`
**Created**: 2026-09-01
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — confirmed queue changes and two decisions pending

## User Scenarios & Testing *(mandatory)*

### User Story 1 — See durable AI work without blocking (Priority: P1)

A user submits longer AI or embedding work, immediately resumes ordinary interaction, and can open
the bottom-right Queue to inspect task type, target, status, progress, result destination, and safe
error information.

**Acceptance Scenarios**:

1. **Given** submitted durable work, **When** the Queue is opened, **Then** recognizable task and
   target descriptions are shown rather than raw input or private payload data.
2. **Given** completed work with a destination, **When** the user selects its result action, **Then**
   the task-specific owning surface opens.
3. **Given** failed or retryable work, **When** retry remains allowed, **Then** a retry action is
   available without changing the source content.

### User Story 2 — Keep interaction work responsive and quiet (Priority: P1)

Chat and other responsive provider interactions pass through one shared Fast Queue throttler. They
remain inline, have no durable status or notification, and do not appear in the visible Queue.

### User Story 3 — Receive each background result in context (Priority: P1)

Draft, Refine, Image Summary, Completion Review, conflict review, translation, Workbench
organization, Lineage, completion report, and embedding results use different completion interfaces
appropriate to their owning content.

### User Story 4 — Recover durable work (Priority: P1)

Durable work survives web requests and worker interruption through stored status, source identity,
leases, retries, idempotency, and source/model-bound checkpoints.

### User Story 5 — Notice a required completion decision (Priority: P2)

Completion Review readiness creates a temporary toast and a persistent unread notification with a
bell count. Reading or dismissing it updates the count.

### User Story 6 — Run the local service roles (Priority: P2)

A user can run the complete local service, web-only role, Fast Queue role, or durable worker role
independently. The complete service opens the browser unless disabled, keeps network listeners on
the local machine, and stops its child workers when the service exits. A macOS user can install the
complete service to start at login.

### Task and Completion Interface Matrix

| Work category | Queue visibility | Current completion interface | Notification |
| --- | --- | --- | --- |
| Next-stage Draft | Visible | Inline proposal in the still-current originating surface; no generic result page | None |
| Current-stage Refine | Visible | Inline proposal in the still-current originating surface; no generic result page | None |
| Work Log Image Summary | Visible | Automatic in-place attachment; result action opens Solution Work and the summary | None |
| Completion Review | Visible | Dedicated review result requiring user follow-up | Temporary toast and persistent unread alert |
| Conflict Review | Visible | Related conflict-review result | None |
| Knowledge translation | Visible | Related translated Knowledge document | None |
| Capture/Work derived translation | Visible | Owning content or readable result summary | None |
| Workbench organization | Visible | Automatically refreshed Workbench | None |
| Lineage inference | Visible | Completed Solution Lineage | None |
| Completion report | Visible | Completed Knowledge document | None |
| Embedding refresh | Visible | Readable embedding-coverage summary | None |
| Problem enrichment | Hidden Fast Queue | Inline response | None |
| Current, next-stage, and completed-Solution Chat | Hidden Fast Queue | Inline streamed response | None |
| Provider setup/model discovery | Excluded | Existing setup feedback | None |

### Edge Cases

- Equivalent active durable requests reuse one work item.
- Queued and retryable work cancels immediately; running work first becomes cancelling.
- Source changes make late Draft, Refine, summary, translation, conflict, Lineage, report, or
  embedding output stale instead of current.
- Expired worker ownership makes unfinished work retryable.
- Transient network, timeout, rate-limit, and server failures retry with bounded attempts; permanent
  failures become failed.
- Fast Queue requests are not recovered after process restart and are not retried automatically.
- Closing or disconnecting an interactive Fast Queue surface aborts its active provider request.
- Queue history contains up to the most recent 100 durable jobs, including completed Knowledge
  translation jobs.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every current AI content and embedding task MUST use either durable background work or
  the hidden Fast Queue classification shown in the task matrix.
- **FR-002**: Durable work and Fast Queue MUST have independent execution capacity.
- **FR-003**: Fast Queue MUST provide one shared FIFO provider request across application web
  processes and MUST NOT persist status, progress, history, result, retry, or notification state.
- **FR-004**: Durable work MUST expose queued, running, awaiting review, completed, cancelling,
  cancelled, failed, retryable, and stale states.
- **FR-005**: Visible work MUST expose recognizable task type, owning target, safe status, timestamps,
  countable progress, result destination, and safe error text where available.
- **FR-006**: Visible work MUST NOT expose its private input payload or provider secret.
- **FR-007**: Queue state changes MUST be observable without manual page refresh.
- **FR-008**: Completed work MUST use the task-specific interface in the matrix rather than requiring
  one generic result page.
- **FR-009**: Queue activity and automatic Image Summary or Workbench refresh MUST preserve the
  active page and avoid unrelated navigation; explicit result actions MAY navigate.
- **FR-010**: Draft and Refine results MUST remain unapplied and MUST attach only to the originating
  target and current surface.
- **FR-011**: Closing or superseding Draft and Refine surfaces MUST request cancellation and MUST
  prevent late attachment.
- **FR-012**: Image Summary MUST attach to the explicitly requested Work Log entry and its result
  action MUST open the related Work view.
- **FR-013**: Completion Review MUST remain advisory and MUST create one idempotent unread
  review-ready notification when successful.
- **FR-014**: Reading or dismissing a notification MUST update its persistent read/dismiss state and
  unread count.
- **FR-015**: Routine durable completion and every Fast Queue request MUST produce no notification.
- **FR-016**: Knowledge translation MUST report paragraph progress, reuse only exact
  source/model/unit checkpoints, write the durable derived reading before completion, and clear its
  paragraph checkpoints after successful publication.
- **FR-017**: Completed Knowledge translation work MUST remain in recent Queue history even after its
  paragraph checkpoints are cleared.
- **FR-018**: New or changed Capture, Work Log, comment, and checklist text MUST enqueue derived
  translation after source persistence and MUST preserve the authored source.
- **FR-019**: Embedding work MUST report document progress, reuse exact document checkpoints, reject
  changed document output, and leave lexical search available.
- **FR-020**: Durable work MUST reject results whose target or source revision no longer matches.
- **FR-021**: Active idempotency keys MUST reuse equivalent active work and terminal publication and
  notification records MUST prevent duplicate side effects.
- **FR-022**: Worker ownership MUST expire and allow safe recovery without two workers publishing the
  same result.
- **FR-023**: Recoverable transient failures MUST retry with bounded attempts; non-recoverable or
  exhausted work MUST become failed with safe error information.
- **FR-024**: User cancellation MUST preserve source content and any prior valid result.
- **FR-025**: Durable result publication, notification creation, and terminal job state MUST commit as
  one observable outcome for task types that publish automatically.
- **FR-026**: Background worker count MUST be configurable from one through 32 and take effect after
  service restart; Fast Queue MUST remain single-request.
- **FR-027**: The local service runner MUST start one Fast Queue worker and the configured number of
  durable worker processes and MUST stop them when the service exits.
- **FR-028**: Korean Queue and notification copy MUST refer to the product user as `사용자`, not
  `인간`.
- **FR-029**: The complete local service MUST bind its user-facing and Fast Queue listeners to the
  local machine, start one Fast Queue worker and the configured durable worker processes, and stop
  those child processes when it exits.
- **FR-030**: Web-only, Fast Queue-only, and durable worker-only service roles MUST be independently
  runnable for process supervision.
- **FR-031**: The complete service MUST open its local browser page unless browser opening is
  explicitly disabled.
- **FR-032**: The bundled login-service installer MUST reject unsupported platforms or missing local
  prerequisites and currently supports macOS login service installation only.
- **FR-033**: Workbench organization MUST automatically populate category, priority, and other
  organization metadata that users cannot currently assign directly; it does not require a separate
  approval step.
- **FR-034**: One completion approval MUST authorize deterministic completed-work publication and
  its source-bound asynchronous AI report update.
- **FR-035**: Managed Knowledge translation MUST be scheduled only by an explicit reading or
  translation request, not by proactive translation of every managed document.
- **FR-036**: Editing an existing translatable checklist body MUST enqueue replacement derived
  translation.
- **FR-037**: Cancellation of durable or Fast Queue work MUST promptly stop active local and provider
  computation and MUST prevent later publication or delivery.
- **FR-038**: Closing a Chat surface MUST abort its active Fast Queue request.

### Key Entities

- **Durable work item**: One source-bound AI or embedding operation with status, progress, attempt,
  result interface, and safe error state.
- **Fast request**: One ephemeral inline provider interaction with no durable identity or history.
- **Work lease**: Time-limited ownership that permits recovery after worker interruption.
- **Checkpoint**: Reusable unit output tied to exact source and model identity.
- **Publication**: One idempotent durable side effect associated with a work result.
- **Notification**: A review-ready follow-up signal with read and dismissed state.
- **Originating surface**: The target-bound UI surface that owns a Draft or Refine proposal.

## Success Criteria *(mandatory)*

- **SC-001**: Every registered durable task has one authoritative handler and one task-specific
  result interface.
- **SC-002**: Fast Queue tasks appear in durable Queue and notification history in zero tests.
- **SC-003**: Concurrent durable workers publish the same work item at most once.
- **SC-004**: Equivalent active submissions create no duplicate active work item.
- **SC-005**: Expired worker ownership ends in completion, retryable, failure, stale, or cancellation
  rather than remaining permanently running.
- **SC-006**: Source-change tests attach stale output to current content in zero cases.
- **SC-007**: Completion Review produces exactly one unread alert and opens the correct review in all
  acceptance tests.
- **SC-008**: Interrupted Knowledge and embedding work reuses only unchanged validated checkpoints.
- **SC-009**: Queue result routing opens the owning content or readable task result for every task
  family in the matrix.
- **SC-010**: Provider failure and cancellation tests preserve all pre-existing user content.

## Assumptions

- Durable Queue history is a bounded recent operational history, not permanent Knowledge storage.
- The service runs locally and shares one Fast Queue endpoint among its web processes.
- Automatic Workbench organization is limited to organization metadata users cannot currently set
  directly.
- Completion is a single approval covering completed-work publication.
- Managed Knowledge translation is request-driven.

## TODO / Decisions Needed

- **TD-006 — Durable history TTL**: Define how long completed durable jobs, including Knowledge
  translation jobs, remain in recent Queue history after their durable result is published and unit
  checkpoints are cleared. The current UI bounds retrieval to 100 recent jobs but no time-based
  retention policy is defined.
- **TD-020 — Queue accessibility acceptance scope**: Select the required accessibility contract for
  Queue, toast, and notification interactions. This includes keyboard-only opening, navigation,
  activation, cancellation, retry, result opening, and dismissal; predictable focus placement and
  restoration; screen-reader announcements for status and unread-count changes; sufficient
  contrast; and reduced-motion behavior for toasts and progress changes.

## Confirmed Implementation Gaps

- **IG-006 — Durable history TTL**: A time-based cleanup policy cannot be implemented until TD-006
  is decided.
- **IG-007 — Checklist edit translation**: Existing checklist edits do not yet enqueue replacement
  translation.
- **IG-015 — Hard cancellation**: Current cancellation can prevent publication but does not
  guarantee immediate termination of an already active provider request.
- **IG-019 — Chat abort**: The Chat surface does not yet maintain and abort a dedicated active Fast
  Queue request controller when it closes.

## Product Spirit Alignment

- **Reduce Cognitive Load**: Durable work is visible in one compact Queue while Fast requests remain
  quiet and inline.
- **Resume Where You Left Off**: Persistent status, results, notifications, and checkpoints preserve
  the place from which work can continue.
- **Private Process, Portable Knowledge**: Queue payloads remain private and completed portable
  Knowledge is discoverable through its owning content.
- **User Authority over AI**: Draft, Refine, conflict review, and Completion Review remain advisory;
  Workbench organization is restricted to non-user-editable metadata, and completion is the explicit
  approval for publication.
