# Research: Background AI Queue

## Async process topology

**Decision**: Use independently startable asyncio roles: stateless FastAPI web processes, exactly one Fast Queue worker process, and configurable durable worker processes. The default CLI supervises them on one host.

**Rationale**: Uvicorn workers are separate spawned processes, including on Windows, so process-local queues cannot provide one global throttle. A single Fast worker preserves throttling while durable workers scale separately.

**Alternatives considered**: Per-web threads multiply provider concurrency; one monolithic process prevents independent recovery; Celery/RQ adds an external broker inappropriate for a local application.

## Fast Queue transport

**Decision**: Keep Fast requests ephemeral and proxy them over a loopback-only internal streaming interface to one Fast worker, whose single `asyncio.Queue` consumer serializes provider requests.

**Rationale**: `asyncio.Queue` is scoped to one event loop and is not cross-process. Loopback streaming is cross-platform and propagates disconnect cancellation without durable Job semantics.

**Alternatives considered**: Unix sockets are not the cross-platform default; multiprocessing queues complicate bidirectional streaming and independent web restart; persistence violates the no-state decision.

## Async provider

**Decision**: Replace blocking `urllib` with one scoped `httpx.AsyncClient` per process behind the existing provider adapter.

**Rationale**: HTTPX supports cancellable async requests, streaming, connection pooling, and explicit closure.

**Alternatives considered**: `asyncio.to_thread` retains cancellation threads; provider-specific SDKs weaken the OpenAI-compatible boundary.

## Durable queue and SQLite

**Decision**: Store durable work in the existing same-host SQLite database in WAL mode. Use aiosqlite connections per process, short `BEGIN IMMEDIATE` claims with busy timeout/backoff, and no transaction during external work.

**Rationale**: WAL allows readers beside one writer. Short claims and batched progress fit local scale and support multiple processes.

**Alternatives considered**: Memory cannot recover; long transactions block writers; a network broker exceeds scope.

## Delivery and recovery

**Decision**: Use at-least-once execution with leases, heartbeat, bounded retry, idempotency keys, staged results, exact source checks, and idempotent publication/notification.

**Rationale**: Exact-once execution cannot span process death and Vault/SQLite boundaries; at-least-once plus idempotent publication yields one observable result.

**Alternatives considered**: Immediate failure discards recoverable work; unbounded retry creates poison jobs; process memory cannot identify dead owners.

## Checkpoints and CPU work

**Decision**: Checkpoint Knowledge by paragraph hash and embeddings by document hash plus model. Run blocking CPU embeddings outside the event loop in a bounded process execution path.

**Rationale**: Valid checkpoints avoid repeated work and reject stale output; asyncio is for I/O rather than CPU parallelism.

**Alternatives considered**: Whole-job restart wastes work; event-loop embeddings harm responsiveness; uncontrolled nested pools multiply processes.

## Migration strategy

**Decision**: Expand–migrate–contract: characterize current APIs, record synchronous execution as terminal Jobs without changing responses, extract shared handlers, then change approved endpoints to `202 + job_id`.

**Rationale**: This creates a comparison oracle before moving most existing code and separates intentional HTTP changes from regressions.

**Alternatives considered**: A big-bang rewrite lacks a baseline; permanently blocking compatibility defeats responsiveness; pre-recording `completed` creates false success.

## Interface updates

**Decision**: Use a bottom-right Queue with snapshot reads plus resumable SSE hints and a separate unread notification bell. Result links remain task-specific; Fast work never appears.

**Rationale**: Snapshot reconciliation stays correct across processes and missed events; SSE fits the current browser architecture.

**Alternatives considered**: Browser-only state loses reload recovery; polling alone delays feedback; a generic result page breaks context.
