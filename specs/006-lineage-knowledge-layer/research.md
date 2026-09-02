# Research: Lineage Knowledge Layer

## Decision 1: Deterministic evidence backbone with optional AI interpretation

**Decision**: Always generate the four-stage lineage and source references deterministically from SQLite. Allow a bounded model call to add only cited `Inferred` interpretations after that backbone exists.

**Rationale**: Completion must produce durable Knowledge even when the provider is unavailable. Facts and human decisions already exist in workflow records and do not need model interpretation. Restricting AI to explicit inference prevents a fluent narrative from being mistaken for history.

**Alternatives considered**: AI-only generation risks outage and hallucination; fully deterministic output loses clearly labeled likely rationale; blocking completion until AI succeeds weakens human authority.

## Decision 2: Append-only event and revision records

**Decision**: Preserve Solution decision events and lineage interpretation revisions as append-only records while keeping existing Solution columns as the current live projection.

**Rationale**: Current manual/refinement updates overwrite fields, so before/after decisions cannot be reconstructed reliably. Corrections also need a durable prior-AI interpretation. Append-only events provide auditability without converting the entire workflow to event sourcing.

**Alternatives considered**: `ai_runs` misses manual decisions; latest-JSON-only storage erases correction history; a full event-sourced rewrite is disproportionate.

## Decision 3: Normalized evidence references and snapshot JSON projection

**Decision**: Store stable evidence references and revision metadata in normalized tables, plus a versioned snapshot JSON projection optimized for rendering.

**Rationale**: Evidence must be queryable and referentially validated, while the browser and Markdown renderer benefit from one ordered document model. The JSON projection is derived and reproducible; normalized records remain the audit source.

**Alternatives considered**: JSON-only storage is weak for evidence validation and revisions; a graph database is unnecessary for a fixed lineage; live rendering would make completed Knowledge drift.

## Decision 4: Fixed four-stage primary graph and progressive disclosure

**Decision**: Show exactly Capture → Problem → Solution → Complete in the primary graph. Include one direct Capture ancestry; expose secondary sources, extra conflicts, and detailed decisions below the graph or in source drill-down.

**Rationale**: A fixed graph communicates lifecycle meaning without turning every event into a node. This matches the product model and keeps mobile rendering tractable.

**Alternatives considered**: Arbitrary-depth graphs add visual complexity; Conflict nodes misrepresent workflow meaning; summary-only cards fail traceability.

## Decision 5: Problem navigation reuses the existing exploration workspace

**Decision**: Selecting the Problem card opens the existing Problem exploration surface in a read-only completed-context mode. Evidence opens in a contextual panel inside the completed workspace.

**Rationale**: This avoids a new screen and respects the non-goal excluding Detail-modal work. Read-only mode prevents completed lineage from accidentally reopening refinement.

**Alternatives considered**: The Detail modal is out of scope; a new page duplicates UI; Markdown-only navigation loses the live workflow connection.

## Decision 6: Explicit confidence vocabulary

**Decision**: Use High, Medium, or Low confidence only on `Inferred` claims and always pair it with `AI inferred`. Observed and Decided claims do not receive confidence scores.

**Rationale**: A small vocabulary is scannable and avoids false precision. Provenance, not confidence, determines authority.

**Alternatives considered**: Numeric percentages imply false precision; no confidence hides uncertainty; confidence on facts blurs provenance.

## Decision 7: Conflict resolution requires structured human/evidence basis

**Decision**: A conflict can become Addressed only when a record identifies `explicit_decision` or `implementation_evidence`, its supporting evidence, and the original requirement disposition. AI may suggest an address but the stored lifecycle status remains Unclear/Unaddressed until supported.

**Rationale**: The current `clear` state and free-text citation cannot prove how a prior conflict was addressed. Structured basis prevents retrospective overclaiming.

**Alternatives considered**: Later `clear` does not prove resolution; AI inference cannot set Addressed; addressed conflicts cannot omit disposition.

## Decision 8: Completion-time snapshot with retryable enrichment

**Decision**: Build and persist the deterministic snapshot synchronously during completion, then render it to Markdown without waiting for a new provider call. Optional AI enrichment is requested explicitly through regeneration, may fail independently, and is source-hash-aware and versioned.

**Rationale**: The final document always gains a lineage structure, completion remains human-controlled, and provider/vault failures have explicit recovery paths.

**Alternatives considered**: A job queue adds unnecessary infrastructure; rebuilding on reads causes drift; overwriting snapshots loses audit history.

## Decision 9: Lineage precedes final-report generation

**Decision**: Generate the final executive summary and report narrative from the selected Lineage projection plus only the evidence excerpts referenced by its claims. Preserve the one-way dependency `source records → Lineage → report → Markdown`.

**Rationale**: The existing report path sends an arbitrary prefix of Raw Data to the model, which can obscure decision meaning and overrepresent whichever records occur first. Lineage has already classified provenance, selected material decisions/conflicts, and connected evidence, so it is a higher-quality and safer synthesis input. The one-way dependency prevents generated prose from citing itself as historical evidence.

**Alternatives considered**: Continuing to prompt from truncated Raw Data is order-sensitive; sending both full Raw Data and Lineage weakens the evidence boundary and token budget; generating Lineage from the final report creates circular provenance.
