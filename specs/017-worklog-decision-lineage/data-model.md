# Data Model: Evidence-Grounded Work Distillation

This document defines feature-owned logical records and their mapping to the shared feature-015 foundation. Migration 18 adds the feature-owned projection tables after Capture provenance migration 17.

## Shared foundation records (feature 015 ownership)

### Prompt Definition and Prompt Version

- Immutable prompt key/version and content hash.
- Input and output schema identifiers.
- Rules compatibility and activation metadata.
- Feature 017 resolves named Work Log and Task journey Distillation prompts through this registry; it does not store prompt text independently.

### Exact Source Reference

- `source_type`, stable `source_id`, immutable `source_revision`.
- Optional field/item/offset locator and source content hash.
- Owner scope (`task_id`, optional `run_id` and `work_log_entry_id`).
- A reference remains valid only for the exact content it names.

### Durable Workflow Job and Result Envelope

- Projection owner, idempotency key, exact input manifest/hash, prompt version, rules version, result schema version, locale, expected prior projection revision.
- Queued/leased/running/retryable/terminal/stale state, attempt/lease/heartbeat/backoff/error metadata.
- Candidate result or validation errors.
- Shared stale-completion and idempotent replay rules.

Feature 017 consumes these contracts and owns the result payload and publication semantics below.

## Distillation Input Manifest

An immutable feature payload inside the shared job input.

### Fields

- `projection_kind`: `work_log` or `task_journey`.
- `task_id` and exact `task_revision`.
- Optional `run_id`, exact Run `revision`, and `work_log_entry_id` for Run-centered Distillation.
- `primary_source`: exact final-report reference when present, otherwise null with a missing-report reason.
- `corroborating_sources`: ordered exact references to completed evidence or Task records selected by deterministic policy.
- `current_projection_revision` and affected stable node/claim IDs.
- `source_set_hash`, locale, prompt key/version, rules version, result schema version.
- Selection notes: truncation, contradiction candidates, deletion tombstones, and full-repair reason when applicable.

### Validation

- All sources belong to the Task and, where applicable, the same Run.
- A final report is primary whenever one exists.
- Raw delta fragments and routine control-only events are not eligible.
- Missing or deleted sources are represented as tombstones, not silently omitted from invalidation.

## Distillation Result

The structured, feature-owned payload inside the shared result envelope.

### Fields

- `claims`: ordered Distilled Claims.
- `node_changes`: candidate semantic actions for the Task journey.
- `relationship_changes`: evidence-backed link candidates.
- `topic_states`: explicit adopted/superseded/withdrawn/unresolved states.
- `completion_snapshots`: immutable completion interpretations tied to exact completion evidence.
- `work_log_view`: prepared readable section ordering and claim references.
- `warnings`: contradictions, missing reports, truncated inputs, or unresolved facts.
- `dependency_manifest`: exact source-to-claim/link dependencies used for targeted invalidation.

The result does not become current merely because it is well-formed. It must pass feature validation and compare-and-publish checks.

## Distilled Claim

### Fields

- Stable claim ID within its projection lineage.
- `kind`: `goal`, `scope`, `criterion`, `decision`, `reason`, `attempt`, `performed`, `evidence`, `result`, `failed_approach`, `condition`, `artifact`, `unresolved`, or `completion`.
- Concise statement and optional structured values such as paths, quantities, check names, or outcomes.
- `actor`: `user`, `ai`, `tool`, `system`, or `unknown`.
- `epistemic_state`: `suggested`, `decided`, `attempted`, `performed`, `observed`, `verified`, or `unresolved`.
- `status`: `current`, `historical`, `contradicted`, or `unknown`.
- One or more exact source references; optional contradiction references.
- Optional topic key, semantic node ID, and display-order hint.

### Invariants

- Every material claim has at least one valid exact source.
- `verified` requires observed verification evidence; a final-report claim alone is insufficient.
- `decided` requires an explicit user decision or authoritative saved decision record.
- Reasons and causal relationships require their own supporting source; adjacency is insufficient.
- A contradiction is visible and cannot be resolved by silently dropping one source.

## Journey Node

### Identity

