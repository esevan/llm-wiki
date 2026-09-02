# Feature Specification: Conflict Resolution Workflow

**Feature Branch**: `feat/conflict-resolution-workflow`

**Created**: 2026-09-02

**Status**: Draft

**Input**: Replace the raw Conflict Review report shown before Solution work with a structured, persistent, per-conflict decision workflow.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand each conflict (Priority: P1)

As a user reviewing a proposed Solution, I can scan distinct conflict cards and immediately understand what the Solution proposes, which existing decision or document disagrees, and why the disagreement matters.

**Why this priority**: A resolution is trustworthy only when the person deciding can understand the competing claims and supporting source without reconstructing a Markdown report.

**Independent Test**: Open a completed Conflict Review containing multiple severities and verify that each conflict is visually separate, ordered consistently, and presents both claims, the target, impact, recommendation, and expandable evidence.

**Acceptance Scenarios**:

1. **Given** a review with multiple conflicts, **When** the result opens, **Then** each conflict appears as a separate card with a severity indicator, concise category, current claim, conflicting target and existing claim, summary, impact, and recommendation.
2. **Given** a conflict with source evidence, **When** the user expands its evidence area, **Then** the cited source location and original excerpt are visible without displacing the primary comparison from the card header.
3. **Given** a review with no detected conflicts, **When** the result opens, **Then** the existing low-friction clear-review decision remains available without empty conflict cards or added resolution work.

---

### User Story 2 - Resolve every conflict (Priority: P1)

As a user, I can independently choose one mutually exclusive resolution for every conflict, explain intentional exceptions, and see whether the review is ready to continue.

**Why this priority**: Conflict Review must lead to explicit human decisions rather than end as a passive report.

**Independent Test**: Resolve a review containing at least three conflicts using a mix of actions and verify the resolved/unresolved summary, rationale rules, and Continue state after every interaction.

**Acceptance Scenarios**:

1. **Given** an unresolved conflict, **When** the user selects Apply recommendation, **Then** that card is counted as resolved and any optional comment is retained.
2. **Given** an unresolved conflict, **When** the user selects Accept conflict without a rationale, **Then** the card remains unresolved, shows a clear rationale requirement, and Continue remains unavailable.
3. **Given** an unresolved conflict, **When** the user selects Accept conflict and enters a non-empty rationale, **Then** the card is counted as resolved.
4. **Given** several conflicts, **When** the user changes one card, **Then** other cards keep their independent actions and comments and the summary reports the exact resolved and unresolved counts.
5. **Given** every conflict has a valid resolution, **When** the user continues, **Then** the review decisions are saved and the Solution workflow proceeds according to the existing human-controlled gate.

---

### User Story 3 - Resume and reuse resolution history (Priority: P2)

As a user returning to a Solution, I can recover what conflicts were found, how each was resolved, and why an intentional conflict was accepted.

**Why this priority**: Preserved rationale prevents repeated rediscovery and creates a safe basis for future similar-conflict detection.

**Independent Test**: Save mixed resolutions, reload the application, reopen the review, and verify the same conflict-to-resolution-to-rationale history is returned.

**Acceptance Scenarios**:

1. **Given** a saved review, **When** the application is restarted and the review is reopened, **Then** each conflict retains its action, rationale, and resolution time.
2. **Given** an accepted conflict with rationale, **When** its stored history is inspected, **Then** the conflict identity and target remain linked to the human resolution and explanation.
3. **Given** a previously saved review result in the earlier report shape, **When** it is opened, **Then** it remains reviewable through a compatible structured presentation or safe legacy detail view.

### Edge Cases

