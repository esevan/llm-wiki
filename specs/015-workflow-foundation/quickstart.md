# Quickstart: Workflow Foundation Integration

1. Choose a stable `OperationKind` and `PromptId`; use the reserved downstream operation names where applicable.
2. Build a deterministic domain snapshot and exact `sourceRevision` before enqueueing.
3. Call `build_prompt` for provider content and store the returned prompt ID/version on the queue row.
4. Validate the parsed provider JSON with `validate_prompt_output`.
5. Re-read the canonical source revision inside the final transaction.
6. Commit the result and `application_disposition` atomically. A mismatch records `superseded` and leaves the canonical head unchanged.
7. When a canonical draft version is appended or restored, record its provenance and source references in the same transaction.

Do not persist interface presentation state, create a new decision store, or publish generated content from the foundation.
