# Data Model: Organized Knowledge and idea archive

## Canonical boundaries

- Feature 020 remains canonical for private Knowledge/idea drafts, immutable draft versions, selected publication inputs, and the published pointer.
- Feature 018 remains canonical for indexed documents, units, exact file revisions, embeddings, ranking, redirects, and evidence grants.
- Feature 019 remains canonical for viewed/mentioned/used/adopted/excluded interaction facts and exact reference snapshots.
- Feature 015 remains canonical for prompt versions, async job execution, application disposition, provenance, and generic document references.
- G owns archive proposals, published logical document identity/revisions, canonical path history, eligible published-source linkage, and the cross-store publication/recovery journal. It stores no second draft body or search index.

## ArchiveProposal

Immutable reviewed organization plan for one exact publication input set.

| Field | Rule |
| --- | --- |
| proposal_id | Stable UUID |
| proposal_version | Monotonic immutable version; a changed user choice appends rather than mutates |
| task_id / knowledge_revision | Exact feature 020 owner/version |
| knowledge_content_hash / generation_snapshot_hash | Exact handoff heads |
| selected_idea_revisions | Ordered explicit IDs; empty by default |
| taxonomy_snapshot_hash | Canonical hash of relevant directories/metadata/MOCs/overlap candidates |
| outcome | `new`, `update`, `merge`, `conflict`, or `supersede` |
| target_document_id / target_revision | Required for update/merge/supersede; absent for new; candidate-only for conflict |
| rationale | Reviewable organization rationale, including why existing taxonomy is suitable |
| new_category_rationale | Required only when no existing category is selected |
| artifacts_json | Ordered frozen `ArchiveArtifact` values |
| moc_patches_json / link_repairs_json | Ordered reviewed bounded patches |
| proposal_hash | Canonical hash of all apply-relevant fields |
| prompt_id / prompt_version | Registry identity for model-assisted recommendation, nullable for deterministic/manual proposal |
| state | `preparing`, `review_needed`, `conflict`, `stale`, `applied`, or `superseded` |
| created_at | Immutable timestamp |

Only one proposal version can be applied. A proposal with outcome `conflict` cannot be applied.

## ArchiveArtifact

Frozen portable file within a proposal; stored inside proposal payload and journal, not as a second draft table.

| Field | Rule |
| --- | --- |
| artifact_id | Stable within proposal |
| kind | `knowledge`, `idea`, `moc`, or `managed_link_repair` |
| information_type / status | Explicit metadata; never inferred only from path |
| source_revision_id | Exact feature 020 Knowledge or idea revision |
| document_id | Existing stable ID or allocated stable ID for new artifact |
| relative_path | Validated canonical Markdown path |
| expected_file_hash | Current authoritative SHA-256, or null only for reviewed create |
| bytes | Exact reviewed UTF-8 YAML + Markdown bytes |
| artifact_hash | SHA-256 of `bytes`; expected authoritative revision after write |
| body_hash | Hash of exact feature 020 body portion for audit |
| tags / aliases | Bounded normalized portable metadata |
| disposition | Exact idea disposition when kind is `idea` |

Final Knowledge artifacts cannot contain idea sections. Idea artifacts require an exact disposition and related final document/task identity.

## ArchiveDocument

Stable published logical identity.

| Field | Rule |
| --- | --- |
| document_id | Stable across move, rename, replacement, withdrawal, and restore |
| information_type | `knowledge` or `idea`; future typed sections do not change document identity |
| current_revision | Monotonic pointer to the last fully indexed/applied `ArchiveRevision`; pending revisions do not advance it |
| current_path | Path of the last fully applied revision; unique while active |
| lifecycle | `current`, `superseded`, or `withdrawn` |
| created_at / updated_at | Metadata only |

## ArchiveRevision

Immutable publication fact.

| Field | Rule |
| --- | --- |
| document_id / revision | Composite identity |
| operation_id | Publication journal that created it |
| path | Exact path at this revision |
| file_hash | SHA-256 of actual full bytes; authoritative revision |
| declared_source_revision | Provenance from YAML, not file identity |
| body_hash | Reviewed body audit hash |
| source_draft_revision | Exact feature 020 revision |
| revision_kind | `publish`, `update`, `merge`, `supersede`, `move`, `rename`, `withdraw`, or `repair` |
| prior_revision | Previous revision for this document, nullable on create |
| successor_document_id | Required when superseded by a different logical document |
| indexed_state | `pending`, `current`, `failed`, or `removed` |
| created_at | Immutable timestamp |

The same file hash may appear at a new path revision after a move. Reading historical revisions never changes the current pointer.

## ArchivePathHistory

