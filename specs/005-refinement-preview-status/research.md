# Research: Refinement Preview Status

## Decision 1: Build the summary from existing same-item records

**Decision**: Add a deterministic workflow-service operation that selects the current saved detail first, then the most recent relevant `workflow_chat` and `workflow_refinement` records for the same item. Normalize whitespace, return at most three entries, and enforce a shared 500-character visible-text budget.

**Rationale**: The database already stores the required item detail and item-scoped history. Reusing it is immediate, local, predictable, and testable while avoiding another AI failure mode.

**Alternatives considered**:

- Generate a new AI summary: rejected because it adds latency, cost, and a second failure path while the Preview is already waiting on AI.
- Use only browser-visible board data: rejected because it omits stored conversation and previous refinement records after the current session.
- Show full history: rejected because the user explicitly chose a summary and wants minimal modal change.

## Decision 2: Expose context through a bounded read endpoint

**Decision**: Add a read-only endpoint for Problem and Solution refinement context. Start it concurrently with the existing refinement-generation request and retain its response in the current browser attempt.

**Rationale**: The existing refinement response is available only after model completion, too late for the required loading state. A focused read avoids inflating every board response with history that most interactions do not need.

**Alternatives considered**:

- Add summaries to the board payload: rejected because it would query and expose history for all visible items on every refresh.
- Convert refinement generation to streaming or a job API: rejected as a disproportionate redesign of the current synchronous JSON proposal contract.
- Make context part of the completed refinement response only: rejected because it cannot satisfy the loading Preview requirement.

## Decision 3: Reuse the existing dialogs as a state transition

**Decision**: Keep the refinement conversation in the existing chat dialog and use the existing draft/review dialog as the Preview shell. Open the review dialog early only when prior context exists, then replace its loading state with editable proposal fields on success. On failure, close it and restore the refinement dialog with a warning.

**Rationale**: This preserves current layout, controls, approval behavior, and visual language while adding only the requested states.

**Alternatives considered**:

- Add a third dialog: rejected because it duplicates controls and introduces unnecessary modal transitions.
- Put the loading summary permanently in the refinement dialog: rejected by the explicit non-goal against an always-present context summary.
- Leave an error Preview open: rejected by the explicit requirement that a failed Preview not be displayed.

## Decision 4: Keep attempt state ephemeral and identity-scoped

**Decision**: Track a unique attempt token together with item type, item ID, status, context response, and cancellation state in the browser only.

**Rationale**: Preview state has no durable business meaning. Identity-scoped state prevents late network responses, rapid retries, or item changes from showing stale content or warnings.

**Alternatives considered**:

- Persist Preview state in SQLite: rejected because it adds schema and recovery semantics without user value.
- Track only a global busy flag: rejected because it cannot distinguish late responses from different items or retries.

## Decision 5: Use an actual accessible warning control

**Decision**: Place one hidden-by-default warning button in the refinement dialog corner. On failure, expose the exact fixed message through both its accessible name and existing hover/focus tooltip behavior.

**Rationale**: A real button is keyboard focusable, works with the existing tooltip convention, and communicates the failure independently of the warning glyph.

**Alternatives considered**:

- Decorative icon with a title attribute: rejected because keyboard and assistive-technology behavior is inconsistent.
- Alert dialog or toast: rejected as more intrusive than the agreed corner warning and a larger UX change.

## Resolved Unknowns

- **Previous context exists** when saved detail differs meaningfully from the short title or at least one completed same-item refinement conversation or record exists.
- **No previous context** preserves the current button-level generation progress and does not open an empty loading Preview.
- **Summary format** is one to three labeled plain-text entries with a combined 500-character limit.
- **Tooltip wording** is exactly `Refinement preview를 띄울 수 없습니다. 다시 시도해 주세요.`
- **Capture boundary** remains unchanged and does not call the new context endpoint or show the new loading/error Preview states.
