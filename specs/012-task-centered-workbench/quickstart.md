# Quickstart Validation: Task-Centered Workbench

## Prerequisites

- Use a disposable app-data directory and Vault fixture; never use a personal database.
- Prepare schema-7 fixtures listed in `contracts/migration-contract.md`.
- Configure the existing fake provider for deterministic suites. Detect real-provider configuration
  for final QA without printing credentials.

## Contract and Migration Check

1. Run Rust unit/contract tests and migrate every fixture.
2. Compare IDs, hashes, bytes, counts, links and evidence; exercise retry and verified restore.
3. Verify unknown/corrupt/incomplete cases do not commit schema 8.

## User Journeys

1. Save Capture and direct Task, force save failure, restart, and inspect canonical categories.
2. Add every Work Log type before start; transition, complete and reopen a Problem-free Task.
3. Refine Capture and Task, interrupt/restart, apply mixed proposal decisions, and inspect revisions.
4. Link many Tasks/Problems, relationships and readiness decisions; reject cycles/duplicates.
5. Review a Capture draft before Task exists and a Task revision; inject late/failure/insufficient
   results and prove they never become current/clear or block work.
6. Complete Task while Problem remains open, then resolve separately and generate an exact Knowledge
   draft. Confirm the wide three-column journey becomes one column when narrow, chronology remains
   distinct from interpreted links, and gaps of at least 24 hours are marked.
7. Inspect accepted semantic links: each is `supersedes`, `derived_from`, or `depends_on`, points
   from a later event to an earlier event, and exposes quotes from both endpoint events.
8. Change Task evidence after draft generation. Confirm Details shows the current journey while
   Review keeps the draft's embedded `{ sourceHash, journey, modelStatus, modelError }` snapshot;
   verify stale asynchronous results and external-file changes are rejected.
9. Switch English/Korean chrome and dates. Record locale-specific provider-label validation as
   blocked unless a real provider generated and rendered both locale graphs; mixed-language fixture
   titles do not satisfy this check.
10. Repeat cross-surface revision and stale-approval journeys through MCP.
11. Repeat primary flows by pointer and keyboard in wide/narrow English/Korean UI.

## Stable Verification

Run `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run typecheck`, and
`git diff --check` separately. After final code/docs only, run one release Tauri build, then
`npm run test:desktop` and UI review against that exact artifact.

The packaged runner defaults to isolated `task-capture`, `task-worklog`,
`task-refinement`, `task-relationships`, `task-review`, `task-publication`,
`task-persistence`, and `task-localization` cases. Each case starts with a
separate temporary app home, SQLite database, and Vault. To rerun one failing
case against the already-built artifact, use `npm run test:desktop -- --scenario task-refinement`.
The webview harness checks rendered centre-point hit targets before synthetic
events; it records this limitation because this runner has no OS-native input adapter.

## Real-Provider Report

Run 12 publication samples and 24 conflict samples: 8 conflicts, 8 compatible, 8 insufficient.
Record document fidelity/evidence/structure/readability (1–5), citation presence, classification,
false-clear count, and end-to-end median/p95 latency. Record provider/configuration absence as blocked;
do not replace it with fake or deterministic output.
