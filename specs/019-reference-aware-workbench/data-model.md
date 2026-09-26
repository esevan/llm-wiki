# Data Model: Reference-aware workbench

## Canonical boundaries

- Existing Task rows and revisions remain canonical for adopted work definition.
- Existing refinement sessions/messages remain canonical for conversation and saved composer state.
- Feature 018 remains canonical for document identity, source revisions, chunks, embeddings, ranking, redirects, and evidence grants.
- Feature 015 remains canonical for prompt identity/version, job execution state, application disposition, generic provenance, and document references.
- E owns immutable work-preview versions, exact generation/reference snapshots, investigation disposition, mention bindings, and interaction/use facts. It does not duplicate document bodies, Task history, Work Logs, decisions, or Knowledge drafts.

## WorkPreview

Current pointer for one refinement subject.

| Field | Rule |
| --- | --- |
| preview_id | Stable ID for one Capture/Task refinement lineage |
| subject_type / subject_id | Existing `capture` or `task` identity; unique active preview lineage |
| current_version | Monotonic immutable preview version |
| task_revision | Nullable for Capture; exact canonical Task revision observed by current version |
| context_revision | Exact refinement message/context head |
| updated_at | Preview metadata only; does not change Task activity ordering |

## WorkPreviewVersion

Immutable structured proposal, never the canonical Task by itself.

| Field | Rule |
| --- | --- |
| preview_id / version | Composite immutable identity |
| derivation_kind | `generated`, `edited`, `restored`, or `reconciled` |
| derived_from_version | Required for restored/reconciled, optional for edited, absent for initial generation |
| prompt_id / prompt_version | Registered E prompt identity |
| generation_job_id | Nullable for manual edit/restore; exact foundation job when generated |
| task_revision / context_revision | Exact input heads |
| description | Required complete working description |
| background | Known context, distinct from assumptions |
| goal | Intended outcome, not observed result |
| scope / non_goals | Explicit included and excluded work |
| constraints | Ordered explicit constraints |
| completion_criteria | Observable completion conditions, never auto-completed |
| initial_approach | Proposed approach, not an execution record |
| content_hash | Canonical hash of all structured fields and locale |
| locale | `en` or `ko`; paired localized fields follow existing Task localization rules |
| created_at | Immutable creation time |

Every append validates field sizes, known keys, localized equivalence rules, and exact current heads. Reading a historical version does not move `current_version`.

## DraftAssumption

| Field | Rule |
| --- | --- |
| preview_id / version / assumption_id | Immutable owner and stable item identity |
| text | Explicit bounded statement |
| status | `open`, `confirmed`, `rejected`, or `superseded` |
| basis | `user_context`, `source`, or `model_inference` |
| source_claim_ids | Zero or more exact source reference claim IDs |
| supersedes_id | Optional prior assumption in the same lineage |

Assumptions never masquerade as evidence. Confirmation/rejection appends a new preview version or explicit status fact; silence is not confirmation.

## PreviewReferenceSnapshot

Immutable generation/use snapshot, linked to foundation document references.

| Field | Rule |
| --- | --- |
| snapshot_id | Stable identity |
| preview_id / version | Exact owner |
| retrieval_request_hash | Exact normalized intent/request identity |
| source_bundle_hash | Canonical ordered hash of included reference bindings |
| document_id / document_version | Required feature 018 identity |
| section / chunk_index | Optional exact passage address |
| aspect / information_type / status | Copied bounded feature 018 evidence metadata |
| role | `support`, `counterevidence`, `constraint`, `historical`, or `context` |
| claim_ids | Preview claim/field bindings actually supported by this reference |
| successor | Optional current decision/document redirect snapshot |
| warnings | Stored qualifications present at generation time |

The snapshot stores identity and bounded excerpts/metadata needed for review, not a second canonical document body. Archive/move operations may change path display while the stable ID/version binding remains valid.

