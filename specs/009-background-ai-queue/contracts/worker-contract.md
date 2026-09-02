# Worker and Handler Contract

## Roles

- `web`: validates, enqueues, serves snapshots/events, and proxies Fast streams; it runs no migrated AI/embedding work.
- `fast-worker`: exactly one process and FIFO consumer with no durable Job state.
- `async-worker`: one or more processes claiming durable Jobs through SQLite leases.

Roles start independently for tests and are supervised together locally. Internal Fast transport is loopback-only.

## Handler

Each durable task registers one async handler accepting a versioned snapshot and cancellation/progress/checkpoint/source context, returning a validated staged result. It does not own HTTP, UI, process spawning, lease claim, or user approval. During migration, the synchronous executor invokes it; final execution invokes it only in durable workers.

## Claim, failure, and recovery

Claim is one short transaction recording worker, attempt, token, and expiry. Heartbeat and every mutation require the live token.

- Transient: timeout, rate limit, temporary busy, worker loss; bounded backoff with jitter.
- Permanent: invalid input/output, missing target, unsupported configuration; no automatic retry.
- Stale: source/target changed; never publish as current.
- Cancelled: stop safely or discard late output.

Fast shutdown abandons ephemeral requests. Durable shutdown stops claims and leaves leases/checkpoints. Startup recovery evaluates expired leases; no Job remains indefinitely running.

## Publication

Handler success is not publication. Validate result, source, lease, and idempotency first. Decision results stop at `awaiting_review`; automatic derived output publishes only to its exact target; notification identity is unique per Job and kind.
