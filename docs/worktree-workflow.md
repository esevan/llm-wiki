# Fast task worktrees

[한국어](worktree-workflow.ko.md)

Repository tasks remain isolated in dedicated Git worktrees, but immutable dependency and build
artifacts are reused from the primary checkout. This avoids reinstalling Node packages,
recompiling the full Rust dependency graph, and downloading the bundled embedding model for every
task.

On macOS or Linux, run from any checkout belonging to this repository:

```text
scripts/create_task_worktree.sh <task-name> <branch-name> [start-point]
```

On Windows, use:

```powershell
.\scripts\New-TaskWorktree.ps1 -TaskName <task-name> -BranchName <branch-name> [-StartPoint main]
```

Both commands create `.worktrees/<task-name>` and reuse:

- the primary checkout's `node_modules`;
- `src-tauri/target`, including compiled Rust dependencies;
- checksum-verified embedding-model assets;
- a task-local copy of the small current `dist/` frontend build so Rust tests can start immediately.

The scripts verify every embedding asset against the committed size and SHA-256 manifest, then stop
before creating a worktree if any primary cache has not been prepared. Prime that
checkout once with `npm ci`, `npm run build:desktop`, and a Cargo build or test. npm's download cache
and Cargo's registry cache are already user-level shared caches.

Task branches start with the same lockfiles and model manifest as their selected start point. If a
task intentionally changes `package-lock.json`, replace the worktree's `node_modules` link with a
task-local installation before running `npm ci`. If it changes the embedding manifest, unlink the
shared model binaries before `npm run prepare:embedding`. Cargo safely serializes access to its
shared target directory and invalidates changed crates through its normal fingerprints.

During implementation, use focused and incremental checks. Run `npm run tauri:build` followed by
`npm run test:desktop` once after the final source and documentation state is ready.
