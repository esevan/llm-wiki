# Implementation Plan: Task Codex Execution

**Branch**: `feature/task-session-execution` | **Date**: 2026-09-21 | **Spec**: [spec.md](spec.md)

## Summary

Extend the existing Task Sessions tab with a supervised Codex app-server connection. Each Task work session binds to one exact Codex thread; each explicit instruction creates one durable local Run and one canonical Task Work Log item before a turn is dispatched. The same connection streams real events and formal requests, resumes exact threads, interrupts exact turns, and stores completed-item evidence plus the turn's final report. Work Log displays joined run metadata without overwriting its initial body or manual content.

## Technical Context

**Language/Version**: TypeScript/React 19; Rust 2021/Tauri 2; Codex CLI 0.154.0 protocol observed for implementation

**Primary Dependencies**: Existing Tauri invoke/Channel, Tokio process and synchronization primitives, rusqlite, serde/serde_json; installed `codex app-server`; no new crate or npm dependency

**Storage**: Existing local SQLite database, additive schema migration 15; Codex thread state remains owned by Codex

**Testing**: Vitest/Testing Library, Cargo unit/integration tests, protocol fixtures, opt-in isolated live Codex integration, targeted packaged desktop scenario, CUA rendered review

**Target Platform**: macOS and Windows desktop

**Project Type**: Tauri desktop application with React UI and one supervised local application boundary

**Performance Goals**: Existing Task/Work Log projections remain under 100 ms p95 after local data arrives; UI reconnect shows saved Run state within one second; partial text is buffered until a completed item instead of being persisted or displayed unsafely

**Constraints**: Explicit execution only; exact Task/session/thread ownership; one nonterminal Run per session; no automatic resend after uncertain dispatch; no automatic Task completion; existing approval/sandbox defaults preserved; at most eight selected evidence passages and 6,000 retrieved-context tokens when Vault evidence is used

**Scale/Scope**: One executable provider, one app-server process per desktop application, one thread per work session, one turn and one Work Log item per Run; no generic execution framework or scheduler

## Constitution Check

*GATE: Passed before research and after design.*

- Conversation stays primary: users keep one composer and separate Save note / Run with Codex actions; formal controls appear only while actionable.
- Exact thread and Run bindings serve Resume Where You Left Off; Work Log remains Task-owned and Task completion remains explicit.
- Execution detail stays private; formal responses cannot be inferred from prose; no workflow transition is automatic.
- Normal Task projection excludes unbounded raw output and retains the 100 ms p95 budget.
- Core Task/Work Log code depends on a narrow execution adapter, not Codex configuration. App-server is a supervised harness process, not a model endpoint client; existing endpoint calls remain behind `OpenAICompatibleProvider`.
- Model report and observed evidence are attributed separately; provider exit does not prove correctness.
- Records stay in local SQLite. Executable discovery, paths, shutdown, and interruption account for macOS and Windows; unavailable Windows live validation is reported.
- Existing Tokio, Tauri Channel, application service, and Work Log paths are reused; no new dependency or provider framework is added.
- Migration 15 is additive, preserves v14 session identities and bytes plus the released v13 Task journey cache, validates ownership/foreign keys, and uses the existing verified backup/restore path.

## Product Spirit Assessment

The feature directly serves “Resume Where You Left Off” and “Tasks Carry Work.” Persistent Codex threads reduce reconstructed context, while one Run-linked Work Log item keeps meaningful progress attached to the Task. Capture, Problem, completion, Knowledge, privacy, and human-decision boundaries remain intact.

## Budgets and Risk Controls

- Atomically store instruction, Run, initial Work Log row, and link before external dispatch.
- Persist and display completed provider items rather than raw deltas; live progress uses safe item/status metadata. Preserve identical adjacent fragments in the bounded internal buffer.
- Allow one nonterminal Run per session. Submission keys and payload hashes distinguish safe replay from conflict.
- Bound thread bootstrap and later context deltas; never replay the whole transcript into a resumed thread.
- Never persist raw RPC, auth material, secret input, full config, or raw stderr.
- Capability-check experimental structured user input and degrade to an ordinary later turn.
- Run one opt-in live two-turn test in an isolated repository.
- Review the UI/UX design with Astra High before substantive UI implementation. Verify the rendered UI separately, then run the selected packaged E2E with Luna. Use Astra debugging for concrete failures.

## Project Structure

```text
specs/014-task-codex-execution/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md, tasks.md
├── contracts/{application-api.md,codex-app-server.md,interaction.md}
└── checklists/

frontend/src/features/workbench/{TaskWorkSessions.tsx,TaskWorkSessions.test.tsx}
frontend/src/services/{taskClient.ts,taskExecutionClient.ts,taskOperation.ts}
frontend/src/types/taskWorkbench.ts
src-tauri/src/application/task_execution_service.rs
src-tauri/src/codex_app_server.rs
src-tauri/src/native/{task_execution_schema.sql,migrations.rs,mod.rs}
src-tauri/src/lib.rs
src-tauri/tests/task_execution.rs
frontend/src/test/{workSessionScenarios.ts,desktopScenario.ts}
```

**Structure Decision**: Extend the existing Task vertical slice. Durable Run/Work Log rules live in an application service; Codex JSON-RPC and process behavior live in one native adapter managed by Tauri.

## Complexity Tracking

No constitution violation requires justification.
