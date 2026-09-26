# Research: Final Knowledge and separate idea drafts

## Exact source snapshot and freshness

**Decision**: Reconcile feature 017's journey first, then capture all generation inputs in one read transaction and hash a canonical manifest. Bounded transient retries may run under the shared job policy. Rebuild the manifest before save and reject any mismatch. When sources change, the last-good projection remains visible but is explicitly stale, never falsely current.

**Rationale**: A request-time Task revision check alone misses new Distillation, journey, completion, or actual-use reference evidence. Two guards ensure generation never silently replaces a newer evidence-based draft.

**Alternatives considered**: Generate from the latest visible Task payload; insufficient because it can omit pending journey updates. Save then mark stale; rejected because a stale body would enter revision history as a valid candidate.

## Standalone Knowledge structure

**Decision**: Select one of four declared article structures—concept, guide, comparison/decision, or research result—based only on source-supported Task intent. All structures lead with final outcome(s), then context, explanation/procedure/rationale as applicable, conditions/limits/unresolved matters, applicability, and sources.

**Rationale**: A single activity chronology is not independently useful, while forcing a decision section invents decisions for guides or research without one.

**Alternatives considered**: Always render a chronological journey; rejected as an activity dump. Always render decision/rationale; rejected because non-decision work would gain unsupported finality.

## Claim fidelity validation

**Decision**: Validate unsupported additions and material omissions as separate finding classes. Every factual or decision-bearing claim must cite exact accepted sources. Verification/effectiveness language requires matching observed evidence, and assumptions must be explicit, bounded, and source-linked when derived from user/task constraints.

**Rationale**: A draft can be free of hallucinations yet still be unsafe because it dropped a condition, exception, failure, counterexample, or unresolved issue.

**Alternatives considered**: One aggregate quality score; rejected because it obscures remediation and conflicts with the product's anti-scoring direction. Model self-approval; rejected because the same candidate cannot be the only evidence of its fidelity.

## Idea separation

**Decision**: The final article schema contains no exploration section. A useful exploration may create a separate idea unit with exact sources, Task/final-Knowledge links, one explicit disposition (`unverified`, `deferred`, `out_of_scope`, `rejected`), and reconsideration conditions. No idea unit is required when the exploration has no durable reuse value.

**Rationale**: Separation preserves useful private reasoning without presenting it as an established conclusion or padding the final article.

**Alternatives considered**: Put a clearly labelled ideas appendix in the final article; rejected because downstream readers and retrieval can still conflate it with conclusions. Archive every exploration; rejected because archival occurrence is not evidence of value.

## Immutable draft revisions

**Decision**: Generation, user correction, and restoration append immutable versions. A separate current-private pointer and published pointer identify active states. Restoration copies the selected version into a new revision with `derivation_kind=restore` and does not mutate versions or source history.

**Rationale**: In-place edits cannot support exact comparison, auditability, or publication of the reviewed body.

**Alternatives considered**: Keep mutable latest plus periodic snapshots; rejected because comparison and publication may not identify the exact reviewed bytes. Repoint current to an old revision; rejected because it makes history appear rolled back.

## Publication ownership

**Decision**: Feature 020 freezes and hands off one exact reviewed revision; feature 021 owns file path, Markdown/YAML serialization, MOC/backlinks, atomic write, external-change guard, and indexing. A failed handoff leaves the private and previously published pointers unchanged.

**Rationale**: It preserves one publication authority and avoids duplicating Vault behavior across domain modules.

**Alternatives considered**: Let feature 020 write temporary Markdown; rejected because it duplicates adapter and recovery behavior. Regenerate during publication; rejected because published content could differ from reviewed content.

## Feature 018 metadata compatibility

**Decision**: Use `llm_wiki.schema: 1`, stable `document_id`, `information_type`, applicability fields, and canonical `decisions[]` with one independently confirmed final state per topic. Computed content SHA-256 remains authoritative; declared `source_revision` is provenance only. `out_of_scope` ideas serialize with top-level `status: deferred` plus `idea.disposition: out_of_scope`; the UI always displays the exact disposition.

**Rationale**: This preserves the exact semantic status without turning an entire document or sibling claims final. Unknown nested keys are preserved safely by the current reader.

**Alternatives considered**: Collapse `out_of_scope` into `rejected`; rejected because it changes meaning. Use a singular document-wide decision; rejected because one Knowledge item may carry multiple topic outcomes.

## UI information hierarchy

**Decision**: Keep one readable article as the visual focus. Put compact version/current-state controls near its heading; compare as a deliberate secondary mode; exact sources and assumptions in disclosure; idea units in a separate, lower region.

**Rationale**: Versions and provenance remain discoverable without making every metadata field compete with the user's main review decision.

**Alternatives considered**: Equal cards for article, metadata, versions, and ideas; rejected because it weakens hierarchy. A version-management modal; rejected because comparison and publication context would be separated from the reviewed article.

## Module boundaries

**Decision**: Extract `knowledge_distillation.rs` and `KnowledgeReviewPanel.tsx`; leave minimal route/job integration in existing shared files.

**Rationale**: C owns current journey work, E owns reference-aware Workbench changes, and the existing `task_assistance.rs`/`TaskDetail.tsx` are shared conflict hotspots.

**Alternatives considered**: Extend all logic in the existing files; rejected due to concurrent ownership and poor test isolation. Create a new process/service; rejected as unnecessary complexity.

## E2E scope

**Decision**: Plan focused native E2E only if implementation review finds a residual Tauri publication or restore boundary that component/Rust integration tests cannot cover. Full E2E is not planned.

**Rationale**: The feature is primarily deterministic local state and UI projection. Cheaper tests can cover most risks precisely; packaged E2E is expensive and must be selected by a concrete gap.

**Alternatives considered**: Always run the existing broad Knowledge scenario; rejected because it tests unrelated work and violates repository guidance.
