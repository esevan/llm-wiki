# Interaction Contract: Preview, references, mentions, and versions

## Workspace hierarchy

- The current structured preview is the primary region and first heading after the dialog title.
- Conversation and composer remain operable while optional investigation runs.
- References are secondary: a quiet rail on wide windows and a collapsible region below conversation on narrow windows.
- Critical findings use one current alert near the preview. Supporting/ancillary results do not inject unsolicited messages or steal focus.
- Requested preview generation and optional investigation have separate labeled status/error/retry surfaces.

## Structured preview

Display description, background, goal, scope, non-goals, constraints, completion criteria, initial approach, and assumptions as distinct labeled fields. Assumptions show their status/basis. Evidence links expose exact source/version/section and open the shared ReferenceViewer. Preview language never implies adoption, execution, verification, completion, or publication.

Generated versions are reviewable proposals. An explicit Apply control is the only canonical Task adoption action. User edits append or remain local until saved; incoming results never overwrite a focused dirty field. Queued newer proposals show separately.

## Reference list stability

- Search, aspect/status/type/usage filters, and sort are keyboard accessible and labeled.
- Result identity is document ID + version; section passages nest under a deduplicated document row.
- During one `orderEpoch`, arriving rows append to a pending/new-results group and existing rows keep their positions. The user may activate “Show new results” or change sort/filter to create a new stable order.
- Deterministic sorts end with stable identity as a tie-breaker. No score-only reorder occurs while reading.
- Empty, searching, no-suitable-result, failed, changed-context, and completed states have distinct text and do not rely on color.

## Shared ReferenceViewer

`ReferenceViewer` is owned by E and reusable by F/G. It accepts only a typed exact binding and a safe application read callback. It renders Markdown through the shared safe renderer; raw HTML, scripts, unsafe protocols, and unscoped filesystem links are never activated.

The dialog provides:

- document title, exact version, information/status labels, and section breadcrumb;
- focused section with enough surrounding Markdown context;
- in-modal link navigation, back/forward, and previous/next reference controls;
- an explicit external-link affordance only for already allowed safe web URLs;
- Escape/close, backdrop behavior consistent with existing modal policy, focus trap, and accessible heading/labels.

Opening captures the originating control, composer text/selection, and all three workspace scroll anchors. Closing restores the origin when it still exists, otherwise the reference row/list heading. Navigation within the viewer never changes composer text, usage beyond `viewed`, preview head, or workflow state.

## Mentions

Typing `@` opens a lookup aligned with the composer. Search supports documents and sections and exposes version/status when titles duplicate. Arrow keys move, Enter/Space inserts, Escape closes, and multi-select inserts multiple ordered tokens. Selection inserts a stable token and readable label without sending. Backspace/delete removes the whole token at its boundary. Paste/plain-text fallback preserves readable labels but cannot silently create a bound source.

The sent request carries exact mention bindings separately from display text. Mentioning records `mentioned`; only a later reply/preview claim that consumes it records `used`.

## Version controls and comparison

`DraftVersionControls` is owned by E and accepts typed summaries/current selection/compare/restore callbacks. F reuses it inside its Knowledge panel rather than creating another picker/modal.

- Version selection enters read-only historical mode and shows current/historical/restored/generated/edited labels.
- Compare selects exactly two immutable versions. Wide layout uses aligned field columns; narrow layout stacks old then new per field. Added/removed/changed/unchanged states include text, not color alone.
- Comparison includes assumptions and exact reference snapshots, not only prose fields.
- Historical mode hides/disables Apply. `Restore as new` names the source and resulting new-version behavior.
- Restore conflict leaves the selected history and unsaved edits intact.
- Reading old versions, switching comparison sides, and rerendering never trigger `ContentRevisionTransition`.

## Transition and continuity

Only confirmed canonical apply or successful preview restore passes `preview_adopted`/`version_restore` to `ContentRevisionTransition`. Historical mode, navigation, locale switch, async arrivals, and ordinary rerenders pass no cause. Reduced motion updates immediately without animation.

Async reconciliation preserves:

- composer draft, mention tokens, selection, and IME composition;
- preview dirty fields and queued proposal badges;
- active tab/version/compare selection;
- preview, conversation, and reference scroll anchors;
- stable reference row position and focused control.

No async completion auto-focuses, auto-scrolls, opens the viewer, sends a message, or applies content.

## Visual verification matrix

Review wide and narrow layouts in English and Korean with long titles/sections, 1,000 references, duplicate titles, no results, stale context, generation failure, optional failure, dirty preview edits, queued proposal, historical version, compare, restore conflict, viewer history, multiple mentions, keyboard-only use, and reduced motion. Confirm the preview remains the visual focus and the composer remains reachable.


## Implemented shared consumer boundary (2026-09-26)

The named exports are `ReferenceViewer`/`ReferenceViewerProps` from
`frontend/src/components/ReferenceViewer.tsx` and
`DraftVersionControls`/`DraftVersionControlsProps` from
`frontend/src/components/DraftVersionControls.tsx`. Both import the shared reference
styles and English/Korean copy; consumers provide their own surrounding layout.

- Viewer consumers supply `open`, an exact optional `binding`, `references`,
  `read(binding): Promise<ReferenceDocument>`, `onNavigate(binding)` and `onClose()`.
  `onViewed(binding)` is optional. A `ReferenceDocument` extends
  `ExactReferenceBinding` with `markdown: string`. The consumer keeps its selected
  binding in state and resolves reads through its authorized application command.
- Version consumers supply `versions: WorkPreviewSummary[]`, `selected`,
  `onSelect(version)`, `onCompare(left, right)` and `onRestore(version)`.
  `currentVersion`, `comparison`, `restoreDisabled` and `disabled` are optional.
  Summaries are labels, not complete version payloads: the consumer fetches both exact
  versions before passing a `PreviewComparison` containing `left`, `right` and field
  states. Restore/apply policy belongs to the consumer and native transaction.
- E's concrete example is `ReferenceAwarePreview.tsx`; F's Knowledge panel adapts its
  immutable Knowledge revisions to these summary/comparison props and supplies its
  exact source-read callback. Neither component publishes or mutates a Task itself.

These implemented props do not imply completion of every option in the design above.
The remaining reference filters/grouping and richer mention editing are tracked in
`tasks.md` under T035–T041.
