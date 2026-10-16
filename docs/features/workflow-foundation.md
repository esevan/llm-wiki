# Workflow foundation

**English** | [한국어](workflow-foundation.ko.md)

LLM Wiki records which versioned prompt produced each structured AI result and validates that result before it can be stored or applied. Prompt contracts live in one registry with feature-owned definition modules, so Capture, Task, journey, retrieval, and Knowledge work can evolve independently while sharing stable identities and validation rules.

AI execution and application are separate. A Queue job can finish successfully while its proposal still needs review, has been superseded by newer source material, or is not meant to change content. The Queue exposes the prompt identity and version, exact source revision, execution outcome, and application disposition alongside its existing status and result.

Important background distillation and index operations are durable. Equivalent active work coalesces, interrupted work recovers after restart, transient failures receive a bounded retry, and a terminal failure remains available for explicit retry. Optional speculative search stays ephemeral: a newer request cancels the older request and no retry record is created. User-requested generation remains visible and retryable.

Every generated version can attach provenance and exact document references without replacing the existing canonical Task, Refinement, or Knowledge revision tables. A reference contains a stable document ID, exact version, and optional section, excerpt, and claim ID. Restoring old content creates a new canonical version with restoration ancestry. Exact-head checks prevent a stale result from overwriting newer work.

These contracts do not give AI authority over Task state, Problem resolution, decisions, or Knowledge publication. The owning feature performs its domain write and source-version check in one transaction, then records whether the result was applied or superseded.

See the [feature specification](../../specs/015-workflow-foundation/spec.md) and [consumer API contract](../../specs/015-workflow-foundation/contracts/workflow-foundation-api.md).

## Provider JSON Schema contracts

All 28 registered structured prompt contracts have a common request-schema and response type-check path. Typed Knowledge, archive organization, and Distillation schemas derive from the same Rust structures used for parsing. Schema checks supplement the existing provenance, exact-quote, revision, and approval validators. They do not confer authority.

The wire schema is strict: objects are closed and all declared fields are required, with empty values or explicit nullable values where the contract permits them. Existing omitted-field recovery remains for compatible legacy responses; supplied wrong types are rejected. Work preview `scope` stays a string and arrays are never coerced into text.

Dynamic journey title keys are bound to recorded event IDs. Distillation target maps list known semantic IDs, with null for unused targets decoded back to the canonical sparse map. Sparse legacy Task patches likewise omit declared null placeholders before applying the existing revision checks. For genuinely arbitrary JSON fields in legacy completion review, publication metadata, search suggestions, and user proposals, objects use an explicit recursive `_entries` key/value wire representation and are restored to the original JSON types before domain validation. Missing entry key/value and duplicate keys are rejected. This keeps open-key objects representable under strict schemas without dropping their content.

Coverage includes common assistance, queued jobs (including images), separate Task Journey inference, and Problem enrichment. Free-form chat and Codex work execution are not structured result consumers. Codex App Server supports per-turn `outputSchema`, but this app's current Codex turns request natural-language work reports and do not parse them as JSON. No extra generation or retry is introduced.
