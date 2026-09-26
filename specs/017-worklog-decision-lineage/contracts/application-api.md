# Application Contract: Work Log Distillation and Task Journey

All operations validate Task ownership. Run-centered reads also validate Task → session → Run → Work Log ownership. Exact source references are opaque to clients except for navigation metadata.

## Task read projection

Existing Task reads keep all original Work Log and journey data. A Run-linked Work Log entry may add:

- `distillation`: current readable sections, claims, warnings, prompt/rules/schema metadata, projection revision, and freshness.
- `originalAvailable`: whether the final report and retained evidence can be opened.
- `sourceLinks`: bounded navigation descriptors for exact cited sources.

The current Task journey read may add:

- semantic nodes, relationships, topic states, completion snapshots, prepared node detail, current projection revision, and freshness;
- the original recorded event trace unchanged and separately identifiable.

Legacy Solution lineage endpoints and snapshots are not used to compute this projection.

## Read operations

- Read current Work Log Distillation by exact Task and Work Log entry.
- Read original final report and retained evidence through existing exact Run detail.
- Read current Task journey projection and recorded event trace.
- Read a saved projection revision for inspection/repair diagnostics.

Normal list/detail reads return the current projection only and never enqueue or invoke AI.

## Mutation operations

- Retry a failed/stale projection job without rerunning the originating Codex Run.
- Request explicit full repair when authorized by repair state or rules/schema change.
- Source edits/deletes schedule targeted jobs through the shared foundation; clients do not submit model patches.

Same operation ID and payload replay idempotently. A changed payload conflicts. No operation changes Task workflow state.

## Freshness states

- `current`: exact source set and projection match.
- `pending`: last-good view is shown while a newer candidate runs.
- `stale`: source changed; last-good view is shown with bounded warning.
- `retryable_failure`: last-good view remains; retry is available.
- `repair_required`: result/rules/schema is incompatible and targeted update is insufficient.
- `unavailable`: no valid result exists; factual fallback and original access remain.

## Source navigation

Each claim and relationship exposes a human-readable source label and an opaque target that opens the exact saved Run item, final report, Task revision field, decision, completion, Work Log entry, or refinement input. Navigation never substitutes the newest revision for the cited revision.

## Node detail interaction

Opening a node reads the prepared detail already present in the current projection. It performs no enqueue, network request to a model, or hidden regeneration. The detail includes applicable before/after/reason/evidence/result/current status/links and exact source actions.

## Errors and privacy

Errors distinguish ownership failure, unavailable original, stale expected projection, retry conflict, repair required, and safe provider/job failure. Raw provider requests, secrets, and unredacted stderr never cross this boundary. Distillation remains private local process.
