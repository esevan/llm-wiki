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

Required fixtures: empty, small complete, orphan Capture, Problem-only, grouped completion, large,
large attachment, localized, duplicate ID, invalid state, corrupt/WAL edge, low disk, interrupted
backup, interrupted migration, hash mismatch, retry and restore.