- Duplicate or missing model-supplied conflict identifiers are replaced with stable review-scoped identifiers so each card and resolution remains addressable.
- Unknown severity values are normalized to a safe default and do not break ordering or rendering.
- Missing optional target titles, categories, impact text, or evidence are shown with concise fallbacks rather than blank primary controls.
- A stale review whose Solution or Vault source changed cannot be used to advance the current Solution.
- Repeated save attempts do not create contradictory active resolutions for the same conflict.
- A failed resolution save keeps the dialog open, preserves entered choices, and explains that nothing was advanced.
- Keyboard users can reach every card, radio option, rationale input, evidence disclosure, and final action in a predictable order.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST return Conflict Review results as a structured collection of individually identifiable conflicts while preserving compatibility with previously stored report results.
- **FR-002**: Each conflict MUST include a normalized severity, concise category or title, target document or decision, summary, current Solution claim, existing claim, expected impact, recommended resolution, and zero or more source evidence entries; missing optional model output MUST receive safe display fallbacks.
- **FR-003**: The review surface MUST display conflicts as visually distinct cards and place the current-versus-existing claim comparison near the top of each card.
- **FR-004**: Severity MUST be available as both text and a visually distinct treatment; meaning MUST NOT depend on color alone.
- **FR-005**: Source citations and original excerpts MUST be available through an expandable secondary area when evidence exists.
- **FR-006**: Each conflict MUST offer exactly one of two mutually exclusive human actions: `Apply recommendation` or `Accept conflict`.
- **FR-007**: Each conflict MUST offer a comment or rationale field, and `Accept conflict` MUST require a non-empty rationale before that conflict counts as resolved.
- **FR-008**: The surface MUST show total, resolved, and unresolved conflict counts and update them accurately as actions or rationales change.
- **FR-009**: Continue MUST remain unavailable while any detected conflict lacks a valid resolution.
- **FR-010**: A conflict-free review MUST retain a concise existing clear/conflicted human decision path without requiring per-conflict inputs.
- **FR-011**: Continuing a review with conflicts MUST atomically persist the review-level decision and every conflict's action, rationale, and resolution timestamp before any workflow state changes.
- **FR-012**: Stored data MUST preserve a queryable `Conflict → Resolution → Rationale` relationship scoped to its review run and Solution.
- **FR-013**: Applying a recommendation MUST record the Solution as still needing revision and MUST NOT represent the conflicting context as clear until a later current review supports that state.
- **FR-014**: Accepting all conflicts with required rationales MUST record explicit human acceptance while preserving the detected conflict history and allow the existing Solution gate to treat the reviewed context as intentionally resolved.
- **FR-015**: AI output MUST never choose or persist a resolution, advance a Solution, or fabricate a citation; all resolution actions are explicit user decisions.
- **FR-016**: Review persistence MUST reject unknown conflict identifiers, duplicate resolutions, invalid actions, missing required rationales, and stale or incomplete reviews with actionable validation feedback.
- **FR-017**: If saving fails, the system MUST preserve the user's in-dialog choices and MUST NOT advance or close the review as if saving succeeded.
- **FR-018**: The review dialog MUST support a scrollable body, compact evidence disclosures, and a visible final summary/action area for long multi-conflict results.
- **FR-019**: All new user-facing Conflict Review language MUST be available in English and Korean, using `사용자` for the person using the product in Korean copy.
- **FR-020**: Existing conflict-check, stale-source, queue, approval, and conflict-free workflows MUST continue to function unless explicitly changed by these requirements.

### Key Entities

- **Conflict Review Run**: One evidence review for a Solution and source snapshot; owns the structured conflicts and review status.
- **Conflict**: A stable, review-scoped description of one disagreement, including target, severity, competing claims, impact, recommendation, and evidence.
- **Conflict Evidence**: A source citation and excerpt supporting a conflict.
- **Conflict Resolution**: The user's mutually exclusive action for one Conflict, optional comment, required acceptance rationale where applicable, and resolution time.
- **Review Decision**: The aggregate human decision recorded after all conflicts are validly resolved, linked to the existing Solution conflict gate.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a review containing five conflicts, a user can identify the current claim, conflicting existing claim, target, severity, and recommendation for any card without opening evidence details.
- **SC-002**: For reviews containing 1–20 conflicts, the displayed total, resolved, and unresolved counts match the card states after every supported interaction.
- **SC-003**: Continue is unavailable in 100% of cases where at least one conflict has no action or an accepted conflict lacks rationale, and becomes available when all conflicts are validly resolved.
- **SC-004**: After restart, 100% of saved conflict actions, rationales, timestamps, targets, and evidence associations are recoverable from the same review.
- **SC-005**: Existing automated conflict-free, stale-review, approval-gate, and queue-result scenarios continue to pass.
- **SC-006**: A review with 20 conflicts remains operable at common desktop viewport sizes without moving the final status/action area outside the dialog's usable navigation flow.

## Assumptions

- `Accept conflict` is an explicit human exception that can satisfy the current conflict gate only after every detected conflict is accepted with rationale; the underlying conflict evidence remains preserved rather than rewritten as absent.
- `Apply recommendation` records required follow-up revision and keeps the Solution blocked until its content changes and a new current review supports continuation; automatic Solution rewriting is out of scope.
- One current resolution per conflict is sufficient for this change; the immutable review/run records preserve historical review generations.
- Existing local SQLite storage, queue result retrieval, provider adapter, localization mechanism, and modal visual language are reused.
- This feature does not add another workflow stage below Solution, publish private review history into portable Knowledge, or introduce collaborative resolution ownership.

## Product Spirit Alignment

- The card comparison and expandable evidence serve **II. Reduce Cognitive Load** by making the decision visible without requiring users to parse prose or retain multiple claims mentally.
- Persistent rationale serves **III. Resume Where You Left Off** by preserving the exact conflict decision context.
- Explicit per-conflict actions and blocked invalid continuation preserve **Human Authority over AI** and **Evidence and Logical Consistency**.
- The workflow remains inside the Solution review gate, preserving **IV. Organize Around Problems, Not Tasks** and the private-process boundary.
