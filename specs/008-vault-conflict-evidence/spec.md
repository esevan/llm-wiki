# Feature Specification: Evidence-Rich Vault Conflict Review

**Feature Branch**: `008-vault-conflict-evidence`
**Created**: 2026-09-01
**Last Reconciled**: 2026-09-02
**Status**: Current baseline reconciled — confirmed progress, deduplication, and cancellation changes pending

## User Scenarios & Testing

### User Story 1 — Request an evidence-grounded review (Priority: P1)

A user starts background conflict review for a Solution. The final result identifies the indexed
Vault scope, semantic coverage, reviewed claims, candidate evidence, potential conflicts, and a
conservative recommendation.

**Acceptance Scenarios**:

1. **Given** relevant Vault evidence, **When** review completes, **Then** every displayed conflict
   includes its claim, severity, exact Vault path, source revision, passage, line range,
   explanation, and required resolution.
2. **Given** no evidence candidate or incomplete semantic coverage, **When** review completes without
   a finding, **Then** the recommendation is `insufficient_evidence`, not `clear`.
3. **Given** at least one valid finding, **When** review completes, **Then** the recommendation is
   `potential_conflict`.

### User Story 2 — Distinguish safe clear from insufficient evidence (Priority: P1)

`clear` is recommended only when there were candidates, semantic coverage was complete, every
retained candidate was reviewed, and no valid conflict finding remained.

### User Story 3 — Reuse or cancel background review (Priority: P2)

An unchanged Solution and unchanged indexed Vault can reuse a completed review. The user can request
cancellation through the Queue or conflict-review cancellation action.

### Edge Cases

- Missing semantic dependencies leave lexical candidates available but prevent complete coverage.
- Malformed screening output retains uncertain candidates for strong review.
- Invalid finding evidence is omitted rather than presented as a conflict.
- Any change to the Solution, parent Problem, locale view, or indexed Vault invalidates active and
  cached review identity.
- Cancellation requested while running enters a cancelling state before terminal cancellation.
- Provider and network failures are exposed through failed or retryable Queue state; partial
  findings are not currently published before terminal completion.

## Requirements

### Functional Requirements

- **FR-001**: Missing and stale document embeddings MUST be prepared in background work after startup
  and Vault changes while lexical search remains usable.
- **FR-002**: Conflict retrieval MUST combine lexical candidates with an independent semantic search
  over current embedded documents when semantic retrieval is available.
- **FR-003**: Review MUST split Solution and parent Problem content into typed claims covering scope,
  requirements, constraints, non-goals, and validation expectations.
- **FR-004**: Candidate evidence MUST retain exact Vault path, source revision, passage text, line
  range, related claim, and retrieval signals.
- **FR-005**: Fast screening MAY exclude a candidate only when it supplies an explicit
  evidence-grounded non-conflict decision; malformed or uncertain decisions MUST retain the
  candidate.
- **FR-006**: Every retained candidate MUST receive strong review before the final recommendation is
  produced.
- **FR-007**: A conflict finding MUST be accepted only when the response identifies the reviewed
  evidence and provides a non-empty explanation.
- **FR-008**: Final results MUST expose scope, embedding coverage, claims, candidate and retained
  counts, reviewed count, findings, progress, recommendation, and recommendation meaning.
- **FR-009**: Final recommendation MUST be `potential_conflict` when any valid finding exists,
  `clear` only with candidates and complete semantic coverage, and `insufficient_evidence`
  otherwise.
- **FR-010**: Running status MUST remain `reviewing` through the Queue and MUST NOT expose a clear
  recommendation before completion.
- **FR-011**: Completed reviews MUST be reusable only for the same Solution/Problem content, locale,
  and complete indexed-Vault revision.
- **FR-012**: Cancellation MUST use the durable work lifecycle and MUST prevent a cancelled result
  from changing Solution or conflict state.
- **FR-013**: AI recommendations MUST remain advisory and MUST NOT apply conflict state or approve a
  Solution.
- **FR-014**: While review is running, the result surface MUST expose safe, factual progress phases
  and available counts, such as claim preparation, evidence search, screening, detailed review, and
  finalization, without exposing hidden model reasoning.
- **FR-015**: Final evidence MUST be deduplicated by document while preserving every related claim
  association and the strongest readable passage for each association.
- **FR-016**: Cancellation MUST stop active search and provider computation promptly, not merely
  discard a late result, and MUST publish no post-cancellation finding.

### Key Entities

- **Conflict review work item**: One source-bound background review and its lifecycle state.
- **Review claim**: A typed statement extracted from the Solution or parent Problem.
- **Evidence candidate**: One claim/passage pair retained for screening or review.
- **Conflict finding**: A validated evidence-backed potential conflict.
- **Recommendation**: A final `potential_conflict`, `clear`, or `insufficient_evidence` advisory
  result.

## Success Criteria

- **SC-001**: Every displayed finding contains a current path, source revision, non-empty passage,
  valid line range, explanation, and required resolution.
- **SC-002**: Zero-candidate and incomplete-coverage tests produce `clear` in zero cases.
- **SC-003**: Repeating an unchanged review reuses a completed result, while changing any indexed
  Vault source prevents that reuse.
- **SC-004**: Conflict Review changes Solution approval or conflict state automatically in zero
  tested cases.
- **SC-005**: Cancellation and provider failure preserve all pre-existing Solution and Vault content.

## Assumptions

- Progress visibility means observable work phases and counts; it does not include private model
  reasoning or fabricated elapsed-time estimates.
- Cache validity uses the complete indexed Vault revision, so any indexed-document change is a
  conservative invalidation.
- The default and Advanced models may be used for separate screening and strong-review passes.

## Confirmed Implementation Gaps

- **IG-016 — Conflict Review progress**: The current review exposes only generic Queue progress
  before the terminal report. It must publish safe intermediate phases and available counts.
- **IG-017 — Evidence deduplication and hard cancellation**: Current candidates may repeat the same
  document across claims, and cancellation is cooperative. Final evidence must deduplicate documents
  while retaining claim links, and cancellation must promptly terminate active computation.
