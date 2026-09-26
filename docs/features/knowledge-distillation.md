# Evidence-bound Knowledge drafts

**English** | [한국어](knowledge-distillation.ko.md)

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
