# Completion and Knowledge

**English** | [한국어](completion-writeback-archive.ko.md)

A Task keeps Work Log entries, attachments, comments, checklist items, and decisions together.
Complete a standalone Task explicitly with evidence when its work is done. Completion changes that Task's
state; it does not resolve linked Problems, complete sibling Tasks, or publish a Vault document.
The Capture and exact linked Problem revisions remain available as provenance.


### Automatic parent Knowledge for Subtasks

Accepting an explicit Subtask enables automatic parent Knowledge publication on each evidence-backed child completion. Each revision accumulates completed-child evidence in the same parent document. The parent closes only after all children finish; reopening a child reopens completed ancestors. Publication is recoverable through a durable outbox and preserves external file edits. Standalone Tasks retain the separate review-and-publish flow below. See [Subtask lifecycle](refinement-preview-status.md#explicit-subtasks).

## Review and publish separately

Create a private Knowledge draft from completed work, review its readable Markdown preview, and make corrections
before publishing. Generation enters the durable **AI Queue** immediately, where its queued, running,
failed, retry, and completed states remain visible. The completed Queue result opens the exact draft
revision it produced; if the draft was later edited, published, or regenerated, that original result stays read-only. Previously saved private drafts
also reopen with their Markdown preview and editable source. The generated Markdown includes the Task's recorded outcome, context, scope,
non-goals, validation, work evidence, checklist, decisions, completion evidence, and exact provenance.
Unrecorded fields are identified as unrecorded rather than filled with invented claims. A queued job captures the
requested Task revision and rejects a changed source snapshot rather than drafting from newer unreviewed work.

Publishing is a separate explicit action on the reviewed draft. It writes Markdown to the Vault,
where it remains usable outside LLM Wiki. A draft revision and source hash identify the content
being published; publication must not silently use an unreviewed revision.

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
