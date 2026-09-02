# Research: Conflict Resolution Workflow

## Decision 1: Normalize at the AI handler boundary

**Decision**: Expand the existing per-passage JSON response to return category, summary, current claim, existing claim, impact, recommendation, and evidence identity, then normalize and default every field before it reaches storage or UI.

**Rationale**: The handler already validates that the model's `evidence_id` matches retrieved evidence and builds citations from trusted retrieval metadata. Normalizing there gives every downstream consumer one stable shape without asking the browser to parse prose.

**Alternatives considered**: Parsing the existing explanation in JavaScript is not a stable contract. A second aggregation model call adds latency, token cost, and another failure mode without improving evidence trust.

## Decision 2: Add normalized run-scoped persistence

**Decision**: Add `conflict_review_conflicts` and `conflict_resolutions`, keyed by run and conflict, while retaining `report_json` and existing `conflict_reports`/`conflict_addresses`.

**Rationale**: The report JSON preserves backward-compatible queue results; normalized rows provide the requested queryable Conflict → Resolution → Rationale history. Reusing the existing report/address model preserves approval and lineage behavior.

**Alternatives considered**: JSON-only resolutions make prior accepted conflicts hard to query and validate. Replacing existing report/address tables creates unnecessary compatibility risk.

## Decision 3: Aggregate actions onto existing gate semantics

**Decision**: When every conflict is accepted with rationale, atomically create a clear conflict evaluation plus explicit conflict address and persist all item resolutions. If any item applies the recommendation, persist the resolutions but keep the Solution conflicted so revision and a fresh review are required.

**Rationale**: This distinguishes an intentional exception from a required change while preserving the invariant that only clear current context can enter In progress.

**Alternatives considered**: Marking any completed review clear is unsafe when revision was chosen. Adding a fourth feature conflict state creates broad churn when the address record already captures the nuance.

## Decision 4: Reuse the existing dialog with card semantics

**Decision**: Keep `item-detail-modal`, introduce dedicated conflict review markup/classes, make its body scrollable, collapse evidence with native `details`, and make the footer sticky.

**Rationale**: It preserves current queue navigation and visual language while correcting the report UX. Native radio, details, and textarea controls provide keyboard behavior without a component dependency.

**Alternatives considered**: A new page expands navigation and state for a bounded review step. A frontend framework violates minimal complexity for one modal.

## Decision 5: Preserve legacy results as secondary detail

**Decision**: Normalize historical `findings` into the new card contract on read/render using deterministic fallback IDs and values. Legacy reports remain reviewable; persistence is allowed only when a valid current run and known conflict identities can be verified.

**Rationale**: This keeps cached and queued results readable without weakening resolution validation or inventing data.

**Alternatives considered**: Invalidating all cache history breaks useful records. Startup migration is broader and riskier than lazy normalization.
