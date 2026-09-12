# MCP and Work-Tracking Contract

MCP and desktop call the same Task application service. Work-tracking durable event kinds are
`CaptureCreated`, `TaskCreated`, `TaskRevisionProposed`, `TaskLinkedToProblem`, `WorkLogCheckpoint`,
`TaskTransitionProposed`, `TaskCompleted`, `ProblemResolutionProposed`, and Knowledge events.
Internal autosave and provider job progress are not workflow-advance events.

`workbench_current` and `workbench_overview` return typed Capture/Task/legacy-refinement items,
Task state and revision, last user activity, exact Problem links, Task relationships, Work Log /
checklist summaries, readiness, and pending user decisions. Existing topic/evidence scope, cursor snapshot, expiry, idempotency and exact elicitation
review rules remain. A target snapshot always includes Task or draft revision and relevant hashes.

Proposal tools may prepare changes but cannot create durable Task/Problem revisions, transition Task,
resolve Problem, or publish Knowledge without exact user approval. A changed target returns the same
`head_conflict`/`draft_conflict` semantics as desktop. Projection replay is append-only, ordered and
idempotent. Background projection and job events never change `lastUserActivityAt`.

ProblemDraft/SolutionDraft and approval-gate schemas are superseded. New legacy adopt/approve
Problem/Solution actions, mandatory conflict gates, and `verify_and_complete` are rejected.
Connection scope names remain
when renaming would revoke existing grants, but descriptions and payloads use Task terminology; all
schema descriptions, resources, workflow skill references, UI copy and contract tests change in the
same release.

`task_context_read {taskId}` reads one authorized SQLite snapshot containing current Task revision fields, shared readiness calculation, Work Log and comments, attachment metadata, checklist, decisions, completions, and exact Problem/Task links. Binary data is excluded; text is limited to 4,000 characters per field and arrays to 100 rows, with explicit `truncated`. The source hash covers the full underlying child material before bounding.

`inbound_work_open` uses the closed modes `create`, `resume`, and `continue_task`. Accepted
`continue_task` binds the exact visible existing Task to a connection-owned session with nullable
Capture; rejection creates no session. Current/topic/overview reads discover authorized desktop
Tasks even before any tracking session exists.

Governed `inbound_work_advance` actions cover Task creation/revision/transition/reopening/completion,
Problem creation/revision/exact resolution, Problem links, Task relationships, readiness decisions,
Work Log, comments, checklist and Task decisions. Stable operation IDs replay terminal outcomes;
changed requests or stale targets require a new exact preview. Current Chat conflict review remains
advisory and requires exact scoped Vault evidence instead of a hidden provider job.

Accepted canonical Task mutations commit their event/head/applied result atomically. Native Task
mutations and refinement decisions append one minimal `desktop_task_change` event per linked
session, suppressing only the DB-resolved originating session. Replay creates no duplicate; the
event payload contains no cross-connection affected record IDs.

`task_refinement_*`, `task_advisory_*`, `task_lineage_read` and `task_knowledge_*` share native
assistance logic. Knowledge correction, publication and withdrawal require exact content and
source hashes. Private draft acceptance and public publication are separate decisions. Retained
Knowledge tool aliases delegate to canonical Task drafts; historical approved publication jobs
recover only their recorded exact draft with external-file guards. Other retired queued actions
remain readable but cannot mutate through the old funnel.
