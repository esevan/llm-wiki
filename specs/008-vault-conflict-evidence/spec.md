# Feature Specification: Evidence-Rich Vault Conflict Review

**Feature Branch**: `feat/vault-conflict-evidence`

**Created**: 2026-09-01

**Status**: Ready for implementation

**Input**: Improve Vault conflict review so its search scope, semantic coverage, candidates, progress, evidence, and decision semantics are inspectable and reliable.

## User Scenarios & Testing

### User Story 1 - Inspect a trustworthy review (Priority: P1)

A person starts conflict review and sees the indexed Vault scope, embedding coverage, candidate counts, and phase progress. Potential conflicts appear with exact Vault passages while remaining candidates continue.

**Why this priority**: Review latency is acceptable only when it produces visible, auditable evidence.

**Independent Test**: Start a review against a Vault containing a conflicting decision and verify progress metadata and the cited finding before the terminal result.

**Acceptance Scenarios**:

1. **Given** indexed Vault documents, **When** review begins, **Then** scope, coverage, candidate counts, and current phase are available.
2. **Given** a potential conflict is confirmed, **When** other candidates remain, **Then** the finding is visible with an exact path and excerpt while progress remains incomplete.

---

### User Story 2 - Distinguish absence from insufficient evidence (Priority: P1)

A person can distinguish a completed evidence-backed clear recommendation, no conflict found so far, and insufficient evidence. Clear is never suggested before every retained candidate is reviewed.

**Why this priority**: False or premature clear recommendations undermine the approval gate.

**Independent Test**: Run reviews with zero candidates, incomplete embedding coverage, and complete non-conflicting candidates and compare terminal states.

**Acceptance Scenarios**:

1. **Given** no candidates or incomplete semantic coverage, **When** review completes, **Then** the state is `insufficient_evidence`, not `clear`.
2. **Given** retained candidates are still pending, **When** status is requested, **Then** the state is `reviewing`, never `clear`.
3. **Given** adequate coverage and all candidates reviewed without conflict, **When** review completes, **Then** an evidence-backed `clear` recommendation may be shown.

---

### User Story 3 - Reuse work and cancel it (Priority: P2)

A person receives cached results for unchanged Solution/Vault inputs and can cancel a running browser review so the server stops additional search and model work.

**Why this priority**: Repeated latency and abandoned provider requests waste time and resources.

**Independent Test**: Repeat an unchanged review, modify one document, and cancel a running review; verify reuse, selective invalidation, and cancellation.

**Acceptance Scenarios**:

1. **Given** identical Solution and Vault hashes, **When** review repeats, **Then** the completed result is reused.
2. **Given** one Vault document changes, **When** review repeats, **Then** stale cached evidence is not reused for that document.
3. **Given** a running review, **When** the browser cancels, **Then** the server marks it cancelled and issues no later model requests.

### Edge Cases

- Semantic dependencies or the embedding model are unavailable; lexical search remains available but review evidence is insufficient.
- A fast screening response is malformed, incomplete, or lacks an exact evidence reference; the candidate is retained for strong review.
- Raw and canonical generated notes describe the same completed Solution; only the canonical evidence is retained.
- A cited document changes during review; the finding fails validation and cannot support clear.
- The provider fails after some findings; findings remain inspectable but the terminal state is insufficient evidence.

## Requirements

### Functional Requirements

- **FR-001**: The system MUST create or refresh document embeddings after Vault changes and on startup for missing or stale entries.
- **FR-002**: Semantic retrieval MUST search all embedded Vault documents independently of lexical results.
- **FR-003**: The system MUST merge lexical and semantic candidates and remove duplicate paths and Raw/canonical duplicates.
- **FR-004**: The system MUST split a Solution into reviewable claims covering requirements, scope, constraints, non-goals, and validation expectations.
- **FR-005**: Each retained claim/candidate pair MUST preserve an exact Vault path, source hash, passage text, and passage line range.
- **FR-006**: A fast screening pass MAY exclude only explicit, complete, evidence-grounded non-conflicts; malformed or uncertain results MUST be retained.
- **FR-007**: Retained candidates MUST receive strong review, with valid potential conflicts exposed before the full run completes.
- **FR-008**: Status MUST expose search scope, embedding coverage, candidate counts, progress, current phase, findings, and search/screen/review timings.
- **FR-009**: `clear` MUST NOT be suggested until every retained candidate is reviewed and every finding has valid evidence.
- **FR-010**: Zero candidates, incomplete semantic coverage, provider failure, invalid citations, and incomplete review MUST result in `insufficient_evidence` or `reviewing`, not `clear`.
- **FR-011**: Results MUST distinguish `reviewing`, `potential_conflict`, `no_conflict_found`, `clear`, `insufficient_evidence`, `cancelled`, and `failed` meanings.
- **FR-012**: Completed results MUST be reusable by Solution content hash plus relevant Vault document hashes; changed evidence MUST be invalidated.
- **FR-013**: Browser cancellation MUST propagate to server review state and prevent subsequent search/model operations.
- **FR-014**: AI recommendations MUST remain evidence for a human decision and MUST NOT change workflow state automatically.

### Key Entities

- **Review Run**: Input hashes, phase, semantic coverage, counts, timings, state, cancellation, and cache provenance.
- **Review Claim**: A stable, typed assertion extracted from the Solution.
- **Evidence Passage**: Vault path, line range, exact text, document hash, lexical/semantic scores.
- **Candidate Review**: Claim/evidence pair with screening and strong-review disposition.
- **Finding**: Evidence-backed potential conflict with explanation, severity, and required human resolution.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Every running review exposes scope, coverage, phase, candidate counts, and progress through the local API and browser.
- **SC-002**: 100% of displayed findings contain an existing Vault path, non-empty source passage, valid line range, and conflict explanation.
- **SC-003**: Tests prove that incomplete coverage, zero candidates, malformed screening, and invalid citations never produce `clear`.
- **SC-004**: Unchanged repeat reviews reuse a completed result; changing a cited Vault source invalidates reuse.
- **SC-005**: Cancellation tests prove no new provider call begins after server cancellation is recorded.
- **SC-006**: Search, screening, and strong-review elapsed times are recorded separately for every terminal run.

## Assumptions

- Existing local provider settings supply a default fast model and an advanced strong model; if only one model is configured it may serve both roles while preserving separate passes.
- Version one uses document embeddings and passage extraction without adding a remote vector database.
- Minimum evidence thresholds remain deliberately conservative: incomplete embedding coverage or zero candidates is insufficient, while a human still owns the final workflow decision.
- Workbench items are excluded from Vault conflict evidence in this feature; the Solution and its parent Problem remain query context, not external evidence.
