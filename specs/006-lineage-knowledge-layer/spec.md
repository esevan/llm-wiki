# Feature Specification: Lineage Knowledge Layer

**Feature Branch**: `feature/lineage-knowledge-layer`

**Created**: 2026-08-21

**Status**: Implemented

**Input**: Improve the Solution Lineage tab so every completed Solution preserves a traceable Capture → Problem → Solution → Complete knowledge layer, including decisions, conflicts, evidence, and correction history.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand completed work from origin to evidence (Priority: P1)

As a user reviewing a completed Solution, I can see how the original Capture became a Problem, how the Problem became the final Solution, and why the work was considered complete without reconstructing the history from separate records.

**Why this priority**: The central value is durable, comprehensible Knowledge. Without the complete lineage, the final document remains a summary rather than a trustworthy account of the work.

**Independent Test**: Complete a Solution with linked Capture, Problem, Solution, and completion records, open its final document, and verify that the four-stage lineage and transition context are present automatically.

**Acceptance Scenarios**:

1. **Given** a Solution with linked source and completion records, **When** a human completes it, **Then** its final document automatically contains Capture → Problem → Solution → Complete lineage without an additional publication approval.
2. **Given** a completed Solution, **When** the user reads its Lineage tab, **Then** each stage distinguishes source-backed facts, human decisions, and AI interpretations.
3. **Given** a transition with no recorded rationale, **When** lineage is generated, **Then** it says `Not explicitly recorded` rather than inventing a reason.
4. **Given** a completed Solution with a ready Lineage snapshot, **When** its final document narrative is generated, **Then** the narrative is based on that Lineage and its referenced evidence rather than an unstructured slice of Raw Data.

---

### User Story 2 - Trace decisions and conflicts to evidence (Priority: P1)

As a user auditing a completed Solution, I can trace major decisions and conflict addresses back to the original records that support them and tell whether an address was explicit, evidenced by implementation, or inferred by AI.

**Why this priority**: Traceability is what prevents an AI-generated narrative from being mistaken for fact or human intent.

**Independent Test**: Complete a Solution containing an explicitly addressed conflict, an implementation-supported decision, and an inferred rationale, then verify their labels, address states, and source links independently.

**Acceptance Scenarios**:

1. **Given** a conflict that affected a Problem → Solution transition, **When** the lineage is displayed, **Then** the conflict appears as transition context rather than a separate workflow stage.
2. **Given** a conflict address with source evidence, **When** the user opens its detail, **Then** the original requirement disposition is shown as Preserved, Modified, Superseded, or Rejected and the supporting record is reachable.
3. **Given** a conflict without sufficient address evidence, **When** the final document is generated, **Then** its status remains Detected, Unaddressed, or Unclear and is never reported as Addressed or Resolved.
4. **Given** an AI-inferred relationship, **When** it is shown, **Then** it is labeled as an AI interpretation with a confidence level and remains distinguishable from Observed and Decided content.

---

### User Story 3 - Navigate from lineage to the Problem (Priority: P2)

As a user reading the lineage graph, I can select the Problem stage and open the corresponding Problem so I can inspect the current record and its related work.

**Why this priority**: The graph must be an entry point into the underlying knowledge, not a dead-end diagram.

**Independent Test**: Open a completed Solution's Lineage tab, select its Problem card, and verify that the existing Problem detail experience opens for the correct record.

**Acceptance Scenarios**:

1. **Given** a lineage whose Problem still exists, **When** the user selects the Problem card, **Then** the corresponding Problem opens through the existing Problem detail experience.
2. **Given** a lineage whose live Problem record is unavailable, **When** the user selects the Problem card, **Then** the preserved snapshot remains readable and the interface explains that the live record cannot be opened.

---

### User Story 4 - Correct knowledge without erasing history (Priority: P2)

As a user, I can correct an AI interpretation in Lineage Knowledge so the final Knowledge reflects the correction while retaining the original source and previous AI interpretation for audit.

**Why this priority**: Automatic generation is useful only if mistaken interpretations can be corrected without rewriting history.

**Independent Test**: Correct an inferred decision rationale, reopen the completed Solution, and verify that the correction is current while the prior interpretation and source evidence remain available in audit history.

**Acceptance Scenarios**:

