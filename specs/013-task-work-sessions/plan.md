# Implementation Plan: Task Work Sessions

**Branch**: `feature/task-session-mvp` | **Date**: 2026-09-21 | **Spec**: [spec.md](spec.md)

## Summary

Add an explicit Sessions tab to Task detail. A Task-owned local aggregate stores multiple sessions, future execution settings, ordered typed records, and bounded attachments. The existing Task aggregate remains the live context. Nothing invokes AI, reads the configured project folder, changes Task state, or writes Work Log automatically.

## Technical Context

**Language/Version**: TypeScript/React 19; Rust 2021/Tauri 2

**Primary Dependencies**: Existing React, Tauri invoke application boundary, rusqlite, serde_json; no new dependency

**Storage**: Existing local SQLite database, schema migration 14; attachment bytes in app-managed storage

**Testing**: Vitest/Testing Library; Cargo tests; one targeted packaged restart scenario

**Target Platform**: macOS and Windows desktop

**Project Type**: Tauri desktop application with React UI and Rust application service

**Performance Goals**: Work-session projection within 100 ms p95 after local data arrives; 10 MB per attachment

**Constraints**: Offline/local-first; no AI/CLI execution; no project-path traversal; no Task state/activity mutation; strict ownership; idempotent failed append

**Scale/Scope**: One Task detail tab, two local entities, five Task-scoped operations, focused tests and one smallest packaged lifecycle scenario

## Constitution Check

*GATE: Passed before research and after design.*

- Natural conversation: chat-style notes avoid taxonomy forms.
- Cognitive load: one explicit tab; no session until requested; settings remain secondary.
- Resume: saved sessions, records, settings, and attachments reopen; failed input remains retryable.
- Task ownership: every session belongs to one Task and cannot complete it, resolve Problems, or publish Knowledge.
- Private process: records remain local and unpublished.
- No worker scoring: no metrics, ranks, or judgments.
- Performance: measure the 100 ms p95 projection budget; no AI/token use.
- Adapter boundary: UI uses the application client; native Task service owns persistence; the lifecycle is provider-independent despite a Codex catalog setting.
- Human authority: approval is stored future intent only; no autonomous action.
- Cross-platform: project paths stay opaque and no platform-specific filesystem call is added.
- Minimal complexity: no dependency, execution engine, native file chooser, or run model.

## Product Spirit Assessment

The feature advances “Resume Where You Left Off” and “Tasks Carry Work.” It preserves Capture, Problem, Knowledge, and human-decision boundaries. The visible execution settings reduce future setup burden and clearly state that they are inactive.

## Budgets and Risk Controls

- Indexed local list/get operations; React Profiler checks the 100 ms p95 projection budget after data arrives.
- 10 MB attachment source limit; no AI or retrieval tokens.
- No new dependency cost.
- Stable operation identity handles uncertain append responses.
- Conflict invalidation is not applicable because sessions do not modify canonical Task revisions or context; settings use explicit last-write save for this single-user MVP.
- One packaged E2E covers only process restart/native persistence; cheaper tests cover other behavior.

## Project Structure

```text
specs/013-task-work-sessions/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md, tasks.md
├── contracts/application-api.md
└── checklists/
frontend/src/features/workbench/
├── TaskDetail.tsx, TaskWorkSessions.tsx, TaskWorkSessions.test.tsx
├── taskWorkbenchText.ts, task-workbench.css
frontend/src/services/{taskClient.ts,taskOperation.ts,taskOperation.test.ts}
frontend/src/types/taskWorkbench.ts
src-tauri/src/application/task_service.rs
src-tauri/src/native/{migrations.rs,task_work_session_schema.sql}
docs/features/{task-work-sessions.md,task-work-sessions.ko.md}
```

**Structure Decision**: Extend the established Task vertical slice. Keep session UI focused and store no Task-content snapshot.

## Complexity Tracking

No constitution violations.
