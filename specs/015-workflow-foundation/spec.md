# Feature Specification: Workflow Foundation

**Feature Branch**: `015-workflow-foundation`

**Created**: 2026-09-26

**Status**: Draft

**Input**: User description: "Create shared prompt, retrieval, version, provenance, decision, and asynchronous-operation contracts for the next workflow initiative while preserving current behavior and reusing the existing queue and version infrastructure."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Trust AI operations across workflows (Priority: P1)

As a user, I can start an AI-assisted operation and understand whether execution succeeded, whether its result still needs review, and whether it was applied or superseded, without AI changing workflow state or publication decisions on its own.

**Why this priority**: Every later workflow depends on a consistent human-control boundary and unambiguous operation state.

**Independent Test**: Submit a user-requested generation against a versioned record, let its source change before completion, and verify that execution success is reported separately from a stale or superseded application result and remains available for review or retry.

**Acceptance Scenarios**:

1. **Given** a user-requested generation, **When** it is accepted, runs, and returns a valid result, **Then** it has visible durable status, a retry path after failure, and an explicit application disposition independent of execution status.
2. **Given** a generation bound to an earlier source version, **When** a newer source version exists before finalization, **Then** the older result cannot overwrite the newer version and is marked superseded or stale.
3. **Given** repeated equivalent requests, **When** one is already active, **Then** the requests share one active operation; when a latest-wins request replaces older speculative work, the older work is cancelled without durable retry.

---

### User Story 2 - Reuse governed prompts and evidence (Priority: P2)

As a product developer, I can invoke registered prompts through stable identifiers and versions, with validated inputs and outputs, consistent retrieval intent, and source attribution so downstream workflow features do not invent incompatible prompt formats.

**Why this priority**: Central contracts keep Capture, journey, retrieval, Knowledge, and archive work consistent and reviewable.

**Independent Test**: Build each registered prompt with valid and invalid data, validate representative provider results, and verify that retrieval intent and every evidence-derived claim use the shared contracts.

**Acceptance Scenarios**:

1. **Given** a registered prompt identifier and contract-compliant input, **When** a provider request is built, **Then** the request records the exact prompt identifier and version and validates the returned output before use.
2. **Given** missing required input or malformed output, **When** validation runs, **Then** the operation fails safely with a contract error and applies no durable content change.
3. **Given** a retrieval decision, **When** the shared contract is produced, **Then** it records whether retrieval is needed, why, queries, aspects, filters, and requery guidance; evidence-derived claims identify their sources.

---

### User Story 3 - Resume and restore versioned work (Priority: P3)

As a user, I can return to evolving work and retain a precise history of drafts, document references, provenance, and explicit decisions, so later workflow screens can restore an earlier draft without losing newer history.

**Why this priority**: Resume and restoration are required foundations for later Capture, Work Log, Knowledge, and archive experiences.

**Independent Test**: Append two content versions, restore the first with an exact-version compare-and-swap, and verify a new head version records restoration provenance while references and decisions retain exact target versions.

**Acceptance Scenarios**:

1. **Given** a current content head, **When** a new version is appended with the expected head, **Then** the new immutable version becomes head and records its prompt, source, and restoration provenance.
2. **Given** an outdated expected head, **When** a writer attempts to append or restore, **Then** the compare-and-swap fails without changing the head.
3. **Given** a document reference or decision, **When** it is saved, **Then** it binds to a stable document identifier, exact version and optional section, and a status that distinguishes proposed, accepted, rejected, deferred, superseded, and withdrawn decisions.

### Edge Cases

