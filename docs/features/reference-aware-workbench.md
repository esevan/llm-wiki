# Reference-aware work previews

[한국어](reference-aware-workbench.ko.md) | **English**

Refinement now prepares its work proposal through a retrieval plan, relevant local
Vault evidence, and a structured final preview. Necessary search finishes before a
useful preview is saved. Missing evidence is represented through assumptions; an
unnecessary search, an empty search and a failed operation remain distinct states.
The canonical Task changes only after an explicit apply.

Each generated preview stores description, background, goal, scope, non-goals,
constraints, completion criteria, initial approach and assumptions. Used claims retain
exact document versions and sections. Finding a document does not mark it used or
adopted. A source, Task, conversation or saved preview change during generation keeps
the late result from replacing the current preview.

Optional investigation runs independently of the conversation and does not retry
forever. It classifies evidence as critical, supporting or ancillary. Current critical
and supporting findings can ground the next reply once; ancillary evidence remains in
the reference data. A newer conversation makes unfinished older investigation stale.

The native application supports reading, comparing, editing and restoring immutable
preview versions. Restore appends a new preview and leaves Task history unchanged.
Explicit apply checks the current preview, content hash, Task revision, conversation,
hierarchy and sources in one transaction. Failed checks preserve the user's work.
Opening Refinement starts the first preview automatically when no preview or pending
job exists. The workspace reloads saved previews and background findings while the
conversation remains available. Editing keeps the original preview version and hash;
a newer background result cannot overwrite an unsaved edit. Save or cancel the edit
before leaving, and use Apply to Task when the current preview is ready.

Type @ in the message composer to search local documents and sections. Arrow keys and
Enter insert an exact mention without sending. Text and mention bindings are saved
before Send and restored when Refinement reopens. Only sent mentions enter conversation
context. References group passages under exact document versions, with 20 documents per page.
Text, status, evidence aspect, information type and recorded-use filters remain
available with zero results. New sources and sections wait behind Show new references;
accepting them or changing a filter/sort is explicit and preserves reading order. Check references starts another bounded optional
investigation without blocking the composer. Exact sources remain openable after an
indexed file move; unavailable old content produces a recoverable read error.

See the [feature specification](../../specs/019-reference-aware-workbench/spec.md) and
[native adapter contract](../../specs/019-reference-aware-workbench/contracts/application-api.md).

The reference viewer keeps exact document-version and section bindings through
back/forward navigation. Loading failures offer retry, and closing restores focus to
the source control. Version comparisons request both saved versions explicitly;
restoring a historical version is a separate action from applying a Task change.

Select several lookup results and use Insert selected mentions to add them in selection
order, or press Enter/Space in the composer to insert the active result. Lookup rows
include source identity and version to distinguish duplicate titles. Backspace/Delete
removes a whole bound token; editing its text or pasting a readable label does not
silently create a source binding. The preview shows each assumption's saved status
separately from its basis. Recorded viewing and mentioning remain distinct from actual
use, adoption and exclusion in the reference filters.

Actual use follows a Capture into the Task created by its recorded acceptance or apply
decision. Sharing an origin Capture does not share references between Tasks. Knowledge
retains exact document versions and sections, combining use and counterevidence with
the stored statements they grounded. Viewing, mentioning and discovery alone do not
create publication links. An explicit exclusion remains effective until the user marks
that source used again; automatic generation or adoption does not undo it.
