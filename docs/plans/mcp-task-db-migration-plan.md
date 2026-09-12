# MCP Task-Centered DB Migration Plan

Status: v9 implemented; populated migration, rollback and recovery fixtures pass
Scope: schema and data migration review only; application implementation is separate
Reviewed: 2026-09-12

## Decision summary

At the initial review, the repository's latest supported native schema was
`CURRENT_SCHEMA_VERSION=8`; no live database was inspected. The implementation adds the narrow
v9 migration described below. `migrations.rs` applied versions 1 through 8 in order and set `PRAGMA user_version`
inside each migration transaction before its commit (`migrations.rs:649–652`).
Version 8 (`migrate_task_centered_workbench`) creates the Task aggregate and assistance tables,
copies legacy feature/work records, preserves legacy ledger rows in
`legacy_task_migration_records`, and validates counts, field equality, and foreign keys. A new
MCP remediation pass must therefore be a versioned, narrowly scoped migration only if a durable
invariant cannot be enforced with the existing tables. The confirmed non-null Capture constraint
requires the precise v9 below for direct-Task continuation. It does not justify historical-table
removal, historical event rewrites, or a second wholesale legacy conversion.

The main observed gaps are query/projection and contract/skill gaps: `current_workbench` reads
only `work_tracking_sessions`, `overview` is built from the Task board and pending event rows,
and the installed workflow skill still describes legacy action terminology. These are reachable
through current native/MCP routes and can be fixed without changing the database. The reviewed
schema change is narrower: v8 drops all legacy Workbench mutation triggers, so v9 must not add
replacement SQL mutation-event triggers. The only approved v9 shape is a constrained rebuild of
`work_tracking_sessions.capture_id`; all other gaps remain service, projection, contract, or skill
work.

## Evidence reviewed

- `src-tauri/src/native/migrations.rs`: `CURRENT_SCHEMA_VERSION = 8`; migrations 1–8; v8
  rebuilds `problems` to remove capture uniqueness, creates Task/assistance schema, maps
  `features` to same-ID Tasks and revision 1, copies work logs/checklists/completions, and
  validates mapping/count/foreign-key invariants.
- `src-tauri/src/native/database.rs`: startup enables foreign keys and WAL, creates a verified
  `VACUUM INTO` backup before an upgrade with data, records a manifest/hash/integrity check, and
  exposes verified restore. Migration failure leaves a marker and does not advance the version.
- `schema.sql`, `task_schema.sql`, `work_tracking_schema.sql`, and
  `task_assistance_schema.sql`: canonical records, append-only Task records, MCP sessions/events,
  projection outbox/results/watermarks, decisions, idempotency, refinement, review, and Knowledge
  tables.
- `work_tracking_service.rs` and `adapters/sqlite/mod.rs`: scope checks, session/event CAS,
  request hashes, projection, current/overview queries, review challenges, and publication
  guards. `current_workbench` selects sessions joined to captures. `overview` calls the Task board,
  then separately selects pending proposal events.
- `task_assistance.rs`: operation replay compares a request hash and stores the exact result;
  lineage reads Task revisions, exact Problem links, work logs, decisions, completions, and
  `task_knowledge_drafts`.
- `specs/012-task-centered-workbench/contracts/migration-contract.md` and `data-model.md`,
  plus migration/projector/concurrency/MCP acceptance tests. No live database was opened or
  mutated for this review.

## Current schema and reachability findings

### Version/order and canonical versus legacy records

The ordered versions are: 1 native baseline, 2 legacy normalization, 3 AI job defaults, 4 dual
chat work tracking, 5 bound reviews, 6 Workbench entity timestamps, 7 solution-child timestamp
propagation, and 8 Task-centered conversion. Existing `features`, `solution_progress_*`,
`completions`, and ledger tables remain present after v8; v8 does not delete historical rows.
Canonical Task reads use `tasks`/`task_revisions` and Task-owned work tables. The old tables remain
historical compatibility data and are also recorded where needed in
`legacy_task_migration_records`. This is preservation, not proof that every legacy route is
reachable.

V8 keeps the legacy feature ID as Task ID, maps `proposed` to `task`, `approved|in_progress` to
`in_progress`, and `completed` to `completed`; unknown states abort. Problems become revision 1;
feature `problem_id` becomes an exact revision-1 link. Problem-only records become refinement
items. Grouped legacy completion is represented by Task completions and a separate Problem
resolution decision. These mappings are protected by `MigrationSnapshot::validate`.

### Sessions, connections, and event ownership

`mcp_connections` owns scope/topic grants. `work_tracking_sessions` is unique by
`(connection_id, conversation_ref_hash)` and, in v8, has one required Capture, head event and revision.
Events are append-only by `(session_id, revision)` and `(session_id, stream_id, source_sequence)`;
idempotency is keyed by source interface/owner/operation. Decisions bind session, source event,
channel, and accepted payload hash. Projection jobs/results and stream watermarks support replay,
late-event handling, and exactly-once materialization. These tables already provide the required
session/connection scope for MCP reads; they do not automatically represent a desktop Task that
has no tracking session.

