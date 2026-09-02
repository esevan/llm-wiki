# Feature Specification: Completion, Writeback, and Archive

**Created**: 2026-08-18
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — compatibility API deprecation pending

## User Scenarios & Testing

### User Story 1 — Record evidence before completion (Priority: P1)

A user records text or screenshot evidence, comments, and checklist state inside a Solution and can
request an advisory Completion Review before deciding whether the Problem is complete.

### User Story 2 — Complete work and create portable Knowledge (Priority: P1)

An explicit completion action records the decision, creates a deterministic completed-work document,
and schedules an AI-generated readable report without making the completed record depend on that AI
request succeeding.

**Acceptance Scenarios**:

1. **Given** an approved Solution with evidence and either a report or explicit no-update reason,
   **When** the manual completion transition is confirmed, **Then** completion is recorded and the
   Problem is completed.
2. **Given** completed work that creates Knowledge, **When** completion succeeds, **Then** a canonical
   completed-work document, raw work record, lineage, and captured images are written before the
   background report finishes.
3. **Given** background report failure, **When** the user opens completed work, **Then** the
   deterministic document and source evidence remain available.

### User Story 3 — Review, regenerate, remove, and reuse Knowledge (Priority: P2)

The user can review structured patches, apply or undo them, regenerate an unchanged completed-work
document, remove generated completion artifacts explicitly, inspect completed work, or create a
linked follow-up Problem.

### Edge Cases

- Completion Review never completes a Problem or changes Solution state.
- Externally changed projections and completed-work documents are not overwritten automatically.
- A lineage correction remains current even when its document synchronization needs a retry.
- Removing a generated completed-work document also removes its generated raw record and captured
  image assets, but not the workflow records.
- A no-update completion path may intentionally skip completed-work document creation.
- Provider failure never removes recorded Work Log evidence.

## Functional Requirements

- **FR-001**: An in-progress Solution MUST support text and image Work Log entries, comments, and an
  editable validation checklist.
- **FR-002**: Completion Review MUST be advisory, omit raw image payloads from review input, and MUST
  NOT change workflow state.
- **FR-003**: The manual completion transition MUST require implementation evidence and either a
  completion report or an explicit no-update reason.
- **FR-004**: The direct Problem completion action MUST remain an intentional user override when
  remote AI review is unavailable or unreliable and MUST record its optional reason and related
  review identity when supplied.
- **FR-005**: Completed-work publication MUST first produce a deterministic readable document from
  retained workflow and lineage evidence.
- **FR-006**: AI report generation MUST run as background work and MUST preserve the deterministic
  document if generation fails or becomes stale.
- **FR-007**: Generated Knowledge, raw evidence, and captured assets MUST use portable Markdown and
  file formats and MUST be indexed after successful publication.
- **FR-008**: Regeneration MUST preserve raw completion evidence and MUST be blocked when the tracked
  document was externally modified.
- **FR-009**: Deleting a tracked completed-work document MUST require an explicit force decision when
  external modification is detected.
- **FR-010**: Patch proposals MUST present append, replace-section, or insert-after-heading changes
  for review before application.
- **FR-011**: Patch application MUST be atomic, source-revision guarded, and reversible from its
  retained preimage.
- **FR-012**: Generated projections MUST use portable Markdown metadata and MUST not overwrite an
  externally changed projection.
- **FR-013**: Archive actions MUST move a tracked projection, retain its mirror identity, and refresh
  search.
- **FR-014**: Completed Solutions MUST remain available in a read-only workspace and MAY create a
  linked follow-up Problem without reopening the completed record.
- **FR-015**: When completion creates completed-work Knowledge, the explicit completion decision
  MUST also authorize deterministic publication and the source-bound asynchronous AI report update;
  no second publication approval is required.

### Key Entities

- **Completion decision**: The explicit Problem-level decision, optional reason, and optional review
  reference.
- **Completed-work document**: Canonical readable Knowledge and its tracked source revision.
- **Raw work record**: Preserved workflow, evidence, review, and decision material used to reconstruct
  completed work.
- **Patch proposal**: A reviewed, source-bound Knowledge change and its reversible preimage.

## Success Criteria

- **SC-001**: Completion, patch apply/undo, projection, lineage, and external-change protection tests
  pass against a temporary Vault.
- **SC-002**: Every tested provider failure leaves all pre-existing Work Log and completion evidence
  unchanged.
- **SC-003**: A completed-work document can be reconstructed from retained completion and lineage
  records.
- **SC-004**: Completed work remains readable when its generated document is missing.
- **SC-005**: A follow-up Problem is created as a new linked record without mutating the completed
  Solution.

## Assumptions

- Completion is a user action; AI review and report generation remain supporting operations.
- Completion is a single approval covering workflow completion and completed-work publication.
- A deterministic completed-work document is a safe fallback when the provider is unavailable.
- Generated document deletion does not delete the underlying workflow history.
- Direct completion and archive APIs remain compatibility paths for the current backend.

## Confirmed Deprecation

- **DEP-009 — Direct completion/archive APIs**: Direct completion, verification, projection,
  archive, and stage endpoints remain supported compatibility contracts for the current backend.
  They must be marked deprecated and are scheduled for removal with the planned Tauri backend.