- Task-scoped stable ID derived from canonical node kind, normalized topic/origin key, and semantic occurrence identity.
- Locale, copy edits, evidence enrichment, job ID, projection revision, and chronological position are excluded from identity.
- A new actual decision, a return to an earlier option for a new reason, or a new completion creates a new ID.

### Fields

- `kind`: goal, scope, criterion, decision, evidence, result, or completion snapshot.
- Title and compact summary derived from supported claims.
- Actor/provenance and epistemic state.
- Topic key and current status when applicable.
- Prepared detail: before, after, reason, evidence claim IDs, result, current status, and related node/source links.
- First/last supported source revision and current projection revision.

### Lifecycle

- `active`: current and supported.
- `historical`: supported but not current.
- `superseded`: replaced by an explicit later decision.
- `withdrawn`: explicitly withdrawn with no replacement required.
- `unresolved`: alternatives or outcome remain open.
- `invalidated`: retained in prior projection history but absent from current publication because its only source was removed or corrected.

## Node Change

- `add`: create a new semantic node.
- `enrich`: attach supported detail/evidence to the same node without changing its meaning.
- `merge`: combine duplicate semantic nodes, retain one canonical ID, and persist redirects for aliases.
- `omit`: publish no semantic change for a nonmeaningful or unsupported candidate.
- `supersede`: create a new node and a sourced `supersedes` relationship; never mutate the prior decision into the new one.

Every non-omit action names expected prior node revisions and exact evidence. Unknown targets or stale revisions fail validation.

## Journey Relationship

### Kinds

- `derived_from`: explicit source says new work or conclusion stems from earlier material.
- `supports`: evidence supports a claim or result without proving full verification.
- `verifies`: an observed check verifies a bounded performed/result claim.
- `contradicts`: sources materially disagree.
- `merged_into`: duplicate semantic identity redirect.
- `supersedes`: a later explicit decision replaces an earlier choice.

`followed_by` remains available only in the original recorded event trace; it is not promoted to a semantic relationship.

An evidence-backed relationship proposed by AI without explicit causal or decision evidence remains an `ai` / `suggested` interpretation. It cannot set topic state, supply a decision reason, or use `verifies`/`supersedes` as though proven.

## Decision Topic State

- Stable Task-scoped topic key.
- Current state: `adopted`, `superseded`, `withdrawn`, or `unresolved`.
- Current node ID when adopted; optional replacement node for superseded state.
- Exact decision/status sources and reason sources.
- Updated projection revision.

Chronology is a display ordering input only. It cannot determine state without explicit evidence.

## Completion Snapshot

- Stable ID keyed to the exact completion record identity, not the Task's latest state.
- Task/completion revision and timestamp.
- Supported completion claim IDs, evidence/result claim IDs, unresolved items, and verification state.
- `current`, `historical_after_reopen`, or `superseded_by_later_completion` display status.

Reopening a Task does not mutate the earlier snapshot.

## Projection Revision

- Owner: Run-linked Work Log entry or Task journey.
- Monotonic revision, shared result-envelope identity, source-set hash, prompt/rules/schema versions, locale, creation time.
- Complete saved result/document, dependency manifest, and publication status.
- Prior revision remains inspectable; exactly one current revision per owner.
- “Current” means the latest validated working projection for that owner, not user approval. The application disposition remains pending until exact-source compare-and-publish succeeds, then records `applied` in that same transaction. This does not adopt a source decision.

### Publication transition

```text
candidate generated
  -> schema/source validation
  -> semantic transition validation
  -> compare exact inputs + expected prior revision
  -> atomically publish current revision

validation failure | stale inputs | stale expected revision
  -> retain last-good current revision
  -> candidate is diagnostic/stale, never current
```

## Source invalidation

- Reverse dependencies map exact source revisions to claims and relationships.
- Edit: introduce a new exact revision and invalidate dependencies on the prior revision.
- Delete: introduce a tombstone and remove or recompute only dependent semantic content.
- Recompute direct dependents, then topic state, prepared detail, and other downstream links reachable from those dependents.
- Unrelated nodes, claims, IDs, and prior projection revisions remain unchanged.

## Preservation boundary

No feature-owned operation mutates Run final reports, Run evidence items, manual Work Log bodies, attachments, comments, Task decisions, Task completions, original Task journey events, or legacy Solution lineage snapshots.
