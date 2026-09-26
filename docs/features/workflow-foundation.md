# Workflow foundation

**English** | [한국어](workflow-foundation.ko.md)

LLM Wiki records which versioned prompt produced each structured AI result and validates that result before it can be stored or applied. Prompt contracts live in one registry with feature-owned definition modules, so Capture, Task, journey, retrieval, and Knowledge work can evolve independently while sharing stable identities and validation rules.

AI execution and application are separate. A Queue job can finish successfully while its proposal still needs review, has been superseded by newer source material, or is not meant to change content. The Queue exposes the prompt identity and version, exact source revision, execution outcome, and application disposition alongside its existing status and result.

Important background distillation and index operations are durable. Equivalent active work coalesces, interrupted work recovers after restart, transient failures receive a bounded retry, and a terminal failure remains available for explicit retry. Optional speculative search stays ephemeral: a newer request cancels the older request and no retry record is created. User-requested generation remains visible and retryable.

Every generated version can attach provenance and exact document references without replacing the existing canonical Task, Refinement, or Knowledge revision tables. A reference contains a stable document ID, exact version, and optional section, excerpt, and claim ID. Restoring old content creates a new canonical version with restoration ancestry. Exact-head checks prevent a stale result from overwriting newer work.

These contracts do not give AI authority over Task state, Problem resolution, decisions, or Knowledge publication. The owning feature performs its domain write and source-version check in one transaction, then records whether the result was applied or superseded.

See the [feature specification](../../specs/015-workflow-foundation/spec.md) and [consumer API contract](../../specs/015-workflow-foundation/contracts/workflow-foundation-api.md).
