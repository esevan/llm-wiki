# Feature Specification: Evidence-Grounded Work Distillation

**Feature Branch**: `017-worklog-decision-lineage`

**Created**: 2026-09-26

**Status**: Draft — design complete; implementation blocked on feature 015 foundation contracts

**Input**: Replace raw-heavy Task Work Log and chronological Task journey views with an evidence-grounded Distillation that preserves the original record, prioritizes final decisions, and never invents reasons or results.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the meaningful result of a Run (Priority: P1)

A user opens a Run-linked Work Log entry and sees a readable account of meaningful goals, scope, decisions, performed work, evidence, results, conditions, concrete values, failed approaches, and unresolved items. The original final report and raw execution evidence remain inspectable.

**Why this priority**: Work Log is the primary resumption surface. A shorter text that drops decision reasons or verification state would make the record less trustworthy, while exposing raw execution output by default keeps the current cognitive burden.

**Independent Test**: Supply a Run with a final report plus corroborating and contradictory completed evidence items. Confirm the default Work Log shows only supported distilled claims with claim-level sources and distinct tried, completed, and verified states, while the untouched original remains available.

**Acceptance Scenarios**:

1. **Given** a successful Run with a final report, **When** Distillation completes, **Then** the final report is the primary source and raw completed events are consulted only to corroborate important claims or add material missing or contradictory facts such as checks, paths, numbers, and failures.
2. **Given** a final report that states work and checks separately, **When** the result is displayed, **Then** attempted work, completed work, and verified work remain distinguishable and no normal exit is treated as proof of correctness.
3. **Given** a failed or interrupted Run without a final report, **When** Distillation completes, **Then** the result contains only facts supported by known saved evidence, identifies the missing report and unresolved outcome, and invents no rationale or result.
4. **Given** routine retry, continue, or control events that do not change the approach, **When** the result is generated, **Then** those events are omitted; an actual approach change remains visible with its evidence.
5. **Given** a claim in the Distillation, **When** the user opens its source, **Then** the application reveals the exact source record and revision that supported that claim.

---

### User Story 2 - Follow how decisions evolved (Priority: P1)

A user opens a Task journey and sees stable, topic-level nodes for meaningful goals, scope, criteria, decisions, evidence, results, and completion snapshots. The journey emphasizes the currently adopted decisions while keeping older choices connected to what replaced them and why.

**Why this priority**: A chronological last message is not a reliable final decision. Decision lineage must preserve change without forcing the user to reconstruct it from every message or event.

**Independent Test**: Build a Task history containing an AI suggestion, a user adoption, performed work, verification, a superseding decision, a withdrawn option, and an unresolved topic. Confirm stable nodes and links preserve every state and source without overwriting the earlier decision.

**Acceptance Scenarios**:

1. **Given** a new supported decision that changes an earlier choice, **When** the journey updates, **Then** a new node is created and linked as superseding the old node with a supported reason; the old node is not overwritten.
2. **Given** several statements about the same topic, **When** the journey is read, **Then** the topic explicitly identifies its current status as adopted, superseded, withdrawn, or unresolved rather than treating the chronologically last statement as final.
3. **Given** an AI suggestion followed by user acceptance and later verified work, **When** the journey is displayed, **Then** suggestion, user decision, performed work, and verification remain distinct facts.
4. **Given** Task completion, reopening, and later completion, **When** the journey updates, **Then** each completion is preserved as a snapshot with its exact evidence and current status.
5. **Given** the user opens a node, **When** its prepared detail appears, **Then** it includes applicable before, after, reason, evidence, result, current status, and links without starting a model call.

---

### User Story 3 - Update only what changed and recover safely (Priority: P2)

A user can continue editing and executing a Task while Distillation runs durably in the background. Edits, deletions, retries, and batches update only affected claims and nodes. Existing readable results remain available when a new job fails.

**Why this priority**: Durable incremental updates keep the view current without repeatedly regenerating the whole history or losing a known-good result after a transient provider failure.

