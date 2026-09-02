# Conflict Resolution Workflow

**English** | [한국어](conflict-resolution-workflow.ko.md)

Conflict Review is a decision workflow inside a proposed Solution, not a raw AI report. When Vault evidence disagrees with the current Solution, the completed Queue result opens one card per conflict.

Each card puts the core comparison first:

- severity and a short category;
- the current Solution claim;
- the existing document or decision and its competing claim;
- expected impact and the recommended resolution; and
- expandable source citation and excerpt.

Severity is always written as High, Medium, or Low and also receives a distinct visual treatment, so color is not the only signal. Evidence stays collapsed until it is needed, and the dialog body scrolls while the review summary and final action remain available.

## Resolve every conflict

Every card requires exactly one human choice:

- **Apply recommendation** records that the Solution needs revision. An optional comment can explain the intended change. The Solution remains conflicted until it is revised and a fresh current review supports continuation.
- **Accept conflict** intentionally preserves the Solution direction. A rationale is required so the exception can be understood and reused later.

The footer reports total, resolved, and unresolved counts. **Continue** remains unavailable until every card has a valid action and every accepted conflict has rationale. If saving fails, the dialog stays open and keeps the entered decisions.

When every conflict is intentionally accepted, LLM Wiki records the explicit human address and lets the existing clear conflict gate continue normally. AI can describe a conflict and recommend a response, but it cannot select a resolution, change workflow state, or invent a citation.

## Durable history and compatibility

LLM Wiki stores the structured conflict, its source evidence, the selected action, rationale, and resolution time in local SQLite. Reopening the review restores that `Conflict → Resolution → Rationale` history. This local review history remains private process and is not automatically published as Knowledge.

Earlier findings-only Queue results remain readable through card fallbacks. Because those reports predate item-level persistence, LLM Wiki asks for a fresh review before saving resolutions from them. A conflict-free result keeps the concise existing clear/conflicted decision path and does not add empty cards.

See the [feature specification](../../specs/010-conflict-resolution-workflow/spec.md), [data model](../../specs/010-conflict-resolution-workflow/data-model.md), and [API contract](../../specs/010-conflict-resolution-workflow/contracts/conflict-review-api.md).