`work_tracking_schema.sql` contains historical trigger definitions for linked legacy `problems`,
`features`, and solution progress. During v8, `drop_legacy_workbench_triggers` removes all such
problem/feature/progress/checklist/completion triggers. They are not active mutation producers.
Do not add SQL replacement triggers in v9: current Task mutations must use the shared application
mutation helper/append path, and old event rows remain unchanged.

### Completion, publication, and lineage

`task_completions` is append-only and binds Task revision, evidence, report, and operation ID.
`knowledge_drafts` binds a session/completion event and exact draft content hash; publication
decisions bind draft ID/revision/hash and path. The assistance-side
`task_knowledge_drafts` binds Task revision and completion ID and stores a lineage JSON snapshot.
The exact Problem revision links are available from `task_problem_links` and are included by
`task_lineage`, but the Knowledge table has no first-class foreign key or revision columns for
those links. The reviewed contract is an immutable lineage snapshot captured at the exact Task
revision/completion: include Problem revisions, historical links/relationships, and evidence in
the snapshot and validate immutable referenced identities/revisions/content, never current active
links or mutable unlink/retirement metadata. `content_hash`
continues to hash the body; a separate source/lineage hash belongs in review JSON. Same-revision
unpublished corrections still bind exact revision and hash; published edits create a new draft.
Legacy `knowledge_drafts` histories remain preserved; all new Task flows use canonical
`task_knowledge_drafts` through the same service as desktop. Only unresolved legacy proposals/jobs
are blocked pending fresh review. Reuse the existing reviewed publication/recovery infrastructure.

`overview` currently returns board item counts/readiness, pending proposal attention, and recent
completed items. It does not return Problem links, Task decisions, relationships, or completion /
publication lineage. That omission is in the projection/query contract, not evidence of missing
rows. `current_workbench` returns only tracked sessions joined to captures, so direct desktop
Tasks are absent unless linked to a session. Both are reachable gaps confirmed by their query
paths and should be fixed in the service projection first.

### Snapshot and mutation freshness

Overview hashes board and pending attention (`adapters/sqlite/mod.rs:2110–2117`), whereas current
selection/link review uses `work_tracking_workspace.revision`. Overview currently reads components
through different connections; direct Task activity does not increment the workspace counter
(`task_repository.rs:49–65`). These are application bugs, not missing schema. Read each complete
projection in one SQLite transaction, hash every returned dependency, and use the shared mutation
helper to invalidate workspace/material target freshness atomically. Include links, relationships,
state, Work Log, checklist, review and Knowledge dependencies; Task content revision alone is
insufficient. Linked sessions receive each mutation once; notification projection must not repeat
the domain mutation. Untracked Tasks keep canonical history without manufactured sessions.

## Migration decision matrix

| Concern | Schema migration required? | Reviewed treatment |
| --- | --- | --- |
| Direct desktop Tasks in `current_workbench` | No for discovery | Query canonical Tasks and tracked sessions without duplication, retaining discovery/owner scopes. |
| Explicit continuation of an existing Task without Capture | Yes, precise v9 below | Make the existing session Capture FK nullable; use existing Task links and generic pre-session review. |
| Problem links, decisions, relationships in overview | No | Extend bounded projection query/DTO; use current Task tables and exact revisions. |
| Legacy `advance` actions and legacy payload nouns | No | Check actual native/MCP/skill reachability; reject unsupported new legacy actions or retain read-only history. |
| Skill still instructs superseded legacy events/actions | No | Replace with supported Task contracts and fixtures; do not backfill events. |
| Task events emitted by current Task mutations | No new SQL schema trigger | Route current writes through the shared mutation helper/append path; retain historical events byte-for-byte. |
| Knowledge exact Problem lineage | No new normalized FK table | Capture and validate immutable lineage snapshots; never compare a historical draft to current active links. |
| Historical legacy tables/events | No | Retain; no drop/rewrite. Use migration ledger and read-only compatibility where reachable. |

## Reviewed v9 migration contract

V9 has one purpose: allow explicit existing-Task continuation without fabricating a Capture.
Discovery remains sessionless. The migration
rebuilds `work_tracking_sessions.capture_id TEXT NOT NULL UNIQUE` as `TEXT UNIQUE` (nullable),
without adding a task/session/event/lineage table or a `task_id` column. Existing
`work_tracking_links` remains the binding for Task targets.

1. Quiesce all writers and open one migration connection with foreign keys enabled. Create and
   independently reopen a `VACUUM INTO` backup; verify manifest size, SHA-256, and integrity.