| Field | Rule |
| --- | --- |
| document_id / sequence | Stable owner/order |
| old_path / new_path | One may be null for create/withdraw |
| reason | `publish`, `move`, `rename`, `withdraw`, or `restore` |
| revision | Archive revision that established the change |
| created_at | Immutable timestamp |

Path history supports redirect display and managed-link repair. It does not authorize rewriting arbitrary user links.

## PublishedReferenceLink

| Field | Rule |
| --- | --- |
| owner_document_id / owner_revision | Published artifact containing the link |
| source_document_id / source_revision | Exact feature 018 identity and authoritative revision |
| source_section | Optional exact section identity |
| role | `support` or `counterevidence`, backed by an actual-use fact |
| rationale | Required human-readable usage explanation |
| rendered_target | Reviewed portable wikilink target at publication |
| source_interaction_id | Exact feature 019 provenance fact |

Duplicate identity + revision + section + role links coalesce. A later path change updates managed rendered targets through a reviewed proposal while historical rows remain immutable.

## ManagedMocPatch

Stored in proposal/journal payload.

| Field | Rule |
| --- | --- |
| moc_path | Existing or reviewed-new MOC Markdown path |
| region_id | Stable unique marker ID |
| expected_file_hash | Authoritative full-file precondition |
| expected_region_hash | Hash from start marker through end marker, null for reviewed insertion |
| replacement_region_bytes | Exact reviewed managed block |
| expected_post_hash | Hash of exact resulting full file |
| insertion_anchor | Required only for a missing reviewed region |
| linked_document_ids | Deduplicated stable identities |

Apply must prove prefix and suffix outside the managed region are byte-identical to the reviewed proposal.

## PublicationOperation

Durable application/journal record. It complements, rather than replaces, the feature 015 job row.

| Field | Rule |
| --- | --- |
| operation_id | Caller idempotency identity |
| proposal_id / proposal_version / proposal_hash | Exact approved plan |
| payload_hash | Replay equality guard |
| state | `approved`, `writing`, `files_written`, `recorded`, `index_pending`, `complete`, `conflict`, `index_failed`, `repair_required`, or `compensated` |
| execution_state | Feature 015 execution state for the current phase |
| application_disposition | Feature 015 value `pending`, `applied`, `superseded`, or `review_needed`; file success alone remains `pending` |
| index_job_id | Existing `publication_index` job, nullable until recorded |
| safe_error | Bounded error without file bytes/provider payload |
| created_at / updated_at | Audit timestamps |

`files_written` is not application completion. `recorded`/`index_pending` means durable candidate bytes and pending revision records exist, while G's current pointer and feature 020's published pointer remain on the last applied revision. An exact index receipt and both pointer updates commit together with `complete/applied`.

## PublicationStep

| Field | Rule |
| --- | --- |
| operation_id / ordinal | Ordered composite identity |
| action | `create`, `replace`, `move`, `withdraw`, `moc_patch`, `link_patch`, `record`, or `index` |
| document_id / path / destination_path | Applicable stable/path identities |
| expected_pre_hash / expected_post_hash | CAS and idempotency evidence |
| recovery_path / recovery_hash | Exact operation-owned backup when prior bytes exist |
| state | `planned`, `staged`, `applied`, `verified`, `compensated`, `conflict`, or `repair_required` |

## Migration 22

Tentative migration 22 adds archive proposal, document, revision, path-history, published-reference, operation, and operation-step tables with foreign keys and uniqueness constraints. Large exact artifact/patch payloads may be canonical JSON blobs in proposal/operation rows to avoid table proliferation; hashes and identity fields remain indexed columns.

The migration runs only when schema head is 21. It preserves all existing rows and validates pre/post counts and foreign keys. Existing published draft rows are not rewritten. On first access, the service may adopt an existing publication by reading actual bytes, matching the feature 020 published path/hash, and creating G identity/revision rows in one transaction. A mismatch becomes `repair_required`; it never rewrites the file. Search units remain feature 018-owned and rebuildable.

## State transitions

```text
exact F revision + selected ideas
  → preparing proposal
  → review_needed | conflict

review_needed + changed source/taxonomy/target hash
  → stale (no writes)

explicit publish + all exact heads
  → approved → writing → files_written
  → recorded → index_pending
  → exact index receipt + pointer CAS → complete/applied

write/hash mismatch
  → conflict (no overwrite) | repair_required (partial operation preserved)

index transient failure
  → index_pending (bounded automatic retry/restart recovery)
  → complete | index_failed (explicit retry available; no file rewrite)

move/rename/withdraw
  → new reviewed proposal → journaled CAS steps
  → append revision/path history → index remove/upsert → complete
```
