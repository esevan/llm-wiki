# Data Model: Final Knowledge and separate idea drafts

## KnowledgeDraftVersion

An immutable private or published review artifact for one completed Task.

| Field | Meaning and validation |
| --- | --- |
| `task_id` | Stable owning Task ID. |
| `revision` | Monotonic per-Task revision; never reused or updated in place. |
| `parent_revision` | Previous current-private revision, when any. |
| `derived_from_revision` | Exact source revision for edit/restore/regenerate derivation. |
| `derivation_kind` | `generated`, `edited`, `restored`, or `regenerated`. |
| `article_type` | `concept`, `guide`, `comparison_decision`, or `research_result`; must match supported source intent. |
| `title` | Standalone title. |
| `body_markdown` | Final Knowledge body only; no intermediate exploration or idea appendix. |
| `content_hash` | SHA-256 of exact reviewed body and canonical structured metadata. |
| `generation_snapshot_id` | Exact EvidenceSnapshot used by generated/regenerated versions; inherited as provenance for edit/restore. |
| `applicability_id` | Required structured applicability. |
| `quality_state` | `valid`, `blocked_unsupported`, or `blocked_omission`; only `valid` can become current/published. |
| `model_status` / `model_error` | Bounded provider result without secrets; fallback does not overwrite valid content. |
| `created_at` | Stable creation time. |

Bodies and hashes are immutable. User edits create a new revision, even when based on the current draft. A byte-identical idempotent replay returns the existing revision.

## KnowledgePointers

One logical pointer set per Task.

| Field | Meaning and validation |
| --- | --- |
| `current_private_revision` | Revision shown as the editable private draft; nullable. |
| `published_revision` | Exact successfully published revision; nullable and independent of current private. |
| `publication_document_id` | Stable feature 021 document identity, once assigned. |
| `publication_content_hash` | Authoritative content hash returned by feature 021. |
| `updated_at` | Pointer change time. |

Publishing does not delete or hide the current private revision. Restoring never changes `published_revision`.

## EvidenceSnapshot

Immutable canonical manifest used for one generation attempt.

| Field | Meaning and validation |
| --- | --- |
| `id` | Stable snapshot ID. |
| `task_id` / `task_revision` | Exact completed Task definition. |
| `distillation_refs` | Primary Run final-report Distillation plus exact corroborating/contradicting evidence references. |
| `journey_projection_revision` | Exact feature 017 incremental journey revision. |
| `journey_source_set_hash` | Feature 017 authoritative source-set hash. |
| `completion_snapshot_id` / `completion_snapshot_revision` | Exact final completion snapshot used. |
| `topic_decisions` | Final/superseded/unresolved states by stable topic; suggestions alone cannot be final. |
| `actual_reference_usages` | References whose disposition is actual use/adoption/counterevidence; excludes candidate/view-only records. |
| `assumptions` | Explicit bounded assumptions and exact supporting source references. |
| `manifest_hash` | SHA-256 over canonical ordered fields and each source content hash. |
| `freshness` | Must be `current` at capture; stored snapshots remain historical evidence thereafter. |
| `created_at` | Capture time. |

Every reference carries stable type/ID, exact revision, locator/section, content hash, role, and bounded display label. No mutable “latest” pointer is accepted as evidence.

## ApplicabilityDescription

Required structured scope for every Knowledge draft.

| Field | Meaning and validation |
| --- | --- |
| `summary` | Concise use situation supported by the body. |
| `representative_questions` | At least one question the article can answer. |
| `helps_with` | Concrete supported help, not a capability promise beyond the body. |
| `conditions` | Preconditions or qualifying situations. |
| `exclusions` | Explicit non-applicable scope and limits. |

Validation rejects applicability broader than the body or cited sources.

## IdeaDraft

A separate optional private unit derived from useful exploration.

| Field | Meaning and validation |
| --- | --- |
| `id` | Stable idea identity across its revisions. |
| `task_id` | Originating Task. |
| `revision` | Monotonic immutable idea revision. |
| `knowledge_revision` | Related final Knowledge draft revision. |
| `title` / `body_markdown` | Portable idea content; never included in final Knowledge body. |
| `disposition` | Exactly one of `unverified`, `deferred`, `out_of_scope`, `rejected`. |
| `reconsideration_conditions` | Required evidence/event/condition that could justify revisiting. |
| `source_refs` | At least one exact exploration/evidence source. |
| `content_hash` | SHA-256 of exact content and metadata. |
| `publication_state` | `private`, `published`, `withdrawn`; feature 021 owns transitions to published/withdrawn. |

Archival occurrence never upgrades the disposition. Promotion to final Knowledge requires later evidence and a new Knowledge generation/review revision.

## QualityFinding

| Field | Meaning and validation |
| --- | --- |
| `id` | Stable within candidate validation. |
| `kind` | `unsupported_addition`, `material_omission`, `idea_leak`, `invalid_finality`, `invalid_verification`, `applicability_overreach`, or `missing_reference`. |
| `severity` | `blocking` or `warning`; all unsupported claims/material omissions are blocking. |
| `claim_or_section` | Candidate claim ID or section locator. |
| `source_refs` | Evidence supporting the finding. |
| `message` | Bounded user-readable explanation. |

There is no aggregate user or quality score.

## PublicationHandoff

| Field | Meaning and validation |
| --- | --- |
| `operation_id` | Idempotency key. |
| `task_id` / `knowledge_revision` | Exact selected valid revision. |
| `expected_content_hash` | Must match stored immutable body. |
| `expected_generation_snapshot_hash` | Snapshot provenance check. |
| `body_markdown` | Exact reviewed bytes; feature 021 must not regenerate them. |
| `metadata` | Contracted `llm_wiki` data and exact source/assumption links. |
| `idea_revision_ids` | Explicitly selected separate ideas, default empty. |

## State transitions

```text
generation requested
  -> waiting_for_journey
  -> generating(snapshot S)
  -> stale_rejected | validation_blocked | private revision N

private revision N --edit--> private revision N+1
private revision N --restore older R--> private revision N+1 (derived_from=R)
private revision N --publish--> published pointer=N (only after feature 021 success)
published pointer N --new edit/generation--> remains N; current private becomes later revision
```

Task definitions, Work Logs, decisions, completion snapshots, Distillation, and journey revisions are read-only inputs throughout every transition.

## Migration invariants

The tentative `F21` migration is implemented only after central assignment. It must preserve all existing `task_knowledge_drafts` bodies, content hashes, source lineage, state, paths, and published hashes; map each row to an immutable version; derive current/published pointers deterministically; be idempotent; restore foreign-key enforcement; and roll back atomically on any invariant failure.
