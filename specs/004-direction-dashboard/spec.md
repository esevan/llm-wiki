# Feature Specification: Direction Dashboard

**Created**: 2026-08-18
**Last Reconciled**: 2026-09-02
**Status**: Implemented baseline — reconciled with current behavior

## User Scenarios & Testing

### User Story 1 — Set direction before measuring work (Priority: P1)

A user records local Compass goals that express the direction approved work should support.

### User Story 2 — Assess importance with evidence (Priority: P1)

A user assesses a Problem using 0–5 alignment, impact, urgency, and leverage factors and supplies
evidence for the assessment.

### User Story 3 — Inspect contribution history (Priority: P2)

A user views contribution events and period totals that explain workflow milestones without scoring
or ranking the user.

**Acceptance Scenarios**:

1. **Given** an evidence-backed importance assessment, **When** its Problem is approved, **Then** a
   contribution event worth 10% of the assessed importance is recorded.
2. **Given** a Solution for that Problem, **When** it is approved, **Then** a contribution event worth
   20% is recorded.
3. **Given** recorded completion whose Knowledge status is resolved, **When** completion is verified,
   **Then** a contribution event worth 70% is recorded.

### Edge Cases

- Missing evidence or any factor outside 0–5 rejects the assessment.
- Goals and contribution history remain available without an AI provider or semantic search.
- Completing a Problem directly without the corresponding milestone actions does not synthesize
  missing contribution events.

## Functional Requirements

- **FR-001**: The system MUST retain active Compass goals locally and expose them with the dashboard.
- **FR-002**: Importance assessment MUST retain all four factors, supporting evidence, and the
  calculated importance independently of contribution events.
- **FR-003**: Contribution MUST be recorded as append-only milestone events rather than replacing
  earlier history.
- **FR-004**: Problem approval MUST award 10% of assessed importance.
- **FR-005**: Solution approval MUST award 20% of assessed importance.
- **FR-006**: Verified completion with resolved Knowledge status MUST award 70% of assessed
  importance.
- **FR-007**: Dashboard totals MUST be derived from contribution events and exposed as direction
  signals.
- **FR-008**: Compass language MUST describe the work and its relationship to goals and MUST NOT
  score, rank, or judge a user.
- **FR-009**: Compass MUST remain usable during provider or semantic-search failure.

### Key Entities

- **Compass goal**: A local statement of direction and supporting description.
- **Importance assessment**: Evidence-backed factors and the resulting importance of one Problem.
- **Contribution event**: One immutable workflow milestone contribution.
- **Direction total**: A period aggregate calculated from contribution events.

## Success Criteria

- **SC-001**: Goal creation, importance assessment, and dashboard reading pass from a fresh local
  application state.
- **SC-002**: Recomputed period totals equal the sum of retained contribution events in every tested
  case.
- **SC-003**: The three milestone events sum to 100% of assessed importance when all three occur.
- **SC-004**: No Compass surface describes its signals as individual productivity or performance.

## Assumptions

- Compass is decision support, not employee or user performance measurement.
- Broader team, synchronization, and period-reporting behavior are outside the current application.
