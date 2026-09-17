# Repository agent instructions

## Mandatory worktree workflow

- Before starting any implementation that changes application code or tests, create or switch to a dedicated Git worktree on a task-specific branch.
- Do not implement features, bug fixes, or refactors in the primary repository checkout.
- Inspect the primary checkout first and preserve all existing tracked and untracked user changes when creating the worktree.
- Run verification and make implementation commits from the dedicated worktree.
- Create task worktrees under `<repository>/.worktrees/<task-name>` so edits remain inside the configured workspace.
- Create every new task worktree with `scripts/create_task_worktree.sh <task-name> <branch-name>`
  on macOS/Linux or `scripts/New-TaskWorktree.ps1` on Windows. Do not call `git worktree add`
  directly when these scripts are available.
- The creation scripts must link the primary checkout's `node_modules`, Cargo `target`, and verified
  embedding-model assets into the new worktree and copy its current `dist/`. Treat shared caches as
  build artifacts, not task-owned source.
- Do not run `npm ci` in a worktree linked to shared `node_modules`. If `package-lock.json` changes,
  replace that link with task-local dependencies before installing. If the embedding manifest
  changes, unlink the shared model assets before preparing the new model.
- Reuse incremental checks while implementing. Decide whether E2E is necessary by assessing the
  change's side-effect and regression risks, affected user flows, and coverage from focused tests.
- Run E2E only when meaningful risks require end-to-end validation, such as changes spanning UI and
  native commands, persistence or migrations, lifecycle or synchronization, or packaging and startup
  behavior that focused tests cannot adequately verify. Documentation-only, copy-only, and isolated
  low-risk changes do not require E2E when appropriate focused checks cover their impact.
- When packaged desktop E2E is necessary, run the release Tauri build and E2E once after the final
  code and documentation changes, not after intermediate edits. Do not run a release build solely
  for E2E that the risk assessment deemed unnecessary.
- State the E2E run or skip decision and its risk-based rationale in the final handoff, including any
  material residual risks or checks that could not be performed.
- Do not create task worktrees under `/private/tmp` or another directory outside this repository.
- Use `<repository>/.tmp/` for non-worktree temporary files. Do not generate task files outside this workspace.

## Stable verification commands

- Run verification as separate commands so persistent approval rules can match each command.
- Use `npm test` for React, adapter, and native UI runtime tests.
- Use `cargo test --manifest-path src-tauri/Cargo.toml` for native unit and command tests.
- When the risk assessment requires packaged application E2E, use `npm run test:desktop` after a
  release build.
- Use `git diff --check` for whitespace validation.
- Do not vary the inline JavaScript or its success message with task-specific wording.

## Completion documentation

- After every completed task, read and follow `docs/DOCUMENTATION_GUIDE.md`.
- Update the affected documentation in the same task whenever the behavior, workflow, interface, setup, or verification expectations changed.
- If no documentation change is needed, state that the guide was reviewed in the final handoff.

## Korean user-facing terminology

- In Korean translations and Korean user-facing copy, refer to the person using the product as `사용자`; do not translate "user" or "human" as `인간`.
- This terminology rule does not require changing English governance terms, source quotations, or technical identifiers.

## UI design quality

- When creating a screen or component, or making substantive layout or visual changes, read and apply
  [the project UI design skill](.agents/skills/ui-design/SKILL.md), even when the request does not name it.
- Follow its design direction, shared-token/component guidance, and rendered visual review before
  declaring the UI complete. Report any visual checks that could not be performed.
- For copy-only changes or behavior fixes without a visual redesign, keep the scope focused; this
  skill does not require an unrelated restyle.
