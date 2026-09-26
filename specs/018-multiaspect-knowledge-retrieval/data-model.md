# Data Model: Multi-aspect Knowledge Retrieval

## DocumentDescriptor

| Field | Type | Rules |
| --- | --- | --- |
| `document_id` | string | Explicit stable ID when supplied; otherwise deterministic path identity. |
| `path` | string | Normalized Vault-relative path; never the evidence revision. |
| `source_revision` | SHA-256 | Always computed from actual file bytes. |
| `declared_revision` | string? | Optional provenance only; cannot override `source_revision`. |
| `information_type` | enum | `knowledge` or `idea`; defaults to `knowledge` only when absent. |
| `status` | enum? | `current`, `historical`, `unverified`, `deferred`, or `rejected`. |
| `metadata_warning` | string? | Bounded parse/validation warning; body remains searchable. |

## ApplicabilityMetadata

Optional explicit values: `summary`, `representative_questions`, `helps_with`, `conditions`, and
`exclusions`. Recognized applicability headings may supply the same units from body text. Missing
values stay absent.

## DecisionMetadata

| Field | Type | Rules |
| --- | --- | --- |
| `id` | stable string | Required for lineage lookup. |
| `topic` | stable string? | Keeps multiple decisions in one document independently scoped. |
| `status` | enum | `adopted`, `superseded`, `withdrawn`, or `unresolved`. |
| `confirmed_final` | boolean | Only true or a C completion snapshot permits final-decision treatment. |
| `successor_id` | string? | Required to redirect a superseded decision; may resolve across documents. |
| `reason` | string? | Explicit saved rationale only. |
| `conditions` | string[] | Conditions under which this conclusion applies. |

Canonical metadata stores `decisions[]`; singular `decision` remains a one-item compatibility alias.
Each entry produces its own search unit. A confirmed decision affects only that identity/topic and
never promotes the containing document or sibling decisions to final status.

## SearchUnit

Stable derived identity consists of `document_id`, `source_revision`, `section_locator`, `aspect`,
and `unit_id`. Aspects are `applicability`, `decision`, `content`, and `exploration`. Each unit stores
bounded text plus zero-based chunk position/count, information type/status, decision link fields,
provenance, and an `input_hash` over
the exact embedding input. Unit state changes by replacement at a new source revision; it is never
silently mutated into a newer source.

## EmbeddingIdentity

`unit_id + source_revision + input_hash + model_id + model_version + dimensions`. Every component
must match before reuse or scoring. Malformed byte length, incompatible dimension/model, or stale
source revision excludes the vector without excluding lexical evidence.

## RetrievalQuery

Contains normalized text, optional applicability context/conditions, scope, requested limit/offset,
and semantic preference. Query-context identity participates in result-cache validity.

## RankedEvidence

Carries stable unit/document identity, exact source revision, path, section, aspect, status,
conditions, lexical and/or semantic rank, fused score, passage, optional historical match, optional
successor, and bounded warnings. Pagination occurs after fusion and same-document deduplication.

## DecisionSuccessor

Resolution walks decision IDs with a visited set. A valid redirection terminates at a confirmed-final
`adopted` decision whose conditions are compatible with the query. Missing targets, cycles,
withdrawn/unresolved nodes, and condition mismatches yield a qualified unresolved result rather than
coercing a current recommendation.