**Independent Test**: Generate an initial Distillation, edit one source, delete another, replay the same change, fail and retry a batch, and change the Distillation rules. Confirm targeted subgraph updates for source changes, idempotent replay, retained last-good results, and full regeneration only for repair or rules changes.

**Acceptance Scenarios**:

1. **Given** an exact source revision changes or is deleted, **When** an update is scheduled, **Then** only claims and the Task subgraph derived from that source are reconsidered and unaffected stable node identifiers remain unchanged.
2. **Given** multiple related source changes, **When** they are queued, **Then** they may be processed as one durable batch without duplicate jobs or partially published results.
3. **Given** a retry or duplicate delivery, **When** processing completes, **Then** the same input and rules produce one current projection and do not duplicate nodes or links.
4. **Given** a failed, stale, or cancelled job, **When** the user returns, **Then** the last-good Distillation remains readable with a nonblocking freshness/error status and a retry action.
5. **Given** an explicit repair or Distillation-rules version change, **When** regeneration is requested, **Then** the whole affected projection may be rebuilt while preserving inspectable prior revisions and stable identities wherever their supported meaning is unchanged.

### Edge Cases

- The final report is empty, malformed, too large, or contradicts completed command/check/file evidence.
- A Run succeeds but contains no observable evidence beyond the model-authored report.
- A Run fails after changing files, or is interrupted before its outcome can be known.
- A source is edited while a batch for its prior revision is running.
- A source deletion removes the only evidence for one claim but also supports unrelated nodes.
- Several sources express the same decision with different wording or languages.
- A user decision later returns to an older option for a new reason.
- Evidence supports what changed but not why it changed.
- A path, number, command, or check result appears only in raw completed evidence.
- A retry repeats the same approach, while another retry materially changes it.
- A Task has only manual Work Log entries and no Runs.
- A Task is completed, reopened, and completed again with different evidence.
- A stale job finishes after a newer input or rules revision has become current.
- A legacy Solution lineage snapshot exists for the same historical material.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The default view of each Run-linked Work Log entry MUST show its current readable Distillation and MUST keep the original final report and retained execution evidence inspectable without modification.
- **FR-002**: Distillation MUST reorganize and reconcile recorded material into a more useful evidence-grounded account; it MUST NOT merely shorten the input, add new advice, or present an activity transcript.
- **FR-003**: The Run final report captured by the existing execution flow MUST be the primary Distillation input when present.
- **FR-004**: Raw Run events MUST be limited to completed retained evidence and used only to corroborate important report claims, identify contradictions, or supply material missing facts such as checks, failures, artifacts, paths, conditions, and numbers.
- **FR-005**: For a failed, cancelled, interrupted, or uncertain Run without a final report, Distillation MUST use only known saved evidence and MUST explicitly preserve unknown or unresolved outcomes.
- **FR-006**: Distillation MUST preserve supported goals, scope, criteria, decisions, decision reasons, work performed, failed approaches, conditions, concrete values, paths, results, verification, and unresolved items.
- **FR-007**: Distillation MUST distinguish proposed or attempted work, completed work, and independently observed verification. A provider exit, final report statement, or AI suggestion MUST NOT be upgraded to a verified result without corresponding evidence.
- **FR-008**: Each material claim MUST carry one or more source references that identify the exact source type, stable source ID, revision, and bounded locator or evidence item.
- **FR-009**: Unsupported reasons, causal links, outcomes, and completion claims MUST be omitted or explicitly marked unresolved. An evidence-backed AI interpretation MAY be shown only as a suggestion with exact sources and MUST NOT appear as a proven reason, user decision, or verified relationship.
- **FR-010**: Routine retry, continue, reconnect, approval, and control events MUST be omitted unless they record a material approach, decision, evidence, result, or status change.
- **FR-011**: The same structured Distillation result MUST drive the readable Work Log projection and incremental active Task journey projection; legacy Solution lineage MUST NOT be extended or made authoritative.
- **FR-012**: The Task journey MUST contain only meaningful goal, scope, criterion, decision, evidence, result, and completion-snapshot nodes, plus evidence-backed relationships between them.
- **FR-013**: Journey updates MUST classify each candidate against the current projection as add, enrich, merge, omit, or supersede. A materially new decision MUST create a new stable node rather than overwrite an old node.
- **FR-014**: Nodes MUST have stable identifiers across retries, batches, locale changes, and unrelated source edits. Merge or supersede operations MUST preserve redirects or links from prior identifiers.
- **FR-015**: Each decision topic MUST explicitly identify its current state as adopted, superseded, withdrawn, or unresolved. Chronological recency alone MUST NOT select the current decision.
- **FR-016**: Superseded or withdrawn choices MUST remain inspectable and link to the current choice when one exists, including the supported reason for the change or an explicit unknown reason.
- **FR-017**: AI suggestions, user decisions, performed work, observed evidence, and verified results MUST remain separate provenance and epistemic states throughout storage, contracts, and UI.
- **FR-018**: Task completion MUST create an immutable completion snapshot rather than converting the latest journey event into a final conclusion. Reopen and subsequent completion MUST preserve earlier snapshots.
- **FR-019**: Node detail MUST be prepared during generation and contain applicable before, after, reason, evidence, result, current status, and links. Opening detail MUST NOT invoke a model.
- **FR-020**: Every accepted Distillation result MUST be bound to its immutable prompt identity, prompt version, result schema version, rules version, locale, exact input revisions, and source-set hash.
- **FR-021**: Source edits and deletions MUST invalidate only claims and downstream relationships that depend on the affected exact source revisions, then update only the relevant Task subgraph.
- **FR-022**: Full-projection regeneration MUST occur only for explicit repair, incompatible result/schema migration, or Distillation-rules changes; ordinary source updates MUST use targeted incremental generation.
- **FR-023**: Related changes MAY be coalesced into a durable batch. Enqueue, lease, retry, replay, cancellation, stale completion, and atomic publication MUST follow the shared background-job foundation and preserve the last-good projection on failure.
- **FR-024**: Duplicate scheduling or delivery of the same projection owner, exact inputs, and rules MUST resolve idempotently to one current result.
- **FR-025**: A newly completed result MUST publish only when all exact inputs and the expected prior projection revision still match. A stale result MUST remain diagnostic and MUST NOT replace the current projection.
- **FR-026**: The system MUST expose pending, current, stale, retryable-failure, and repair-required freshness states without blocking Task notes, execution, completion, or other ordinary Task work.
- **FR-027**: The default Work Log UI MUST prioritize the readable Distillation, current result, unresolved items, and source access; raw/original material MUST remain secondary but discoverable.
- **FR-028**: The journey UI MUST prioritize current adopted decisions and unresolved topics, while letting users follow supersession and evidence links to historical nodes and exact sources.
- **FR-029**: Work Log and journey detail MUST support keyboard use, visible focus, non-color status labels, narrow windows, long paths and values, English and Korean text, and screen-reader names that distinguish provenance and status.
- **FR-030**: Distillation MUST remain private local process, MUST NOT complete or reopen a Task, resolve a Problem, publish Knowledge, or apply a user decision, and MUST NOT overwrite manual Work Log bodies, attachments, comments, decisions, or source records.
- **FR-031**: Existing Run final reports, execution evidence, Work Log records, Task journey events, and legacy lineage snapshots MUST remain intact and inspectable through any later migration.
- **FR-032**: The feature MUST use the shared prompt registry, exact-revision reference contract, result versioning, and durable job protocol supplied by feature 015; this feature MUST NOT introduce an independent shared job or prompt schema.
- **FR-033**: Implementation MUST wait until feature 015 foundation contracts are integrated and verified. Any contract mismatch MUST be resolved in the plan and contract artifacts before a schema migration is authored.
- **FR-034**: Focused automated verification MUST cover source selection, claim citations, epistemic distinctions, incremental invalidation, stable IDs, stale-result rejection, idempotent retry, and original-record preservation.
- **FR-035**: Performance verification MUST demonstrate that normal Task and Work Log reads remain within the existing 100 ms p95 projection budget after local data is available, and that opening prepared node detail performs no model request.

