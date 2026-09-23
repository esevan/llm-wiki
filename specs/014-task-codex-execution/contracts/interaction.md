# Interaction Contract: Task Session Execution

## Composer and settings

- **Save note** stays note-only; **Run with Codex** is separate and never automatic.
- Failed start preserves draft/input; confirmed start clears only submitted content.
- Show provider, model, canonical folder, and effective approval/sandbox. Replace inert “AI is not run” copy.
- Ask routes supported approvals to the user. Auto review is a new explicit opt-in; legacy auto requires review.
- Existing attachments remain visible; only explicitly supported execution inputs are sent.

## Run timeline

- Ordered Run cards show status, time, instruction, safe live item/status progress, completed output/evidence, report/error, Stop, retry, and Work Log link.
- Switch/close detaches without Stop; return selects exact active/requested Run.
- Stop shows “requested” until terminal evidence. Retry creates a new Run.

## Formal requests

- Approval buttons are explicit decision actions from the provider request and disable while submitting.
- Structured questions show actual labels/descriptions. Users select per question and press one Submit for the request; nothing is default-confirmed.
- Null/empty options use free text. With options, extra text appears only when allowed.
- Pending, submitting, answered summary, stale, and error states use text and non-color cues.
- Keyboard access, visible focus, predictable return focus, and nonintrusive live status are required.
- Failed response retains nonsecret selections/text. Secret requests use existing Codex secret handling.
- Nonblocking requests remain answerable without disabling unrelated progress.

## Work Log link

- The existing Work tab shows an execution block inside the linked Work Log article: attributed report, observed evidence, status, artifacts, limitations, and **Open execution**.
- Open execution switches to Sessions, selects exact session/Run, and focuses its heading.
- Manual body, attachment, summary, comments, and comment form remain unchanged.

## Astra High design decisions (2026-09-21)

- Keep this DOM/read order at wide and 640 px widths: session selector; active/response-needed indicator; chronological notes and Runs; effective-settings summary; composer; supporting Task context/settings.
- First explicit action is **Prepare conversation / 대화 준비**, subsequently **Check settings / 설정 확인**. It creates/resumes the exact Codex thread and checks settings, without a turn, Run, or Work Log. Explain that work starts only with Run. Never prepare on open. Run needs current ready settings/revision; mismatch keeps the draft. Bound-folder mismatch requires a new session.
- Put effective model, folder, reviewer, sandbox, and Edit settings immediately above the composer. Missing settings expand initially. Ask me / Automatic review are labelled choices; legacy automatic intent remains Ask.
- Completed Runs collapse to instruction, localized status, time, and report preview; expand the active or deep-linked Run. Keep an active/response-needed Open indicator when viewing history. Do not duplicate an execution instruction as both a note and a Run card.
- Put a formal request directly below its Run heading, before output. Blocking says Waiting for your response; nonblocking says Response requested · execution continues. During a blocking request, Submit is the visual primary action. Validate all questions and focus the first incomplete one; no partial submission for mixed secret requests unless supported.
- Key answer drafts by Task/session/Run/request/question. Preserve nonsecret input across failure, detachment, Task close/reopen, and locale change. Show accepted answers after submission and stale proposed answers read-only; never copy them into a new request. Loading is request-scoped; keep notes/navigation/Stop available.
- Use instruction again prepares a retry in the composer without overwriting another draft. Explicit Run starts a new attempt with lineage. Unknown outcomes say work may already have happened; checking state does not rerun. Work Log sync has its own retry.
- Session → Work Log switches to Work and focuses the exact article. Reverse links select the exact session/Run and focus its heading. Use a small aria-live status region; progress does not steal focus or scroll.
- Composer label is Instruction or note, with secondary Save note and primary Run with Codex. During active work keep input editable and note saving usable, explaining why another Run is unavailable.
- Use Execution finished, Codex final report, and Observed evidence as separate concepts. Collapse verbose evidence. Mark unsupported attachments Saved with note · not sent to Codex. Wrap labels, paths, and output without overflow.

Review completed by Astra High as a design review only. No rendered UI approval or runtime test is implied.
