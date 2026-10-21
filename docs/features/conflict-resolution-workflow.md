# Task conflict review

**English** | [한국어](conflict-resolution-workflow.ko.md)

Conflict review is advisory, revision-bound evidence for a Task. The retained panel contract describes the latest result, citations, status and earlier attempts; it is not currently mounted in Task detail. A Task can continue while the review is queued, running, failed, cancelled, or reports findings. The review never approves, blocks, completes, or publishes the Task.

![Task conflict review with cited findings, a Retry review action, and attempt history](images/task-review.en.png)

> Historical screenshot: `ConflictReviewPanel` is retained as a component/API contract but is not mounted by the current App at `584ae3c`. The image is preserved as previous-interface evidence, not an available Task screen. See the [image audit](../testing/documentation-image-audit.md).


## Read and retry a review

Each result identifies the Task revision and Vault context it examined. Changing relevant Task or Vault evidence makes a prior result stale, so run a fresh review before relying on it. A failed or cancelled attempt stays visible with its status and can be retried. Citations are evidence for the user's judgment; the AI cannot decide the outcome or invent a source.

Current Task detail does not provide per-conflict accept/apply controls or a clear-conflict gate. Those controls belong only to retained legacy Queue/report records and do not define the current Task workflow.

See [Task-centered Workbench](conflict-gated-workflow.md) and the historical [conflict-resolution record](../../specs/010-conflict-resolution-workflow/spec.md).