### Key Entities

- **Distillation Input Set**: Immutable exact references to one Run report and completed evidence or to Task sources, plus prompt, rules, locale, and expected projection revisions.
- **Distillation Result**: Versioned structured output shared by Work Log and Task journey projections.
- **Distilled Claim**: A bounded statement with kind, epistemic state, actor/provenance, status, exact source references, and optional topic/node identity.
- **Journey Node**: A stable Task-owned goal, scope, criterion, decision, evidence, result, or completion snapshot with prepared detail.
- **Journey Relationship**: Evidence-backed derived-from, supports, verifies, contradicts, merges, or supersedes link whose own sources are recorded.
- **Decision Topic**: A stable grouping that identifies adopted, superseded, withdrawn, and unresolved choices independently of chronology.
- **Projection Revision**: An atomically published Work Log or Task journey result bound to exact inputs and rules, retaining prior revisions for inspection and repair.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a fixture set covering success, failure without report, interruption, contradiction, and retry, 100% of displayed material claims cite at least one supplied exact source revision and 0 unsupported reasons or results are introduced.
- **SC-002**: For every fixture, attempted, completed, and verified work remain correctly distinguishable, including Runs that exit normally without check evidence.
- **SC-003**: Routine control-only events produce 0 journey nodes, while every fixture with a material approach change retains that change and its evidence.
- **SC-004**: A one-source edit or deletion leaves 100% of unrelated node IDs and content unchanged and recomputes only the affected claims and downstream links.
- **SC-005**: Replaying the same batch 100 times yields one current projection revision with no duplicate claims, nodes, or relationships.
- **SC-006**: Stale and failed jobs replace 0 last-good projections; a retry can complete without rerunning the originating Codex Run.
- **SC-007**: Every node detail opens from saved data with no model request and exposes all applicable before, after, reason, evidence, result, status, and links.
- **SC-008**: In user acceptance fixtures, the current decision for every topic follows explicit adoption/supersession/withdrawal evidence rather than the last chronological message.
- **SC-009**: Existing original reports, retained evidence, manual Work Log content, comments, attachments, Task decisions, and completion records remain byte-for-byte unchanged in preservation checks.
- **SC-010**: Normal Task and Work Log local projections remain within 100 ms p95 after data arrives with Distillation detail collapsed.