1. **Given** an AI-inferred lineage statement, **When** the user submits a correction, **Then** the corrected statement becomes the current Knowledge interpretation.
2. **Given** a corrected statement, **When** audit context is opened, **Then** the prior AI interpretation, correction timestamp, and source references remain available.
3. **Given** a correction to narrative interpretation, **When** it is saved, **Then** the original Capture, decisions, conflicts, and completion evidence are unchanged.

### Edge Cases

- A completed Solution has no surviving source Capture but does have a Problem and completion evidence.
- A Problem has multiple Capture ancestors; only directly linked, source-bearing ancestors appear in the primary lineage while additional sources remain available in evidence detail.
- Multiple conflicts affected one transition; the primary graph shows only the most consequential unresolved or outcome-changing state and exposes the others under Decision Changes or conflict detail.
- A conflict was detected after an earlier draft but has no explicit address record.
- A decision is supported only by implementation or verification evidence and not by an explicit user statement.
- A source record changes or becomes unavailable after the final document is generated.
- AI generation fails during completion; the completion decision remains human-controlled and lineage generation can be retried without losing source records.
- A correction disagrees with an earlier human decision; the system records the correction as a new knowledge interpretation and does not silently rewrite the decision record.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST automatically create and preserve Lineage Knowledge for every Solution at completion time without requiring a separate lineage publication approval.
- **FR-002**: The final completed-Solution document MUST provide Detail, Lineage, Decision Changes, Conflicts & Addresses, and Completion Evidence as distinct readable sections.
- **FR-003**: The primary lineage MUST display Capture → Problem → Solution → Complete as four stages connected by transition decisions.
- **FR-004**: The Capture stage MUST preserve the earliest directly linked user feedback as close to the original wording as the retained source allows.
- **FR-005**: The Problem stage MUST show the refined user problem and desired outcome.
- **FR-006**: The Solution stage MUST show the final direction and the material decision changes made within the Solution.
- **FR-007**: The Complete stage MUST show the evidence used for completion and the human completion review or decision.
- **FR-008**: Each stage, transition, decision change, conflict, conflict address, and completion claim MUST link to one or more retained source records or explicitly state that supporting evidence was not recorded.
- **FR-009**: Lineage claims MUST be classified as Observed, Decided, or Inferred.
- **FR-010**: Inferred claims MUST be labeled as AI interpretation and include a confidence level; they MUST NOT be presented as observed fact or human intent.
- **FR-011**: The system MUST apply the rule `No evidence → no assertion`; absent rationale MUST appear as `Not explicitly recorded`, and plausible interpretation MUST be explicitly labeled as AI-inferred.
- **FR-012**: Conflict MUST be represented as context affecting a transition or decision, not as a peer stage in the four-stage lineage.
- **FR-013**: Conflict status MUST distinguish Detected, Addressed, Unaddressed, and Unclear.
- **FR-014**: Conflict address basis MUST distinguish Explicit decision, Implementation evidence, and AI inferred.
- **FR-015**: The system MUST NOT mark a conflict Addressed or Resolved without an explicit decision or implementation evidence that supports that status.
- **FR-016**: Each conflict address MUST record the original requirement disposition as Preserved, Modified, Superseded, or Rejected.
- **FR-017**: The primary graph MUST show only conflicts material to the final Solution; additional conflicts MUST remain available in Decision Changes or Conflicts & Addresses.
- **FR-018**: Users MUST be able to select the Problem stage and open the corresponding Problem through the existing Problem detail experience.
- **FR-019**: Users MUST be able to inspect the source record behind any major decision, conflict address, or completion claim without leaving the completed-Solution experience unnecessarily.
- **FR-019a**: Capture, Problem, and Solution stages MUST open their corresponding read-only records; stage timestamps MUST use the viewer's system locale.
- **FR-019b**: Evidence links MUST use stable, Lineage-wide Reference numbers and open contextual detail beside the citation rather than in a shared panel at the end of Lineage.
- **FR-019c**: A transition without explicit rationale MUST show a deterministic summary of the recorded change and MUST NOT present the absence as an empty decision reason.
- **FR-020**: Users MUST be able to correct generated lineage interpretations after automatic generation without altering immutable source records.
- **FR-021**: A correction MUST become the current Knowledge interpretation while retaining the prior AI interpretation, correction metadata, and evidence links as audit context.
- **FR-022**: Regenerating or correcting Lineage MUST NOT delete or overwrite original Capture, Problem, decision, conflict, or completion evidence records.
- **FR-023**: Lineage generation failure MUST NOT fabricate a partial successful lineage or silently block the human completion decision; the final document MUST expose the missing lineage state and allow a safe retry.
- **FR-024**: This feature MUST NOT modify the completed-work Detail modal.
- **FR-025**: The system MUST preserve private exploration and intermediate chat as private process unless a specific excerpt is already retained as an approved decision or evidence record.
- **FR-026**: The system MUST create or select the current Lineage snapshot before generating the final document narrative.
- **FR-027**: AI-generated executive summary and report narrative MUST use the current Lineage projection and only its referenced evidence excerpts as input; unstructured Raw Data MUST NOT be the primary report-generation input.
- **FR-028**: Final document narrative MUST NOT become evidence for the Lineage snapshot from which it was generated, preventing circular provenance.
- **FR-029**: Lineage and the completed-work document MUST share one lifecycle: completion and document regeneration rebuild current Lineage before rebuilding the report, without presenting regeneration as document Version Control.
- **FR-030**: Internal snapshot, claim, evidence, revision, workflow-record, and source UUIDs MUST remain internal. Final Knowledge and its report-generation context MUST cite stable human-readable evidence labels such as `Original capture`, `Work log 1`, `Validation criterion 2`, and `Completion decision`; opaque IDs remain permitted only in the private inference-validation contract.

