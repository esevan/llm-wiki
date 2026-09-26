# Feature Specification: Final Knowledge and separate idea drafts

**Feature Branch**: `020-knowledge-idea-distillation`

**Created**: 2026-09-26

**Status**: Specification validated; implementation planning pending dependencies

**Input**: User-approved knowledge-workflow program, spec F; see [requirements ledger](../knowledge-workflow-program.md).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read final outcomes without reconstructing the session (Priority: P1)

A user generates a standalone Knowledge draft from completed work and sees the final decision, evidence, and limits.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given receipt-only was replaced by approval-required, when drafting, then the final approval decision is central and earlier history is traceable.

### User Story 2 - Preserve useful unverified ideas separately (Priority: P1)

A user retains an exploration without presenting it as final Knowledge.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given reminder automation was considered but not tested, when drafting, then it becomes a separate optional idea draft with unverified state and reconsideration conditions.

### User Story 3 - Review exact draft versions (Priority: P2)

A user compares drafts and understands which source snapshot produced each.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given new task evidence arrives during generation, when the old response returns, then it cannot silently replace a draft based on newer evidence.

### Edge Cases

- Source revisions change while asynchronous work is running.
- Evidence is missing, contradictory, superseded, or explicitly unverified.
- A provider or local model is unavailable; existing user content remains usable.
- Titles are duplicated or paths change; identity and reference versions remain distinct.
- User edits, retries, and repeated notifications must not duplicate or overwrite results.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST generate from work definition, evidence-grounded Distillation, topic-level final decisions, verification, and exact journey snapshot; ensure pending updates are reconciled before snapshot use.
- **FR-002**: The system MUST provide final outcomes first, context, explanation/procedure/rationale, application limits/reconsideration, applicability, and source links.
- **FR-003**: The system MUST support concept, guide, comparison/decision, and research result structures without inventing decisions for non-decision tasks.
- **FR-004**: The system MUST define applicability with use situations, representative questions, actual help, scope/exclusions; never overstate what the body supports.
- **FR-005**: The system MUST validate unsupported additions and material omissions separately; retain conditions, exceptions, counterevidence, and unresolved matters.
- **FR-006**: The system MUST keep intermediate explorations out of the final article body and create separate valuable idea units only when warranted.
- **FR-007**: The system MUST distinguish untested, deferred, out-of-scope, and evidence-rejected ideas with source/task/final-Knowledge links and reconsideration conditions.
- **FR-008**: The system MUST expose draft version selection/comparison/restore-as-new with source snapshots, distinguish current private draft from publication.
- **FR-009**: The system MUST do not claim an executed action was effective without supporting evidence; do not promote ideas solely because they were archived.
- **FR-010**: The system MUST preserve the reviewed draft for subsequent publication rather than unrestricted regeneration at write time.

### Key Entities

- **KnowledgeDraftVersion**: Feature-owned information with stable source identity and revision where relevant.
- **IdeaDraft**: Feature-owned information with stable source identity and revision where relevant.
- **ApplicabilityDescription**: Feature-owned information with stable source identity and revision where relevant.
- **EvidenceSnapshot**: Feature-owned information with stable source identity and revision where relevant.
- **QualityFinding**: Feature-owned information with stable source identity and revision where relevant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All fidelity fixtures retain designated material conditions and contain zero unsupported completion/verification claims.
- **SC-002**: Unverified exploration is absent from final-Knowledge bodies and present only in separately labelled idea drafts when useful.
- **SC-003**: Every generated factual or decision-bearing claim has traceable source evidence in the accepted fixture set.
- **SC-004**: All stale-generation fixtures retain current evidence and draft history; user-selected restoration never rewrites actual work history.

## Assumptions

- Dependencies: C logs/decisions, A versions, reference provenance contracts. Contract availability does not imply implementation completion.
- Scope boundary: Directory/MOC placement and filesystem publication belong to G; retrieval ranking belongs to D. Draft generation does not publish or complete a Task.
- User-specified terms and existing locale are preserved; Korean copy uses 사용자.
- Product Spirit: reduces cognitive load, preserves resumption context and portable evidence; does not score people or transfer decision/publication authority to AI.
- Numeric retrieval weights and timing optimizations are implementation-plan decisions; existing binding performance budgets remain applicable.
