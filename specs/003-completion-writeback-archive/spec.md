# Feature Specification: Completion, Writeback, and Archive

**Created**: 2026-08-18
**Status**: Implemented and maintained as the completion flow

## User Scenarios & Testing

### User Story 1 — Record evidence before completion (Priority: P1)

A person records Solution progress, comments, checklist work, and completion evidence, then reviews
an AI-generated completion assessment without the assessment completing the work automatically.

**Acceptance**: the report evaluates each validation criterion, shows remaining work and a decision
basis, and completion still requires an explicit human action and optional reason.

### User Story 2 — Review and apply knowledge changes safely (Priority: P1)

A person reviews a structured Markdown patch with base, current, and proposed content, applies it
only when the source is unchanged, and can undo the applied patch.

**Acceptance**: writes are adapter-owned and atomic; a source-hash mismatch blocks application;
undo restores the saved preimage.

### User Story 3 — Reuse completed work (Priority: P2)

A person completes a Problem and gets a generated, regenerable completed-work Playbook in the vault
with the concise summary and raw evidence retained separately. They can open, regenerate, or remove
the generated file; an archived projection remains discoverable from the Workbench.

**Acceptance**: external edits block automatic replacement until reviewed; archive/mirror tracking
detects missing files and reindexes moved content.

## Functional Requirements

- **FR-001**: A Solution MUST support text and image progress entries, comments, and checklist
  entries while it is in progress.
- **FR-002**: Completion review MUST be advisory and MUST never advance state by itself.
- **FR-003**: Human completion MUST record evidence, report/review data where supplied, and an
  explicit no-update reason when no knowledge writeback is chosen.
- **FR-004**: Patch proposals MUST be structured as append, replace-section, or insert-after-heading
  operations with base/current/proposed review data.
- **FR-005**: Patch application MUST be atomic, source-hash guarded, and reversible from a stored
  preimage.
- **FR-006**: Generated projections and Playbooks MUST use Obsidian-compatible Markdown frontmatter.
- **FR-007**: Playbook regeneration MUST preserve raw completion data and MUST block when the
  generated document has been externally modified.
- **FR-008**: Archive moves MUST be adapter-owned, mirrored in local state, and trigger reindexing.

## Success Criteria

- **SC-001**: Tests cover completion, patch apply/undo, source-hash conflicts, projections, and
  archive behavior against a temporary local vault.
- **SC-002**: An externally edited projection or Playbook is not overwritten automatically.
- **SC-003**: A completed-work Playbook can be reconstructed from immutable local completion data.

## Assumptions

- Image summarization uses the configured image-summary model or the Solution model fallback. If
  the provider is unavailable, the original local work record is retained for human review.
- Generated documents are local vault artifacts; they are never sent to an external provider solely
  for archiving.
