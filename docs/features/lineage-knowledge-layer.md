# Task lineage and Knowledge

**English** | [한국어](lineage-knowledge-layer.ko.md)

Task detail keeps its Capture provenance, optional exact Problem revision, Work Log evidence, comments, checklists, decisions, completion evidence, and relationships together. This deterministic record is the source for a private Knowledge draft after a Task is completed; it is not an automatic publication.

![Task detail keeps work evidence and provenance together](images/task-detail.en.png)

Create a draft deliberately, review or correct it, then publish the reviewed Markdown separately. Publication writes the English canonical Markdown to the Vault and automatically queues its Korean translation under `Translations/ko/`. Draft revision and content hash identify the publication source. If the published Vault file changes externally, hash checks prevent a silent overwrite. Regenerating refreshes the draft from the recorded Task; it does not erase the Task's private record.

Earlier Solution lineage tabs, inferred-claim correction views, and automatic completed-work reports are retained legacy behavior/specification history. They are not current Task interface promises. The current contract is the Task-centered record in [spec 012](../../specs/012-task-centered-workbench/spec.md); [spec 010](../../specs/010-lineage-knowledge-layer/spec.md) remains the historical Solution-era design.

See [Completion and Knowledge](completion-writeback-archive.md) and [Task-centered Workbench](conflict-gated-workflow.md).

The Review tab shows the current app-published version even while a newer private draft is being edited. Published articles and draft previews use the open-source react-markdown viewer with tables, links, code blocks, and task lists. FrontMatter is excluded from the reading view, and the draft editor uses normal-weight body text. New generated articles explain the work context, approach, decisions, results and verification; AI synthesis draws out evidence-supported learning and future reference value. Empty sections and missing-data placeholders are omitted. Internal IDs, revisions, hashes and source lineage live in the published file’s FrontMatter. Existing published documents are preserved until a reviewed replacement is published.

## Recorded journey and AI interpretation

Task detail renders the recorded journey as a graph. On a wide panel, events follow a three-column
snake path; on a narrow panel, they become a single vertical path. Pale arrows preserve recorded
chronology and never claim cause. Nodes keep fixed visual spacing regardless of elapsed time; a gap
of at least 24 hours gets a dated torn-paper marker. Origin, material refinement or Task changes,
decisions, completion, and highly connected junctions receive
distinct shapes or emphasis. Selecting a node opens its full recorded activity.

AI may add short, locale-specific labels and the later-to-earlier relationships `supersedes`,
`derived_from`, and `depends_on`, shown as **Supersedes** (a later choice replaces an earlier one),
**Derived from** (a later result grows from earlier evidence), and **Requires** (a later step depends
on an earlier prerequisite). A relationship is retained only when its kind and direction are
valid and quoted evidence is found in both endpoint events. Labels are limited to 48 characters.
These links are shown separately as interpretation; they never replace or rewrite the recorded
events and chronological edges. When a provider is unavailable or its output is invalid, the graph
remains usable with deterministic labels and no inferred relationships.

There is currently no UI for correcting an inferred relationship. Users can inspect its rationale
and endpoint quotes, while the underlying activity remains authoritative.

## Exact journey used by a Knowledge draft

Generating a Knowledge draft in the app first creates or reuses the current journey for the selected
locale. The immutable draft lineage stores that exact snapshot as
`lineage.journey = { sourceHash, journey, modelStatus, modelError }`; see the
[current data model](../../specs/012-task-centered-workbench/data-model.md). Draft generation uses
the journey to describe decision evolution, replaced choices, derivations, and prerequisites, while
qualifying inferred relationships as interpretation. If Task evidence, completion, or journey input
changes while asynchronous generation is running, the stale result is rejected
instead of being saved over newer evidence. A user- or MCP-supplied draft body uses its supplied
source boundary and does not trigger journey inference automatically.

The Details view shows the current Task journey. The Review view shows the journey captured by the
selected draft, so a later graph refresh cannot silently change the evidence behind an existing
draft; when no draft snapshot exists, Review falls back to the current journey. Switching the
interface language localizes graph chrome and dates. A newly generated current graph can have labels
for that locale, while an immutable draft keeps the labels and locale captured
when it was generated; switching language does not retranslate that snapshot.

The localized-label provider path still needs a real-provider UI validation. Existing visual
captures verified English/Korean chrome and mixed long titles, not independently generated English
and Korean provider labels.
