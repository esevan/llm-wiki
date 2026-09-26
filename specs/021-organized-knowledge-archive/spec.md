# Feature Specification: Organized Knowledge and idea archive

**Feature Branch**: `021-organized-knowledge-archive`

**Created**: 2026-09-26

**Status**: Specification validated; implementation planned behind dependency gates

**Input**: User-approved knowledge-workflow program, spec G; see [requirements ledger](../knowledge-workflow-program.md).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Organize without duplicating existing knowledge (Priority: P1)

A user reviews a proposed archive operation appropriate to the existing Wiki.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: With overlapping and non-overlapping Wiki fixtures, prepare proposals without writing files and verify the five outcome choices, stable target identity, existing-taxonomy preference, and stale-target rejection.

**Acceptance Scenarios**:

1. Given an existing overlapping article, when archiving, then update/merge/conflict/supersede are considered before creating a duplicate.

### User Story 2 - Publish linked portable documents (Priority: P1)

A user approves documents, locations, tags, MOC changes, and actual-use references.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Publish one reviewed final artifact and selected idea artifacts, then verify exact file hashes, final/idea separation, eligible source links and rationales, preserved MOC user bytes, and distinct index-pending/completed states.

**Acceptance Scenarios**:

1. Given reviewed Knowledge and explicitly selected idea drafts, when publishing, then separate portable files and actual-use support/counterevidence wikilinks preserve their different epistemic status.

### User Story 3 - Recover without losing external edits (Priority: P2)

A user edits a Vault file or encounters an indexing failure without silent overwrites or broken recovery.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Interrupt publication after each file/database/index boundary, restart it, and verify hash-directed idempotent recovery; introduce an external edit and verify it remains intact with a review-required state.

**Acceptance Scenarios**:

1. Given an external file change, when replacement is attempted, then it is guarded and the user's content remains intact.
2. Given exact files and database records but a failed index job, when the application restarts or the user retries indexing, then it indexes the same file revisions without rewriting or duplicating publication.

### Edge Cases

- Source revisions change while asynchronous work is running.
- Evidence is missing, contradictory, superseded, or explicitly unverified.
- A provider or local model is unavailable; existing user content remains usable.
- Titles are duplicated or paths change; identity and reference versions remain distinct.
- User edits, retries, and repeated notifications must not duplicate or overwrite results.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST propose new/update/merge/conflict/supersede based on existing Wiki structure, body, applicability, and related knowledge.
- **FR-002**: The system MUST prefer existing taxonomy, assign one canonical location, use tags/aliases/MOCs for multiple contexts, and propose new categories only when needed.
- **FR-003**: The system MUST show content, path, metadata, MOC changes, references, and unresolved conflicts before publication; retain explicit publication authority.
- **FR-004**: The system MUST write separate idea/ documents with uncertainty/deferred/rejected state, not exploration embedded in final Knowledge.
- **FR-005**: The system MUST link actually used supporting or counterevidence documents with wikilinks and usage rationale; retain exact source ID/version/section internally.
- **FR-006**: The system MUST update only reviewed managed MOC regions, preserve every byte outside those regions, reject malformed or changed regions, and prevent duplicate links.
- **FR-007**: The system MUST maintain stable document IDs across moves/renames and repair managed links and index visibility on move, replacement, or recoverable withdrawal without silently rewriting user-authored links.
- **FR-008**: The system MUST treat Markdown/YAML as portable content and search as rebuildable; distinguish file-write success from database recording and indexing success; and recover partial operations through a durable, idempotent publication journal with bounded indexing retries and explicit retry after terminal failure.
- **FR-009**: The system MUST guard external modifications and preserve all existing records during migration; incrementally add metadata/applicability without bulk rewriting.
- **FR-010**: The system MUST use explicit type/state metadata rather than directory alone for search trust, preserve a compatible typed-section extension point for future mixed-content documents, and exclude external personal Vault connection from this feature.
- **FR-011**: The system MUST feed reviewed publication to version-aware indexing so later work finds final decisions first and ideas as qualified suggestions.
- **FR-012**: The system MUST publish the exact reviewed artifact bytes without regeneration and MUST NOT publish merely because a Task completed.
- **FR-013**: The system MUST compare the authoritative hash of every target file and managed region against the reviewed proposal before applying it; a mismatch MUST preserve the external edit and require review.
- **FR-014**: The system MUST derive published support/counterevidence links only from actual-use provenance, include a visible rationale, and never promote viewed, mentioned, candidate, or excluded references to published links.

### Key Entities

- **ArchiveProposal**: Feature-owned information with stable source identity and revision where relevant.
- **PublicationRevision**: Feature-owned information with stable source identity and revision where relevant.
- **CanonicalPath**: Feature-owned information with stable source identity and revision where relevant.
- **ManagedMocPatch**: Feature-owned information with stable source identity and revision where relevant.
- **ReferenceLink**: Feature-owned information with stable source identity and revision where relevant.
- **RecoveryRecord**: Feature-owned information with stable source identity and revision where relevant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every published reference in acceptance fixtures resolves to the intended document/section.
- **SC-002**: External-edit and partial-failure fixtures preserve user content and can recover indexing without duplicate publication.
- **SC-003**: Final Knowledge and idea files remain readable and linked without the application.
- **SC-004**: Published final decisions are retrievable in the next-work scenario; speculative ideas remain visibly qualified.

## Assumptions

- Dependencies: D retrieval, F drafts, A storage/reference and E reference usage. Contract availability does not imply implementation completion.
- Scope boundary: Actual external personal Vault connection, bulk rewrite of existing notes, autonomous publication, and installation/deployment are excluded.
- User-specified terms and existing locale are preserved; Korean copy uses 사용자.
- Product Spirit: reduces cognitive load, preserves resumption context and portable evidence; does not score people or transfer decision/publication authority to AI.
- Numeric retrieval weights and timing optimizations are implementation-plan decisions; existing binding performance budgets remain applicable.
