# Workflow Foundation API Contract

## Stable operation names and policies

| Name | Durability | Coalescing | Retry | Application default |
| --- | --- | --- | --- | --- |
| `capture_distillation` | durable | equivalent active request | bounded automatic + explicit after terminal failure | applied after same-transaction source check |
| `work_log_distillation` | durable | equivalent/batched source revision | bounded automatic + explicit after terminal failure | applied after same-transaction source check |
| `run_report_distillation` | durable | equivalent final-report hash | bounded automatic + explicit after terminal failure | applied after same-transaction source check |
| `task_journey_increment` | durable | equivalent source bundle | bounded automatic + explicit after terminal failure | applied after same-transaction source check |
| `publication_index` | durable | equivalent published revision | bounded automatic + explicit after terminal failure | not applicable |
| `speculative_search` | ephemeral | latest wins by scope | none | not applicable |
| `user_generation` | durable | equivalent active request | explicit | review needed |

Existing `lineage_inference` maps to `task_journey_increment` policy for compatibility.

## Shared Rust contracts

```rust
pub fn operation_policy(kind: OperationKind) -> OperationPolicy;
pub fn ensure_expected_head(expected: i64, actual: i64) -> Result<(), ContractError>;
pub fn validate_retrieval_intent(intent: &RetrievalIntent) -> Result<(), ContractError>;
pub fn validate_source_attributions(items: &[SourceAttribution]) -> Result<(), ContractError>;
```

## Persistence helpers

```rust
pub fn record_version_provenance(
    tx: &rusqlite::Transaction<'_>, provenance: &VersionProvenance
) -> Result<(), String>;

pub fn record_document_references(
    tx: &rusqlite::Transaction<'_>, owner: &VersionOwner,
    references: &[SourceAttribution]
) -> Result<(), String>;

pub fn record_job_application_disposition(
    tx: &rusqlite::Transaction<'_>, job_id: &str,
    expected_source_revision: &str, actual_source_revision: &str,
    disposition: ApplicationDisposition
) -> Result<ApplicationDisposition, String>;
```

If expected and actual source revisions differ, the helper records `superseded` and does not report `applied`. Consumers must perform their canonical domain mutation, exact source check, result write, and disposition update in the same caller-owned transaction; the helper does not mutate domain content.

## Queue read shape

`jobs.list`, `jobs.get`, and `jobs.result` add:

```json
{
  "prompt": {"id": "...", "version": 1},
  "source_revision": "opaque exact revision",
  "execution_outcome": "pending|succeeded|failed|cancelled",
  "application_disposition": "pending|review_needed|applied|superseded|rejected|not_applicable"
}
```

Existing fields and status values remain available.

## Consumer submission shape

Durable consumers submit the existing queue input with `taskKind`, `entityType`, `entityId`, exact `sourceRevision`, and their domain payload. The foundation chooses the registered prompt and policy. Important durable operations recover after restart and receive bounded automatic transient retries; `jobs.retry(jobId)` remains an explicit retry after terminal failure and retains the original prompt version and source revision. A missing historical prompt definition fails safely.
