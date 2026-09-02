# Feature Specification: Task-Level AI Model Routing

**Feature Branch**: `006-ai-task-model-routing`  
**Created**: 2026-08-21  
**Status**: Complete
**Input**: User description: "AI Setup should accept only a default and an advanced model, and let users select the model tier for each AI task through collapsed Advanced options."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Configure Two Model Tiers (Priority: P1)

A person configuring AI can provide one Default model for routine work and one Advanced model for
quality-sensitive work without having to understand Capture, Problem, or Solution model stages.

**Why this priority**: This is the essential simplification: all AI work must have an understandable
model choice before task-level preferences can work.

**Independent Test**: Save an endpoint, API key, Default model, and Advanced model; reopen AI Setup
and confirm both model choices remain available without exposing stage-specific model fields.

**Acceptance Scenarios**:

1. **Given** a person opens AI Setup, **When** they view model configuration, **Then** they see
   Default model and Advanced model fields with brief explanations.
2. **Given** either model field is saved, **When** AI Setup is reopened, **Then** the saved value is
   displayed without exposing the API key.

---

### User Story 2 - Choose a Model Tier Per AI Task (Priority: P1)

A person can expand Advanced options and decide which named AI tasks use the Advanced model, while
the default choices make discussions, refinement, drafting, review, and reporting quality-focused.

**Why this priority**: Different AI tasks have different quality needs; task-level choices preserve
control without requiring people to manage model names repeatedly.

**Independent Test**: Enable or disable a named task in Advanced options, save, reopen AI Setup, and
confirm the same selected state is displayed.

**Acceptance Scenarios**:

1. **Given** Advanced options are collapsed, **When** a person expands them, **Then** they can see
   every named AI task and whether it uses the Advanced model.
2. **Given** a person changes one task selection, **When** they save and reopen AI Setup, **Then**
   only that task's selection changes and other task selections remain intact.
3. **Given** no saved task selections exist, **When** AI Setup first loads, **Then** discussions and
   refinement, drafting, conflict review, image summary, completion review, and completion report
   are selected for the Advanced model by default.

---

### User Story 3 - Receive Safe Model Fallbacks (Priority: P2)

A person can continue using every AI task if the Advanced model has not been supplied or is later
cleared; selected advanced tasks transparently use the Default model instead of failing because a
second model is absent.

**Why this priority**: The two-tier setup should reduce configuration burden and not turn an optional
quality choice into an availability problem.

**Independent Test**: Select an advanced task, leave the Advanced model empty, invoke the task, and
confirm it uses the configured Default model.

**Acceptance Scenarios**:

1. **Given** an advanced task is enabled and the Advanced model is blank, **When** the task runs,
   **Then** it uses the Default model.
2. **Given** an advanced task is disabled, **When** the task runs, **Then** it uses the Default
   model even if an Advanced model is configured.

### Edge Cases

- The Default model is empty when either a routine task runs or an advanced task needs fallback.
- Saved task preferences contain an unknown or removed task identifier.
- A pre-existing installation contains legacy stage-specific model preferences.
- The provider cannot list models or is unreachable while a person edits settings.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: AI Setup MUST present exactly one Default model field and one Advanced model field,
  each with a concise explanation of its use.
- **FR-002**: AI Setup MUST group task-tier choices inside a collapsed Advanced options control.
- **FR-003**: The system MUST let a person independently select whether each named AI task uses the
  Advanced model.
- **FR-004**: The initial task defaults MUST select the Advanced model for Capture, Problem, and
  Solution discussion and refinement; Problem and Solution drafting; conflict review; image summary;
  completion review; and completion report.
- **FR-005**: The initial task defaults MUST select the Default model for workbench organization,
  completed-Solution discussion, and Problem enrichment.
- **FR-006**: A disabled advanced task MUST use the Default model.
- **FR-007**: An enabled advanced task MUST use the Advanced model when one is configured and MUST
  otherwise fall back to the Default model.
- **FR-008**: The system MUST preserve valid task-tier preferences across restart and ignore unknown
  task identifiers safely.
- **FR-009**: The system MUST retain secure API-key handling and must not return or display the API
  key in configuration responses.
- **FR-010**: Image summary and conflict review MUST resolve their own named task preferences rather
  than inheriting an unrelated workflow-stage model.

### Key Entities

- **Model tier configuration**: The Default model, optional Advanced model, endpoint, and secure
  credential reference used for AI requests.
- **Task-tier preference**: A named AI task and a boolean indication that it should prefer the
  Advanced model.
- **AI task**: A stable user-visible operation such as discussion, refinement, drafting, review,
  report generation, image summary, or workbench organization.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person can save both model tiers and one task preference in under 60 seconds from
  opening AI Setup.
- **SC-002**: After save and reload, 100% of the supported task preferences retain their selected
  tier in automated configuration tests.
- **SC-003**: 100% of supported AI task routes resolve either the selected Advanced model or the
  Default model; none relies on a legacy stage-specific preference.
- **SC-004**: An enabled task with no Advanced model completes model selection by using the Default
  model rather than producing an advanced-model-missing error.

## Assumptions

- The person chooses compatible model identifiers for the configured OpenAI-compatible endpoint.
- The Default model remains required for all AI use, because it is the safety fallback.
- Advanced options name operations in user language rather than exposing implementation stages.
- Legacy stage-specific stored values are not interpreted as one of the two new model choices; the
  person reviews and saves the new two-tier configuration after upgrade.
