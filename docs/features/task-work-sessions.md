# Task work sessions

[한국어](task-work-sessions.ko.md) | **English**

The Sessions tab in Task detail keeps focused work separate from the Task's canonical definition and Work Log. Its left rail lists saved sessions while the conversation remains the main workspace. Opening a Task does not load session history or start Codex session services; that work starts only after you select **Sessions**. A Task starts with no sessions. Choose **New session** when you want one, or choose **Link Codex session** to attach a saved conversation from Codex CLI, the Codex app server, or the VS Code extension in the same local Codex home. Linking can create the Task session in one step and does not start or resume provider work.

Each session stores chat-style user notes, one optional file up to 10 MB per note, and its own title and execution settings. Pasted images show a preview before saving; saved images remain visible and every saved attachment can be downloaded. A failed save leaves the note and attachment in the composer for retry. Saved records survive an app restart; unsaved drafts are retained while Task detail remains open, but are not restored after restart.

The conversation shows user messages, Codex Markdown responses, and tool activity in provider order. Commands appear as compact rows whose bounded output opens on demand, while a visible status marks queued, running, or response-needed work. Task context and execution settings remain available in collapsible sections below the conversation. The context reads the current Task and references without saving a second snapshot, and another Task's sessions are never included.

Only a persisted, inactive Codex conversation that is not owned by another Task can be linked. A local target that already contains notes or Runs cannot absorb an unrelated history. This prevents two records from being silently combined.

Provider, model, approval reviewer, and project folder path are saved with each session. **Prepare conversation** connects the exact session to Codex and checks its effective settings without starting work or creating a Run. Afterward, **Check settings** refreshes that result. **Save note** records only the note and attachment. **Run with Codex** is a separate explicit action: it saves one immutable Run and linked Work Log row before Codex starts. The session retains one exact Codex conversation across later Runs, without replaying the previous transcript.

The project folder can be entered directly, selected from the native **Browse** picker, or chosen from the ten most recently saved session folders. Browse only returns a path; it does not change the Vault. The folder is still validated by Codex preparation and execution before work starts. Execution instructions keep file discovery within that project folder and require explicit approval before searching broader external paths; this is an instruction boundary, not an operating-system sandbox.

Runs appear in the conversation at their actual time alongside linked Codex history. The current Run keeps its status, Stop action, approvals, and questions visible even when an older Run is selected. Live assistant text appears in the conversation; command output and implementation evidence remain expandable so they do not displace the composer. The Work Log synchronization state stays visible independently of the provider outcome. Stop only requests interruption; the provider's final status determines whether the Run finished, failed, or was cancelled. An interrupted or uncertain result can include work that already happened, so review its saved evidence before starting another Run. **Use instruction again** loads a prior instruction only when the composer is empty and never overwrites another draft.

The composer remains available while a Run is active. **Run with Codex** is the primary send action and **Save note** records local context without invoking Codex. Images attached to a Run are forwarded to Codex; saved attachments remain part of the session record and can be downloaded later.

Formal approvals expose only the choices sent by Codex. Structured questions require one explicit submission after each non-secret answer is chosen or entered; stale requests are disabled and failed answers remain available to retry. A Work Log execution block links back to its exact session and Run. Codex execution never completes a Task, resolves a Problem, or publishes Knowledge. See the [feature specification](../../specs/014-task-codex-execution/spec.md) for the complete boundary.

**Ask me** keeps approvals with the user; **Automatic review** requires an explicit current choice and preserves Codex’s configured approval policy and sandbox. Legacy automatic preferences do not grant approval.

Using a session does not complete or reopen a Task. Saving a note does not invoke AI.

The external session list and linked history are paginated snapshots from the same local Codex home. Use **Load more** to retrieve additional conversations or older turns. LLM Wiki refreshes the exact bound conversation when its session opens, but changes made simultaneously by another Codex process are not guaranteed to stream into the view. Continuing that conversation still requires an explicit **Run with Codex** action.
