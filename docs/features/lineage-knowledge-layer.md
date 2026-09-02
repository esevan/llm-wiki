# Lineage Knowledge Layer

**English** | [한국어](lineage-knowledge-layer.ko.md)

> A completed Solution keeps its origin, decisions, conflicts, and completion basis traceable.

When a Solution is completed, LLM Wiki automatically builds the current
**Capture → Problem → Solution → Complete** snapshot. The completed-work report is generated only
after this snapshot exists and uses the Lineage projection plus its referenced evidence as input.
The final Markdown contains Detail, Lineage, Decision Changes, Conflicts & Addresses, and Completion
Evidence.

Database UUIDs stay inside the private Lineage audit model. The completion-report model and final
Markdown receive deterministic human labels such as `Original capture`, `Problem record`,
`Work log 1`, `Validation criterion 2`, and `Completion decision`. The private inference-validation
request still uses opaque evidence IDs so inferred claims can be rejected when they cite anything
outside the exact evidence bundle; those IDs are never reused as reader-facing citations.

Lineage and the completed-work document share one lifecycle. Regenerating completed Knowledge first
rebuilds deterministic Lineage from current source records, optionally refreshes AI interpretations,
and then rebuilds the report and document. The product does not present these regenerations as
document versions; broader Version Control remains a separate future capability.

The Lineage tab presents the four stages as a compact vertical flow that does not require horizontal
page scrolling. Capture, Problem, and Solution cards open their read-only records; each stage shows
its recorded time in the viewer's system locale. Evidence is cited as globally numbered
**References** such as `[1]` and `[2]`. Repeated citations to the same retained source reuse the same
number, and selecting one opens its excerpt directly beneath the citing card or transition.

Transitions distinguish an explicit **Decision basis** from a deterministic **Recorded change**.
When no rationale was recorded, the UI explains the observable change between stages instead of
showing an empty reason or inventing intent.

Every statement shows its provenance:

- **Observed** comes directly from a retained record.
- **Decided** comes from a recorded human or implemented workflow decision.
- **AI inferred** is an interpretation with High, Medium, or Low confidence.
- **Corrected** is the current user correction to an earlier AI interpretation.

Missing reasons remain `Not explicitly recorded`. AI inference cannot mark a conflict addressed.
An Addressed conflict requires an explicit decision or implementation evidence, and records whether
the original requirement was Preserved, Modified, Superseded, or Rejected.

Users can correct inferred claims from the Lineage tab. The correction becomes current Knowledge;
the previous AI interpretation and immutable source evidence remain in revision history. Regeneration
carries the correction forward when the same claim is reconstructed.

AI Setup exposes **Lineage interpretation** as its own task. It uses the Advanced model by default
when one is configured, independently from the **Completion report** task. AI unavailability never
removes the deterministic Lineage.

Related Spec Kit: [010 — Lineage Knowledge Layer](../../specs/010-lineage-knowledge-layer/spec.md)
