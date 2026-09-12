# Implementation Plan: Task-Centered Workbench

**Branch**: `feat/task-centered-workbench` | **Date**: 2026-09-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/012-task-centered-workbench/spec.md`

## Summary

Replace the persisted Capture → Problem → Solution gate with canonical Capture and independent,
revisioned Task aggregates. A single Rust application service and typed frontend contract own Task
behavior; optional refinement and conflict jobs bind to exact Capture drafts or Task revisions.
Migrate all legacy evidence through a validated SQLite backup, then replace the legacy Workbench
board with React while retaining unrelated runtime surfaces. Completion, Problem resolution, and
Knowledge publication remain distinct append-only decisions.

The change traces to LLM Wiki Problems `7e1bd5c7-7801-42d3-a143-11d0ee18ef71` (a Capture's
already-derived Solution must not be rediscovered) and `dc1c8b22-367e-4662-a9db-305e6fac1f85`
(Problem approval is mechanical in a single-user workflow), plus existing Solution
`39575c74-9a31-4325-97de-d8e3da61fcb2`. Attempts to open a new unified tracking record returned
`elicitation_required` because this host cannot complete that elicitation, so no new LLM Wiki record
was saved; these existing IDs are the durable provenance references.

## Technical Context

**Language/Version**: Rust 2021; TypeScript 5.9; React 19; Node.js 20+

**Primary Dependencies**: Tauri 2, rusqlite, serde/serde_json, React, Vite, existing provider and vault adapters; no new runtime dependency

**Storage**: Local SQLite WAL plus Obsidian-compatible Markdown Knowledge files

**Testing**: `cargo test --manifest-path src-tauri/Cargo.toml`, Vitest via `npm test`, `npm run typecheck`, release Tauri build, packaged desktop E2E, fixed-rubric real-provider review

**Target Platform**: macOS and Windows desktop

**Project Type**: Tauri desktop application with Rust-native domain/storage and React UI

**Performance Goals**: capture/direct-Task persistence p95 <50 ms; local Workbench projection <100 ms p95 after response; warm FTS <75 ms; conflict latency recorded at median and p95

**Constraints**: local-first; no compatibility shim; no record deletion; exact revisions and operation idempotency; background jobs never affect recency; one final release artifact reused for E2E and model QA

**Scale/Scope**: existing single-user databases including large legacy fixtures; wide/narrow bilingual desktop UI; desktop and MCP consumers

## Constitution Check

*GATE: Passed before research and passed again after contracts.*

- **I / II / III**: Natural input remains primary; active and refining shortcuts reduce recall cost;
  canonical category groups and full Work Log preserve resumption context.
- **IV**: Task owns work, Problem owns independent revisioned context, and optional assistance does not
  gate Task work. The design directly implements the amended principle.
- **V / C**: Drafts remain local; exact user decisions control proposal application, completion,
  Problem resolution, and Knowledge publication.
- **VI**: readiness exposes fields and evidence only; it never produces a person score.
- **A**: tasks include persistence, projection, recency, job latency, and required benchmark checks.
- **B / F**: existing adapters and one Rust application boundary are retained; no dependency added.
- **D**: review results bind exact identity and citations; non-success states cannot become clear.
- **E / Governance**: backup, rollback, restore, byte/hash preservation, and both platforms are covered.

No justified constitution violation remains.

## Project Structure

### Documentation (this feature)

```text
specs/012-task-centered-workbench/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── impact.md
├── quickstart.md
├── checklists/requirements.md
├── contracts/application-api.md
├── contracts/mcp-task-contract.md
├── contracts/migration-contract.md
└── tasks.md
```

### Source Code (repository root)

```text
frontend/src/
├── types/application.ts
├── services/tauriApplicationClient.ts
├── features/workbench/
│   ├── WorkbenchView.tsx
│   ├── WorkbenchView.test.tsx
│   ├── TaskDetail.tsx
│   └── task-workbench.css
└── features/chat/

