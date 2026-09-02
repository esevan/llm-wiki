# Feature Specification: Conflict-Gated Workflow

**Created**: 2026-08-18
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — compatibility API deprecation pending

## User Scenarios & Testing

### User Story 1 — Turn a Capture into an approved Problem (Priority: P1)

A user records an immediate Capture, explores or edits it, promotes it to a linked Problem, and
explicitly approves that Problem before proposing a Solution.

**Acceptance Scenarios**:

1. **Given** a saved Capture, **When** it is promoted, **Then** one linked Problem is created and the
   Capture leaves the active inbox while remaining in history.
2. **Given** an unapproved Problem, **When** Solution creation is attempted, **Then** creation is
   rejected.

### User Story 2 — Explore and approve a Solution safely (Priority: P1)

The user can discuss, draft, refine, manually edit, and review a Solution without AI changing its
workflow state. Approval is a separate user action governed by the current conflict decision.

**Acceptance Scenarios**:

1. **Given** a proposed Solution with `unknown` or `conflicted` conflict state, **When** ordinary
   approval is attempted, **Then** approval is rejected.
2. **Given** a cited `clear` conflict decision, **When** the user approves the Solution, **Then** it
   becomes in progress.
3. **Given** the manual transition's skip path and a non-empty reason, **When** the user confirms the
   transition, **Then** the reason is recorded as a clear conflict decision and the Solution is
   approved.

### User Story 3 — Organize and recover work (Priority: P2)

The user sees active work grouped on the Workbench and can soft-delete and restore workflow records
without deleting Vault Knowledge. AI organization may populate organization metadata that has no
direct user-editing control.

### Edge Cases

- Promoting the same Capture again returns the existing linked Problem.
- Chat and AI proposals do not create, approve, complete, or advance records by themselves.
- A stale Draft, Refine, or conflict result cannot be applied to changed source content.
- Deleting a parent hides linked descendants; restoring it makes the linked records visible again.
- Provider failure leaves manual workflow actions and existing records available.

## Functional Requirements

- **FR-001**: The system MUST maintain Capture, Problem, and Solution as distinct workflow records.
- **FR-002**: Capture MUST remain a lightweight inbox and promotion MUST preserve its historical
  link to the resulting Problem.
- **FR-003**: A Solution MUST belong to an approved Problem.
- **FR-004**: Current-stage and next-stage conversations MUST preserve their turns and MUST remain
  state-neutral.
- **FR-005**: AI Draft and Refine results MUST remain unapplied proposals until the user explicitly
  accepts them.
- **FR-006**: Manual update and manual transition paths MUST remain available when AI is unavailable
  or unwanted.
- **FR-007**: A Solution MUST own its Work Log and editable validation checklist; no additional
  workflow stage may be introduced below Solution.
- **FR-008**: Ordinary Solution approval MUST require an explicit user action and a current cited
  `clear` conflict decision.
- **FR-009**: A manual skip of conflict checking MUST require a user-supplied reason and MUST record
  that skip before approval. This override MUST remain available because remote AI review can be
  unavailable or unreliable.
- **FR-010**: Conflict review MUST remain advisory and MUST NOT apply its recommended state.
- **FR-011**: Copyable handoff MUST include the approved Problem, intended outcome, constraints, and
  definition of done without generating technical implementation steps.
- **FR-012**: Soft deletion MUST be reversible and MUST NOT delete Vault files or retained workflow
  history.
- **FR-013**: Provider configuration MUST remain local and provider credentials MUST not be returned
  in public configuration responses.
- **FR-014**: Active workflow views MUST omit promoted Captures and completed or deleted records while
  retaining their lineage for history and completed-work views.

### Key Entities

- **Capture**: Immediate authored input retained as the source of a promoted Problem.
- **Problem**: The approved reason for work and parent of Solutions.
- **Solution**: The intended outcome, boundary, Work Log, validation criteria, conflict state, and
  workflow state associated with a Problem.
- **Conflict decision**: A user-owned clear, conflicted, or unknown evaluation and its recorded
  basis.

## Success Criteria

- **SC-001**: Capture → Problem → Solution → Work/Completion journeys pass local workflow and API
  tests.
- **SC-002**: No tested AI result changes approval or completion state without a user action.
- **SC-003**: Unknown and conflicted ordinary approval attempts are rejected in all tested cases.
- **SC-004**: A deleted workflow record can be restored without changing Vault files.
- **SC-005**: Chat, Draft, Refine, manual update, and manual transition paths preserve the same
  workflow identity and lineage.

## Assumptions

- “Solution” is the user-facing label for records whose API type remains `features`.
- This is a single-user local workflow; approval records represent actions by that user.
- The explicit skip-with-reason path is a supported resilience policy, not a temporary exception.
- Direct workflow APIs remain compatibility paths while the current backend is in service.

## Confirmed Deprecation

- **DEP-009 — Direct workflow APIs**: Direct promotion, approval, Solution creation, and stage
  endpoints remain supported compatibility contracts for the current backend. They must be marked
  deprecated and are scheduled for removal when the backend is replaced by the planned Tauri
  backend.
