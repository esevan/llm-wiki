# Implementation Plan: Conflict Resolution Workflow

**Branch**: `feat/conflict-resolution-workflow` | **Date**: 2026-09-02 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/010-conflict-resolution-workflow/spec.md`

## Summary

Extend the existing Conflict Review result from evidence-backed `findings` into normalized structured conflicts, persist one human resolution per conflict in SQLite, and replace the raw report rendering in the existing item-detail dialog with scannable cards, radio actions, rationale validation, a live summary, expandable evidence, and a sticky Continue footer. The current queue, provider, source-hash invalidation, conflict report gate, visual language, and conflict-free path remain intact.

## Technical Context

**Language/Version**: Python 3.12; browser-native HTML/CSS/JavaScript

**Primary Dependencies**: FastAPI, Pydantic, SQLite standard library, existing OpenAI-compatible provider and queue runtime; no new dependencies

**Storage**: Existing local SQLite database, adding normalized conflict and resolution tables linked to `conflict_review_runs`

**Testing**: pytest API/service/browser-contract tests; browser script parse check; whitespace validation

**Target Platform**: Local web application on macOS and Windows

**Project Type**: Single Python web application with an embedded browser client

**Performance Goals**: Rendering and recalculating a 20-card review must stay synchronous and complete within 100 ms in a browser-contract benchmark; persistence uses one bounded transaction; provider review remains one retained passage per call with the existing maximum of 12 candidates

**Constraints**: Human authority over workflow state; stale source hashes remain blocking; no provider or vault dependency enters `WorkflowEngine`; no raw private review data is published to Knowledge; no new workflow stage; backward-compatible cached report loading

**Scale/Scope**: One local user, one active review dialog, 0–12 AI-generated conflicts under the current candidate cap, validation coverage through 20 synthetic cards

## Constitution Check

### Product Spirit assessment

- **I. You Talk. The Work Organizes Itself**: Pass. Structured model output organizes evidence automatically; the user supplies only the decision and rationale needed for an intentional exception.
- **II. Reduce Cognitive Load**: Pass. Cards place competing claims first, evidence is collapsed, and only two mutually exclusive actions are exposed.
- **III. Resume Where You Left Off**: Pass. Resolutions, rationale, timestamps, target, and evidence remain queryable after restart.
- **IV. Organize Around Problems, Not Tasks**: Pass. Resolution remains part of the existing Solution conflict gate and adds no subordinate workflow stage.
- **V. Private Process, Portable Knowledge**: Pass. Review records stay in local workflow storage and do not publish into the Vault.
- **VI. Understand the Work, Never Score the Worker**: Pass. Counts describe review completion, not personal performance.

### Engineering guardrails

- **Measured Performance**: Add a deterministic 20-card browser interaction benchmark/assertion and preserve the existing candidate/token caps.
- **Independent Adapters**: Provider calls remain in `handlers/conflict_review.py`; persistence remains in `WorkflowEngine`; no direct vault/provider access crosses those boundaries.
- **Human Authority over AI**: The model returns descriptions and recommendations only. The resolution endpoint accepts explicit user actions and is the only path that changes conflict state.
- **Evidence and Logical Consistency**: Conflict citations remain tied to retrieved passage IDs and source hashes. Resolution rejects stale/incomplete runs; Apply recommendation remains blocking; fully rationalized Accept conflict records an explicit address before setting the current gate clear.
- **Local and Cross-Platform**: Uses SQLite and browser-native controls only; no platform-specific paths or behavior.
- **Minimal Complexity**: Two small normalized tables and one endpoint are sufficient; no ORM, component framework, or new dependency.

**Pre-design gate**: PASS. No constitution violation or unresolved clarification.

## Phase 0: Research Decisions

See [research.md](research.md). The selected design normalizes conflicts at the handler boundary, stores immutable conflict rows per run and one resolution row per conflict, and maps aggregate actions onto the existing conflict report/address model without changing approval semantics.

## Phase 1: Design

- Data entities and state transitions: [data-model.md](data-model.md)
- HTTP and structured result contract: [contracts/conflict-review-api.md](contracts/conflict-review-api.md)
- End-to-end validation: [quickstart.md](quickstart.md)

### Conflict invalidation

`conflict_source_hash` continues to bind the queued review to the current localized Solution, Problem, and Vault manifest. Resolution persistence re-computes that hash and compares it with the job/run source context represented by the run query. Any content or manifest change requires a new review. Cached legacy reports are normalized for display but cannot invent missing conflict decisions.

### Performance and token budgets

- Candidate retrieval remains capped at 12 and the context-pack constitution limits remain unchanged.
- Per-evidence provider output adds concise structured fields but no additional provider round trip.
- Conflict-card summary updates scan only the active review's cards; the acceptance benchmark is 20 cards under 100 ms for render/state calculation.
- SQLite writes for one review are performed in one transaction with at most one conflict row and one resolution row per finding.

### Post-design constitution re-check

PASS. The API makes the human decision explicit, the model schema does not contain a resolution action, the storage is local and normalized, stale review rejection remains mandatory, and the UI reduces visible report complexity without adding a workflow stage or dependency.

## Project Structure

### Documentation (this feature)

```text
specs/010-conflict-resolution-workflow/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── conflict-review-api.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
llm_wiki/
├── controllers/
│   ├── application.py
│   └── schemas.py
├── services/
│   ├── handlers/
│   │   └── conflict_review.py
│   └── workflow.py
└── static/
    ├── index.html
    └── i18n/
        ├── en.json
        └── ko.json

tests/
├── test_ai_jobs.py
├── test_api.py
├── test_workflow.py
└── test_browser_menu.py

docs/features/
├── conflict-resolution-workflow.md
└── conflict-resolution-workflow.ko.md
```

**Structure Decision**: Extend the existing single-project layers at their current conflict-review touch points. Keep model interpretation in the handler, HTTP validation in controllers, durable human state in `WorkflowEngine`, and the lightweight browser UI in the embedded page.

## Complexity Tracking

No constitution violations require exceptions.