## Investigation

| Field | Rule |
| --- | --- |
| investigation_id | Stable latest-wins operation identity |
| preview_id / context_revision | Exact conversation context requested |
| kind | `speculative_search` or `conflict_review` |
| state | `queued`, `running`, `completed`, `failed`, `cancelled`, or `changed_context` |
| query / aspects / filters | Normalized retrieval request |
| source_bundle_hash | Present only for completed evidence |
| started_at / finished_at | Operational timestamps |
| safe_error | Bounded user-safe failure, no provider payload |

Only one current active investigation exists per preview/kind/context. A newer context cancels or supersedes older active work. Optional failures do not block messages or requested preview generation.

## InvestigationFinding

| Field | Rule |
| --- | --- |
| finding_id / investigation_id | Immutable identity |
| priority | `critical`, `supporting`, or `ancillary` |
| summary | Grounded bounded statement |
| reference_snapshot_ids | At least one exact source binding |
| delivered_in_message_id | Optional next reply that consumed supporting evidence |
| surfaced_at | Optional prompt alert time for critical finding |

Changed-context findings cannot be surfaced or delivered as current. They may remain auditable with that disposition.

## DocumentMention

| Field | Rule |
| --- | --- |
| mention_id | Stable token identity independent of visible title |
| session_id / draft_revision | Composer binding and saved draft head |
| document_id / document_version | Required exact identity |
| section | Optional exact section path |
| display_label | User-facing title/section captured for rendering only |
| range_anchor | Structured token/order metadata, not a fragile path-only string |

Selecting one or many mentions inserts tokens and never sends the message. Removed tokens remain absent from the submitted message binding.

## ReferenceInteraction

Append-only fact separating discovery from authority.

| Field | Rule |
| --- | --- |
| interaction_id | Stable identity |
| preview_id / context_revision | Context in which interaction occurred |
| document_id / document_version / section | Exact source identity |
| kind | `viewed`, `mentioned`, `used`, `adopted`, or `excluded` |
| use_scope | Optional preview field, reply, claim, or counterevidence role |
| reason | Required for `used`, `adopted`, and `excluded` |
| preview_version / message_id | Exact consuming artifact when applicable |
| created_at | Interaction time |

`viewed` and `mentioned` never imply `used`; `used` never implies `adopted`; only an explicit canonical Task apply can append `adopted`. Actual-use and useful counterevidence facts survive later archive/path changes and are eligible for F/G lineage.

## ReferenceViewState

Client-persisted workspace state, not evidence provenance.

| Field | Rule |
| --- | --- |
| session_id | Owner |
| query / filters / sort | Explicit list controls |
| order_epoch / ordered_reference_ids | Frozen visible order during an arrival batch |
| selected_reference_id | Current list selection |
| viewer_history / history_index | Exact document-version-section navigation stack |
| composer_selection | Cursor/token selection anchor |
| preview_scroll / conversation_scroll / reference_scroll | Independent scroll anchors |
| focused_control | Restorable semantic control key |

Server persistence is required for saved draft and semantic selection; transient pixel offsets may remain client-local while the dialog is mounted. Reopening restores meaningful anchors without inventing evidence facts.

## State transitions

```text
requested preview
  → planner(retrieval intent)
  → needed retrieval? searching → results | no_suitable_result | failed
  → validate exact context/source bundle
  → append generated preview version (review_needed)

explicit apply + exact Task/preview/edit heads
  → canonical Task mutation + adopted/used facts + applied disposition

explicit apply + changed head
  → conflict; no Task mutation; edited preview remains available

follow-up message
  → context revision advances
  → older optional investigation cancelled/changed_context
  → conversation continues; newest optional investigation may run

historical select
  → read only; no pointer/job/motion/application change

restore version N + exact preview head
  → append version M derived from N; M becomes preview head
  → Task, Work Logs, decisions, journey, and messages unchanged
```
