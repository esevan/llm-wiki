# Reference-aware work previews

[한국어](reference-aware-workbench.ko.md) | **English**

New preview generation keeps the description focused on the work and the goal on
its intended outcome. Background, scope and approach add distinct information rather
than repeating the description; optional areas can be empty when they add nothing.
Material constraints, completion criteria, assumptions and exact source bindings are
retained. Existing stored versions and explicit Apply checks are unchanged.

Preview planning and final writing each use the shared **AI request timeout (seconds)** in AI setup (default 180; range 1–3600). The deadline includes the complete response body. These stages run sequentially with local retrieval between them, so the whole preview can exceed one request deadline. Queue waiting is outside that deadline. Saving a timeout applies to new requests without restarting. A timeout leaves the job failed and available for an explicit retry; preview generation does not automatically retry.

Refinement now prepares its work proposal through a retrieval plan, relevant local
Vault evidence, and a structured final preview. When a complete retrieval plan requests necessary search, it finishes before the preview is saved. Missing evidence is represented through assumptions; an
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

## Partial generated previews

Absent optional strings and lists are completed locally. An omitted Task field keeps its saved canonical value on apply, including after restoring a preview; omission never means clearing an existing title, detail, scope, non-goals, or completion criteria. A missing description or goal can reuse the exact saved Task context. Capture text can supply an unrefined description; an unavailable goal remains incomplete.

An incomplete retrieval decision skips the lookup and marks reference coverage unchecked, rather than claiming no search was necessary. Missing optional investigation is skipped. Partial previews expose their diagnostics for user review. Explicit wrong types, invalid supplied references and unsupported source-backed assumption bindings remain errors. These repairs do not add a model request or retry: planning and finalization still use one request each.

Generated preview text is limited to 12,000 Unicode characters per field, rather than UTF-8 bytes. Errors identify the field and distinguish a wrong JSON type from excess length. Omitted fields inherited from saved Task/Capture context retain the complete original text and are excluded from the Task patch; they are not silently truncated to fit a model-output limit.

Structured preview requests now send a strict JSON Schema to the provider. `description`, `background`, `goal`, `scope`, and `nonGoals` are strings; `constraints`, `completionCriteria`, and `initialApproach` are string arrays. The prompt includes a string example for multi-line scope. A returned scope array remains a type error; it is never silently joined or adopted. Provider refusal or incomplete output is reported before preview application. Existing missing optional-field recovery and explicit user Apply are preserved.

Preview source bindings use request-local reference IDs. Search results and explicit mentions are merged before IDs are assigned; the provider schema permits only those IDs. The application restores the exact document, revision, and section from the request snapshot, then runs the existing claim, assumption, revision, and transaction checks. Unknown IDs, duplicate claim/reference bindings, and malformed references fail without dropping evidence. With no references, claimSources must be empty. This adds no model call or automatic retry; Knowledge keeps its separate exact-source and quote contract.

Refinement keeps the Task title separate from its description. Chat can request a title change; Edit preview also edits the title directly. New or changed Task titles must contain 1-70 Unicode scalar values, counted consistently by JSON Schema, Rust and the UI; this is not a 140-byte limit. Korean syllables and ordinary emoji each count as one; combining marks and joined emoji contain multiple values. Over-limit edits remain visible and cannot be saved or applied. Existing Task titles are preserved when other fields change. Historical previews without a title keep their original bytes and hash and display their full description as a compatibility title; shorten and save an over-limit legacy title before applying. No title enforcement call or automatic retry is added.

Apply to Task commits the reviewed revision, records it as refined and completes the refinement session in one transaction. The Workbench closes refinement, refreshes the lanes and opens the acknowledged Task, including when a Capture becomes a Task. Existing Tasks keep their identity. Failed Apply retains the preview; a post-save refresh problem is not reported as a failed write. Reference, head, context and Task revision checks still apply.

General Task edits may retain an unchanged legacy title over 70 characters. Refinement Apply always requires a valid title, including a title recovered from an older preview or omitted AI output; explicitly edit and save a shorter title first.

### Retrieval contracts and draft saves

Retrieval plans use separate search/no-search schema branches. No-search plans require empty queries and null requery. Search plans require 1-4 nonblank queries and at most 2 requery queries, each limited to 500 Unicode characters. Unsupported aspects are rejected before dispatch. Missing legacy controls remain explicitly unverified without invented evidence or model retries.

After an acknowledged preview edit, the saved version remains current even if refreshing fails. Refresh failure is reported separately; the next edit uses the saved version and hash. Knowledge drafts without an evidence snapshot are read-only. Generate a new draft to recover editable, source-bound content.

If Knowledge correction is acknowledged but the subsequent read fails, the UI retains the acknowledged body separately and offers a read-only recovery screen. Editing, restoration, notes, and publication stay unavailable until the canonical Task, revision, content hash and body are confirmed. Retrying recovery only reads; it does not repeat the correction or invent evidence metadata. Responses from an earlier Task cannot update a different Task after navigation.

Routine search decisions, empty search outcomes and technical diagnostics are available in collapsed **Reference search details**. Unverified or failed search, partial-preview guidance and critical missing information remain visible. Title validation appears beside an invalid current title; valid titles have no error guidance.

Workbench cards stay 320 px wide on desktop and shrink only when the available lane is narrower. Lanes wrap to fit the workspace. Each lane has its own viewport-bounded, keyboard-focusable vertical card scroller; headings stay outside it, and the page scroll reaches other categories.

Workbench cards use a fixed 320 × 240 px desktop size. Each lane uses its available width for additional card columns without stretching cards; the lane keeps its independent vertical scroll. Titles and original-content previews show at most two lines with ellipsis, and actions share a consistent bottom baseline. Full text remains available in Task detail or Capture refinement. The Subtasks button opens the parent Task detail, where its hierarchy can expand without changing board card size.

The preview starts with summary, goal and completion criteria. Scope, constraints
and approach are expandable; editing and comparison retain all existing fields.
Applying adds nonempty completion criteria as unchecked Task checklist items in the
same transaction. Generating alone changes no Task. Content-based IDs prevent
replay/reordering duplicates. Existing checked, manually edited and manually added
items are never overwritten or deleted. Changed wording adds a new unchecked item
and preserves the old one. Checking records user checklist state; it is not proof
of execution or evidence verification and does not complete or publish the Task.

The wide Refinement screen keeps the preview in a large reading card on the left. The existing conversation is at the upper right, with short evidence titles and excerpts below it. Full source filters remain available in the evidence disclosure. The reading and support columns scroll independently; narrow windows stack the preview, conversation, then evidence.

The preview modal and main reading card expand with the window, including ultrawide displays. Prose line length is constrained inside the fluid surface; the existing chat and evidence remain in the supporting column.
