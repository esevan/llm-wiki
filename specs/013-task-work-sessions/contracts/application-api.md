# Application Contract: Task Work Sessions

URL identifiers override request-body identifiers at the application trust boundary.

## Operations

- `GET /tasks/{taskId}/work-sessions` returns `{ sessions }`, newest updated first, without creation.
- `POST /tasks/{taskId}/work-sessions` accepts `{ operationId, title }`; defaults to `codex`, `gpt-5.6-sol`, `ask`, and empty path; replay returns the same result.
- `GET /tasks/{taskId}/work-sessions/{sessionId}` returns `{ session, entries }`; another Task's session is not found.
- `PUT /tasks/{taskId}/work-sessions/{sessionId}` accepts `{ operationId, title, provider, model, approvalMode, workspacePath }` and never inspects the path or runs a provider.
- `POST /tasks/{taskId}/work-sessions/{sessionId}/entries` accepts `{ operationId, author, kind, body, attachment? }`; MVP client sends `user` + `note`; retry keeps the same operation ID.

Supported provider is `codex`; model IDs are `gpt-6-astra`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`, and `gpt-5.5`.

## Invariants and errors

Append does not change Task state/activity, Work Log, completion, Problem, or Knowledge state. Errors are `Task not found`, `Work session not found`, `invalid_input: ...`, or `operation_conflict`.
