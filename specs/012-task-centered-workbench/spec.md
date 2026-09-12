# Feature Specification: Task-Centered Workbench

**Feature Branch**: `feat/task-centered-workbench`

**Created**: 2026-09-05

**Status**: Ready for implementation

**Input**: Replace the required Problem-to-Solution approval flow with a Task-centered Workbench
where Capture and Task are canonical records, refinement and conflict review are optional, existing
records migrate without loss, and completion, Problem resolution, and Knowledge publication remain
separate user decisions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Capture a Thought or Register a Task (Priority: P1)

A user enters natural text and explicitly saves it as a lightweight Capture or as a Task ready for
work. The Workbench then shows active Task shortcuts, refining shortcuts, and the existing category
groups containing canonical Capture and Task cards.

**Why this priority**: This removes the compulsory workflow funnel and establishes the new daily entry
point without redesigning the user's categories.

**Independent Test**: Create one item in each input mode, restart the app, and verify the Capture and
Task remain canonical category items while the Task can appear as a shortcut without duplication.

**Acceptance Scenarios**:

1. **Given** an empty input using the default mode, **When** the user enters text and chooses `생각 남기기`, **Then** one canonical Capture is saved and no Task is created.
2. **Given** text and `Task로 등록`, **When** save succeeds, **Then** one Task and its hidden input provenance are committed atomically, with no duplicate Capture card.
3. **Given** a save failure, **When** either mode is submitted, **Then** the original input and selected mode remain available for retry.
4. **Given** active and refining items, **When** Workbench loads, **Then** active shortcuts appear first, refining shortcuts second, and the unchanged category groups last.

---

### User Story 2 - Work and Resume Independently (Priority: P1)

A user can write Work Log evidence on a Task immediately, start it, complete it, reopen it, and resume
from recent user activity without first creating a Problem or waiting for AI.

**Why this priority**: Task must be a useful independent work unit, and existing Work Log evidence is
the most valuable context to preserve.

**Independent Test**: Create a Task without a Problem, add text, image, comment, checklist, and a
decision, move through every state, restart, then reopen and verify every record and timestamp.

**Acceptance Scenarios**:

1. **Given** a new Task, **When** the user adds Work Log evidence before starting, **Then** all evidence types persist and remain editable according to Task state.
2. **Given** a Task in `task`, **When** the user starts it despite missing readiness fields or unresolved prerequisites, **Then** it becomes `in_progress` and records any explicit continue decision.
3. **Given** an `in_progress` Task, **When** the user completes it with evidence, **Then** only that Task closes and its completion is append-only.
4. **Given** a completed Task, **When** the user chooses reopen or follow-up, **Then** the action is explicit and preserves the completed evidence.

---

### User Story 3 - Refine Without Blocking Work (Priority: P2)

A user can refine a Capture or Task through an autosaved conversation, receive multiple Task and
Problem proposals, and apply, edit, or reject each proposal independently.
When a Capture already states a Solution, refinement treats that Solution as the initial Task draft
and asks only for missing execution details instead of making the user rediscover it.

**Why this priority**: Conversation can organize ambiguous work while the user retains control over
which durable records exist.

**Independent Test**: Start refinement from both entity types, interrupt and resume the session, apply
selected proposals, and verify exact revisions and atomic Task/Problem links.

**Acceptance Scenarios**:

1. **Given** a Capture that already contains a concrete Solution, **When** refinement opens, **Then** that text is preserved as the starting Task draft and prompts request only missing details.
2. **Given** a Capture or Task, **When** refinement is interrupted after typing or receiving messages, **Then** messages, input, draft, active tab, and view position resume after restart.
3. **Given** several proposals, **When** the user accepts one and rejects another, **Then** only the accepted operations become durable and every decision remains recorded.
4. **Given** an internal Problem snapshot and Task proposal, **When** the user applies the coherent result, **Then** selected exact revisions and links commit atomically without a separate Problem approval gate.
5. **Given** no provider response, **When** the user returns to the Task, **Then** ordinary Task work remains available.

---

### User Story 4 - Connect Tasks and Problems (Priority: P2)

A user can link Tasks to independently revisioned Problems, relate Tasks to each other, and inspect
readiness field by field without receiving a worker score or a start permission.

**Why this priority**: Rich context and decomposition remain available without restoring a rigid parent
workflow.

**Independent Test**: Link multiple Tasks and Problems, revise a Problem, add every relationship type,
exercise cycle rejection, and inspect readiness evidence and `not_applicable` decisions.

**Acceptance Scenarios**:

