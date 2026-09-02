# Implementation Plan: Completion, Writeback, and Archive

**Status**: Implemented; this is the as-built design summary.

## Current design

Solution work logs, comments, validation checklists, completion records, reviews, patch proposals,
projections, mirrors, and Playbook metadata live in SQLite. `MarkdownVaultAdapter` is the only
writer and mover for vault content.

Completion review sends recorded criteria and evidence to the configured Solution model as an
advisory report. Human completion writes a summary-first completed-work Playbook and raw evidence
bundle; a provider outage preserves the factual local record rather than inventing a report.

## Integrity boundaries

- Patches are append, replace-section, or insert-after-heading operations with base/current/proposed
  review data, source-hash conflict checks, atomic application, and stored undo preimages.
- Projection, Playbook regeneration, and archive moves check mirrored hashes before replacing a
  locally modified document.
- Archive actions move content inside the configured vault and reindex it afterwards.

## Verification surface

`tests/test_api.py`, `tests/test_patches.py`, `tests/test_workflow.py`, and
`tests/test_completion_dashboard.py` cover completion evidence, generated documents, external-edit
blocking, patch apply/undo, and archive/index behavior.
