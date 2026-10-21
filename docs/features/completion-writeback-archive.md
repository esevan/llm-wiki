# Completion and Knowledge

**English** | [한국어](completion-writeback-archive.ko.md)

A Task keeps Work Log entries, attachments, comments, checklist items, and decisions together.
Complete a standalone Task explicitly with evidence when its work is done. Completion changes that Task's
state; it does not resolve linked Problems, complete sibling Tasks, or publish a Vault document.
The Capture and exact linked Problem revisions remain available as provenance.


### Automatic parent Knowledge for Subtasks

Accepting an explicit Subtask enables automatic parent Knowledge publication on each evidence-backed child completion. Each revision accumulates completed-child evidence in the same parent document. The parent closes only after all children finish; reopening a child reopens completed ancestors. Publication is recoverable through a durable outbox and preserves external file edits. Standalone Tasks retain the separate review-and-publish flow below. See [Subtask lifecycle](refinement-preview-status.md#explicit-subtasks).

## Explicit one-click publication

Generate or edit a private Knowledge draft, then choose **Publish Knowledge** once.
Quality, citation and source-freshness notes remain nonblocking metadata. User
experience is a primary source; external citations are optional. Legacy and failed
drafts can publish without regeneration. Storage/hash/version conflicts still
protect files and pointers. [Publication and recovery](knowledge-archive.md).

## Correct, regenerate, or withdraw

Correct the draft deliberately. Regeneration refreshes the draft from the current recorded work and
provenance. If a published file has changed externally, exact file/hash checks prevent silently
overwriting that edit.

Withdrawal is also explicit and preserves a recoverable local copy. It does not erase the completed
Task or its private work evidence. Publishing, regeneration, and withdrawal have their own results
and errors; standalone Task completion or reopening does not imply them.

See [Lineage Knowledge Layer](lineage-knowledge-layer.md) for provenance and the
[interactive coverage record](../testing/interactive-coverage.md) for actual verification evidence.

Related Spec Kit: [012 — Task-centered Workbench](../../specs/012-task-centered-workbench/spec.md).
Historical contract: [003 — Completion, Writeback, and Archive](../../specs/003-completion-writeback-archive/spec.md).
