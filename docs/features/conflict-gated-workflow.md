# Task-centered Workbench

**English** | [한국어](conflict-gated-workflow.ko.md)

## Capture and shape work

Capture a thought, problem, or existing solution without choosing a mandatory sequence of stages.
The compact two-line Workbench entry keeps the Capture and Task choices with its save action below
the text, where the row wraps when the available width is narrow.
Simple work can become a **Task** immediately. Use [Refinement](refinement-preview-status.md) when
more context or a reviewable proposal is useful; preserve solutions already recorded in the Capture.
Press **Enter** in the Workbench entry to save it; use **Shift+Enter** for a line break. Enter during
IME composition is left to the composition, so the text can be completed safely.

A Capture can lead to multiple Problems and Tasks. Problems have immutable revisions, and Tasks
link to the exact revisions they address. A Task can also refer to another Task as a prerequisite
or related work. These links preserve the flow of work without imposing a one-to-one relationship.
Related work is visible and can be unlinked from either Task; prerequisites retain their direction.

## Resume from the board

Workbench places shortcuts to work in progress first, followed by saved refinement work. These
shortcuts open the same records shown in the categorized main list; they are not duplicate Tasks.
Category redesign is outside this change.

A Task is distinct from a Task in progress. Start work explicitly, then complete it with evidence;
a completed Task can be reopened. Refinement readiness shows which preparation fields are filled
and which still need attention. A field can be marked not applicable with a reason. Readiness helps
resume work and does not create another mandatory approval gate.

## Keep the work record

Opening a Task divides the workspace equally between the Workbench and detail when space permits;
the detail resizes the Workbench instead of covering it. Each side scrolls independently. The detail has
**Work**, **Details**, and **Review** tabs: Work groups checklist, Work Log, and decisions;
Details reads the definition first and enters an explicit edit mode; Review groups readiness,
conflict review, completion, and Knowledge. Narrow windows replace the Workbench with the detail
until it is closed. Task-local drafts, the selected tab, and panel position remain available while
the Workbench is open.
In-progress Tasks initially open Work, Tasks not yet started open Details, and completed Tasks open
Review. Later openings restore the Task's selected tab. Work shows up to five unfinished checklist
items initially; expand the list to see all items, including completed ones.

Task detail keeps six editable definition fields—title, detail, outcome, scope, non-goals, and
validation criteria—beside Work Log entries, attachments, comments, checklists, and decisions.
Press **Enter** in a Work Log entry to add it; use **Shift+Enter** for a line break.
Work Log entries appear newest first, and each recorded entry shows its date and time in the local
locale.
Edits stay local until **Save changes**. An independent saved mutation or refresh preserves a dirty
definition draft; it does not silently replace those six fields. Saving sends the draft's expected
Task revision. If the same field changed elsewhere, compare the latest record with the user's draft
and deliberately keep the draft or use the latest value. A failed save keeps the input and its
error visible for retry.

Closing Task detail or selecting another Task with a dirty definition asks the user to **Save**,
**Discard**, or **Keep editing**. The shell keeps an open detail mounted while its route is hidden,
so normal navigation does not discard that draft. This is explicit-save protection for Task
definition fields, not Task autosave: it does not promise durability on app quit/crash or cover
every auxiliary form.

Use **Add completion evidence** to begin completion. Completing a Task does not automatically resolve its linked Problems or complete related Tasks.
Resolve a Problem separately against its current revision and evidence. An older linked revision
cannot be resolved as though it were the current one. Creating a private Knowledge draft and
publishing it are also separate explicit decisions; see [Completion and Knowledge](completion-writeback-archive.md).

## Review conflicts without blocking work

Conflict review runs asynchronously and preserves its attempts and evidence. Cancel or retry a
review from its controls. A result becomes stale when the relevant Task or Capture, Vault evidence,
or review scope changes. Failure does not turn missing evidence into a clear result, and a conflict
does not automatically approve or block a Task.

Queue can also open reports retained from the previous workflow. Their visible conflict-decision
controls save the user's decision and note; they do not restore the retired Problem approval flow.

## Remove a Workbench item

Each Capture, Task, and migrated Problem card has a Delete action. Task detail also offers Delete. The confirmation shows the item title; Keep item or Escape cancels it. Unsaved Task changes first require a save, discard, or keep-editing choice. Confirmed deletion preserves the original record and hides it from the Workbench. Cancelling leaves the card in place; a failed request leaves the confirmation open with the error. Confirmed items disappear after refresh, while related Knowledge records and vault files remain intact. A pending refinement result cannot make a deleted item visible again.

Related Spec Kit: [012 — Task-centered Workbench](../../specs/012-task-centered-workbench/spec.md).
Historical contract: [002 — Conflict-Gated Workflow](../../specs/002-conflict-gated-workflow/spec.md).

Refinement also occupies a workspace column rather than floating above the Workbench. Capture refinement shows Workbench and refinement in equal columns; Task refinement keeps the mounted Task detail on the left and refinement on the right, preserving unsaved Task inputs and the selected tab. Narrow windows show refinement with a return action. Conversation and a document-style proposed-result preview are visible together, with the private saved note in a disclosure. Each proposal remains unapplied until explicitly accepted; editing updates its actual fields. Accepted Task changes refresh the retained detail. Workbench-only legacy Problem refinement uses the same dock layout with its existing preview and APIs.
