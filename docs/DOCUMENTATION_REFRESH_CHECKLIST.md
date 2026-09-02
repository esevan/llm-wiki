# Documentation Refresh Checklist

**Started**: 2026-08-21
**Scope**: Bring Spec Kit artifacts and user-facing feature documentation in line with the current, implemented LLM Wiki workbench.

## Inventory and source of truth

- [x] Preserve the primary checkout's existing user changes in a dedicated worktree.
- [x] Inventory the current UI, API routes, tests, README, and all five Spec Kit features.
- [x] Record the documentation update scope and acceptance criteria in this checklist.

## Spec Kit refresh

- [x] Update `001-fast-vault-search` with semantic search, watcher/SSE, pagination, and current verification status.
- [x] Expand `002-conflict-gated-workflow` from a boundary statement into testable user stories, requirements, plan, and completed work record.
- [x] Expand `003-completion-writeback-archive` with completion review, playbook, patch, projection, and archive behavior.
- [x] Expand `004-direction-dashboard` with Compass evidence, score ledger, and current dashboard behavior.
- [x] Mark `005-refinement-preview-status` as implemented and align its plan/tasks with the delivered state.

## Feature introduction docs and visual evidence

- [x] Create a browsable feature-document index with clear links to all five guides.
- [x] Write one standalone guide per Spec Kit feature: purpose, key user flow, human-control boundary, and related spec.
- [x] Capture one current UI screenshot per guide: Search, Workbench, Completion/Archive, Compass, and Refinement Preview.
- [x] Add meaningful image alt text and captions that identify the represented feature and UI state.
- [x] Link the feature guide index from the project README.

## Final quality gate

- [x] Check all Markdown links and image paths resolve inside the repository.
- [x] Reconcile every guide claim against the current implementation and API/UI evidence.
- [x] Run the project test suite, browser-script syntax check, and whitespace check.
- [x] Mark this checklist complete and commit the dedicated documentation worktree.
