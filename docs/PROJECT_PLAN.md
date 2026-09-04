# LLM Wiki Project Plan

**Updated**: 2026-09-03
**Project status**: Native React/Tauri desktop migration complete. The FastAPI browser delivery was
retired to Git history at `caef236`. The items below are post-migration product enhancements.

## Product boundaries

- The Markdown vault is a structured Markdown corpus; no Obsidian application or plugin access.
- Models are accessed only through `OpenAICompatibleProvider`; the default configurable endpoint is
  `http://127.0.0.1:8317/v1`.
- Humans control state transitions, approvals, completion, archive, and knowledge integration.
- AI enriches only after a user request and never generates technical implementation directions in
  workflow handoffs.
- Structural search and workflow actions remain usable without a model, provider, or embeddings.

## Delivery roadmap

| Feature | Outcome | Current state | Next acceptance work |
|---|---|---|---|
| 001 Fast Vault Search | Fast, Obsidian-compatible local retrieval | Implemented in native and browser deliveries | Add corpus-wide semantic mode and ratified memory fixtures |
| 002 Conflict-Gated Workflow | Human-approved capture-to-handoff flow | Implemented and desktop-E2E protected | Add richer progressive conflict-review presentation |
| 003 Completion, Writeback, Archive | Reviewed, safe knowledge integration | Implemented with atomic cross-platform writes | Add three-way merge for non-overlapping external edits |
| 004 Direction Dashboard | Compass-aligned progress scoring | Implemented with 10/20/70 milestone ledger | Add period/drift views and dashboard benchmark fixtures |

## Phase 1 — Finish user-facing workflow surfaces

1. AI enrichment review
   - Use saved provider configuration to enrich an approved problem.
   - Keep the implemented AI-first structured drafting flow: each stage validates its own draft
     shape and requires an explicit human finalization.
   - Add normalized problem, categories, context citations, and importance proposals as editable
     drafts.
   - Keep all workflow state changes as explicit user actions.
2. Conflict review
   - Store and display individual findings: claim, severity, exact citation, explanation,
     confidence, and required resolution.
   - Invalidate a clear report when cited source hashes change.
3. Importance and Compass workflow
   - Add factor/evidence entry and goal allocation controls.
   - Show daily, weekly, monthly, yearly totals and a direction/drift report.

**Exit gate**: Every implemented workflow/AI API capability is accessible through the browser,
and no user can approve a Feature without a current clear report.

## Phase 2 — Complete knowledge-integration UX

1. Patch-review surface
   - Display base, current, and proposed document content.
   - Require explicit apply/reject decision; expose undo.
2. External edit handling
   - Provide import-or-regenerate choice for changed generated projections.
   - Implement deterministic three-way merge for non-overlapping Markdown sections.
3. Cross-platform file safety
   - Add `portalocker` and Windows-specific sharing-violation/move tests.

**Exit gate**: A user can safely review, merge, apply, undo, or reject a knowledge update without
overwriting external vault changes.

## Phase 3 — Performance and portability acceptance

1. Build a 1,000-note, 10 MB mixed Korean/English reference fixture.
2. Measure and gate startup, capture p95, board reads, warm FTS, semantic reranking,
   structural/incremental indexing, dashboard aggregates, and memory.
3. Fail CI on performance regression greater than 15% of the ratified budget.
4. Resolve the local Playwright browser-download certificate issue and run browser regression on
   macOS and Windows through the existing CI matrix.

**Exit gate**: All constitutional budgets have automated results, and macOS/Windows acceptance
tests pass.

## Operating plan

1. Read [.specify/memory/constitution.md](../.specify/memory/constitution.md).
2. Select one feature directory in `specs/` and complete its unchecked `tasks.md` items.
3. Update its requirements, performance, and conflict-safety checklists before implementation.
4. Run the React/runtime/Rust suites and packaged desktop E2E after each slice.
5. Update [CONTINUATION.md](CONTINUATION.md) when the working state or environment changes.

## Current acceptance evidence

- Native desktop: 14 React tests and 33 Rust unit/command tests pass with no skips.
- The retired browser contract's final 196 tests passed at `caef236`; equivalent application
  behaviors remain represented by native command and desktop E2E coverage.
- The packaged macOS application passes real launch, workflow, provider-double, bundled semantic
  search, filesystem, full process relaunch, and restoration E2E.
- Structural reference benchmark: 1,000 notes index under the 3-second budget.
- The packaged desktop opens no internal socket and starts no Python process. No browser delivery
  remains in the current source tree.