### Key Entities

- **Lineage Snapshot**: The immutable, completion-time account tying a completed Solution to its Capture, Problem, Solution, Complete stages, transition decisions, generation metadata, and current interpretation revision.
- **Lineage Stage**: One of Capture, Problem, Solution, or Complete, containing a source-backed snapshot and links to retained records.
- **Transition Decision**: The recorded basis for moving between adjacent stages, including rationale classification, evidence links, and material conflict context.
- **Decision Change**: A material change in Solution direction, with before/after meaning, reason or explicit absence of reason, impact, and supporting records.
- **Conflict Event**: A conflict that affected a transition or decision, with lifecycle status and links to evidence.
- **Conflict Address**: The treatment of an original requirement, including address basis and Preserved, Modified, Superseded, or Rejected disposition.
- **Evidence Reference**: A stable reference to an original Capture, user decision, workflow event, Solution record, implementation record, verification result, or completion review.
- **Interpretation Revision**: An AI-generated interpretation or user correction, including provenance, confidence where applicable, timestamp, current status, and relationship to the prior revision.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of newly completed Solutions produce a final document containing all five required sections and a four-stage lineage, or an explicit retryable lineage-generation failure state.
- **SC-002**: 100% of major decisions, conflict addresses, and completion claims either reach a retained source record in at most two user interactions or display `Not explicitly recorded`/AI-inference labeling.
- **SC-003**: 0 conflicts without explicit-decision or implementation evidence are labeled Addressed or Resolved in acceptance testing.
- **SC-004**: 100% of AI-inferred claims are visibly distinguishable from Observed and Decided claims and include a confidence level.
- **SC-005**: In usability validation, users can open the correct Problem from the lineage graph on the first attempt in at least 95% of trials.
- **SC-006**: 100% of lineage corrections preserve the prior interpretation and all referenced source evidence while making the correction the current Knowledge view.
- **SC-007**: For completed Solutions with up to 20 decisions and conflicts, users can identify the four lifecycle stages and the conflicts material to the final direction without horizontal page scrolling at supported viewport sizes.
- **SC-008**: 100% of generated final-document narratives use the current Lineage created in the same completion or regeneration operation and contain no factual claim outside that Lineage or its referenced evidence.

## Assumptions

- The primary lineage includes the directly linked Capture ancestry for the completed Solution; secondary or indirect source records are available through evidence detail rather than additional graph branches in the first version.
- Selecting the Problem stage reuses the existing Problem detail experience instead of introducing a new Problem screen.
- Source drill-down opens a contextual evidence panel within the completed-Solution experience, with navigation to an existing live record when one is available.
- AI confidence is expressed as High, Medium, or Low together with an `AI inferred` label; confidence never upgrades an inference to a decision or observation.
- The existing completion decision remains exclusively human-controlled. Automatic lineage generation publishes only as part of already human-approved completed Knowledge.
- Existing completed Solutions are outside the automatic backfill requirement; regeneration may add lineage when sufficient retained source evidence exists.
