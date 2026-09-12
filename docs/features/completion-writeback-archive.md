# Completion and Knowledge

**English** | [한국어](completion-writeback-archive.ko.md)

A Task keeps Work Log entries, attachments, comments, checklist items, and decisions together.
Complete the Task explicitly with evidence when its work is done. Completion changes that Task's
state; it does not resolve linked Problems, complete sibling Tasks, or publish a Vault document.
The Capture and exact linked Problem revisions remain available as provenance.

## Review and publish separately

Create a private Knowledge draft from completed work, review its content, and make corrections
before publishing. The generated Markdown includes the Task's recorded outcome, context, scope,
non-goals, validation, work evidence, checklist, decisions, completion evidence, and exact provenance.
Unrecorded fields are identified as unrecorded rather than filled with invented claims.

Publishing is a separate explicit action on the reviewed draft. It writes Markdown to the Vault,
where it remains usable outside LLM Wiki. A draft revision and source hash identify the content
being published; publication must not silently use an unreviewed revision.

## Correct, regenerate, or withdraw

Correct the draft deliberately. Regeneration refreshes the draft from the current recorded work and
provenance. If a published file has changed externally, exact file/hash checks prevent silently
overwriting that edit.

Withdrawal is also explicit and preserves a recoverable local copy. It does not erase the completed
Task or its private work evidence. Publishing, regeneration, and withdrawal have their own results
and errors; none is implied by merely completing or reopening a Task.

See [Lineage Knowledge Layer](lineage-knowledge-layer.md) for provenance and the
[interactive coverage record](../testing/interactive-coverage.md) for actual verification evidence.

Related Spec Kit: [012 — Task-centered Workbench](../../specs/012-task-centered-workbench/spec.md).
Historical contract: [003 — Completion, Writeback, and Archive](../../specs/003-completion-writeback-archive/spec.md).
