# Research: Evidence-Grounded Work Distillation

## Decision: Use the captured Run final report as the primary source

**Rationale**: `task_execution_service::capture_final_report` already stores the same-turn model-authored final account after completed items settle. It is the closest available account of requested work, performed changes, checks, artifacts, and unresolved items. Completed Run items are factual evidence, but using the raw stream as the primary narrative would recreate the current raw-heavy problem.

**Alternatives considered**:

- Summarize all Run events: rejected because control chatter and tool detail dominate meaning and can imply chronology or causality.
- Make a second execution-time summarization call: rejected because the final report already exists and feature 017 needs durable, versioned background processing.
- Trust the final report alone: rejected because material checks, paths, failures, or contradictions may exist only in completed evidence.

## Decision: Treat Distillation as evidence reconciliation, not summarization

**Rationale**: The useful record must preserve supported reasons, conditions, concrete values, failed approaches, verification state, and unresolved outcomes. A shorter narrative can lose those distinctions or add connective reasoning that no source establishes. Typed claims with exact sources make omission and uncertainty explicit.

**Alternatives considered**:

- Free-form summary only: rejected because it cannot reliably support claim-level sources or incremental invalidation.
- Copy selected paragraphs verbatim: rejected because it does not reconcile contradictions or decision state.
- Generate new recommendations: rejected because the feature records work; it does not create new work or decisions.

## Decision: Use one structured result for Work Log and active Task journey

**Rationale**: Both surfaces need the same supported claims and epistemic distinctions. Work Log presents a readable Run-centered projection; Task journey organizes the same claims into durable semantic nodes across sources. Separate model outputs would drift and double cost.

**Alternatives considered**:

- Independent Work Log and journey prompts: rejected due to inconsistent claims and duplicated source processing.
- Extend legacy Solution lineage: rejected because current governance makes Task the durable unit and the active code path is `task_journey.rs`.
- Replace the current Task event trace: rejected because original recorded events remain valuable and must stay inspectable.

## Decision: Stable semantic identities come from supported meaning, not chronology

**Rationale**: Source revisions and timestamps change when evidence is corrected, while a decision topic or completion snapshot needs a durable identity. Candidate stable IDs use Task ownership, node kind, normalized topic key, and origin identity; later enrichments retain the node. A new decision or new reason creates a new node and link.

**Alternatives considered**:

- Use array position or timestamp: rejected because unrelated insertions renumber nodes.
- Use output text hash: rejected because copy or locale changes would replace identity.
- Reuse one node per topic forever: rejected because it would overwrite decision history.

## Decision: Apply add/enrich/merge/omit/supersede explicitly

**Rationale**: Incremental generation needs a reviewable disposition for every candidate. `enrich` retains identity while adding supported details; `merge` records aliases and redirects; `supersede` creates a new node and relationship. This prevents the model from silently rewriting history.

**Alternatives considered**:

- Replace the whole graph: rejected for cost, unstable identities, and broad blast radius.
- Let the model choose arbitrary patch operations: rejected because unsupported mutation would be difficult to validate.
- Treat every statement as a node: rejected because it recreates a raw activity feed.

## Decision: Resolve topic status from explicit evidence

**Rationale**: The last message can be a question, retry, or unaccepted suggestion. Adopted, superseded, withdrawn, and unresolved are topic states derived only from explicit user decision or recorded workflow evidence. A missing reason remains unknown.

**Alternatives considered**:

- Last-write-wins: rejected because chronology is not authority.
- AI confidence score: rejected because the product must explain evidence, not score people or silently choose decisions.
- Drop old choices: rejected because decision change and rationale are part of resumable context.

## Decision: Depend on feature 015 for shared orchestration

**Rationale**: Prompt identity, exact references, versioned result envelopes, job leases/retries, and stale completion are cross-feature infrastructure. Feature 017 should own semantics and projections without creating a second queue or prompt table.

**Alternatives considered**:

- Extend `ai_jobs_v2` directly before foundation integration: rejected because it would preempt shared schema ownership and migration allocation.
- Store a best-effort JSON result on Work Log rows: rejected because it lacks versioning, targeted invalidation, and durable retry.
- Block Task reads while generating: rejected because AI assistance cannot gate ordinary Task work.

## Decision: Use one Run Distillation operation

**Rationale**: `run_report_distillation` matches the primary source and coalesces by exact final-report hash. Its validated structured result supplies the readable linked Work Log and then becomes input to `task_journey_increment`. Enqueuing `work_log_distillation` for the same Run would create duplicate model work and possible claim drift.

**Alternatives considered**:

- Enqueue both Run-report and Work-Log Distillation: rejected because both would interpret the same Run for one Work Log entry.
- Use only `work_log_distillation`: rejected because the operation does not express that the captured Run final report is authoritative for this scope.
- Redistill inside `task_journey_increment`: rejected because the journey must consume the same validated claims, not produce a competing account.

## Decision: Prepare detail at generation time

**Rationale**: A node click should be immediate, deterministic, and usable offline. Saved before/after/reason/evidence/result/status/links make review possible without an extra model call or a changing explanation.

**Alternatives considered**:

- Generate detail on click: rejected due to latency, nondeterminism, availability, cost, and weak revision binding.
- Show only source excerpts: rejected because users would still need to reconstruct the relationship manually.

## Decision: Preserve last-good projections and atomically publish batches

**Rationale**: A provider failure or stale job should not remove a trustworthy existing view. A candidate batch validates fully against exact inputs and expected projection revision before one atomic publication.

**Alternatives considered**:

- Clear the projection while refreshing: rejected because it increases cognitive load and loses resumption context.
- Publish each model item as it arrives: rejected because users could see an internally inconsistent decision state.

## Open integration facts

These are implementation gates rather than product ambiguities:

- Exact feature-015 Rust type and method names for prompt lookup, reference manifests, result envelopes, enqueue, leasing, and publication.
- Feature-specific migration number after foundation lands.
- Whether the shared foundation persists dependency indexes or feature 017 needs a feature-owned reverse-reference table.
