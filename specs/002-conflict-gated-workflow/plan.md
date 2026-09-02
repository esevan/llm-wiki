# Implementation Plan: Conflict-Gated Workflow

**Status**: Implemented; this is the as-built design summary.

## Current design

Workflow state, approvals, AI-run history, categories, progress records, and soft-deletion flags are
SQLite-owned. The browser shell calls typed local API routes; only `OpenAICompatibleProvider` talks
to a configured model. API keys are held by the OS keyring through `ProviderSettings`.

Capture is promoted to a historically linked Problem, then an approved Problem owns Solutions and a
Solution owns its Work Log and validation checklist. Explore and Draft-next stream conversation separately from state changes.
Validated AI draft output enters an editable review form; manual forms provide the non-AI fallback.

## Control and safety boundaries

- Approval is a human operation. A Solution requires an approved Problem, cited conflict evidence,
  and a `clear` evaluation; `unknown` and `conflicted` block the transition.
- AI can categorize/rank the Workbench, propose drafts, and write conflict findings, but cannot
  create, approve, or advance a record on its own.
- Deletion is a reversible local visibility change. It neither removes history nor writes the vault.
- The Flow view derives Problem → Solution relationships from local record references.

## Verification surface

`tests/test_workbench_flow.py`, `tests/test_api.py`, `tests/test_transitions.py`, and
`tests/test_browser_menu.py` cover local workflow transitions, approval gates, provider routing,
draft/refinement boundaries, and browser-shell behavior.
