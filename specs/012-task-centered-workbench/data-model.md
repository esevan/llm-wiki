# Data Model: Task-Centered Workbench

All timestamps are UTC ISO-8601. Mutable aggregates use integer revisions. Append-only records are
never overwritten. Exact SQL names may change only if contract tests and this document change first.

## Core Records

### `captures`

Retains existing IDs and original text. Add `source_mode` (`capture`, `direct_task_provenance`) and
`last_user_activity_at`. Direct-Task provenance does not appear as a canonical Capture card. A
canonical Capture remains visible after creating Tasks.

### `tasks` and `task_revisions`

`tasks(id, origin_capture_id?, current_revision, state, archived_at?, category, created_at,
last_user_activity_at, started_at?, completed_at?, reopened_at?)`.

State is `task | in_progress | completed`. `task_revisions` uses `(task_id, revision)` and stores
title, detail, outcome, scope, non-goals, validation criteria, content hash, author, source reference,
and created time. Revision creation and head update are atomic.

### `problems` and `problem_revisions`

Problem identity stores current revision and `open | resolved | archived`. Mutable statement/detail
moves to `(problem_id, revision)`. Existing Problem becomes revision 1. Capture association is
provenance and is not unique.

### `task_problem_links`

Historical exact link `(task_id, problem_id, linked_problem_revision)` with relationship, note,
creator, creation time, and unlink time. Updating to a new Problem revision is an explicit new link.

### `task_relationships`

Historical edge `(source_task_id, target_task_id, kind)` where kind is `prerequisite | split_from |
related`. Related endpoints store sorted IDs. `split_to` is an inverse projection. Active self-links,
duplicates, and prerequisite cycles are invalid.

## Work and Decisions

`task_work_log_entries`, `task_work_log_comments`, `task_checklist_items`, and `task_attachments`
preserve legacy IDs, timestamps, bodies, base64 bytes, media metadata, hashes, AI summaries, and source
legacy IDs. Task Work Log is available in every Task state.

`task_decisions` is append-only for transitions, readiness overrides, continue-with-findings,
relationship changes, reopen, and contribution statements. `task_completions` is append-only and
binds Task revision, evidence, report, user, and time. `problem_resolution_decisions` independently
binds a Problem revision and evidence references.

## Refinement and Readiness

`refinement_sessions` has exactly one of `capture_id` or `task_id`, state, current draft revision,
active tab, scroll anchor, input draft, and activity/update timestamps. Messages and drafts are
append-only sequences. Draft stores normalized material hash, internal Problem snapshots, proposal
operations, provider metadata, and error. Proposal decisions bind proposal ID, exact draft revision,
decision, resulting Task/Problem revisions, operation ID, and time.

`refinement_items` exposes only legacy Problem-without-Task history as a canonical migration bridge.
It binds original Problem and exact revision, category identity, and refinement session.

Readiness definitions are stable field keys. Decisions bind Task revision, field, status override,
reason, evidence, and user. Current readiness is calculated from deterministic data plus explicitly
marked AI suggestions; no total score is stored.

## Reviews and Activity

`task_conflict_review_runs` contains a tagged subject (`capture_draft` or `task_revision`), subject ID,
subject revision, normalized material hash, Vault revision, evidence grant/scope revision, trigger,
status, cancellation/supersession and timing. Findings and resolution decisions are append-only.
Currentness is computed by exact identity. Last-current result and newest attempt are separate.

`user_activity_events` contains allowlisted user operation, entity identity, operation ID, and time.
The Task or session activity timestamp derives only from these events.

## Lineage and Publication

Lineage snapshot nodes are `capture`, `task_revision`, `problem_revision`, `work_log`, `decision`,
`completion`, or `knowledge_revision`; edges are `derived_from`, `refined_into`, `linked_problem`,
`prerequisite`, `split_from`, `related`, `evidences`, `completed_by`, or `published_as`.

Knowledge drafts bind exact Task revision, completion, Problem revisions, lineage snapshot and source
hash. Publication decision binds exact draft revision/hash. Existing external-change guards and
reversible patch evidence remain required.

## Migration Mapping and Invariants

- legacy feature ID → Task ID; `proposed→task`, `approved|in_progress→in_progress`,
  `completed→completed`; archived remains archived and does not reappear active;
- unknown legacy state aborts migration;
- legacy Problem → Problem revision 1; feature `problem_id` → exact Task-Problem revision-1 link;
- orphan Capture stays Capture; Problem without feature → refinement item, never invented Task;
- Work Log, completion, conflict, lineage, localization, override and tracking IDs remain stable;
- legacy grouped completion becomes individual Task completions plus Problem resolution with
  `legacy_group_completion` provenance;
- active relation/link uniqueness and all foreign keys are validated before schema version commit.

## Transition Rules

`task→in_progress→completed`; `completed→in_progress` only through explicit reopen. Revisions use
expected head. A mismatch returns `head_conflict` with current revision and conflicting fields.
Review/readiness state never blocks transition. Completion, Problem resolution, and publication do
not imply one another.
