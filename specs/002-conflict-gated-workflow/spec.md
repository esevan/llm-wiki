# Feature Specification: Conflict-Gated Workflow

**Created**: 2026-08-18
**Status**: Implemented and maintained as the Workbench flow

## User Scenarios & Testing

### User Story 1 — Turn a Capture into an approved Problem (Priority: P1)

A person records a thought as a Capture, explores it without changing its state, then reviews and
explicitly finalizes a structured Problem. The original Capture remains available as history but is
removed from the active inbox.

**Acceptance**: Capture creation is immediate; Explore persists conversation context only; a
Problem is created only by explicit human finalization; approval is a separate human action.

### User Story 2 — Propose and carry out a safe Solution (Priority: P1)

A person drafts a Solution from an approved Problem, reviews conflict evidence, approves only a
`clear` Solution, and records execution evidence in that Solution's Work Log and validation checklist.

**Acceptance**: `unknown` and `conflicted` Solutions cannot be approved; cited findings and the
recommended conflict state are visible before the decision; copied handoff contains outcome and
done evidence rather than implementation instructions.

### User Story 3 — Organize work without surrendering authority (Priority: P2)

A person manually updates, soft-deletes/restores, categorizes, ranks, and views work relationships
without an AI action silently changing workflow state.

**Acceptance**: deletion is reversible and does not change vault files; optional organization only
changes category/rank metadata; Flow view shows Problem → Solution relationships.

## Functional Requirements

- **FR-001**: The system MUST manage Capture, Problem, and Solution records in local SQLite.
- **FR-002**: Capture MUST remain a distinct inbox; promotion MUST preserve its historical link.
- **FR-003**: Same-stage Explore and next-stage Draft-next conversations MUST persist their turns
  and MUST not themselves create, approve, or advance an item.
- **FR-004**: AI-assisted drafts MUST have a validated stage structure and remain editable until
  the human selects the finalization action.
- **FR-005**: Manual forms MUST remain available for user-authored updates and transitions.
- **FR-006**: A Solution MUST reference an approved Problem and MUST retain its execution evidence
  in a Solution-owned Work Log and validation checklist.
- **FR-007**: A Solution MAY be approved only after an explicit human action and a cited `clear`
  conflict evaluation. `unknown` and `conflicted` evaluations MUST block approval.
- **FR-008**: Conflict review MUST present claim, severity, citation, explanation, required
  resolution, and a recommended state without auto-applying that recommendation.
- **FR-009**: Flow view MUST preserve Problem → Solution lineage without introducing another
  workflow stage below Solution.
- **FR-010**: Copyable handoff MUST include outcome and done criteria and MUST exclude technical
  implementation steps.
- **FR-011**: Soft deletion MUST be reversible and MUST not delete vault content or stored history.
- **FR-012**: Provider endpoint/model configuration MUST be local; API keys MUST be kept in the OS
  keyring rather than the vault or application database.
- **FR-013**: Per-stage model overrides MAY fall back to a default model.

## Success Criteria

- **SC-001**: The complete Capture → Problem → Solution → Work Log/Completion flow is covered by local API tests
  using a mock provider.
- **SC-002**: No automated path can approve a conflicted or unknown Solution.
- **SC-003**: Chat, draft, and manual-update flows preserve human review before state-changing
  actions in every tested stage.
- **SC-004**: Restoring a soft-deleted item makes it available again without altering vault files.

## Assumptions

- “Solution” is the UI label for the `features` record type retained by the API.
- AI is a required product capability. When its provider is unavailable, manual actions preserve
  existing records and human authority as a fallback; they do not replace AI-assisted workflow.
