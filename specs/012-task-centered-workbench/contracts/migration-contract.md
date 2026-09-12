# Migration Contract

Migration from schema 7 to 8 runs only after the application quiesces writers and creates a
consistent SQLite backup through a dedicated connection. A manifest records source schema/app
versions, size, hash, timestamp and backup location. Backup fsync, hash, integrity check and
independent reopen must pass before migration begins; live DB/WAL/SHM files are never naively copied.

The migration transaction creates new tables, maps all records per `data-model.md`, then checks:

1. source/target counts and complete identifier sets for every mapped table;
2. canonical field hashes, attachment byte hashes, timestamps and locale variants;
3. exact Problem/Task links, relationship validity and foreign keys;
4. Work Log/comment/checklist counts for every completed Task;
5. work-tracking, completion, conflict and lineage references;
6. no unknown legacy state and no active orphan reference.

Only then is `user_version=8` committed. Any error rolls back and records a safe stage. Startup offers
retry or explicit restore. Restore verifies manifest/hash, moves the failed database to a timestamped
recovery copy, restores atomically, reopens independently and revalidates. No automatic delete occurs.

The reviewed follow-up is v9 and has one schema purpose: rebuild
`work_tracking_sessions.capture_id TEXT NOT NULL UNIQUE` as nullable `TEXT UNIQUE` for reviewed
captureless Task continuation. It adds no Task/session/event/lineage table and no `task_id` column;
existing `work_tracking_links` remains the target binding. Use deferred foreign keys and
replacement-copy-drop-old-rename while preserving every value, child/self-parent chain, foreign
key, unique constraint, index, and revision/timestamp trigger. Set `user_version=9` inside the
transaction before commit. Validate row/ID equality, event bytes/hashes, idempotency, decisions,
jobs, links, session heads, indexes/triggers, foreign keys, and SQLite integrity. No backfill or
historical event rewrite is allowed; failures roll back to v8.

Discovery remains sessionless. Exact reviewed continuation with `mode:"continue_task"` and `taskId`
requires connection-owned `session:write` plus current-workbench discovery scope and rechecks target,
revision, topic membership, and evidence grants. Acceptance creates a connection-owned captureless
session and initial event/link/idempotency atomically; cancellation creates no session. Older
binaries fail closed on v9 while supported lower schemas still upgrade in order. Existing legacy
draft histories remain readable; fresh Task Knowledge uses immutable lineage snapshots and exact
body/source/lineage hashes.

Required fixtures: empty, small complete, orphan Capture, Problem-only, grouped completion, large,
large attachment, localized, duplicate ID, invalid state, corrupt/WAL edge, low disk, interrupted
backup, interrupted migration, hash mismatch, retry and restore.
