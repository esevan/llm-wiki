# Feature Specification: Refinement Preview Status

**Feature Branch**: `005-refinement-preview-status`
**Created**: 2026-08-21
**Last Reconciled**: 2026-09-02
**Status**: Implemented — reconciled with current behavior

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Resume from current context (Priority: P1)

When a user opens Explore for a Problem or Solution, the workspace displays saved item detail and a
bounded summary of available current and lineage context before a new AI refinement is ready.

**Independent Test**: Open Explore for records with title-only detail, saved detail, recent chat,
previous refinement, and parent lineage, then verify the current record's context appears without a
new summary-generation request.

**Acceptance Scenarios**:

1. **Given** a record with only a title, **When** Explore opens, **Then** the title is available as
   current context.
2. **Given** saved detail or prior conversation, **When** Explore opens, **Then** the most recent
   distinct context is shown in a bounded list.
3. **Given** a prior unapplied refinement or next-stage Draft, **When** Explore reopens, **Then** the
   latest proposal can be restored for review.

### User Story 2 — Continue chatting while refinement runs (Priority: P1)

After a successful Problem or Solution chat response, a background Refine job updates the adjacent
Preview while the user can keep using the conversation.

**Acceptance Scenarios**:

1. **Given** a current Problem or Solution conversation, **When** the chat response completes,
   **Then** the Preview reports that refinement is updating and remains on the current context until
   the proposal is ready.
2. **Given** a completed proposal, **When** it is displayed, **Then** it remains unapplied until the
   user selects Apply Refinement.
3. **Given** a newer item or request supersedes the current attempt, **When** the older result
   arrives, **Then** it is not displayed or applied to the new target.

### User Story 3 — Recover from Preview failure (Priority: P1)

A failed Preview leaves the underlying Explore conversation and existing context available, marks
the Preview status as needing attention, and permits a later attempt.

**Acceptance Scenarios**:

1. **Given** a visible context Preview, **When** background refinement fails, **Then** the context
   remains readable and the status control exposes an error icon and accessible explanation.
2. **Given** a failed attempt, **When** a new attempt begins or the workspace closes, **Then** the
   previous warning is cleared.

### User Story 4 — Preview the next workflow stage (Priority: P2)

Capture→Problem and Problem→Solution conversations can prepare the next-stage Draft beside the
conversation. Creating the next record remains a separate user action.

### Edge Cases

- Context is limited to 500 visible content characters and at most five distinct entries.
- Long entries are truncated with an ellipsis and duplicate context is removed.
- Boilerplate such as “unknown” or “not yet known” is not treated as meaningful detail.
- Capture Explore has conversation but no same-stage background refinement Preview.
- Closing or superseding the workspace cancels the associated Draft or Refine job when one exists.
- Applying a proposal can fail independently of generating it; the proposal remains reviewable.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Same-stage background Refinement Preview MUST apply to Problem and Solution Explore
  workspaces.
- **FR-002**: Capture and Problem next-stage conversations MUST support background Draft Preview.
- **FR-003**: Existing saved detail and context MUST be available before a new AI proposal completes.
- **FR-004**: Context summaries MUST be deterministic from retained workflow context and MUST NOT
  require a separate AI summary request.
- **FR-005**: Context summaries MUST contain no more than five distinct entries and 500 visible
  content characters.
- **FR-006**: Current saved detail, recent relevant conversation, refinement history, and linked
  source records MAY contribute to the context summary.
- **FR-007**: The active Preview MUST identify whether it is loading context, refining, drafting,
  ready, applied, created, or needs attention.
- **FR-008**: Completed proposals MUST show the current proposal fields together with retained
  context and MUST remain unapplied until explicit user action.
- **FR-009**: A failed generation MUST preserve the underlying Explore workspace and any previously
  loaded context.
- **FR-010**: Failure status MUST have an accessible name and a pointer- and keyboard-readable
  explanation.
- **FR-011**: A new attempt, target change, or workspace close MUST clear the previous failure state.
- **FR-012**: Closing or superseding a Draft or Refine surface MUST request cancellation and MUST
  prevent late results from attaching to a different target.
- **FR-013**: Capture conversation MUST remain available without introducing a same-stage Capture
  refinement Preview.

### Key Entities

- **Refinement context**: Bounded current and lineage information shown before a proposal is ready.
- **Preview attempt**: One target-bound Draft or Refine request and its visible status.
- **Refinement proposal**: An unapplied current-stage change prepared for user review.
- **Next-stage Draft**: An unapplied Problem or Solution proposal prepared from its source stage.

## Success Criteria *(mandatory)*

- **SC-001**: Context-bearing Problem and Solution workspaces display retained context before the
  background proposal completes in all browser acceptance tests.
- **SC-002**: Context output remains within five entries and 500 visible content characters in all
  boundary tests.
- **SC-003**: Forced Preview failures preserve context and expose an accessible error status in all
  tested Problem and Solution cases.
- **SC-004**: A stale or superseded result replaces the current Preview in zero tested race cases.
- **SC-005**: Draft and Refine proposals change durable workflow content only after the explicit
  apply/create action.

## Assumptions

- “Context” includes the current title even when no longer detail or conversation exists.
- Visible status wording is localized and is not constrained to the former exact Korean strings.
- Current status is communicated through text and accessible semantics as well as color.

## Confirmed Product Decisions

- Title-only records count as prior context.
- Refinement context is limited to at most five distinct entries and 500 visible content characters.
