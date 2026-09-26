# Feature Specification: Multi-aspect Knowledge retrieval

**Feature Branch**: `018-multiaspect-knowledge-retrieval`

**Created**: 2026-09-26

**Status**: Specification validated; implementation planning pending dependencies

**Input**: User-approved knowledge-workflow program, spec D; see [requirements ledger](../knowledge-workflow-program.md).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Find knowledge by when it is useful (Priority: P1)

A user describes a new problem in different words from existing notes and finds documents whose applicability matches that problem.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given matching applicability without shared keywords, when searching, then the relevant document remains eligible and its supporting passage is returned.

### User Story 2 - Follow historical choices to final decisions (Priority: P1)

A user searches an earlier approach and sees its successor decision with rationale and valid conditions.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given A was replaced by B, when A matches a query, then B is primary and A is labelled historical rather than recommended.

### User Story 3 - Keep exploratory ideas useful but qualified (Priority: P2)

A user may discover an unverified idea without confusing it with an established result.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given an applicable final decision and an unverified idea, when searching, then the final decision has priority and the idea retains its uncertainty.

### Edge Cases

- Source revisions change while asynchronous work is running.
- Evidence is missing, contradictory, superseded, or explicitly unverified.
- A provider or local model is unavailable; existing user content remains usable.
- Titles are duplicated or paths change; identity and reference versions remain distinct.
- User edits, retries, and repeated notifications must not duplicate or overwrite results.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST represent applicability, final-decision context, body passages, and exploration as distinct searchable aspects when present; do not manufacture missing aspects.
- **FR-002**: The system MUST preserve document identity, source revision, section, information type/status, decision successor, and source provenance on every result.
- **FR-003**: The system MUST obtain independent lexical and semantic candidates; do not restrict semantic discovery to exact keyword matches.
- **FR-004**: The system MUST weight applicability strongly and unverified exploration lower; deduplicate same-document passages and bound result/context volume.
- **FR-005**: The system MUST check current applicability and show contradictory conclusions with their differing conditions rather than silently choosing a universal winner.
- **FR-006**: The system MUST reuse results only while query context and source versions remain valid; unavailable semantic search must retain lexical fallback without claiming completeness.
- **FR-007**: The system MUST update changed search units, support model/version/dimension compatibility, and prevent stale vectors from masquerading as current evidence.
- **FR-008**: The system MUST maintain existing-note body search while applicability is progressively added; do not bulk rewrite source files.

### Key Entities

- **SearchUnit**: Feature-owned information with stable source identity and revision where relevant.
- **EmbeddingIdentity**: Feature-owned information with stable source identity and revision where relevant.
- **RetrievalQuery**: Feature-owned information with stable source identity and revision where relevant.
- **RankedEvidence**: Feature-owned information with stable source identity and revision where relevant.
- **DecisionSuccessor**: Feature-owned information with stable source identity and revision where relevant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of historical-choice fixtures link to the correct current decision without recommending a superseded choice.
- **SC-002**: All returned evidence passages identify their document and revision, including uncertainty status for idea fixtures.
- **SC-003**: Warm lexical retrieval remains within the existing 75 ms budget and context respects the existing eight-passage/6,000-token limit.
- **SC-004**: Relevant applicability-only fixtures remain discoverable even when no exact query keyword appears.

## Assumptions

- Dependencies: A foundation and C decision contracts. Contract availability does not imply implementation completion.
- Scope boundary: External Vault connection, generation of final articles, and publication UI are outside this feature; their contracts are consumed.
- User-specified terms and existing locale are preserved; Korean copy uses 사용자.
- Product Spirit: reduces cognitive load, preserves resumption context and portable evidence; does not score people or transfer decision/publication authority to AI.
- Numeric retrieval weights and timing optimizations are implementation-plan decisions; existing binding performance budgets remain applicable.