1. **Given** existing Tasks and Problems, **When** links are created, **Then** each link retains the exact Problem revision and both sides remain independently navigable.
2. **Given** Task relationships, **When** the user creates `prerequisite`, `split_from`, or `related`, **Then** inverse navigation works and self-links, duplicates, and prerequisite cycles are rejected.
3. **Given** incomplete Task fields, **When** readiness is shown, **Then** each applicable field is `resolved`, `missing`, or `not_applicable` with reason and evidence, and no aggregate score appears.

---

### User Story 5 - Review Conflicts Asynchronously (Priority: P2)

A user can run Conflict Review on a stable Capture refinement draft before any Task exists or on an
exact Task revision, continue working while it runs, and distinguish current evidence from stale or
failed attempts.

**Why this priority**: Conflict evidence remains valuable only if it is accurate and never blocks the
work it is meant to help.

**Independent Test**: Review a Capture draft and Task revision, mutate material and non-material
fields while jobs run, inject cancellation/provider/parse/evidence failures, and validate status,
latency, citations, and continued Task access.

**Acceptance Scenarios**:

1. **Given** a stable Capture draft, **When** review starts before proposal application, **Then** it runs against exact draft identity without persisting a Task.
2. **Given** a running review, **When** material content changes, **Then** the old run is cancelled or marked stale and can never become current.
3. **Given** a failed or insufficient attempt after a current result, **When** status is displayed, **Then** the current result remains visible and the failed attempt is shown separately.
4. **Given** findings, **When** the user starts or completes the Task, **Then** the action remains available with a nonblocking warning and optional explicit continue decision.

---

### User Story 6 - Preserve Existing Work Through Migration (Priority: P1)

An existing user upgrades once and finds every Capture, Problem, Solution, Work Log record, completion,
conflict decision, and lineage record represented truthfully in the Task-centered model.

**Why this priority**: Data loss or silent semantic change would make the release unacceptable.

**Independent Test**: Run migration and restore scenarios over small, large, interrupted, corrupt,
duplicate-ID, invalid-state, and low-disk fixtures and compare identities, counts, canonical hashes,
attachment bytes, relationships, and completed evidence.

**Acceptance Scenarios**:

1. **Given** a valid legacy database, **When** upgrade runs, **Then** a verified backup precedes migration and every mapped record passes post-migration checks before the schema version changes.
2. **Given** orphan Capture or Problem-only history, **When** migration completes, **Then** Capture remains canonical and Problem-only history remains discoverable as a refinement item without an invented Task.
3. **Given** any failed validation, **When** migration aborts, **Then** changes roll back and the user can retry or explicitly restore the verified backup.

---

### User Story 7 - Complete, Resolve, and Publish Deliberately (Priority: P3)

A user treats Task completion, Problem resolution, and Knowledge publication as three explicit,
traceable decisions and can inspect the full lineage of the published document.

**Why this priority**: This protects private process and prevents a completed action from claiming that
the broader Problem is solved or that a draft is ready for publication.

**Independent Test**: Complete one Task, leave its Problem open, generate and revise a Knowledge draft,
publish an exact revision, and verify graph lineage and external-change protection.

**Acceptance Scenarios**:

1. **Given** a Task linked to a Problem, **When** the Task completes, **Then** the Problem remains unchanged until a separate resolution decision.
2. **Given** a completed Task, **When** a Knowledge draft is generated, **Then** nothing is published until the user approves the exact draft revision and hash.
3. **Given** a published document, **When** lineage is opened, **Then** Capture, exact Task and Problem revisions, Work Log, decisions, completion, and Knowledge revision are inspectable.

---

### User Story 8 - Continue the Same Work Through MCP (Priority: P3)

A user sees the same Task identities, revisions, decisions, and evidence in desktop and connected chat,
with exact approval boundaries for durable changes.

**Why this priority**: Two surfaces must not create competing workflow truth.

**Independent Test**: Create and revise Tasks through each surface, resume the same session in the
other, exercise stale approval and scope denial, and verify projection recovery and publication rules.

**Acceptance Scenarios**:

1. **Given** a desktop Task, **When** chat reads the Workbench, **Then** it returns the same identity, revision, state, user activity, links, and relationships.
2. **Given** a chat proposal, **When** its target revision changed before approval, **Then** application is rejected with current conflict fields and no partial mutation.
3. **Given** projection delay or retry, **When** events catch up, **Then** replay is idempotent and background activity does not reorder user recency.

### Edge Cases

