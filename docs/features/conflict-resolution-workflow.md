# Task conflict review

**English** | [한국어](conflict-resolution-workflow.ko.md)

Conflict review is advisory, revision-bound evidence for a Task. Run a review from Task detail; the panel shows the latest result, citations, status, and earlier attempts. A Task can continue while the review is queued, running, failed, cancelled, or reports findings. The review never approves, blocks, completes, or publishes the Task.

![Task conflict review with cited findings, a Retry review action, and attempt history](images/task-review.en.png)

## Read and retry a review

Each result identifies the Task revision and Vault context it examined. Changing relevant Task or Vault evidence makes a prior result stale, so run a fresh review before relying on it. A failed or cancelled attempt stays visible with its status and can be retried. Citations are evidence for the user's judgment; the AI cannot decide the outcome or invent a source.

Current Task detail does not provide per-conflict accept/apply controls or a clear-conflict gate. Those controls belong only to retained legacy Queue/report records and do not define the current Task workflow.

See [Task-centered Workbench](conflict-gated-workflow.md) and the historical [conflict-resolution record](../../specs/010-conflict-resolution-workflow/spec.md).
