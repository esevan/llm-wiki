# Feature Specification: Reference-aware work previews and conversation

**Feature Branch**: `019-reference-aware-workbench`

**Created**: 2026-09-26

**Status**: Specification validated; implementation planning pending dependencies

**Input**: User-approved knowledge-workflow program, spec E; see [requirements ledger](../knowledge-workflow-program.md).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Receive an evidence-backed work preview first (Priority: P1)

A user supplies a request and receives a structured work preview without a mandatory clarification exchange.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given sufficient context and relevant sources, when preparing work, then description/background/scope/completion criteria are proposed with evidence and explicit assumptions.

### User Story 2 - Keep talking while investigation progresses (Priority: P1)

A user changes conditions while background research continues without stale advice overwriting the latest intent.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given an investigation for revision 1, when revision 2 changes the approver, then arriving findings are checked against revision 2 before use.

### User Story 3 - Inspect and mention many references (Priority: P2)

A user filters a reference list, opens an exact passage, and mentions documents or sections in a follow-up message.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given multiple matching documents, when selecting references, then the input receives stable document mentions without sending automatically.

### User Story 4 - Choose a working version (Priority: P2)

A user compares or restores an earlier work preview without losing actual activity history.

**Why this priority**: Preserves useful evidence and user control while reducing repetitive work.

**Independent Test**: Exercise this story with fixed source records and verify the stated outcome without downstream publication where unnecessary.

**Acceptance Scenarios**:

1. Given versions 1–3, when restoring 1, then a new current version preserves the old history and reference snapshots.

### Edge Cases

- Source revisions change while asynchronous work is running.
- Evidence is missing, contradictory, superseded, or explicitly unverified.
- A provider or local model is unavailable; existing user content remains usable.
- Titles are duplicated or paths change; identity and reference versions remain distinct.
- User edits, retries, and repeated notifications must not duplicate or overwrite results.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST structure work, not a Knowledge article: description, background, goal, scope/non-goals, constraints, completion criteria, initial approach, uncertainty.
- **FR-002**: The system MUST consume prompt-provided retrieval intent; necessary retrieval precedes the first complete preview, while follow-up speculative investigation runs beside conversation.
- **FR-003**: The system MUST apply evidence-backed draft preparation automatically within the agreed scope, label assumptions/provenance, and never infer user adoption from silence.
- **FR-004**: The system MUST NOT overwrite explicit user conditions or intervening edits; conflicting changes remain proposals.
- **FR-005**: The system MUST surface important contradictions or failed historical approaches promptly, supporting evidence in a subsequent answer, ancillary results in the reference area.
- **FR-006**: The system MUST provide a searchable/filterable/sortable reference list that deduplicates documents and does not unexpectedly reorder during reading.
- **FR-007**: The system MUST open hyperlinks in one Markdown modal with section focus, navigation/back, source-version distinction, keyboard close, and restored input/focus/scroll.
- **FR-008**: The system MUST support @ lookup and document/section/multiple-document mentions bound to source identity/version; distinguish viewed, used, adopted, excluded.
- **FR-009**: The system MUST provide meaningful version history, field comparisons, restore-as-new, exact assumptions/source snapshots, and queued proposals during editing.
- **FR-010**: The system MUST use shared applied-content transitions for adoption/auto apply/refinement/restore, not historical navigation or rerender; respect reduced motion.
- **FR-011**: The system MUST represent searching, no suitable result, failure, and changed context distinctly; optional research failure must not block conversation.
- **FR-012**: The system MUST record actual source use for later lineage/archive; discovery or mere mention does not assert adoption.

### Key Entities

- **WorkPreviewVersion**: Feature-owned information with stable source identity and revision where relevant.
- **DraftAssumption**: Feature-owned information with stable source identity and revision where relevant.
- **Investigation**: Feature-owned information with stable source identity and revision where relevant.
- **ReferenceUsage**: Feature-owned information with stable source identity and revision where relevant.
- **DocumentMention**: Feature-owned information with stable source identity and revision where relevant.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All stale-response fixtures preserve the latest input and user edits.
- **SC-002**: Every reference and section mention in acceptance fixtures opens the intended source in the modal.
- **SC-003**: Restoration fixtures preserve all earlier versions and leave actual Work Logs/decisions unchanged.
- **SC-004**: Users can complete initial preview generation without a mandatory clarification reply for the supported contract-change scenario.

## Assumptions

- Dependencies: B Capture, D retrieval, A version/reference contracts. Contract availability does not imply implementation completion.
- Scope boundary: Knowledge generation/publication and external Vault connection are excluded. No new mandatory workflow stage or automatic completion.
- User-specified terms and existing locale are preserved; Korean copy uses 사용자.
- Product Spirit: reduces cognitive load, preserves resumption context and portable evidence; does not score people or transfer decision/publication authority to AI.
- Numeric retrieval weights and timing optimizations are implementation-plan decisions; existing binding performance budgets remain applicable.
