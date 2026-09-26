# Interaction Contract: Automatic Capture Distillation

## Card

- Keep the same DOM/entity key, Inbox lane, order, and geometry anchor through status/result changes.
- Before a result, use trimmed raw text or localized image placeholder. Modal access keeps raw source discoverable.
- Queued/running status is concise and nonblocking. Failure remains labeled with retry when allowed. Review-needed output is explicit and never styled as current applied content.
- Successful current output replaces display title/content in place. No refining/refined badge or version count appears.

## Modal

- Opening reads current projection and never starts or retries work.
- Raw text and every attached image are immediately discoverable in a source region.
- Processing/success/failure/superseded-proposal state remains visible without disabling or covering conversation input.
- Event updates never replace controlled input, selection/composition, focus, active tab, page scroll, or modal scroll.
- Card and modal compare the same source/current/result revisions and show the same state.

## Shared content revision transition

Motion is eligible only when stable entity is mounted, prior displayed revision is known, a newer current revision reconciles, cause is `automatic_apply`, `preview_adopted`, `refinement_apply`, or `version_restore`, view is live, and reduced motion is off.

Initial mount, navigation, modal open, ordinary rerender, restart hydration, replay of same revision, locale-only change, historical reading, and reduced motion render final content statically.

### Title sequence

Keep old/new title text stationary in the same layout box. Clip old title right-to-left, then reveal new title left-to-right. Remove old layer and leave new text as normal document content. Wrapping and intrinsic height must not produce horizontal translation. Assistive technology receives one current title, not two announced layers.

### Body sequence

Use short opacity and/or text-color settling with existing tokens. Do not blur, translate, move the container, or block interaction.

## Accessibility and review

- Existing modal focus trap, opener restoration, Escape behavior, and focus rings remain.
- Retry has a Capture-specific accessible name. Status is not color-only. Processing uses polite live status; a newly relevant failure alerts once rather than on rerender.
- Rendered review covers wide/narrow, Korean/English, image-only, long wrapping content, empty optional fields, all states, active input/selection/scroll, keyboard, normal motion, and reduced motion.
- Record actual visual checks and any unavailable real OS reduced-motion, IME, or assistive-technology checks.
