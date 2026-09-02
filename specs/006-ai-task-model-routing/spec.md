# Feature Specification: Task-Level AI Model Routing

**Feature Branch**: `006-ai-task-model-routing`
**Created**: 2026-08-21
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — confirmed configuration changes pending

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Configure two model tiers (Priority: P1)

A user configures one required Default model and one optional Advanced model for an
OpenAI-compatible provider. The API key is stored separately from visible configuration.

**Acceptance Scenarios**:

1. **Given** valid provider configuration, **When** it is saved and reopened, **Then** endpoint,
   model tiers, task preferences, and background worker count are retained while the API key is not
   returned.
2. **Given** no API key or no usable model, **When** an AI request begins, **Then** the request fails
   without changing source content.

### User Story 2 — Select a tier per AI task (Priority: P1)

The user can independently assign visible AI tasks to the Advanced tier. Disabled tasks use the
Default tier, and enabled tasks fall back to Default when no Advanced model is configured.

### User Story 3 — Test provider connectivity (Priority: P2)

The user can request the configured provider's available models without creating Queue history or a
content result.

### User Story 4 — Configure background capacity (Priority: P2)

The user can select between one and 32 durable background workers. The setting takes effect after
the service restarts and does not change Fast Queue's single-request rule.

### Edge Cases

- Unknown task preference keys are ignored when configuration is saved.
- Malformed stored preference data falls back to known task defaults.
- A configured Advanced task falls back to Default when Advanced model is blank.
- Provider-test failure is reported without disabling local non-AI operation.
- Leaving the API-key field blank preserves the existing stored secret.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: AI Setup MUST present one Default model field and one optional Advanced model field.
- **FR-002**: Task-tier choices MUST be grouped under Advanced options rather than shown as primary
  setup fields.
- **FR-003**: The user MUST be able to choose the model tier for Capture, Problem, and Solution
  discussion/refinement; Problem and Solution drafting; Workbench organization; completed-Solution
  discussion; conflict review; image summary; completion review; completion report; lineage
  interpretation; Problem enrichment; and Knowledge translation.
- **FR-004**: Capture, Problem, and Solution assistance; Problem and Solution drafting; conflict
  review; image summary; completion review; completion report; and lineage interpretation MUST
  initially prefer Advanced.
- **FR-005**: Workbench organization, completed-Solution discussion, Problem enrichment, and
  Knowledge translation MUST initially prefer Default.
- **FR-006**: A disabled Advanced preference MUST resolve to Default.
- **FR-007**: An enabled Advanced preference MUST resolve to Advanced when configured and otherwise
  fall back to Default.
- **FR-008**: Valid known preferences MUST survive restart and unknown preference keys MUST NOT
  become selectable tasks.
- **FR-009**: Public configuration MUST indicate whether an API key is configured without returning
  the key.
- **FR-010**: Image summary, conflict review, completion review, completion report, lineage
  interpretation, and Knowledge translation MUST resolve their own task preference rather than an
  unrelated workflow-stage preference.
- **FR-011**: Durable background worker count MUST be restricted to one through 32 and changes MUST
  be disclosed as effective after restart.
- **FR-012**: Provider connection tests MUST remain operational feedback and MUST NOT create AI
  content work or notifications.
- **FR-013**: Public and stored provider configuration MUST NOT include an unused report-language
  setting.

### Key Entities

- **Model tier configuration**: Endpoint, Default model, optional Advanced model, and secure
  credential presence.
- **Task-tier preference**: A known AI task and whether it prefers the Advanced model.
- **Background capacity**: The configured number of durable workers used after service restart.

## Success Criteria *(mandatory)*

- **SC-001**: Saving and reloading configuration preserves every supported visible task preference
  and background worker count.
- **SC-002**: Every tested task resolves to Advanced, Default, or Default fallback according to its
  saved preference.
- **SC-003**: Public configuration and provider-test responses expose the API key in zero tested
  cases.
- **SC-004**: Invalid worker counts are rejected without replacing the previous configuration.

## Assumptions

- The user supplies model identifiers compatible with the configured endpoint.
- Knowledge translation defaults to the Default tier but is a user-selectable Advanced-option task.
- Provider setup and model discovery are operational checks, not AI content tasks.

## Confirmed Implementation Gaps

- **IG-008 — Remove report-language setting**: The current configuration stores and exposes an
  unused report-language value. The field must be removed from the supported configuration contract
  and migrated out of stored settings.
- **IG-012 — Knowledge translation tier control**: Knowledge translation is routable but currently
  has no visible Advanced-option checkbox. AI Setup must expose the missing task control.
