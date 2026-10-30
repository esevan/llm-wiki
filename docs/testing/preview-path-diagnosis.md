# Preview form path diagnosis (2026-10-30)

This is a code-path diagnosis, not a measured explanation of the reported stutter.
The affected preview has not yet been identified by screenshot or reproduction.
A reproduced intermediate-form transition is corrected without redesign. Controlled React and
WKWebView fixtures measure the transition separately from real-model latency.

## Distinct Task surfaces

- `WorkbenchView.tsx` initially collects one controlled text value (plus Capture
  images). Direct Task creation uses it as the title/provenance input. This is not
  a complete WorkPreviewFields form.
- `TaskDetail.tsx` edits six canonical Task definition fields with `taskDraft.ts`:
  title, detail, outcome, scope, nonGoals and validationCriteria, all strings.
  Dirty values and incoming snapshots are merged through the existing draft state.
- `ReferenceAwarePreview.tsx` presents AI WorkPreviewFields: title, description,
  background, goal, scope, nonGoals, constraints, completionCriteria and
  initialApproach. The last three are arrays. Editing joins/splits arrays by
  newline; reading renders prose and a completion list. These DTOs deliberately
  serve different stages; their difference alone is not a schema failure.
- `RefinementPanel.tsx` switches between the supplied Task definition surface and
  ReferenceAwarePreview when an AI preview arrives. It keeps the definition editor
  when taskDefinitionEditing is true. Current preview controls use a stable
  `previewId:current` key; historical version selection changes the version key.
  Receiving a new object is a rerender, not automatically a React remount.
- CSS using `:has(.reference-aware-preview)` selects parent workspace columns,
  header spacing and support-rail composition. The manual Task fallback already
  has that class, so a Task manual-to-AI transition does not by itself prove this
  grid selector changes. Reference refreshes replace workspace objects; profiling
  must show whether that creates expensive commits before optimizing them.

## Knowledge is a separate path

`KnowledgeReviewPanel.tsx` switches its body textarea and a reading surface that
uses KnowledgeMarkdown inside ContentRevisionTransition. Version selection and
correction are immutable Knowledge operations, not Task preview normalization.
ContentRevisionTransition uses a layout effect to select revision/animation layers;
it does not measure scrollHeight or element geometry. The new claim bodyLocator
and imageDecisions are source/assessment metadata; the body renderer does not use
them as component keys or replace form shape based on those fields.

## Reproduced hydration mismatch and correction

The first packaged profiling attempt held the canonical reference-workspace
response, yet the manual fallback disappeared before release. Separately polled
legacy proposals were already available: the fallback condition selected their
old form as soon as proposals.length became nonzero. Canonical WorkPreview arrival
then replaced it again. This is a reproduced parallel-hydration path, not proof
that the unidentified user symptom has the same cause.

TaskDetail supplies a canonical taskDefinition. RefinementPanel now keeps that
surface until the canonical preview is available and does not render its mirrored
legacy proposals as an intermediate Task form. Legacy surfaces without a supplied
Task definition retain their existing proposal path. No source DTO, application
contract or stored proposal is changed. The App regression injects legacy proposals
before the canonical preview and checks that the manual subtree stays mounted,
then verifies one transition with preserved parent/composer and unsent text.

## Validation and next observation

Native optional-field normalization fills absent optional values and rejects wrong
types; it does not convert Task string fields into WorkPreview arrays in the UI.
The generated Knowledge contract has a realistic-fixture parse/validate/private
apply/revision/publication regression, including missing optional fields and exact
prose locators. It is not evidence of real-model prose quality or UI performance.

Identify the affected Task or Knowledge tab and reproduce the first result arrival
with long Korean/English content. Record React Profiler commit timing and browser
layout/paint events around that transition; compare DOM identity, focused element,
scroll position, bounding boxes and the same response with reduced motion. Only a
measured regression should select a layout, hydration or normalization fix. Existing
modal layout, draft preservation and revision transition tests remain the baseline.

## Controlled measurements

`TaskWorkspaceApply.test.tsx` wraps the actual App in React Profiler for English
and Korean fixtures. It records commit count and render duration around a held
preview response, and asserts the modal/composer DOM identity and unsent text
survive while the manual subtree is replaced. jsdom cannot measure layout/paint;
its elapsed time includes the existing polling interval and is not input latency.

`previewTransitionProfile.ts` runs inside the signed packaged WKWebView through
`task-preview-transition-profile`. It holds the native reference-workspace response
at the bridge, captures the manual DOM and a two-frame baseline, then measures release-to-paint, DOM
mutation batches, preview subtree mounts/removals, ResizeObserver changes, parent
bounds/grid columns and scroll position. It uses English/Korean content at 1200
and 900 px. DOM mutation batches are not described as React commits. Results are
recorded as `PreviewTransition` receipts in the isolated scenario report. These
are synthetic deterministic fixtures, not the unidentified original user case.

## Verification receipts

The controlled React fixtures pass in both locales. English recorded four
transition commits and 12.76 ms React duration; Korean recorded five and 16.65 ms.
Elapsed times of 698.54/706.49 ms include polling and are not input/paint latency.
Legacy proposals caused one or two commits while the manual form stayed mounted.

The packaged measurement is unavailable in this environment: its baseline
requestAnimationFrame did not fire within five seconds, with visibility=hidden
and focus=false. No release-to-paint, layout or mount figures are claimed. The
fixture now bounds that wait and reports the limitation instead of hanging.
The Mac was locked; no unlock or access bypass was attempted. Repeat the dedicated
scenario in a visible unlocked test session to obtain the four paint receipts.
Knowledge publication/image rendering and reviewed stop-and-complete packaged
scenarios pass independently. Their success is not evidence of paint performance.