- Two windows apply changes to the same Task or refinement proposal revision.
- A direct Task transaction creates provenance but fails before Task insertion.
- A Problem advances after a Task linked an older revision.
- A Task is split repeatedly or a prerequisite would close a longer cycle.
- Background AI finishes after the subject was revised, deleted, completed, or reopened.
- Category overrides exist for legacy entity types or localized category labels.
- A legacy attachment has empty metadata, large base64 content, or byte content that cannot be decoded.
- Migration is rerun after rollback or after backup creation but before schema commit.
- Knowledge changed outside the app after draft generation.
- Narrow windows, long Korean and English text, reduced motion, keyboard-only use, empty/error/loading states.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST offer explicit Capture and Task input modes, default to Capture, and preserve input and mode on failure.
- **FR-002**: Capture input MUST create one canonical Capture; direct Task input MUST atomically create one Task plus non-card input provenance.
- **FR-003**: Workbench MUST order active Task shortcuts, refining shortcuts, then existing category groups without changing category semantics.
- **FR-004**: Shortcuts MUST reference canonical identities, keep items in category groups, and exclude an active Task from the refining shortcut set.
- **FR-005**: Shortcut and recent ordering MUST use allowlisted user activity only; background jobs MUST NOT update it.
- **FR-006**: Task MUST support `task`, `in_progress`, and `completed` with explicit reopen and optimistic revision conflict handling.
- **FR-007**: Task MUST own text/image/file Work Log entries, comments, checklists, attachments, and append-only decisions from creation onward.
- **FR-008**: Completed evidence MUST remain inspectable; follow-up and reopen MUST preserve original completion records.
- **FR-009**: Problem MUST be independently revisioned and MUST have a lifecycle independent from Task.
- **FR-010**: Task and Problem MUST support many-to-many links bound to an exact Problem revision.
- **FR-011**: Task relationships MUST support prerequisite, split, and symmetric related navigation and reject self-links, duplicates, and prerequisite cycles.
- **FR-012**: Readiness MUST report applicable fields individually with status, reason, evidence, provenance, and source revision, without a score or workflow gate.
- **FR-013**: Users MUST be able to record revision-bound `not_applicable` readiness decisions with a reason.
- **FR-014**: A refinement session MUST belong to exactly one Capture or Task and preserve messages, drafts, input, selected view, scroll anchor, and decisions.
- **FR-015**: Refinement MUST support independently actionable Task patches, new Tasks, Problem snapshots, and Task-Problem links.
- **FR-016**: Applying a proposal MUST require exact draft revision and operation identity and MUST atomically persist every selected Task, Problem revision, link, and decision.
- **FR-017**: Provider absence or refinement failure MUST NOT block Task work or discard locally authored content.
- **FR-018**: Conflict Review MUST accept either an exact persisted Task revision or an exact material Capture refinement draft without requiring a Task.
- **FR-019**: Review identity MUST include normalized material content, Vault revision, and evidence scope/grant revision; category, panel position, and background timestamps MUST be non-material.
- **FR-020**: Review MUST expose queued, running, clear, findings, insufficient-evidence, failed, cancelled, and stale states.
- **FR-021**: Clear MUST require a successful scoped search, citations, and an explicit clear result; failure, cancellation, insufficient evidence, parsing error, and stale output MUST never become clear.
- **FR-022**: New material revisions MUST cancel queued reviews, request cancellation for running reviews, and reject late results as current.
- **FR-023**: Failed new attempts MUST preserve the last current result and appear as separate attempts.
- **FR-024**: Findings MUST expose source identity and excerpt and MUST remain immutable; resolution MUST be a Task revision or separate decision.
- **FR-025**: Review MUST remain nonblocking for Task creation, Work Log, start, completion, and refinement.
- **FR-026**: Task completion MUST close only its Task; Problem resolution MUST be a separate exact-revision user decision.
- **FR-027**: Knowledge draft generation, user correction, approval, publication, regeneration, and withdrawal MUST preserve exact source and publication revisions.
- **FR-028**: Knowledge publication MUST require explicit approval of the exact draft revision and hash and protect external changes with reversible writes.
- **FR-029**: Lineage MUST represent Capture, Task revision, Problem revision, Work Log, decision, completion, and Knowledge revision as a graph with typed edges.
- **FR-030**: Desktop, native routes, MCP tools, resources, and projections MUST share one Task aggregate contract and exact revision semantics.
- **FR-031**: Breaking legacy routes and Problem/Solution approval gates MUST be removed at one release boundary without a compatibility shim.
- **FR-032**: Upgrade MUST create and independently validate a consistent backup before changing durable records.
- **FR-033**: Migration MUST preserve legacy IDs, Capture records, Solution-to-Task state, exact Problem links, Work Log text and bytes, comments, checklists, decisions, completion, conflict, localization, category/priority overrides, work tracking, and lineage.
- **FR-034**: Problem-only records MUST remain discoverable as migration refinement items; migration MUST NOT invent Tasks for them.
- **FR-035**: Migration MUST validate per-table counts, identifier sets, canonical field and attachment hashes, foreign keys, and per-completed-Task evidence counts before schema commit.
- **FR-036**: Migration failure MUST roll back, identify the safe failure stage, retain a recovery copy, and offer retry or explicit verified-backup restore without automatic deletion.
- **FR-037**: User-facing Task-centered behavior and terms MUST remain aligned in English and Korean documentation and interface copy.
- **FR-038**: Automated tests MUST cover every major acceptance journey, migration fault, exact-revision conflict, background recency rule, narrow/wide keyboard UI, and final packaged desktop behavior.
- **FR-039**: Final validation MUST reuse one release artifact for packaged E2E and real-provider app review, and MUST report provider unavailability instead of fabricating AI quality results.
- **FR-040**: Real-provider review MUST use a fixed rubric and exact sample counts for document quality plus conflict latency and evidence accuracy.

