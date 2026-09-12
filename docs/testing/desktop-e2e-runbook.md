# Desktop E2E runbook

Use the packaged desktop runner for deterministic interaction evidence. From the repository root:

```sh
npm run test:desktop -- --help
npm run test:desktop -- --list
npm run test:desktop -- --full
npm run test:desktop -- --scenario task-refinement --scenario task-refinement-provider-recovery
```

`--full` is the release acceptance command. It runs all 32 registered scenarios and fails unless the union reports complete 169-control coverage; a focused run is explicitly partial and never satisfies that gate. An unknown scenario fails with a `--list` hint.

The runner uses the release executable at `src-tauri/target/release` by default. Build the release bundle before a full run. To deliberately reuse an existing signed build, set `LLM_WIKI_E2E_EXECUTABLE=/absolute/path/to/executable`; the runner reports the missing path and build/reuse choice clearly. The deterministic local provider is started automatically. No external provider credentials are needed.

Each scenario receives a fresh temporary Vault, SQLite database, workbench home, IPC endpoint, and local fake provider. Results are written to `.tmp/desktop-e2e-artifacts-*/results.json` (or `--artifact-dir PATH`), with coverage in `interactive-coverage.json`; failed cases retain logs, isolated state, and `failure.txt`. Add `--keep-state` when inspecting the temporary state after a run.

When adding a scenario, register its name in `DESKTOP_E2E_SCENARIO_NAMES`, implement the scenario in the desktop test flow, and report its rendered/exercised/asserted control IDs. Keep focused commands for diagnosis; only the full command is coverage acceptance. Common failures are a missing release executable, stale artifact identity, provider startup timeout, or an isolated state/IPC collision; inspect the retained case directory and rerun the smallest affected scenario.

The runner and artifact layout are supported on macOS, Linux, and Windows. The signed bundle path and code-signing identity checks are macOS-specific; Windows and Linux provide executable and interaction evidence without macOS signing metadata.

After code changes, build and verify the new release in this order. Building requires platform tooling and macOS signing configuration ([packaging guide](../macos-packaging.md)).

```sh
npm run tauri:build
npm run test:desktop -- --full
```

To retain a focused run in a chosen workspace directory, use a fresh directory name for each run:

```sh
npm run test:desktop -- --scenario task-refinement --artifact-dir .tmp/refinement-check-01 --keep-state
```

Register new scenarios in both the [runner registry](../../scripts/desktop_e2e_helpers.mjs) and the [application scenario dispatcher](../../frontend/src/test/desktopScenario.ts). Follow the [interaction coverage guide](interactive-coverage.md) when expanding assertions. The 32-scenario and 169-control counts describe the current acceptance baseline; update them as features are added.

The fake provider does not validate real AI response quality or latency. Packaged execution evidence for this task was collected on macOS; Windows/Linux device verification remains separate.
