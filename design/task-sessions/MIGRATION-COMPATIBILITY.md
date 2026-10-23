# Session migration compatibility evidence

## Historical v9 fixture comparison

All runs used `cargo test --manifest-path src-tauri/Cargo.toml --test task_tracking_v9_migration -- --test-threads=1`. Logs are local verification artifacts under `.tmp/` in this worktree.

| Tree / experiment | Result | Evidence |
| --- | --- | --- |
| Exact base `f3ede4566962ce4f1f24e69fe36111da20496021`, isolated `.worktrees/session-v9-baseline` | 1 passed, 2 failed | `session-v9-base.log`: actual schema 25 versus hard-coded 24; retained `knowledge_revision_turns` fails migration 25 |
| Original session checkpoint `6d40050`, unchanged historical test | 1 passed, 2 failed | `session-v9-original.log`: same failures, actual schema now 26 |
| Diagnostic only: compensate those two baseline assumptions | 1 passed, 2 failed | `session-v9-probe-before.log`: migration 26 finds retained `task_session_queue_control` |
| Same diagnostic plus teardown of the two new queue tables | 3 passed, 0 failed | `session-v9-probe-after.log`: all original data preservation, foreign key, custom object, backup, rollback, retry and newer-schema assertions remain |
| Final source: only session-owned queue teardown retained | 1 passed, 2 failed | `session-v9-scoped-final.log`: original two baseline failures remain |

The base and original session test files had identical SHA-256 `52bb117f0f829e0d5824a2ae922ab21e6bad3ed38fd8c1b5cc6d91cebf97076b`. The session change introduced a fixture maintenance omission masked by an earlier baseline failure: the v8 fixture constructs the latest schema then removes later tables, but left new queue children behind. The scoped fix drops queue items and queue control before their Run/session parents. No production migration change or assertion relaxation is needed. Diagnostic version/Knowledge adjustments were reverted; the official group is still failing and is not reported as passing. Snapshot probes remain `.tmp/session-v9-probe-before.rs` and `session-v9-probe-after.rs`.

## Held notifications branch: future integration collision

Read-only inspection of held `406b106b` shows migration 26 adds notification inbox/auth incidents/scan objects and migration 27 adds retry events/heads and updates inbox occurrence keys. Session migration 26 instead adds reasoning/settings fields and durable queue objects. Their SQL object names do not overlap, but the global migration identity **26 conflicts**. Common ancestor with `f3ede45` is `9940d91`. Notifications were neither edited nor merged.

The migration engine uses `PRAGMA user_version` and validates a contiguous version plan; it does not record per-migration identity. Therefore these are integration risks inferred from code, not reproduced merged-release regressions:

- A session-v26 database opened by the held notification-v27 lineage skips notification migration 26; migration 27 then references missing inbox objects.
- Notification-v27 databases are correctly rejected by an application supporting only v26.
- A notification-v26 database could appear current to the session-v26 lineage while lacking queue/settings objects.

Do not resolve by choosing the largest version constant, retaining duplicate version 26 entries, resetting user_version, dropping real data or blanket `IF NOT EXISTS`. After the notification hold is lifted, choose an explicit approved migration lineage and additive reconciliation plan. Gate integration on upgrades from base v25, session v26, notification v26 and notification v27 with real rows, foreign-key checks, rollback/retry and idempotent reopen. No reconciliation or notification implementation is part of this session checkpoint.

## Unchanged pending acceptance

The Mac remains locked. Actual production-component wide/narrow screenshots and pixel comparison are pending manual unlock; no further automatic unlock or alternative screen-control attempt was made. No merge or push occurred. Existing execution/regression evidence and packaged E2E rationale remain in [IMPLEMENTATION.md](IMPLEMENTATION.md).