- Provider execution succeeds after the source record is deleted or its version changes.
- A result is valid JSON but violates the registered output contract.
- Two processes append a content version using the same expected head.
- A retry uses a prompt version that is no longer registered.
- A speculative latest-wins request is replaced while provider cancellation is still in flight.
- A source attribution identifies a missing document version or section.
- An operation completes successfully but its proposal remains unapplied and needs review.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST maintain one central prompt registry whose definitions have stable identifiers, explicit versions, input contracts, output contracts, request builders, and output validators.
- **FR-002**: Current native AI invocation paths MUST use registered prompt definitions and record the exact prompt identifier and version used.
- **FR-003**: The system MUST reject missing required prompt inputs and malformed prompt outputs before any durable result is applied.
- **FR-004**: The system MUST expose a retrieval-intent contract containing `needed`, `reason`, `queries`, `aspects`, `filters`, and `requery`.
- **FR-005**: Evidence-derived output MUST support source attribution to a stable document identifier, exact document version, optional section, and excerpt or claim locator.
- **FR-006**: The system MUST distinguish user-owned workflow state, asynchronous execution state, immutable content versions, and interface presentation state.
- **FR-007**: The system MUST report execution outcome separately from application disposition, including review-needed, applied, superseded, rejected, and not-applicable outcomes.
- **FR-008**: The system MUST reuse the existing asynchronous queue, cancellation, retry, and source-version infrastructure rather than introducing a competing execution engine.
- **FR-009**: Initial Capture distillation, run-report distillation, incremental journey maintenance, and publication index maintenance MUST be classified as durable important operations.
- **FR-010**: Optional speculative search MUST support latest-wins cancellation and MUST NOT create durable retry state.
- **FR-011**: User-requested generation MUST expose visible durable status and retry after safe failure.
- **FR-012**: Equivalent active durable requests MUST coalesce, and stale or replaced work MUST be cancelled or prevented from applying.
- **FR-013**: Durable content writes and restorations MUST use exact-head compare-and-swap and preserve immutable version history.
- **FR-014**: Draft versions MUST record provenance sufficient to identify source versions, prompt identity and version, operation identity, and restoration ancestry when applicable.
- **FR-015**: Restoration MUST create a new content version from an older version instead of moving or deleting history.
- **FR-016**: Document references MUST bind stable document identity to an exact version and optional section.
- **FR-017**: Decision records MUST support proposed, accepted, rejected, deferred, superseded, and withdrawn statuses without advancing workflow state automatically.
- **FR-018**: Existing AI-backed workflows and existing user records MUST continue to work after migration.
- **FR-019**: The migration MUST be additive, transactional, validate referential integrity, and preserve a restore path through the existing database backup and recovery mechanism.
- **FR-020**: The foundation MUST provide contracts and persistence services only; Capture experience, Work Log and journey UI, retrieval orchestration, preview UI, Knowledge publication behavior, and archive behavior remain separate downstream features.

### Key Entities

- **Prompt Definition**: Stable prompt identity, version, operation class, required input and output fields, request builder, and validator.
- **Retrieval Intent**: A versioned decision about whether and how evidence should be retrieved, including reason, queries, aspects, filters, and requery guidance.
- **Source Attribution**: An exact reference from an evidence-derived claim to a document version and optional section or excerpt.
- **Asynchronous Operation**: Existing queued work augmented with prompt identity, source version, execution outcome, and application disposition.
- **Content Version**: Immutable draft or derived-content snapshot with head position, source hash, provenance, and restoration ancestry.
- **Document Reference**: Stable relationship to an exact document version and optional section.
- **Decision Record**: Explicit human or system proposal outcome with a bounded status and exact content target.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All current native provider calls that produce structured workflow results resolve a registered prompt identity and validate structured output before durable application.
- **SC-002**: Tests demonstrate that stale concurrent writers and stale asynchronous results produce zero unintended head-version changes.
- **SC-003**: Two equivalent durable submissions produce one active operation, while two successive latest-wins speculative submissions leave only the newest active.
- **SC-004**: Every persisted generated content version can be traced to its operation, prompt version, and source version, and every restoration can be traced to the restored version.
- **SC-005**: Existing focused native tests for the queue, refinement, journey, and Knowledge draft paths continue to pass after the additive migration.
- **SC-006**: Contract construction and validation add no provider or semantic-model work to capture persistence, search, or other hot paths.

## Assumptions

- SQLite remains the durable local store, and its existing backup and migration recovery path covers this additive migration.
- Existing `ai_jobs_v2` identity, cancellation registry, idempotency key, retry, and source hash remain authoritative for durable execution.
- Presentation state is returned to callers but is not persisted as workflow state by this feature.
- Later workflow specs consume these contracts and own their feature-specific behavior and UI.
