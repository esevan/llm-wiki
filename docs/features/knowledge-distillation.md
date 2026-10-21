# Evidence-bound Knowledge drafts

**English** | [한국어](knowledge-distillation.ko.md)

New articles use a compact result-first structure followed by when the result helps,
its conditions and limits. Extra explanation remains when needed for safe reuse;
material evidence, counterevidence, unresolved issues and warnings are never removed
for brevity. Empty headings, a second executive summary and repeated conclusions are
not requested. Structured applicability and exact source bindings remain required.
Supported applicability summary, helps-with items, conditions and exclusions still
appear verbatim in the article. Representative questions remain in metadata for
reference retrieval without requiring another prose section.

Publication keeps all five applicability fields in `llm_wiki.applicability` YAML.
Indexing turns these into applicability search units, including representative
questions. Shorter prose therefore does not remove the structured reuse search path.
Existing documents, stored versions, quality validation and publication approval are
unchanged. Prompt versions advance; no extra generation or retry call is added.

AI generation and regeneration of a completion Knowledge draft use the shared **AI request timeout (seconds)** in AI setup (default 180; range 1–3600), including receipt of the complete response body. Queue waiting and local snapshot preparation are outside this deadline. Saving applies to new requests without restarting. A timeout preserves existing drafts and allows an explicit retry. The completion-report queue route uses the same setting.

Completed Tasks can generate a private Knowledge draft from the current Task revision, canonical completion, current Distillation journey, full Run reports and completed evidence, and references that were actually used. For Tasks with a Run, generation waits for current Distillation data. All generation saves only when the same source manifest is still current.

A completed Task with no session Run uses a `recorded_only` snapshot of its definition, completion, ordered manual Work Logs, and decisions. These exact records may support reported facts, but do not establish verified results or final decision authority. Any Task with a Run still requires current Distillation; missing or failed Run Distillation never falls back to manual authority. Editing a recorded log invalidates the generation snapshot.

The article keeps final outcomes, applicability, conditions, exclusions, and limitations together. A final topic outcome must reuse the exact decided claim from the current adopted Distillation node. Verified and decided claims keep their validated epistemic state and verbatim source quote; ordinary source prose remains reported. A receipt, proposal, attempt, or unrelated successful check cannot become approval or verification.

Reusable exploration is stored as a separate private idea revision with its exact `unverified`, `deferred`, `out_of_scope`, or `rejected` disposition, source quotes, and revisit conditions. Idea text is rejected if it leaks into the final article.

Each generation appends an immutable private version. Reading a version recomputes freshness from the current Task, completion, journey, Distillations, evidence, and actual reference usage; the last valid version remains readable as `stale` when those sources change. Restoring an older version appends a new private revision and leaves Task logs, decisions, Distillation history, earlier versions, and the published pointer unchanged.

Task Review presents final outcomes before the article, followed by applicability and its limits. Evidence opens as the exact stored source quote. `Ideas to revisit` stays collapsed and separate from the article, and each idea retains its disposition and reconsideration conditions. The version picker reads stored versions without generation, compares exact bodies and provenance, and restores an older selection as a new private version. Editing the current draft also appends a version; source and content hash guards reject a stale save while preserving the text in the editor.

An existing version keeps an explicit regeneration action so a stale draft or failed generation can be retried. Regeneration is disabled while an edit is unsaved, preserving the editor text until it is saved or cancelled.

Publication remains a separate archive transaction. The review first shows the proposed file paths, computed byte sizes, hashes, readable before/after MOC changes, reference links, and unresolved managed-link repairs. Opening a proposed path shows the frozen Markdown bytes from that immutable proposal. Only a current, conflict-free proposal with no unsaved edits can be submitted. A private or restored revision does not advance the published pointer. The archive may advance it only after writing the exact reviewed body and receiving the required index receipt; `index_pending` is shown as pending and is not published.

An existing publication can be moved, renamed, withdrawn, or repaired only through another reviewed proposal tied to its stable document ID and exact current hash. Index failure offers an explicit retry. A partial or externally changed operation shows its error and offers exact completion or safe compensation of unchanged operation-owned files; compensation never overwrites external edits.

See the [feature specification](../../specs/020-knowledge-idea-distillation/spec.md) and [publication transaction](../../specs/021-organized-knowledge-archive/contracts/publication-transaction.md).

Reference provenance includes actual use from explicitly adopted Capture previews as
well as Task previews. Exact document/version/section matches become one reference
entry, retaining counterevidence and the recorded statements or explicit user reasons.
A candidate, viewed, mentioned or excluded source is not exported on that basis. If a
legacy usage record has no recoverable statement, generation reports unresolved
provenance instead of inventing a reason. Normalizing older reference records or
changing usage can make a saved draft stale; regenerate it before preparing publication.

## Incomplete drafts and human review

A generation with a nonempty article body is retained as a private unverified draft when classification, applicability, or supporting citations are incomplete. A missing title uses the saved Task title provisionally; missing classification remains visibly unclassified. Missing optional lists are filled locally. This does not add a model call or an automatic retry. Invalid JSON/types, an empty body, or a supplied source identity outside the exact snapshot still fails clearly. A quote absent from a known exact source is removed from authenticated citation bindings and retained as plain review text in a blocking finding. The body remains a private unverified draft; neither another valid citation nor a review note grants it publication eligibility.

Unverified versions show the validation issue and cannot prepare or publish an archive proposal. Editing or restoring a version does not silently grant verification. A user can add review notes to an exact immutable version and content hash; notes remain separate, append-only history and never upgrade claims or publication eligibility. Unsaved notes remain protected when switching versions or leaving the review. Saving a note performs only a local database write.

Database schema 23 adds the review-note table without rewriting previous drafts, hashes, sources, or publications.
