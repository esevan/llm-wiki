# Repository agent instructions

## Mandatory worktree workflow

- Before starting any implementation that changes application code or tests, create or switch to a dedicated Git worktree on a task-specific branch.
- Do not implement features, bug fixes, or refactors in the primary repository checkout.
- Inspect the primary checkout first and preserve all existing tracked and untracked user changes when creating the worktree.
- Run verification and make implementation commits from the dedicated worktree.
- Create task worktrees under `<repository>/.worktrees/<task-name>` so edits remain inside the configured workspace.
- Do not create task worktrees under `/private/tmp` or another directory outside this repository.
- Use `<repository>/.tmp/` for non-worktree temporary files. Do not generate task files outside this workspace.

## Stable verification commands

- Run verification as separate commands so persistent approval rules can match each command.
- Use `npm test` for React, adapter, and native UI runtime tests.
- Use `cargo test --manifest-path src-tauri/Cargo.toml` for native unit and command tests.
- Use `npm run test:desktop` for the packaged application E2E suite after a release build.
- Use `git diff --check` for whitespace validation.
- Do not vary the inline JavaScript or its success message with task-specific wording.

## Completion documentation

- After every completed task, read and follow `docs/DOCUMENTATION_GUIDE.md`.
- Update the affected documentation in the same task whenever the behavior, workflow, interface, setup, or verification expectations changed.
- If no documentation change is needed, state that the guide was reviewed in the final handoff.

## Korean user-facing terminology

- In Korean translations and Korean user-facing copy, refer to the person using the product as `사용자`; do not translate "user" or "human" as `인간`.
- This terminology rule does not require changing English governance terms, source quotations, or technical identifiers.
