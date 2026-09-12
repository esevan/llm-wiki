# Task-centered Workbench

**English** | [한국어](conflict-gated-workflow.ko.md)

## Capture and shape work

Capture a thought, problem, or existing solution without choosing a mandatory sequence of stages.
Simple work can become a **Task** immediately. Use [Refinement](refinement-preview-status.md) when
more context or a reviewable proposal is useful; preserve solutions already recorded in the Capture.

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

Task detail keeps six editable definition fields—title, detail, outcome, scope, non-goals, and
validation criteria—beside Work Log entries, attachments, comments, checklists, and decisions.
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

Related Spec Kit: [012 — Task-centered Workbench](../../specs/012-task-centered-workbench/spec.md).
Historical contract: [002 — Conflict-Gated Workflow](../../specs/002-conflict-gated-workflow/spec.md).
