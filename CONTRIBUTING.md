# Contributing

Leave a clear chain of evidence: domain model, user-visible behavior, persisted result, and checks that exercised them. Start with the smallest relevant paths and keep the change boundary explicit.

## Model the workflow first

- Map actual domain relationships, independent lifecycles, and source evidence before choosing direct Task creation or optional Problem context. Do not force a one-to-one chain the product does not have.
- Reuse an already recorded Capture, Task context, or proposed approach instead of rediscovering it or treating mechanical approval as quality evidence.
- Trace each UI action through its API or IPC contract, service boundary, and persisted state. Assign ownership before editing; keep unrelated restructuring separate.

## Verify behavior across boundaries

- E2E count is not coverage. Inventory interactive controls and verify what was rendered, exercised, and meaningfully asserted; record why unavailable or disabled paths are excluded.
- For multi-turn flows, check input sequence, provider history, and actual saved output. A deterministic fake provider proves application behavior and repeatability, not real AI quality or latency.
- For async work, cover success, duplicate clicks, delay, failure, retry, close and reopen, and process restart where in scope. Closing a UI does not imply server or job cancellation.

## Work incrementally and leave evidence

- After repeated failures, stop blind retries. Preserve the failure, narrow the cause with evidence and a hypothesis, then make the smallest useful change.
- Choose models by total delegation, context, execution, and rerun cost. Start with the cheapest fitting model and raise capability only at a demonstrated bottleneck.
- For application code or test changes, use a dedicated task worktree. Run relevant incremental checks, then run the release build and packaged full E2E once after final implementation and documentation changes. Do not repeat passing checks without new changes or failure evidence; documentation-only changes need documentation checks.
- Use the [desktop E2E runbook](docs/testing/desktop-e2e-runbook.md) and its repeatable `--list`, `--scenario`, `--full`, `--artifact-dir`, and `--keep-state` commands.
- Completion means claims match the actual artifact, code, and documentation. Report verification and distinguish fake-provider checks from real-provider quality claims.

Before handoff, run `git diff --check`, verify changed links, and review English and Korean counterparts together. See [docs/DOCUMENTATION_GUIDE.md](docs/DOCUMENTATION_GUIDE.md).
