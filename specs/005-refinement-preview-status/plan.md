# Implementation Plan: Refinement Preview Status

**Branch**: `codex/refinement-preview-status` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-refinement-preview-status/spec.md`

## Summary

Problem and Solution refinement will fetch a deterministic summary of already stored item context in parallel with Preview generation. When context exists, the existing review dialog opens immediately in a loading state with “Refine 중...” and up to three summary entries; success converts the same dialog into the existing editable review, while failure restores the existing refinement dialog and exposes one accessible corner warning. Capture and no-context refinement retain their current behavior.

## Technical Context

**Language/Version**: Python 3.12; browser-native JavaScript, HTML, and CSS

**Primary Dependencies**: FastAPI, SQLite from the Python standard library, existing browser dialog APIs; no new dependency

**Storage**: Existing SQLite workflow-item and `ai_runs` records; no migration or new persisted state

**Testing**: pytest, FastAPI TestClient, Playwright with Chromium, JavaScript syntax parsing through Node

**Target Platform**: Local web application on macOS and Windows with a modern browser

**Project Type**: Single Python web application with a server-rendered static browser client

**Performance Goals**: Make stored context available to the browser within 100 ms under the local test workload and display the loading summary within one second of starting Preview generation

**Constraints**: Three entries and 500 visible characters maximum; no additional AI request; no Capture Preview; no automatic application; no changes to existing adapter boundaries; provider-backed refinement remains lazy and outside non-AI hot paths

**Scale/Scope**: One current Problem or Solution per Preview attempt, reading at most the current item plus its three most relevant stored AI-run records

## Constitution Check

*GATE: Passed before Phase 0 and re-checked after Phase 1.*

- **I. Measured Performance First — PASS**: The new context read is bounded and contains no AI call. A focused timing assertion covers the local summary path; this feature does not alter capture, dashboard, indexing, or search budgets.
- **II. Independent Adapters — PASS**: No Markdown vault access is added. Model generation remains behind `OpenAICompatibleProvider`; context summary construction stays inside the workflow service.
- **III. Human-Managed, AI-Enriched — PASS**: The Preview remains a proposal and requires the existing explicit human approval action.
- **IV. Evidence and Logical Consistency — PASS**: Only context stored for the same item is displayed; request-local identity prevents cross-item leakage. No knowledge-file write occurs.
- **V. Local and Cross-Platform — PASS**: The implementation uses existing SQLite and browser primitives with no platform-specific path or process behavior.
- **VI. Minimal Complexity — PASS**: Existing dialogs, records, endpoint conventions, and test tools are reused; no dependency, schema, background job, or streaming redesign is introduced.

**Post-design re-check**: PASS. The API contract is read-only and bounded, the UI state is ephemeral, the token budget is explicit, and all error paths leave human authority intact.

## Project Structure

### Documentation (this feature)

```text
specs/005-refinement-preview-status/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── api.md
└── tasks.md
```

### Source Code (repository root)

```text
llm_wiki/
├── api/
│   └── app.py                 # Read-only context endpoint and existing refinement endpoint
├── services/
│   └── workflow.py            # Deterministic, bounded same-item context summary
└── static/
    └── index.html             # Loading Preview, warning, attempt-local UI state

tests/
├── test_workflow.py           # Summary selection, isolation, bounds, performance
├── test_workbench_flow.py     # Endpoint and Capture-boundary integration
└── test_browser_menu.py       # Loading, failure, accessibility, retry, and regression UI states
```

**Structure Decision**: Extend the existing single-project service, API, and static-client boundaries. The feature requires no new package or component directory.

## Phase 0: Research

- Confirmed that current item detail and item-scoped `ai_runs` already contain the required context.
- Chose a deterministic service summary over a second AI operation.
- Chose a small read endpoint called in parallel with existing refinement generation over expanding every board response or redesigning refinement as a streaming job.
- Chose request-local browser state and reuse of the existing refinement and review dialogs.

See [research.md](research.md).

## Phase 1: Design

- [data-model.md](data-model.md) defines prior context, bounded summary entries, and the Preview attempt state machine.
- [contracts/api.md](contracts/api.md) defines the read-only context response and error boundary.
- [quickstart.md](quickstart.md) defines the required UI and regression validation matrix.

## Performance and Dependency Budget

- Context selection reads one workflow item and no more than three relevant recent records.
- Returned summary content is capped at 500 visible characters plus three short labels.
- Context construction performs zero model/provider calls and zero vault reads.
- No package or optional dependency is added.
- The UI starts the context and refinement requests concurrently; a slow context read must not create a second AI operation.
- A focused service test measures the deterministic local summary path against the 100 ms budget; the complete browser state matrix is covered by Playwright.

## Adapter, Platform, and Conflict Boundaries

- **Adapter boundary**: No vault access is introduced, and the summary path does not import or invoke provider code. Existing refinement generation continues through `OpenAICompatibleProvider` only.
- **Cross-platform behavior**: Only existing SQLite queries, FastAPI routing, and standards-based browser APIs are used; no OS-specific path, lock, or process behavior changes.
- **Conflict invalidation**: Not affected. A context summary is orientation inside an unapproved Preview, not cited conflict evidence, and this feature does not change conflict state, approval state, or their invalidation rules.
- **Dependency cost**: Zero new runtime or development dependencies.

## Risk Controls

- Use an attempt identity containing item type, item ID, and unique token so late success or failure cannot affect a newer attempt.
- Treat explicit user cancellation differently from generation failure so cancellation does not produce an error warning.
- Escape all summary labels and text before inserting them into the Preview.
- Clear warnings on retry, modal close, or item change and keep the warning absent for Capture.
- Preserve existing no-context and Capture behavior through explicit browser regression tests.

## Complexity Tracking

No constitution violation or justified complexity exception is required.