frontend/public/runtime/          # remove Workbench board bindings only
src-tauri/src/
├── application/                  # Task aggregate and work-tracking orchestration
├── domain/                       # Task/revision/readiness/review/publication rules
├── native/                       # SQLite migration, repositories, thin routes/jobs
│   └── task_schema.sql           # schema-v8 definitions; legacy baseline remains immutable
├── mcp.rs
└── mcp_ipc.rs
scripts/                          # final artifact build/E2E/review runners
tests/                            # desktop scenario and characterization ledger
docs/                             # bilingual current behavior and feature index
```

**Structure Decision**: Keep the current Tauri/React repository. `frontend/src/types/application.ts`
is the shared UI contract. Rust application services own mutations and validation; native routing is
thin. React owns the Workbench and Task detail. Only the old board/event bindings leave the legacy
runtime so Search, Settings, and queue behavior can migrate separately.

## Architecture and Delivery Boundaries

1. **Migration boundary**: `native/migrations.rs` coordinates writer quiescence, backup/manifest,
   schema v8 transaction, invariant validation, rollback, and restore commands. New repositories do
   not read legacy tables after commit.
2. **Task aggregate boundary**: application/domain modules atomically handle Task creation/revisions,
   transitions, Problem links, relationships, Work Log, decisions, completion, and user activity.
   Every mutation accepts operation ID and expected revision where applicable.
3. **Refinement/review boundary**: one service owns sessions, proposals, exact-draft application,
   debounce/cancellation and review identity for either Capture draft or Task revision. Jobs cannot
   write Task state or user activity.
4. **Projection/UI boundary**: `/workbench` returns ordered typed shortcut projections and existing
   canonical categories. React uses the same identities for shortcuts and cards and disconnects
   legacy `#board`/`loadBoard` handlers.
5. **MCP/Knowledge boundary**: work-tracking events, MCP resources/tools, lineage graph and Knowledge
   use the Task aggregate. Durable actions and publication retain existing exact-review controls.

Schema/DTO and migration contracts land before parallel backend, frontend, asynchronous job, and MCP
implementation. Cross-cutting final validation follows integration.

## Delivery Phases

### Phase 0 - Contract and Migration Spike

Resolve schema rebuild details with real SQLite constraints/query plans, finalize typed DTOs, amend
governance and supersede conflicting specs. Exit when migration and API contract tests can be written.

### Phase 1 - Preserving Schema and Core Aggregate

Implement verified backup/restore, v8 migration, repositories, Task lifecycle, Work Log, Problem links,
relationships, readiness, completion and activity allowlist. Exit when provider-free lifecycle and
all migration fixtures pass.

### Phase 2 - Refinement and Conflict Orchestration

Implement resumable session state, multiple proposal decisions, Capture-draft review, Task review,
exact identity, cancellation, stale/result preservation, and cited findings. Exit when delayed and
faulted providers cannot block work or produce false clear.

### Phase 3 - React Workbench and Task Detail

Implement explicit input modes, ordered shortcuts, canonical category cards, Task detail panels and
accessible responsive states. Remove only Workbench legacy bindings. Exit when full UI journeys pass.

### Phase 4 - MCP, Lineage, and Knowledge

Replace Problem/Solution events and resources with Task contracts, graph lineage, separate Problem
resolution, and exact Knowledge publication. Exit when desktop and MCP see the same revisions.

### Phase 5 - Documentation and Final Validation

Update all bilingual current docs and characterization coverage. Build release once; run packaged E2E
and UI/model review against that artifact. If provider configuration is absent, record model QA as
blocked without inserting fallback output into quality results.

## Verification Strategy

- Write contract/domain/migration tests before each implementation slice, including generated large
  fixtures, failures, retries, concurrency and byte/hash checks.
- Exercise all spec acceptance paths in React interaction tests and packaged desktop E2E, including
  pointer, keyboard, restart, narrow/wide, Korean/English, empty/loading/error and injected failure.
- Measure persistence/projection p95 and prove background job events do not alter recency.
- Use a fixed final-provider corpus: 12 document publications and 24 conflict cases (8 conflict,
  8 compatible, 8 insufficient-evidence). Record median/p95 latency, citation presence, classification
  accuracy, false-clear count, and rubric scores for fidelity, evidence, structure, and readability.
- Reuse the same signed/packaged release artifact for desktop E2E and real-app review. Report missing
  configuration or provider failure as a blocked row, never as a deterministic model-quality pass.

## Risks and Controls

| Risk | Control |
|---|---|
| Legacy data loss or semantic invention | Consistent backup, manifest, transaction rollback, invariant ledger, explicit Problem-only refinement bridge |
| Two domain implementations | One Rust aggregate service; thin router; React-only Workbench ownership |
| Late AI output shown as current | Exact identity comparison, cancellation request, stale persistence, previous-current preservation |
| Background work changes recency | User activity allowlist and clock-controlled repository/E2E tests |
| Breaking route drift across desktop/MCP | Typed application contract and shared service-level contract tests |
| Final QA silently uses fallback | Inspect configured-provider availability without exposing secrets; fixed corpus and explicit blocked result |

## Complexity Tracking

No constitution violation requires an exception. Additional revision, migration, and decision tables
encode required provenance; no new dependency or application boundary is introduced.
