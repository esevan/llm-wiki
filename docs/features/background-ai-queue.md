# Background AI Queue

**English** | [한국어](background-ai-queue.ko.md)

LLM Wiki separates AI execution into two process-level paths so interaction stays responsive
without losing recoverable work.

- The **Fast Queue** has exactly one FIFO worker. Chat and other immediate interactions use it as
  a global request throttle. It has no database state, Queue UI entry, retry history, or
  notification.
- The **Asynchronous Queue** stores durable AI, translation, and embedding Jobs in SQLite. Its
  worker count is configurable in AI Setup and workers claim jobs with leases and heartbeats.

The bottom-right Queue shows durable work, progress, safe failures, cancellation, retry, and a
task-specific result action. Draft and Refine results stay bound to their originating dialog and
are cancelled when that surface closes. Image Summary attaches to the exact Work Log entry without
changing scroll position. Completion Review also creates a temporary toast and a persisted unread
bell alert because it requires a user decision.

Knowledge translation resumes from paragraph checkpoints and publishes the completed translation
to the Vault before deleting its SQLite working checkpoints. Capture and Work Log text enqueue
derived translations immediately; authored source text is never overwritten. Embedding refresh is
durable and lexical search remains available while it runs.

Workers use bounded retry, exponential backoff, source hashes, live lease tokens, cancellation,
and idempotent notifications. AI output remains a proposal or derived representation: workflow
state, approval, completion, and Knowledge decisions remain under user control.

See [feature specification](../../specs/009-background-ai-queue/spec.md) and
[worker contract](../../specs/009-background-ai-queue/contracts/worker-contract.md).
