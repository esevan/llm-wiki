# Context-preserving Refinement

**English** | [한국어](refinement-preview-status.ko.md)

Refinement helps turn a Capture or Task into reviewable work while preserving its original context.
A Capture that already contains a solution does not have to discover that solution again. Simple work
can become a Task directly; refinement and Problem approval are not mandatory gates.

## Work in the refinement panel

Open **Refine** from a Capture or Task, or resume the saved refinement shortcut in Workbench.
Refinement opens in a focused dialog: the proposed result stays in a scrollable left pane while the
full conversation history and composer stay together in a right-hand chat pane. The rest of Workbench
is unavailable until the dialog closes. Keep working notes with the proposed result; the saved notes,
scroll position, and conversation history support returning later.

**Send**, `Ctrl+Enter`, and `Cmd+Enter` submit a message. Plain Enter writes another line. Send is
unavailable while the current submission or response is being processed. Every provider turn includes
the original Capture and ordered conversation history, even after a transient draft is cleared.

The dialog keeps a stable reading height up to the available window height. Longer conversations keep their own
scroll area while the composer stays at the bottom of the chat pane. Short conversations sit directly above
the composer; new messages appear below earlier messages, moving the history upward. While a response is being prepared, the conversation shows
an accessible localized status with an animated ellipsis. The current refinement endpoint returns a complete
assistant message after polling; when that new message arrives, the panel reveals it progressively as a
presentation effect. Stored conversation history is shown immediately and is not replayed. Opening the
refinement dialog keeps the underlying Workbench scroll position fixed until the dialog closes.

A background preview can produce proposals. Review each proposal separately: edit, accept, or reject it.
Accepting one proposal does not accept its siblings. Chat responses and proposal retrieval proceed independently; a delayed read still displays the latest proposals.
Task proposal previews show the complete meaningful Task result fields—title, detail, outcome, scope,
non-goals, and validation criteria—including responses that return the detail as `body` or nest the result;
accepting the proposal preserves those values in the Task detail.

## Refine the same Task and focus active work

Refinement updates the existing Task ID and creates a new definition revision. A Capture creates
its first Task; subsequent refinement continues that Task. A provider response labelled “new Task”
in an existing Task session is treated as a Task revision, so Apply does not create an unrelated copy.
After Apply, **Work status** opens beside **Preview** and shows **Refined - Revision N**. Refinement
is separate from execution state: **Start work** moves the Task to **In progress**. Applied sessions
leave the Refining shortcuts. Workbench places all in-progress Tasks immediately below Capture, with **Focus
active work** to hide the other sections without losing selection or drafts.

## Explicit Subtasks

Create a Subtask only for independently completable work. The preview states why it needs a separate
boundary, its parent, scope, non-goals and validation criteria. Accepting it also accepts the parent
Knowledge rollup described below. The parent and children appear as an expandable, indented tree;
Task detail links to the parent, siblings and children. Historical `split_from` links remain provenance,
not ownership. Hierarchy members retain their records; deleting them is blocked to preserve completion
and Knowledge evidence.

Every Task refinement includes its parent, siblings and children, including their scope and non-goals.
A changed family snapshot invalidates a pending proposal: refine again before applying. This avoids
applying a split based on outdated boundaries, including after accepting another Subtask proposal.

Completing a Subtask with evidence creates a cumulative Knowledge revision on its parent and
**automatically publishes it to the same parent Vault document**. Other Subtasks remain open. The
parent closes automatically when every Subtask is complete, and completed nested parents roll up
to their own parents. Reopening a child reopens completed ancestors while preserving historical
completion evidence and Knowledge revisions.

Completion and local Knowledge revision creation commit together; retrying the same completion
cannot create duplicates. A durable publication outbox retries after reopening the Task or restarting
the application. External edits to the Vault file are preserved and publication errors remain visible
on the parent. Restore the expected file or resolve the edited document before retrying publication.

## Interruptions and errors

- Provider or proposal-loading errors remain visible. Previously saved context is preserved.
- If loading the session, job status, or proposals fails, use **Retry** to reload the saved result
  without resending the AI request. Message and workspace-note drafts remain in the panel.
- If an assistant response is confirmed failed, the message submitted in the open panel returns
  to the composer when it is empty, so the user can choose to send it again.
- After opening a session, status polling uses read requests and waits for each request to finish
  before starting another.
- Refinement writes acquire the database writer slot before reading state, with bounded waiting
  for contention. This prevents stale read snapshots from failing when promoted to writes.
- Closing saves the current workspace notes and position. If saving fails, the panel stays open
  with the error and the notes so closing can be retried.
- Closing during a provider job does not cancel that durable server job. Its late response must not
  overwrite a different open item; reopening the original item can recover the saved result.
- Returning after a successful save, including an app restart, restores the saved refinement workspace.


### Independent chat and preview

Chat requests generate a short answer first. Once it is saved, Send becomes available again. Full
proposal generation runs separately as **Refinement preview** in the AI queue, with its own progress,
cancellation, and retry. The preview can keep updating while you send the next message. A preview
failure leaves the saved chat answer available; retry from the queue or send another message.

A new message supersedes older preview work. Delayed results and proposals from an earlier turn cannot
replace or apply over the current conversation. Closing the dialog leaves background work running;
reopening restores its status. If the application exits during generation, the interrupted preview
can be retried from the queue after restart. Changes still require explicit review and Apply.

## Keyboard behavior

Refinement and Task detail open in focused modal dialogs. Tab and Shift+Tab stay within the active dialog, including the saved-note disclosure; `Escape` closes it and restores focus to its trigger when it remains available. `Cmd+Enter` and `Ctrl+Enter` send only outside an active IME composition. Automated checks cover the synthetic composition and focus contract; real Korean IME and VoiceOver verification remain manual work.

## Continue a migrated Problem

Problem-only records from the previous workflow remain discoverable through **Refine**. Their
conversation uses the existing Context/Detail Preview. The Preview status opens Context; a loading
error leaves a visible retry action. Apply a generated Preview only after reviewing it.

Applying changes creates a new Problem revision. The active conversation follows that revision,
and a later reviewed Task proposal retains its exact Problem provenance. A stale change is rejected
instead of silently overwriting newer work. Tracking the conversation and accepting a Task remain
explicit user decisions; no retired Problem approval or Solution-creation step is required.

The previous **Explore next Solution** and **Create Problem** modal routes are not entrypoints in the
current Task Workbench. They do not define required stages for new work.

## Verification scope

Automated scenarios cover ordered multi-turn requests, preserved Capture context, proposal decisions,
retry, interruptions, and exact Problem provenance. Deterministic local provider tests verify request
contents and workflow behavior; they do not establish real-model answer quality or latency.

See the [interactive coverage record](../testing/interactive-coverage.md) for actual packaged results
and exclusions, and the [Task-centered Workbench guide](conflict-gated-workflow.md) for the full flow.

Related Spec Kit: [012 — Task-centered Workbench](../../specs/012-task-centered-workbench/spec.md),
[005 — Refinement Preview Status](../../specs/005-refinement-preview-status/spec.md).
