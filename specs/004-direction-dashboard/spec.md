# Feature Specification: Direction Dashboard

**Created**: 2026-08-18
**Status**: Implemented baseline (Compass)

## User Scenarios & Testing

### User Story 1 — Set a direction before measuring activity (Priority: P1)

A person adds Compass goals that express the direction they want their approved work to support.

**Acceptance**: goals persist locally and are shown in the Compass view.

### User Story 2 — Assess importance with evidence (Priority: P1)

A person records an evidence-backed importance assessment for a Problem instead of treating raw
activity as impact.

**Acceptance**: the assessment stores its cited factors and remains distinguishable from Solution
completion or score events.

### User Story 3 — Inspect contribution history (Priority: P2)

A person views immutable milestone contribution events and aggregate direction signals in Compass.

**Acceptance**: approved, verified, and completion milestones are recorded as ledger events; totals
are precomputed from the ledger rather than replacing its history.

## Functional Requirements

- **FR-001**: The system MUST persist Compass goals locally and expose them through the dashboard.
- **FR-002**: Importance assessments MUST retain evidence and MUST be distinct from achievement.
- **FR-003**: Contribution scoring MUST be written to an immutable event ledger.
- **FR-004**: Milestone weights MUST award 10% at approval, 20% at verification, and 70% at
  completion.
- **FR-005**: Dashboard totals MUST be refreshed from score events and exposed as direction signals.
- **FR-006**: Compass MUST preserve existing goals, evidence, and direction signals during a model
  provider or semantic-search outage as a local fallback for the AI-centered product workflow.

## Success Criteria

- **SC-001**: API tests can create a goal, record importance, and read the dashboard from a fresh
  local database.
- **SC-002**: Recomputed aggregate totals match the immutable score-event ledger in tests.
- **SC-003**: Dashboard language and UI distinguish direction/importance from busyness/achievement.

## Assumptions

- Compass is a decision-support surface, not a productivity score or automatic prioritization tool.
- Current aggregation is the local baseline; broader period reports remain a future enhancement.
