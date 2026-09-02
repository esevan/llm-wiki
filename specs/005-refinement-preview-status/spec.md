# Feature Specification: Refinement Preview Status

**Feature Branch**: `codex/refinement-preview-status`

**Created**: 2026-08-21

**Status**: Implemented

**Input**: User description: "Show the available prior-context summary while a Problem or Solution refinement preview is being generated; if preview generation fails, do not show the preview and instead expose a corner warning with a tooltip containing ‘preview를 띄울 수 없습니다’. Preserve the current refinement UX and do not add Preview to Capture."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand Context While Preview Is Generated (Priority: P1)

A person refining a Problem or Solution that already has meaningful context can see that the refinement is still underway and review a concise summary of the context accumulated so far, without waiting for the completed preview.

**Why this priority**: The main reported problem is that the refinement experience can feel empty and disconnected from the item and its prior reasoning while the preview is unavailable.

**Independent Test**: Start refinement for a Problem and a Solution with saved context, hold preview generation in progress, and verify that each loading preview identifies the ongoing work and presents the corresponding context summary.

**Acceptance Scenarios**:

1. **Given** a Problem has prior context, **When** its Refinement Preview is being generated, **Then** the Preview surface displays “Refine 중...” and a concise summary of the context accumulated for that Problem.
2. **Given** a Solution has prior context, **When** its Refinement Preview is being generated, **Then** the Preview surface displays “Refine 중...” and a concise summary of the context accumulated for that Solution.
3. **Given** a Problem or Solution has prior context, **When** its completed Refinement Preview becomes available, **Then** the Preview continues to provide the prior-context summary alongside the proposed refinement for human review.

---

### User Story 2 - Understand Preview Failure Without a Misleading Window (Priority: P1)

A person whose Problem or Solution preview fails remains in the refinement modal, sees no empty or broken Preview window, and can discover the failure from a warning icon without an intrusive change to the rest of the workflow.

**Why this priority**: A failed Preview must not look like valid review content, and the user explicitly chose a compact warning over displaying an error Preview.

**Independent Test**: Force preview generation to fail for both a Problem and a Solution and verify that no Preview remains visible, a warning appears in the refinement modal corner, and its keyboard- and pointer-accessible tooltip contains the required error text.

**Acceptance Scenarios**:

1. **Given** a Problem or Solution Refinement Preview is being generated, **When** generation fails, **Then** the Preview is not displayed and the underlying refinement modal remains usable.
2. **Given** Preview generation failed, **When** the user hovers over or focuses the warning icon in the refinement modal corner, **Then** a tooltip displays “Refinement preview를 띄울 수 없습니다. 다시 시도해 주세요.”
3. **Given** a Preview failure warning is visible, **When** the user starts another Preview attempt, closes the refinement modal, or changes to another item, **Then** the stale warning is cleared; a new failure may show a new warning.

---

### User Story 3 - Preserve Existing Refinement Behavior (Priority: P2)

A person using refinement without prior context, or refining a Capture, experiences the current workflow without new context panels or a newly introduced Preview.

**Why this priority**: The user considers the current UX good and explicitly excluded a permanent context block and Capture Preview from the change.

**Independent Test**: Compare a no-context Problem or Solution refinement and a Capture refinement with their current flows, verifying that no context summary placeholder is introduced and Capture gains no Preview.

**Acceptance Scenarios**:

1. **Given** a Problem or Solution has no prior context, **When** Preview generation is underway, **Then** the existing progress treatment remains and no empty context-summary section is added.
2. **Given** a Problem or Solution is in its ordinary refinement state and no Preview is being generated or has failed, **When** the refinement modal is displayed, **Then** no always-visible prior-context summary or warning is added.
3. **Given** the user refines a Capture, **When** refinement progresses or completes, **Then** no Refinement Preview behavior from this feature is shown.

### Edge Cases

