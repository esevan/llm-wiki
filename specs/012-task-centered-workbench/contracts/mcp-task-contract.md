# MCP and Work-Tracking Contract

MCP and desktop call the same Task application service. Work-tracking durable event kinds are
`CaptureCreated`, `TaskCreated`, `TaskRevisionProposed`, `TaskLinkedToProblem`, `WorkLogCheckpoint`,
`TaskTransitionProposed`, `TaskCompleted`, `ProblemResolutionProposed`, and Knowledge events.
Internal autosave and provider job progress are not workflow-advance events.

`workbench_current` and `workbench_overview` return typed Capture/Task/legacy-refinement items,
Task state and revision, last user activity, exact Problem links, Task relationships and pending user
decisions. Existing topic/evidence scope, cursor snapshot, expiry, idempotency and exact elicitation
review rules remain. A target snapshot always includes Task or draft revision and relevant hashes.

Proposal tools may prepare changes but cannot create durable Task/Problem revisions, transition Task,
resolve Problem, or publish Knowledge without exact user approval. A changed target returns the same
`head_conflict`/`draft_conflict` semantics as desktop. Projection replay is append-only, ordered and
idempotent. Background projection and job events never change `lastUserActivityAt`.

ProblemDraft/SolutionDraft and approval-gate schemas are superseded. Connection scope names remain
when renaming would revoke existing grants, but descriptions and payloads use Task terminology; all
schema descriptions, resources, workflow skill references, UI copy and contract tests change in the
same release.
