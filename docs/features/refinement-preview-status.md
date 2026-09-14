# Context-preserving Refinement

**English** | [한국어](refinement-preview-status.ko.md)

Refinement helps turn a Capture or Task into reviewable work while preserving its original context.
A Capture that already contains a solution does not have to discover that solution again. Simple work
can become a Task directly; refinement and Problem approval are not mandatory gates.

## Work in the refinement panel

Open **Refine** from a Capture or Task, or resume the saved refinement shortcut in Workbench.
The conversation and proposals have separate tabs. Keep working notes alongside the conversation;
the saved notes, selected tab, scroll position, and conversation history support returning later.

**Send**, `Ctrl+Enter`, and `Cmd+Enter` submit a message. Plain Enter writes another line. Send is
unavailable while the current submission or response is being processed. Every provider turn includes
the original Capture and ordered conversation history, even after a transient draft is cleared.

A response can produce proposals. Review each proposal separately: edit, accept, or reject it.
Accepting one proposal does not accept its siblings. Response processing includes loading the
resulting proposals; a delayed proposal read must not silently discard the result.

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

## Keyboard behavior

Refinement and Task detail are nonmodal panels: opening one moves focus to its heading or compose control without trapping Tab navigation. `Escape` closes the panel and restores focus to its trigger when it remains available. `Cmd+Enter` and `Ctrl+Enter` send only outside an active IME composition. Automated checks cover the synthetic composition and focus contract; real Korean IME and VoiceOver verification remain manual work.

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
