# Completion, Knowledge, and archive

**English** | [한국어](completion-writeback-archive.ko.md)

> **Private process, portable knowledge.** Working context stays private; completion creates reusable Knowledge.

![Completion review presents evidence and leaves the final decision with the human](images/03-completion-archive.png)

An In Progress Solution owns a Work Log with text, screenshots, comments, and a validation checklist.
AI can summarize a screenshot and assess each validation criterion, but it cannot complete the work.

![The Solution Work Log keeps visual progress and validation evidence together](images/06-work-log.png)

Human-approved completion creates an Obsidian-compatible Playbook plus raw evidence. Regeneration,
structured Markdown patches, and archive moves use source hashes to block accidental overwrites of
external edits. Completed Knowledge remains portable and searchable without LLM Wiki.
If a tracked completed-work file is missing, its Solution remains visible: regeneration queues a
replacement in the background, while deletion clears the stale generated-file record. Once no
tracked path remains, the unavailable delete action is no longer shown.

Completing from a Solution closes the whole linked work chain in one database transaction. Every
open Solution belonging to the origin Problem and the Problem itself receive the explicit
`completed` state; an already archived Solution keeps its archival state. The originating Capture
remains preserved for Lineage but, because it has been refined into that Problem, is no longer an
open inbox item. The command result reports the closed Solution, Problem, and Capture identifiers
so clients do not need to infer whether the cascade succeeded.

Before the final report is generated, LLM Wiki rebuilds the current Lineage and uses its referenced
evidence as the report input. Regeneration refreshes Lineage and the document together. See
[Lineage Knowledge Layer](lineage-knowledge-layer.md).

Related Spec Kit: [003 — Completion, Writeback, and Archive](../../specs/003-completion-writeback-archive/spec.md)
