# Research: Workflow Foundation

## Decision 1: Extend the existing asynchronous queue

- **Decision**: Add prompt identity, source revision, execution outcome, and application disposition to `ai_jobs_v2`; keep its identity, cancellation registry, retry, idempotency key, source hash, and result storage.
- **Rationale**: The queue already implements durable status, coalescing, cancellation, retry, and guarded finalization. A second scheduler would violate minimal complexity and fragment recovery.
- **Alternatives rejected**: A new operation table or new executor would duplicate ownership and require UI/runtime compatibility logic.

## Decision 2: Attach metadata to canonical versions

- **Decision**: Store generation/restoration provenance and document references in auxiliary tables keyed by owner type, owner ID, and exact owner version. Keep all content bytes in their existing canonical revision tables.
- **Rationale**: Tasks, refinements, Knowledge drafts, and work tracking already have distinct validated revision models. A generic content table would create competing heads and orphaned decisions.
- **Alternatives rejected**: Moving existing draft content into a generic table would expand migration risk and downstream scope.

## Decision 3: Use one registry with data-driven contracts

- **Decision**: Define stable `PromptId` values and `PromptDefinition` records with version, operation policy, required input fields, required output fields, builder, and validator. Callers provide structured `instructions` and `context`; the registry assembles the provider message and validates the structured result.
- **Rationale**: This centralizes identity, versioning, validation, and policy while allowing current feature modules to retain domain-specific snapshot assembly.
- **Alternatives rejected**: Merely adding constants would not validate contracts; moving all domain snapshot logic into one registry would make it a monolith.

## Decision 4: Separate four kinds of state

- **Decision**: Workflow state remains in canonical Task/Problem/publication models; execution lifecycle remains queue `status`; immutable content state remains canonical revision tables; presentation state is computed. Queue rows additionally expose `execution_outcome` and `application_disposition`.
- **Rationale**: A successful provider response can still be stale, require review, or be rejected. Conflating these outcomes causes false success signals and accidental state changes.

## Decision 5: Allocate stable downstream operation names

- **Decision**: Reserve `capture_distillation`, `work_log_distillation`, `run_report_distillation`, `task_journey_increment`, `publication_index`, `speculative_search`, and `user_generation`. The first five are durable important operations with restart recovery, bounded automatic transient retries, and explicit retry after terminal failure. Speculative search is ephemeral latest-wins with no retry. User generation is durable, visible, and explicitly retryable after terminal failure.
- **Rationale**: Stable names let downstream specs integrate independently with the same policy vocabulary.
- **Compatibility**: Existing `lineage_inference` remains supported and maps to the durable incremental-journey policy until its consumer migrates.

## Decision 6: Bind freshness to exact source revisions

- **Decision**: Store an opaque `source_revision` string and require callers to compare it with the canonical current revision before application. Shared helpers validate expected versus actual heads and commit application disposition with the result transaction.
- **Rationale**: Opaque strings can represent integer revisions, hashes, or a deterministic bundle without weakening exactness.
- **Alternatives rejected**: Timestamp freshness is ambiguous; source hashes alone cannot distinguish intentional identical revisions.