### Key Entities

- **Capture**: Canonical uncommitted thought with original text, category, importance, provenance, and user activity.
- **Task / Task Revision**: Independent execution identity plus immutable versions of title, detail, outcome, scope, non-goals, and validation criteria.
- **Problem / Problem Revision / Task-Problem Link**: Independent context history and exact many-to-many linkage.
- **Task Relationship**: Historical prerequisite, split, or related edge between Task identities.
- **Work Log / Completion / Decision**: Task-owned evidence and append-only user decisions.
- **Refinement Session / Draft / Proposal Decision**: Resumable conversation and exact proposal application boundary for one Capture or Task.
- **Readiness Field Decision**: Revision-bound field status, reason, evidence, and provenance.
- **Conflict Review Run / Finding / Resolution**: Exact-input asynchronous attempt, cited result, and separate user disposition.
- **User Activity Event**: Allowlisted action used for recency independent of background processing.
- **Lineage Snapshot / Knowledge Revision**: Immutable provenance graph and separately approved portable publication.
- **Migration Backup Manifest**: Verified source database, hashes, versions, and restore metadata.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of input-mode, lifecycle, Work Log, refinement, relationship, review, migration, completion, publication, MCP, and accessibility acceptance journeys pass in automated or fixed-rubric final validation.
- **SC-002**: Capture and direct Task saves complete under 50 ms p95 on the reference fixture, and Workbench local projection renders within 100 ms p95 after data arrives.
- **SC-003**: Users can create, start, log work on, complete, and reopen a Task with no Problem, AI response, readiness clearance, or conflict clearance.
- **SC-004**: Every migrated fixture preserves 100% of required identifiers, canonical hashes, attachment bytes, links, and evidence counts; every injected failure leaves the source restorable.
- **SC-005**: 100% of late, stale, cancelled, failed, insufficient-evidence, or malformed review results are prevented from appearing current or clear.
- **SC-006**: Background-only processing changes the order of zero active or refining shortcuts.
- **SC-007**: All keyboard-only journeys complete without focus loss, and every state remains understandable without color in wide and narrow layouts in both supported languages.
- **SC-008**: In the fixed real-provider corpus, every reported conflict claim has an inspectable citation; median and p95 end-to-end latency plus rubric accuracy are recorded without substituting deterministic fallback output.
- **SC-009**: Every published Knowledge document is traceable to the exact Task revision, completion, Problem revisions, evidence snapshot, approved draft revision, and hash.

## Assumptions

- Version one remains single-user, local-first, macOS and Windows, with no sync, OCR, attachment indexing, or Obsidian application integration.
- Existing categories, priority/importance overrides, locale behavior, Vault adapters, model provider adapter, and connection scopes remain concepts; their Task-facing schemas and copy change together.
- The active shortcut count defaults to three and remains an implementation constant in this release.
- Existing legacy tables may remain read-forbidden backup evidence after migration; they are not automatically deleted in this release.
- Real-provider quality validation uses configured credentials when available and reports an explicit blocked result when unavailable; credentials are never copied into artifacts.
- The agreed implementation detail is maintained in [the source plan](../../docs/plans/task-centered-workbench.md); this specification is the user-observable authority.
