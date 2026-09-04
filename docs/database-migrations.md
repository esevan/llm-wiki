# SQLite schema migrations

[한국어](database-migrations.ko.md)

The native application owns one SQLite schema migration boundary in
`src-tauri/src/native/migrations.rs`. SQLite's `PRAGMA user_version` records the last committed
version. Application startup applies every later migration in ascending order before any domain
command or legacy settings import can read the database.

## Guarantees

- Version `0` represents an unversioned database created by the former Python application or an
  earlier native build.
- Every migration runs in its own `IMMEDIATE` transaction. Its schema/data changes and
  `user_version` update commit together; an error rolls both back.
- Migration versions must be contiguous and ordered. An invalid compiled plan fails before making
  a database change.
- Running startup again at the current version is a no-op.
- A database with a version newer than the application supports is rejected without modification.
- The version 2 compatibility migration preserves the former field-level localization data and
  adds every column known to have been introduced conditionally by the Python application.
- Version 3 rebuilds the legacy `ai_jobs_v2` table with native defaults while preserving every
  existing job. New jobs therefore receive execution, result, progress, and timestamp defaults
  even when the database originated in the Python application.

## Adding a migration

1. Do not edit the version 1 baseline in `native/schema.sql` or an already released migration.
2. Add one focused migration function to `native/migrations.rs`.
3. Append its version, name, and function to `MIGRATIONS`, then increment
   `CURRENT_SCHEMA_VERSION`.
4. Keep destructive or data-rewriting changes explicit and deterministic. Do not perform network,
   Vault, provider, or UI work inside a database migration.
5. Add tests for upgrading the immediately preceding version, preserved data, rollback on failure,
   idempotent reopening, and newer-version rejection where relevant.
6. Run `cargo test --manifest-path src-tauri/Cargo.toml` and the packaged desktop relaunch E2E before
   release.

Application settings are a separate versioned JSON file under `~/.llm-workbench`. The one-time
legacy settings import reads SQLite only after its schema migrations succeed.
