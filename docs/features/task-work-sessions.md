# Task work sessions

[한국어](task-work-sessions.ko.md) | **English**

The Sessions tab in Task detail keeps focused work separate from the Task's canonical definition and Work Log. A Task starts with no sessions. Choose **New session** when you want one, then select any saved session later to continue its private record.

Each session stores chat-style user notes, one optional file up to 10 MB per note, and its own title and execution settings. Pasted images show a preview before saving; saved images remain visible and every saved attachment can be downloaded. A failed save leaves the note and attachment in the composer for retry. Saved records survive an app restart; unsaved drafts are retained while Task detail remains open, but are not restored after restart.

The context column reads the current Task content and existing references. It does not save a second Task snapshot, and another Task's sessions are never included.

Provider, model, approval reviewer, and project folder path are saved with each session. **Prepare conversation** connects the exact session to Codex and checks its effective settings without starting work or creating a Run. Afterward, **Check settings** refreshes that result. **Save note** records only the note and attachment. **Run with Codex** is a separate explicit action: it saves one immutable Run and linked Work Log row before Codex starts. The session retains one exact Codex conversation across later Runs, without replaying the previous transcript.

The project folder can be entered directly, selected from the native **Browse** picker, or chosen from the ten most recently saved session folders. Browse only returns a path; it does not change the Vault. The folder is still validated by Codex preparation and execution before work starts.

The execution timeline separates the Run outcome/report from Work Log synchronization. Observed evidence and implementation details are collapsed by default, while the Codex final report remains in the outcome summary. It also shows the effective model, canonical folder, approval policy, reviewer, and sandbox returned by Codex. Stop only requests interruption; the final provider status determines whether the Run finished, failed, or was cancelled. An interrupted or uncertain result can include work that already happened, so review saved evidence before starting another Run. **Use instruction again** loads a prior instruction only when the composer is empty. **Run with Codex** then creates a new attempt; it never overwrites an unrelated draft.

While a Run is active, the composer remains available. The input mode distinguishes **Save as note**, **Queue for next Run**, and **Steer current Run**. Queue and Steering are shown with an explicit unavailable state until native durable Queue and `turn/steer` APIs are connected; selecting either mode never reports a false delivery or silently saves a note.

Formal approvals expose only the choices sent by Codex. Structured questions require one explicit submission after each non-secret answer is chosen or entered; stale requests are disabled and failed answers remain available to retry. A Work Log execution block links back to its exact session and Run. Codex execution never completes a Task, resolves a Problem, or publishes Knowledge. See the [feature specification](../../specs/014-task-codex-execution/spec.md) for the complete boundary.

Attachments in this increment are saved with notes and are not sent to Codex. **Ask me** keeps approvals with the user; **Automatic review** requires an explicit current choice and preserves Codex’s configured approval policy and sandbox. Legacy automatic preferences do not grant approval.

Using a session does not complete or reopen a Task. Saving a note does not invoke AI.
