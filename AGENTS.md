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
- Use `uv run pytest -q` for the test suite.
- Use exactly `node -e "const fs=require('fs'); const s=fs.readFileSync('llm_wiki/static/index.html','utf8').match(/<script>([\\s\\S]*)<\\/script>/)[1]; new Function(s); console.log('browser script parses')"` for browser-script syntax validation.
- Use `git diff --check` for whitespace validation.
- Do not vary the inline JavaScript or its success message with task-specific wording.

## Completion documentation

- After every completed task, read and follow `docs/DOCUMENTATION_GUIDE.md`.
- Update the affected documentation in the same task whenever the behavior, workflow, interface, setup, or verification expectations changed.
- If no documentation change is needed, state that the guide was reviewed in the final handoff.