## Assumptions

- Feature 015 owns shared prompt, immutable reference, result-version, and background-job contracts; feature 017 consumes them after integration.
- Feature 014's `capture_final_report` value and completed Run evidence are the authoritative Run inputs available today.
- The active Task journey is the Task-owned projection built from recorded Task sources; legacy Solution lineage is historical only.
- A deterministic local pre-pass selects bounded candidate sources and preserves exact references before any model call.
- Original records and prior projection revisions remain local and inspectable; only a user-approved completed result can later become Knowledge.
- Standard checklist depth and reviewer/PR audience apply to this specification package.

## Product Spirit Alignment

This feature serves **Reduce Cognitive Load** by making supported meaning readable before raw detail, and **Resume Where You Left Off** by retaining decision reasons, failed approaches, exact evidence, conditions, and unresolved work. It keeps **Tasks Carry Work; Problems Carry Context** by updating the active Task Work Log and Task journey rather than legacy Solution lineage. It preserves human authority, private process, portable Knowledge boundaries, and every original source; it adds no score, automatic decision, workflow gate, Task transition, Problem resolution, or Knowledge publication.

## Out of Scope

- Changing how Codex produces or captures the Run final report.
- Distilling arbitrary chat, Capture, or legacy Solution histories in this work package.
- Replacing source records, deleting raw evidence, or rewriting manual Work Log content.
- Automatically choosing, applying, or reversing a user decision.
- Automatically completing a Task, resolving a Problem, or publishing Knowledge.
- Whole-history regeneration after every source change.
- A model call launched by opening a Work Log section or journey node.
