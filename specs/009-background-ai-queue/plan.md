# Implementation Plan: Background AI Queue

**Branch**: `feat/background-ai-queue` | **Date**: 2026-09-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-background-ai-queue/spec.md`

## Summary

Move every durable AI, translation, and embedding operation behind a SQLite-backed asynchronous work contract while preserving responsive Chat behind one process-wide, memory-only Fast Queue throttler. Use an expand-migrate-contract transition: first characterize current APIs, then record synchronous executions as terminal jobs without changing responses, extract reusable asynchronous handlers, and finally switch eligible endpoints to `202 + job_id`. Run web, Fast Queue, and durable workers in independently startable asyncio processes; use leases, checkpoints, idempotent publication, source hashes, and atomic Vault writes for recovery.

## Technical Context

**Language/Version**: Python 3.12+

**Primary Dependencies**: FastAPI, Uvicorn, HTTPX AsyncClient, aiosqlite, watchfiles; optional fastembed/onnxruntime for semantic embeddings

**Storage**: Same-host SQLite in WAL mode for workflow, jobs, leases, checkpoints, results, and notifications; Obsidian-compatible Markdown Vault for durable Knowledge translations

**Testing**: pytest, FastAPI TestClient/HTTPX ASGI transport, deterministic fake streaming provider, subprocess integration tests, Playwright browser coverage

**Target Platform**: Local macOS and Windows application service; all cooperating processes on one host

**Project Type**: Single-package local web application with Python API/worker processes and a framework-free browser UI

**Performance Goals**: Existing capture p95 under 50 ms and readiness under 1.5 seconds remain binding; regular enqueue returns within 1 second in 95% of local runs; Queue changes appear within 2 seconds; Fast Queue admits at most one provider request globally; 1,000-note indexing regression stays under 15%

**Constraints**: No Redis/Celery or network-hosted broker; private inputs remain local except configured provider calls; no long SQLite write transaction across provider/embedding work; no automatic workflow-state or Knowledge-publication decision; Fast Queue has no durable state; exact-hash stale-result rejection; cross-platform process spawning

**Scale/Scope**: Single local user; two supported locales; one Fast worker; configurable durable worker processes/concurrency; all existing AI task kinds plus document-level embeddings and paragraph-level translations

## Constitution Check

*GATE: Passed before research and re-checked after design.*

- **I. Conversation-first — PASS**: Chat remains inline and throttled without exposing queue mechanics.
- **II. Reduce Cognitive Load — PASS**: Only durable work is visible and notifications are restricted to required decisions.
- **III. Resume Where You Left Off — PASS**: Leases, checkpoints, deep links, source identity, and unread alerts preserve continuation context.
- **IV. Problems, not tasks — PASS**: Queue entries are execution telemetry, not a workflow stage.
- **V. Private Process, Portable Knowledge — PASS**: SQLite retains private work; validated outputs cross only existing Vault boundaries.
- **VI. Never Score the Worker — PASS**: Status describes work, never a person.
- **A. Measured Performance — PASS**: Characterization, enqueue latency, pool isolation, indexing, and browser checks are planned.
- **B. Independent Adapters — PASS**: Provider I/O and Vault writes retain their dedicated adapters.
- **C. User Authority — PASS**: Draft, Refine, review, organization, publication, and workflow-state changes require explicit user action.
- **D. Evidence/Consistency — PASS**: Hashes, leases, idempotency, and atomic publication reject stale or duplicate output.
- **E. Local/Cross-platform — PASS**: Same-host WAL and spawn-safe roles support macOS and Windows.
- **F. Minimal Complexity — PASS WITH JUSTIFICATION**: HTTPX and aiosqlite provide real async HTTP and non-blocking SQLite access; external brokers are excluded.

## Project Structure

### Documentation (this feature)

```text
specs/009-background-ai-queue/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
└── tasks.md
```

### Source Code (repository root)

```text
llm_wiki/
├── web/app.py
├── controllers/
│   ├── jobs.py
│   ├── workflow.py
│   ├── knowledge.py
│   └── settings.py
├── services/
│   ├── handlers/
│   ├── jobs.py
│   ├── localization.py
│   └── retrieval.py
├── repositories/
│   ├── jobs.py
│   ├── workflow.py
│   ├── localization.py
│   └── retrieval.py
├── adapters/
│   ├── provider.py
│   └── vault.py
├── static/index.html
├── static/i18n/
└── cli.py

tests/
├── test_api.py
├── test_ai_jobs.py
├── test_async_workers.py
├── test_fast_queue.py
├── test_job_recovery.py
├── test_localization.py
├── test_performance.py
└── test_browser_menu.py
```

**Structure Decision**: Preserve one deployable Python package while applying a layered dependency direction: `web` composes the app and browser delivery; `controllers` own HTTP validation and response mapping; `services` own use cases, with `handlers` reserved for durable asynchronous work; `repositories` own durable Queue persistence; `adapters` own provider and filesystem integrations. No synchronous AI execution path remains after migration, so its temporary package and the `api/app.py` compatibility import are removed rather than retained as empty layers.

## Complexity Tracking

| Choice | Why Needed | Simpler Alternative Rejected Because |
| --- | --- | --- |
| Two new runtime dependencies | Real async provider cancellation/streaming and event-loop-safe SQLite access | Wrapping blocking calls in shared threads undermines the requested asyncio/multiprocess boundary |
| Three process roles | One global Fast throttle plus independently recoverable durable workers | Per-web queues multiply throttling; in-process durable threads cannot survive web-worker replacement |

## Post-Design Constitution Re-check

Phase 1 design preserves every pre-design result. Contracts keep Queue telemetry separate from workflow state, the data model makes approval and source identity explicit, Vault publication remains atomic, and validation binds performance, recovery, cross-process idempotency, and terminology checks. No new violation was introduced.