2. Verify initial foreign-key integrity, disable enforcement on the migration connection before
   the v9 transaction, and use replacement-copy-drop-old-rename. Preserve every column, value,
   child/self-parent chain, foreign key, unique constraint, and surviving session index. Recreate
   revision/timestamp triggers, indexes, and dependent views explicitly. Inventory all
   indexes/triggers/views first and never rewrite child FKs to reference a temporary replacement table.
3. Validate row and ID equality, immutable event bytes/hashes, idempotency/decisions/jobs/links,
   session heads, indexes/triggers/views, foreign keys, and SQLite integrity before commit. There is no
   backfill and no historical event rewrite.
4. Set `user_version=9` inside the migration transaction, then commit. Any failure rolls back to
   v8; retry starts from the unchanged database or an explicitly restored manifest-verified backup.
5. Restore and verify foreign-key enforcement on success and failure, and recheck all foreign keys
   before the application can resume. Never use writable_schema or rename the original parent first.

Implementation correction: the original Astra-reviewed FK-ON/deferred approach produces a clean
foreign_key_check but fails COMMIT for a populated parent/child replacement. The retained disposable
reproduction is `.tmp/sqlite-parent-rebuild-repro.json` (diagnostic SQLite 3.51.0); the application
bundles SQLite 3.46.0. The correction follows the [SQLite generalized ALTER TABLE procedure](https://www.sqlite.org/lang_altertable.html#otheralter). It changes enforcement only on the
quiesced migration connection, not durable FK definitions. An additional Astra consultation was
not executed because the agent thread limit was reached; this correction is based on the official
procedure and behavioral migration tests, not an additional Astra endorsement.

Sessionless discovery requires its bounded read scope and explicit topic membership when applicable;
a read grant cannot authorize continuation. Explicit continuation requires active `session:write`
plus the authorized discovery scope. At acceptance recheck grants, membership, target identity,
revisions and material hashes. Exact target acceptance creates a connection-owned,
captureless session, initial event/link/idempotency rows atomically through the generic pre-session
review path. Cancellation creates no session. It cannot take over another connection's session or
fabricate a Capture/lineage. The initial event describes binding to an existing Task, not creating
that Task or a Capture. Capture-required joins/open paths/DTOs must support both forms. Binding does
not grant access to another Task/Problem; additional targets need separate authorized context and
exact review. Session details/events remain owner-scoped. Untracked Task history stays canonical.

Supported older/version-zero databases continue through the ordered migrations. Older binaries
fail closed on v9; they do not reject all lower supported schemas. A newer binary fails closed when
required tables/invariants are absent. No downgrade migration is proposed.

## Hash, revision, atomicity, and secret rules

Content hashes must use the existing canonical field ordering (`content_hash` for Task/Problem
revisions; SHA-256 for request, attachment, and publication content). A retry with the same
operation ID and request hash returns the stored JSON byte-for-byte; a reused ID with a different
hash is an idempotency conflict. Event revisions advance with a head compare-and-swap in the same
transaction as the event and idempotency record. Completion and publication are separate commits;
completion never implies publication. No API key or provider secret belongs in event payloads,
migration manifests, lineage JSON, or activity logs. Settings are stored atomically in
`settings.json`; do not claim keychain storage as part of this migration contract.

## Recovery, rollback, and validation fixtures

Required fixtures/checks: empty v8, migrated feature/problem/work-log/completion, orphan Capture,
Problem-only, multiple Tasks per Problem, duplicate operation ID with same/different payload,
exact revision link drift, grouped completion, large attachment, localized/override/deleted rows,
legacy event payload, direct desktop Task with no session, tracked session with linked Task, pending
projection, late/duplicate event, publication hash drift, invalid state, broken foreign key,
interrupted backup, interrupted migration, corrupt WAL/SHM, low disk, backup hash mismatch, retry,
restore, opening with an older binary, captureless continuation acceptance, continuation cancellation,
wrong-connection takeover, unauthorized target/revision/grant, session-head preservation, and
historical Knowledge lineage after current-link edits, multi-connection continuation, grant/topic
revocation, all session child references and parent chains, restored index/trigger behavior, direct
desktop snapshot invalidation, and interrupted publication recovery. Assertions must include no row loss, no
fabricated Task/Problem/Capture, unchanged historical event bytes, exact hashes/revisions, atomic
rollback, and safe errors without secrets.

## Astra review record

Astra review completed 2026-09-12 at high reasoning. Conditional approval: the corrections above
were incorporated into the plan; runtime implementation and live-database verification had not
yet been performed at that review. Required implementation checks included the reachability audit for legacy adopt/approve
Problem/Solution actions, verification that new legacy actions are rejected while historical
completed events remain readable, and fresh canonical review for unresolved old proposals. Legacy
new adoption/approval, mandatory conflict, and grouped completion are not replayed. An already
approved legacy publication job may recover only through the existing handler with its original
reviewed draft/hash and external-file guards; other unresolved legacy mutation jobs/proposals are
preserved and safely blocked pending fresh canonical review.