- Context containing only blank text, boilerplate, or the item’s short title alone is treated as no prior context.
- If context exceeds the visible summary limit, the summary ends cleanly with an ellipsis and does not expand or resize the modal unexpectedly.
- The context shown must belong to the currently refined item; switching items must never retain another item’s summary or warning.
- Repeated Preview attempts cannot leave duplicate warnings, overlapping Preview surfaces, or stale loading messages.
- If generation succeeds after a previous failed attempt, the old warning is absent from the completed Preview and refinement modal.
- Closing the Preview while generation is underway returns the user to the still-usable refinement modal and does not apply any refinement.
- Preview status and warnings remain understandable when animation is reduced or unavailable.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The feature MUST apply only to refinement of Problem and Solution items.
- **FR-002**: The system MUST consider prior context present when the current item has meaningful saved detail beyond its short title or has at least one completed refinement conversation or refinement record associated with that same item.
- **FR-003**: When a Problem or Solution has prior context, the system MUST make its context summary available before completed Preview content is available.
- **FR-004**: While a prior-context Preview is being generated, the Preview surface MUST display the exact status text “Refine 중...”.
- **FR-005**: The loading Preview MUST display a plain-language summary of the context accumulated so far for the current item, prioritizing current saved detail, decisions, evidence, constraints, and trade-offs from the most recent relevant refinement history.
- **FR-006**: The context summary MUST contain no more than three short entries and no more than 500 visible characters in total, excluding labels; excess content MUST be truncated with an ellipsis.
- **FR-007**: The loading summary MUST be derived only from context already available for the current item and MUST NOT depend on completion of a separate new summary-generation operation.
- **FR-008**: A completed Refinement Preview for an item with prior context MUST continue to show the prior-context summary together with the proposed refinement.
- **FR-009**: If a Problem or Solution has no prior context, the feature MUST preserve the existing generation-progress treatment and MUST NOT add an empty summary region.
- **FR-010**: If Preview generation fails, the Preview surface MUST close or remain unopened so that no empty, partial, or error-only Preview is displayed.
- **FR-011**: After Preview failure, the system MUST keep the underlying refinement modal available and MUST display one warning icon in its upper corner without covering its title, content, or controls.
- **FR-012**: The warning icon MUST expose a tooltip on both pointer hover and keyboard focus containing the exact message “Refinement preview를 띄울 수 없습니다. 다시 시도해 주세요.”
- **FR-013**: The warning icon MUST have an accessible name that communicates the same Preview failure independently of its visual symbol.
- **FR-014**: The failure warning MUST remain visible until the next Preview attempt begins, the refinement modal closes, or the user changes to another item, whichever happens first.
- **FR-015**: Starting a new attempt MUST clear the previous failure warning, and a successful attempt MUST NOT restore it.
- **FR-016**: This feature MUST NOT add prior-context content to the ordinary refinement modal outside the Preview loading, completed Preview, and Preview failure states described above.
- **FR-017**: This feature MUST NOT introduce a Refinement Preview for Capture items or change Capture refinement behavior.
- **FR-018**: Loading and error states MUST be verifiable in UI tests for both Problems and Solutions, including tooltip access by pointer and keyboard.
- **FR-019**: A proposed refinement MUST remain unapplied until the human explicitly approves it through the existing review action.

### Key Entities

- **Refinement Target**: The current Problem or Solution, identified by item type and item identity, with its saved title and detail.
- **Prior Context**: Meaningful saved detail and the current item’s completed refinement conversations or refinement records; context from another item is excluded.
- **Context Summary**: A bounded, read-only presentation of the most relevant prior context available when Preview generation starts.
- **Refinement Preview State**: One of idle, generating, ready, or failed for the current item and current attempt.
- **Preview Failure Warning**: A transient, accessible warning associated with one failed Preview attempt and shown on the underlying refinement modal.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of UI test runs for context-bearing Problems and Solutions, “Refine 중...” and the correct item’s context summary are visible within one second of the Preview attempt starting.
- **SC-002**: In 100% of forced Preview failure tests for Problems and Solutions, no Preview remains visible and exactly one corner warning is present on the refinement modal.
- **SC-003**: In 100% of warning tests, both pointer hover and keyboard focus reveal a tooltip containing “preview를 띄울 수 없습니다”.
- **SC-004**: In the agreed UI state matrix—Problem loading, Solution loading, Problem failure, Solution failure, no-context generation, successful retry, and Capture refinement—all expected states pass without cross-item or stale-state leakage.
- **SC-005**: Existing refinement completion and explicit human-approval flows continue to pass with no additional approval step and no automatic application of proposed content.
- **SC-006**: In a review with at least five representative context-bearing items, users can correctly identify the current item’s key prior decision, evidence, constraint, or trade-off from the loading summary in at least 90% of attempts.

## Assumptions

- “Refinement modal” means the existing Problem or Solution refinement conversation surface; “Refinement Preview” means the review surface that presents the proposed refinement before explicit approval.
- When context exists, the Preview shell may be shown immediately in a loading state. If generation fails, that shell is dismissed and the user returns to the refinement modal where the warning appears.
- The summary is a compact presentation of already available item context, not an additional AI-authored artifact. This avoids adding latency or a second failure mode.
- “Most recent relevant history” means history attached to the same item and used for refinement; unrelated workflow, Capture, or other-item history is excluded.
- Three entries and 500 visible characters provide enough orientation while keeping the current modal density and layout substantially unchanged.
- The fixed tooltip message is sufficient for this scope. Provider-specific or technical failure details remain outside the Preview and are not required in the tooltip.
- Existing retry, close, edit, and approval controls retain their current placement and behavior.
- Human review, implementation, and automated verification were completed on 2026-08-21.
