# Implementation Plan: Workflow Foundation

**Branch**: `015-workflow-foundation` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/015-workflow-foundation/spec.md`

## Summary

Add a shared Rust contract layer for prompt identity/versioning, structured input/output validation, retrieval intent and source attribution, operation durability, version provenance, exact-head checks, references, and decision status. Extend the existing SQLite-backed `ai_jobs_v2` queue additively with prompt/source metadata and separate execution/application outcomes. Migrate current structured provider calls to the registry while retaining their current provider routes, queue behavior, canonical revision tables, and user-facing flows.

## Technical Context

**Language/Version**: Rust 2021; TypeScript 5.9 only for unchanged consumers

**Primary Dependencies**: Existing `serde`, `serde_json`, `rusqlite`, `reqwest`, `tokio-util`, and SHA-256 utilities; no new dependency

**Storage**: Existing local SQLite schema and migration runner; additive migration 16

**Testing**: Focused Rust unit and native integration tests via `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check`

**Target Platform**: Local Tauri desktop application on macOS and Windows

**Project Type**: Desktop application with Rust-owned workflow domain and native provider boundary

**Performance Goals**: Registry lookup and contract validation remain in-process and sub-millisecond for normal payloads; capture persistence and search hot paths perform no provider or semantic-model work

**Constraints**: Preserve current records and behavior; reuse the existing queue; no UI redesign; no full E2E or release build; no duplicate canonical draft or decision store

**Scale/Scope**: One shared contract module, one native persistence/queue integration module, additive queue/provenance/reference schema, and adoption by current structured native AI invocations

## Constitution Check

- **Product Spirit I / III**: Shared version, provenance, and resumability contracts reduce the organization users must perform and retain exact context needed to resume.
- **Product Spirit IV**: Contracts model Task work independently and do not make refinement or review a mandatory workflow gate.
- **Product Spirit V**: Generated output remains private process until an existing explicit publication action; the foundation never publishes.
- **Product Spirit VI**: No worker scoring or person-level ranking is introduced.
- **Guardrail A**: Registry lookup is synchronous local data work; no AI import or invocation enters capture/search hot paths. Focused timing assertions cover validation overhead.
- **Guardrail B**: Provider HTTP remains behind existing native provider helpers; no provider-specific domain coupling is added.
- **Guardrail C**: Application disposition distinguishes `review_needed`, `applied`, and `superseded`; AI execution never owns workflow state or publication.
- **Guardrail D**: Source references bind stable IDs and exact versions; stale CAS prevents late application.
- **Guardrail E**: SQLite-only additive schema and Rust logic are cross-platform.
- **Guardrail F**: The existing queue, canonical revision tables, cancellation registry, and migration recovery are reused; no dependency or competing engine is added.
- **Durable migration**: Migration 16 adds nullable/defaulted columns and auxiliary provenance/reference rows without rebuilding canonical records. Existing backup, transaction, foreign-key validation, and retry recovery remain authoritative.

Post-design re-check: PASS. The design deliberately avoids a generic replacement draft or decision table. Existing `task_revisions`, `refinement_drafts`, `task_knowledge_drafts`, `knowledge_drafts`, `task_decisions`, and `work_tracking_decisions` stay canonical; foundation metadata attaches through stable owner identity/version.

## Project Structure

### Documentation (this feature)

```text
specs/015-workflow-foundation/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── prompt-registry.md
│   └── workflow-foundation-api.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── workflow_foundation.rs
├── workflow_foundation/prompts/  # Feature-owned definitions, central lookup
├── lib.rs
├── provider.rs
└── native/
    ├── workflow_foundation.rs
    ├── jobs.rs
    ├── task_assistance.rs
    ├── migrations.rs
    ├── schema.sql
    └── mod.rs

docs/features/
├── workflow-foundation.md
└── workflow-foundation.ko.md
```

**Structure Decision**: The domain contract is public at the Rust crate root so application and native modules share one vocabulary. Persistence stays under `native` beside the existing SQLite queue. Current canonical content and decision repositories remain in place.

## Performance and Risk Plan

- Benchmark prompt lookup/build/validation with repeated local calls; require no observable 15% regression in existing binding performance tests.
- Verify migration from schema 15, fresh schema 16, foreign keys, and preservation of existing job rows.
- Exercise stale CAS, restoration provenance, duplicate coalescing, latest-wins cancellation, and execution/application separation with focused tests.
- Skip E2E and release builds because there is no UI/native boundary change or packaging change; focused native tests directly cover the remaining persistence and queue risks.

## Complexity Tracking

No constitution violations or new dependencies require justification.
