# Data Model: Workflow Foundation

## Prompt Definition

| Field | Rule |
| --- | --- |
| id | Stable enum/string; never reused for a different semantic task |
| version | Positive integer; increments for instruction or contract changes |
| operation_kind | Maps invocation to durability/coalescing/retry policy |
| required_input | Top-level fields required before request construction |
| required_output | Top-level fields required before a structured result is accepted |
| instructions | Registry-owned provider instructions |

## Retrieval Intent

`needed: bool`, `reason: string`, `queries: string[]`, `aspects: string[]`, `filters: object`, and `requery: object|null`. When `needed=false`, queries and requery are empty. Requery contains the trigger/reason and replacement or supplemental queries.

## Source Attribution

`document_id`, `document_version`, optional `section`, optional `excerpt`, and optional `claim_id`. Version is opaque but non-empty. A persisted attribution must resolve through the owning feature's canonical document repository.

## AI Job additions

| Field | Rule |
| --- | --- |
| prompt_id / prompt_version | Exact registry definition; empty/zero only for legacy or non-provider jobs |
| source_revision | Exact opaque source version or version-bundle hash |
| execution_outcome | `pending`, `succeeded`, `failed`, `cancelled` |
| application_disposition | `pending`, `review_needed`, `applied`, `superseded`, `rejected`, `not_applicable` |

Queue `status` remains lifecycle (`queued`, `running`, `completed`, `failed`, `cancelled`, `stale`, and current compatibility values). A job may have `execution_outcome=succeeded` and `application_disposition=review_needed` or `superseded`.

## Version Provenance

One auxiliary row per canonical version: owner type/ID/version, source owner type/ID/version, prompt ID/version, operation ID, optional restored-from version, source references JSON, and creation time. The owner content remains in its canonical table.

## Document Reference

One stable reference row: reference ID, owner type/ID/version, referenced document ID/version, optional section/excerpt/claim ID, and creation time.

## Decision Status

Shared vocabulary: `proposed`, `accepted`, `rejected`, `deferred`, `superseded`, `withdrawn`. Feature-owned decision tables remain canonical and may map narrower legacy states into this vocabulary at their boundaries.

## Version transitions

```text
head N --append(expected=N)--> head N+1
head N --append(expected<M or >N)--> head conflict, unchanged
head N --restore(version K, expected=N)--> head N+1 with restored_from=K
```

Restoration copies content into a new canonical version. It never deletes a version or rewinds a head pointer.
