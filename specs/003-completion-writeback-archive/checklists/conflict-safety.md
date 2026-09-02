# Conflict-Safety Checklist: Completion, Writeback, and Archive

- [x] Reviewed patch applies only at the reviewed source hash.
- [x] External modifications block overwrite.
- [x] Reverse content is retained for undo.
- [ ] Add three-way merge for non-overlapping edits.
