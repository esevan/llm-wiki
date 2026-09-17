# Task lineage and Knowledge

**English** | [한국어](lineage-knowledge-layer.ko.md)

Task detail keeps its Capture provenance, optional exact Problem revision, Work Log evidence, comments, checklists, decisions, completion evidence, and relationships together. This deterministic record is the source for a private Knowledge draft after a Task is completed; it is not an automatic publication.

![Task detail keeps work evidence and provenance together](images/task-detail.en.png)

Create a draft deliberately, review or correct it, then publish the reviewed Markdown separately. Draft revision and content hash identify the publication source. If the published Vault file changes externally, hash checks prevent a silent overwrite. Regenerating refreshes the draft from the recorded Task; it does not erase the Task's private record.

Earlier Solution lineage tabs, inferred-claim correction views, and automatic completed-work reports are retained legacy behavior/specification history. They are not current Task interface promises.

See [Completion and Knowledge](completion-writeback-archive.md) and [Task-centered Workbench](conflict-gated-workflow.md).

The Review tab shows the current app-published version even while a newer private draft is being edited. Published articles and draft previews use the open-source react-markdown viewer with tables, links, code blocks, and task lists. FrontMatter is excluded from the reading view, and the draft editor uses normal-weight body text. New generated articles explain the work context, approach, decisions, results and verification; AI synthesis draws out evidence-supported learning and future reference value. Empty sections and missing-data placeholders are omitted. Internal IDs, revisions, hashes and source lineage live in the published file’s FrontMatter. Existing published documents are preserved until a reviewed replacement is published.
