# Knowledge publication and archive

**English** | [한국어](knowledge-archive.ko.md)

Choose **Publish Knowledge** once from the Task review tab. The selected nonempty
saved draft is published without an AI organization request or a second approval
screen. Edited body text is saved as a new revision first. The button itself is the
explicit publication decision; generating or applying a chat proposal never publishes.

Quality notes, citation mismatches, source freshness, reuse interpretations and
insufficient evidence do not block normal publication. Existing failed drafts and
legacy drafts without a generation snapshot can publish without regeneration.
User experiences and directly written reports are primary sources. External
citations are optional; their absence alone is not a quality failure.

Normal publication creates an independent immutable file in `Knowledge/Published/`.
It preserves existing files and both sides of a potential disagreement. Publication
does not mark AI content verified or turn it into the user's firsthand experience.
Source links, user statements, AI bindings and review findings remain metadata.
Published Knowledge is searchable as content, while its unverified authority stays
explicit. Retrieval returns source revisions, conditions, provenance and comparison
sources so consumers can compare context instead of silently merging conflicting
claims. This is comparison support, not automatic semantic conflict adjudication.

Storage failures remain protective errors: a changed selected version, unexpected
file changes, occupied destination, unsafe path or disk failure cannot silently
advance the publication pointer. Repeated clicks replay the same payload-bound
operation. `index_pending` means exact bytes are saved but the previous pointer
remains until the real index worker confirms those same bytes. Reopening review
restores durable operation state. Index retry does not regenerate content; recovery
and compensation preserve externally changed files.

Move, rename, withdrawal and repair keep their explicit exact-file review because
they modify existing archive structure. Existing advanced organization proposals
remain compatible. Ideas are still separate optional documents with their own
unverified/deferred/out-of-scope/rejected disposition and reconsideration conditions.

[Specification and policy amendment](../../specs/021-organized-knowledge-archive/spec.md)
