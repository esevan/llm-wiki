# Product Spirit in LLM Wiki

**English** | [한국어](product-spirit.ko.md)

Product Spirit is the first test for every product and engineering decision. The
[constitution](../.specify/memory/constitution.md) turns these principles into a mandatory review gate.

## 1. You talk. The work organizes itself.

Capture accepts natural thought. AI conversation and Refinement discover structure, preserve the
speaker's intent, and produce editable proposals. The user reviews the organization instead of
performing it up front.

## 2. Reduce cognitive load.

Capture remains deliberately small. The Workbench shows only Capture, Problem, and Solution, while
In Progress Solutions receive a dedicated highlight. Detail, validation, and completion controls
appear only when the current decision needs them.

![The Workbench keeps Capture light and gives current Solutions visual priority](features/images/02-workbench.png)

## 3. Resume where you left off.

Solution Work Log accepts text, screenshots, comments, and validation checks. Refinement Preview
keeps prior decisions, evidence, constraints, and trade-offs visible. Conflict review compares the
current Solution with searchable Knowledge, so the user does not reconstruct context manually.
The global Korean/English setting changes the language without discarding the active view, unsaved
input, or workflow lineage. Newly approved, AI-generated Problems and Solutions keep both stored
versions. Explicit AI Image Summaries also store both languages in their existing request, while
the Work Log's authored evidence and legacy content remain readable in their original form.

![A Solution Work Log preserves the latest visual state and validation context](features/images/06-work-log.png)

![Refinement Preview keeps prior context beside the active conversation](features/images/05-refinement-preview.png)

## 4. Organize around problems, not tasks.

The durable workflow is **Capture → Problem → Solution**. A Solution owns its Work Log and validation
checklist. Execution never becomes a separate planning hierarchy, so every action remains attached
to the Problem that explains why it matters.

## 5. Private process, portable knowledge.

Chats, drafts, refinements, and progress remain private local process. Human-approved completion
creates an Obsidian-compatible Playbook and raw evidence bundle. That Markdown remains useful
without LLM Wiki and can be searched as Knowledge for future conflict review.
App-managed Knowledge uses English Markdown as its canonical portable source. A Korean reading
version is derived on request and can be reused only for the exact current source; it never replaces
or rewrites the canonical file.

![Completion review separates private evidence assessment from the human publication decision](features/images/03-completion-archive.png)

## 6. Understand the work, never score the worker.

Compass explains goals, evidence, milestone events, and direction. It must never convert those
signals into an employee score, productivity rank, or personal judgment. Future team features must
preserve this boundary in language, data, access, and visualization.
