# Feature Specification: Lineage Knowledge Layer

**Feature Branch**: `010-lineage-knowledge-layer`
**Created**: 2026-08-21
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — inference failure indicator pending

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Understand completed work from origin to evidence (Priority: P1)

A user opens a completed Solution and sees its result, Capture→Problem→Solution→Complete lineage,
completion evidence, Work Log, checklist, and generated Knowledge without reopening the workflow.

**Acceptance Scenarios**:

1. **Given** completed work, **When** its completed workspace opens, **Then** Result, Lineage,
   Evidence, and Archive views remain read-only.
2. **Given** a lineage claim with evidence, **When** its reference is opened, **Then** the preserved
   source excerpt and any available live record are shown in context.
3. **Given** no explicit rationale or evidence, **When** lineage is rendered, **Then** the absence is
   stated rather than filled with an unsupported fact.

### User Story 2 — Distinguish evidence, decisions, and inference (Priority: P1)

Lineage separates observed evidence, user decisions, and AI inference. Conflicts remain context for
transitions and decisions rather than an additional lifecycle stage.

### User Story 3 — Regenerate or correct interpretation safely (Priority: P2)

A user can rebuild deterministic lineage, optionally request AI inference in the background, and
correct a claim without changing preserved source records or deleting the prior interpretation.

### User Story 4 — Start follow-up work explicitly (Priority: P2)

A completed Solution remains immutable. New work begins as a new Problem linked back to the completed
Solution.

### Edge Cases

- Deterministic lineage remains usable without AI inference.
- A background inference failure leaves the current usable Lineage visible and adds a retryable
  failure icon with a readable tooltip.
- A correction with an outdated revision is rejected and requires reloading current lineage.
- External modification of completed Knowledge blocks automatic regeneration or synchronization.
- A correction can succeed while document synchronization reports that a retry is needed.
- Raw private chats are not treated as published evidence unless retained in an approved decision or
  evidence record.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Completing work MUST create or select deterministic Lineage for each related Solution
  before the completed-work document is produced.
- **FR-002**: Completed Solution experience MUST provide distinct Result, Lineage, Evidence, and
  Archive views.
- **FR-003**: Primary lineage MUST show Capture → Problem → Solution → Complete as four stages.
- **FR-004**: Stages, transitions, decisions, conflicts, and completion claims MUST link to retained
  evidence or state that evidence was not explicitly recorded.
- **FR-005**: Claims MUST be classified as observed, decided, or inferred.
- **FR-006**: Inferred claims MUST be visibly identified as AI interpretation and include confidence.
- **FR-007**: Conflict MUST be represented as context affecting a transition or decision, not as a
  peer lifecycle stage.
- **FR-008**: Conflict status and address basis MUST preserve whether resolution was explicit,
  implementation-evidenced, inferred, unclear, or unaddressed.
- **FR-009**: The system MUST NOT label a conflict addressed solely from AI inference.
- **FR-010**: Major evidence references MUST use stable readable labels in generated Knowledge while
  internal identifiers remain hidden.
- **FR-011**: Users MUST be able to inspect evidence and available Capture, Problem, and Solution
  records from the completed workspace.
- **FR-012**: Regeneration MUST preserve immutable workflow, decision, conflict, completion, and
  evidence records.
- **FR-013**: A correction MUST become the current interpretation while retaining prior AI/user
  revisions and evidence links.
- **FR-014**: Completed-work narrative MUST be generated only from current Lineage and its referenced
  evidence, not from the narrative it is replacing.
- **FR-015**: Deterministic lineage MUST remain available when optional AI inference fails.
- **FR-016**: Completed Solutions MUST remain read-only and new work MUST use the explicit linked
  follow-up Problem action.
- **FR-017**: Regeneration and correction MUST respect external changes to the tracked completed-work
  document.
- **FR-018**: When optional inference fails, the last usable deterministic or successful Lineage
  snapshot MUST remain visible and the completed workspace MUST show a retryable failure icon with
  a readable tooltip.

### Key Entities

- **Lineage snapshot**: One immutable version of the assembled lifecycle and its source revision.
- **Lineage claim**: An observed, decided, or inferred statement associated with a stage or decision.
- **Evidence reference**: A stable readable label and preserved source excerpt supporting a claim.
- **Claim revision**: The current and prior interpretations of a claim, with correction metadata.

## Success Criteria *(mandatory)*

- **SC-001**: Newly completed Solutions expose all four lifecycle stages or an explicit absence of a
  source stage.
- **SC-002**: Every major displayed claim reaches retained evidence or an explicit no-evidence label
  within two user interactions.
- **SC-003**: AI inference alone marks a conflict addressed in zero acceptance cases.
- **SC-004**: Every inferred claim is visibly distinguishable and includes confidence.
- **SC-005**: Corrections preserve prior revisions and evidence in all tested cases.
- **SC-006**: Completed-work rendering remains usable without horizontal overflow at supported
  desktop and mobile widths.
- **SC-007**: Report generation tests contain no factual input outside current Lineage and its
  referenced evidence.

## Assumptions

- Deterministic evidence assembly and optional AI inference are separate outcomes.
- The completed workspace, not the former small Detail modal, is the current completed-work
  experience.
- Private exploration remains local process and is not automatically promoted to Knowledge.

## Confirmed Implementation Gap

- **IG-013 — Inference failure indicator**: The current completed workspace does not consistently
  retain a retryable inference-failure indicator on the Lineage view. It must preserve the existing
  usable snapshot and expose the failed inference through an icon and tooltip.

## Documentation Migration

- This feature moved from duplicate prefix `006` to `010`; AI task model routing retains `006`.
