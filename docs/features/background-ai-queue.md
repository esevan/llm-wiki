# Background AI Queue

**English** | [한국어](background-ai-queue.ko.md)

LLM Wiki separates AI execution into two process-level paths so interaction stays responsive
without losing recoverable work.

- The **Fast Queue** has exactly one FIFO worker. Chat and other immediate interactions use it as
  a global request throttle. It has no database state, Queue UI entry, retry history, or
  notification.
- The **Asynchronous Queue** stores durable AI, translation, and embedding Jobs in SQLite. Its
  worker count is configurable in AI Setup and workers claim jobs with leases and heartbeats.

The bottom-right Queue names the target item and explains what each durable task is doing. Its cards
show readable status, step progress, time, safe failures, cancellation, retry, and only the result
actions that make sense for that task. A task with a result names its destination while running,
then enables a prominent **Open result page** action when complete. Results open the owning workflow
surface or a concise summary; raw job JSON is not used as the user-facing result. Draft and Refine
results stay bound to their originating dialog and are cancelled when that surface closes. Image
Summary attaches to the exact
Work Log entry without changing scroll position. Completion Review also creates a temporary toast
and a persisted unread bell alert because it requires a user decision.

Knowledge translation resumes from paragraph checkpoints and publishes the completed translation
to the Vault before deleting its SQLite working checkpoints. Capture and Work Log text enqueue
derived translations immediately; authored source text is never overwritten. Queue cards distinguish
Capture text, Work Log entries, comments, and checklist items so each translation target remains
understandable without exposing its internal ID. Embedding refresh is durable and lexical search
remains available while it runs.

Workers use bounded retry, exponential backoff, source hashes, live lease tokens, cancellation,
and idempotent notifications. Concurrent workers use isolated application/SQLite connections;
SQLite writer contention is retried rather than published as a terminal failure. When the optional
semantic runtime is absent, embedding work completes with lexical fallback and zero semantic
coverage. AI output remains a proposal or derived representation: workflow
state, approval, completion, and Knowledge decisions remain under user control.

See [feature specification](../../specs/009-background-ai-queue/spec.md) and
[worker contract](../../specs/009-background-ai-queue/contracts/worker-contract.md). The
[backend architecture guide](../architecture.md) maps every durable task to its authoritative
handler module and documents the enforced dependency rules.
