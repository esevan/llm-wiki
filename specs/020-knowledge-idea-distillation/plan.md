# Implementation Plan: Final Knowledge and separate idea drafts

**Branch**: `020-knowledge-idea-distillation` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/020-knowledge-idea-distillation/spec.md`

## Summary

Replace the current activity-oriented Knowledge generator with an evidence-bound distillation pipeline that waits for the latest feature 017 Task journey, captures one exact generation snapshot, and produces a standalone final Knowledge draft plus zero or more separate idea drafts. Final Knowledge leads with topic-level outcomes and preserves conditions, limits, applicability, counterevidence, unresolved matters, and exact source/assumption references. Explorations remain outside the final body and become optional idea units only when useful, with explicit `unverified`, `deferred`, `out_of_scope`, or `rejected` disposition and reconsideration conditions.

Every meaningful generation, user edit, or restoration creates an immutable draft revision. The Task Review UI distinguishes the current private draft from the published revision, supports version selection and comparison, and restores an older version only by creating a new revision. Publication continues to require explicit user action and sends the exact reviewed body and metadata to feature 021; publication never regenerates the body or rewrites Task logs, decisions, Distillation, or journey history.

## Technical Context

**Language/Version**: Rust 2021 edition; TypeScript 5.x with React 19

**Primary Dependencies**: Tauri 2, rusqlite, serde/serde_json, existing application request router and feature 015 prompt/job/reference contracts, React Testing Library/Vitest

**Storage**: Existing local SQLite WAL database; centrally assigned migration 21 (`F21`), additive after E20; portable Markdown/YAML is written only by feature 021

**Testing**: `cargo test --manifest-path src-tauri/Cargo.toml`, `npm test`, focused component/integration fixtures, `git diff --check`; selected desktop E2E only if final risk review finds an uncovered UI/native publication boundary

**Target Platform**: Tauri desktop on macOS and Windows

**Project Type**: Local desktop application with Rust domain/application boundary and modular React UI

**Performance Goals**: Task detail local projection remains within the constitution's 100 ms p95 after data arrives; version selection and comparison perform no model call; generation timings are measured by stage without adding an unmeasured latency promise

**Constraints**: Exact immutable sources; latest journey must be reconciled before generation; stale results cannot save; no unsupported claims; provider failure keeps existing drafts usable; publication preserves the reviewed bytes; no external personal Vault connection; migration 21 is fixed; runtime acceptance still requires the corrected feature 017 result-v2 contract on the integration branch
**Scale/Scope**: One completed Task at a time, potentially many immutable Knowledge/idea revisions and exact source references; Task Review UI at supported wide and narrow desktop widths; English and Korean system copy

## Constitution Check

*GATE: Passed before Phase 0 research and re-checked after Phase 1 design.*

- **I. Conversation organizes the work**: generation derives structure from completed Task evidence. The user is not asked to classify the article or manually extract decisions.
- **II. Reduce cognitive load**: final outcomes lead; applicability and limits are readable in the article; version controls are compact and secondary; idea drafts are separated and collapsed when irrelevant.
- **III. Resume where you left off**: immutable draft history, exact source snapshots, comparison, and restore-as-new preserve review context without changing actual work history.
- **IV. Tasks carry work**: Knowledge generation reads a completed Task but does not complete/reopen it, resolve a Problem, or modify its logs or decisions.
- **V. Private process, portable Knowledge**: draft and idea units remain private until explicit publication. Intermediate exploration is prohibited from final Knowledge. Feature 021 receives portable Markdown/YAML publication material.
- **VI. Never score the worker**: quality findings concern claim support and omission, never a person or productivity.
- **A. Measured performance**: local read/compare paths receive focused profiling or benchmark coverage; provider stages record timings. No hot capture/search path imports generation logic.
- **B. Independent adapters**: feature 020 does not write Vault files. Feature 021 owns `MarkdownVaultAdapter`; model work continues through `OpenAICompatibleProvider` and the shared job/prompt system.
- **C. Human authority**: generation, editing, comparison, and restoration remain draft operations. Only an explicit user publication request crosses the feature 021 boundary. Restore creates a new private revision.
- **D. Evidence and logical consistency**: every factual or decision-bearing claim and every assumption is bound to the exact generation snapshot; unsupported additions and material omissions are separately validated; stale candidates are rejected.
- **E. Local and cross-platform**: SQLite migration follows the existing backup/recovery framework and UI avoids platform-specific behavior.
- **F. Minimal complexity**: Knowledge-only domain logic moves into one Rust module; existing router/jobs receive small integration patches; the React review section becomes one focused component. No new dependency or service is introduced.

Post-design re-check: the data model, contracts, and UI states preserve all gates. Migration 21 is assigned and implemented additively. KnowledgeDraft advances from the legacy markdown-only prompt v1 to typed nested prompt v2 because its result schema changed; there are no skipped semantic versions.

## Dependency Gates

Implementation must not begin until the integration branch contains:

1. feature 015's final prompt/reference/job/content-version contracts and centrally assigned migration sequence;
2. feature 017's implemented Distillation and incremental Task journey, including an exact readable completion snapshot and freshness/reconciliation behavior;
3. feature 018's accepted `llm_wiki` metadata contract or an agreed compatibility mapping for `out_of_scope` idea disposition; and
4. a feature 021 handoff agreement that publication consumes an exact reviewed revision without regeneration.

At implementation start, rebase or recreate the task worktree from the integrated dependency commit. Reconfirm actual paths and migration head. Do not copy partially written dependency files between worktrees.

## Architecture and Flow

1. A user requests a draft for a completed Task.
2. The Knowledge service asks feature 017 to reconcile the latest Task journey. `current` continues; `pending`, `stale`, `retryable_failure`, `repair_required`, or `unavailable` returns a bounded status and never generates from a guessed snapshot.
3. In one read transaction, the service captures the completed Task definition revision, primary Run final-report Distillation, exact incremental journey projection and completion snapshot, per-topic final decisions, retained conditions/counterevidence/unresolved items, and references marked actually used. Candidate/view-only references are excluded.
4. The service hashes the canonical manifest into `generation_snapshot_hash` and submits it through feature 015's registered operation. The prompt/result schema returns a final article, applicability, claim/source bindings, assumption bindings, quality findings, and optional idea units.
5. Deterministic validation rejects unsupported additions and material omissions independently. It rejects exploration in the final body, a decision structure invented for a non-decision Task, ungrounded effectiveness/verification claims, and incomplete idea status/link/reconsideration fields.
6. Before save, the service rebuilds the authoritative manifest in a transaction. Any changed Task revision, Distillation source, journey projection, completion snapshot, actual-use reference, or content hash marks the result stale and leaves draft history untouched.
7. A valid candidate creates immutable Knowledge and idea revisions. User edits and restore-as-new also append revisions with exact parent/derivation metadata. They never update an earlier body in place.
8. The Review UI reads versions without enqueueing work. A selected pair may be compared locally. Restore creates the next private revision and never changes Task history or the published pointer.
9. Explicit publication hands feature 021 the selected exact body, metadata, idea units selected for publication, and expected hashes. Feature 021 writes/organizes/indexes them; feature 020 records the returned publication reference only after success.

## UI Design Direction

The primary visual focus is a readable Knowledge article headed by a clear `Private draft` or `Published` state. A compact version selector sits beside the state, with `Compare` and `Restore as new draft` as secondary actions. Wide comparison uses two labelled columns; narrow layouts stack the same revisions while preserving headings and keyboard order. Source snapshot, assumptions, and quality findings use disclosure sections below the article rather than competing cards.

Useful explorations appear in a separate `Ideas to revisit` region after the final article. Each unit shows a text label for its disposition, reconsideration condition, and links to its Task, exact sources, and related final Knowledge. The region is absent when no valuable idea exists. Color supplements rather than carries status meaning. All controls have visible focus, descriptive names, loading/empty/error/success states, and accommodate longer Korean strings. Existing cream/white surfaces, ink text, restrained pink actions, mint/lemon status tokens, spacing, radii, and `KnowledgeMarkdown` are reused.

To reduce shared-file conflicts, create `KnowledgeReviewPanel.tsx` and its focused stylesheet/types, then replace only the existing Knowledge section inside `TaskDetail.tsx`. Add localized strings in one sequential integration patch after feature 019 UI work lands.

## Project Structure

### Documentation (this feature)

```text
specs/020-knowledge-idea-distillation/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── application-api.md
│   ├── generation-result.md
│   └── publication-metadata.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/src/native/
├── knowledge_distillation.rs     # feature-owned snapshot, generation, validation, version logic
├── task_assistance.rs            # minimal routing/backward-compatibility integration only
├── jobs.rs                       # shared queue finalization hook and stale acceptance guard
├── migrations.rs                 # assigned additive F21
└── mod.rs                        # route registration/response mapping

frontend/src/
├── components/KnowledgeMarkdown.tsx
├── features/workbench/
│   ├── KnowledgeReviewPanel.tsx
│   ├── KnowledgeReviewPanel.test.tsx
│   ├── knowledge-review.css
│   ├── TaskDetail.tsx
│   ├── TaskDetail.test.tsx
│   └── taskWorkbenchText.ts
├── services/taskClient.ts
└── types/taskWorkbench.ts

src-tauri/tests/
└── knowledge_distillation.rs

docs/features/
├── knowledge-distillation.md
└── knowledge-distillation.ko.md
```

**Structure Decision**: Keep the existing Tauri/React boundary. Extract Knowledge-only Rust logic from the large `task_assistance.rs` and render the existing Task Review section through a dedicated component. Feature 020 stores private reviewed revisions; feature 021 remains the sole portable file/publication owner.

## Resolved integration contracts

- Feature 015 registry owns KnowledgeDraft prompt v2 and its complete nested result.
- Feature 017 result schema v2 supplies canonical claims with `epistemicState` and exact citation `quote`; final F acceptance runs after integration commit `1bbb916`.
- Feature 019 actual reference usage keeps the canonical `documentVersion` field losslessly.
- Feature 021 advances the published pointer only after exact archive and index receipts; `index_pending` is not published.
- Migration 21 is checkpointed in `a534bcc` and preserves legacy draft bytes, hashes, and publication references.

## Sequencing and Shared Ownership

- Land the new Rust module and tests without editing C's journey module or E's reference logic.
- Apply schema/migration and minimal router/job wiring only after dependency integration, using the centrally assigned revision F21.
- Apply `TaskDetail.tsx`, shared types/client, and localization edits sequentially after feature 019 UI integration.
- Feature 020 may define handoff payloads but must not implement feature 021 paths, MOC updates, backlinks, moves, or index repair.
- Feature 020 may consume feature 018 metadata but must not implement ranking or retrieval indexing.

## Validation Strategy

- Contract and Rust fixture tests cover receipt-not-approval, report/evidence contradiction, missing final report, historical-to-final decision, deferred idea separation, stale generation, incremental enrichment, and restore-without-history-rewrite.
- Component tests cover current private vs published state, version selection, wide/stacked comparison semantics, restore-as-new confirmation/result, source/assumption disclosure, idea disposition labels, keyboard focus, and long EN/KO content.
- Migration tests preserve every existing `task_knowledge_drafts` body/hash/publication reference and prove idempotence, rollback, foreign keys, and new immutable revision semantics.
- Measure Task Review projection and version selection; record provider/generation stage durations without a new binding latency threshold.
- Run `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check` as separate commands.
- E2E decision: default skip. Focused component and Rust integration tests cover local version/state behavior. Run only the smallest `npm run test:desktop -- --scenario <name>` after the final release build if the final implementation introduces a material unverified Tauri publication/restore boundary; never run the full suite by default.

## Documentation Impact

At implementation completion, add paired English/Korean user guides and index links describing final Knowledge, idea separation, applicability, versions, restore-as-new, and publication authority. Update Product Spirit only if observable behavior requires wording beyond the existing private-process/portable-Knowledge principle. Keep this Spec Kit record current. Paired user-facing documentation now describes the wired native generation, freshness, immutable restore, idea separation, and publication boundary.

## Complexity Tracking

No constitution violations or new dependencies are proposed. The immutable revision tables add storage structure because in-place correction cannot satisfy exact comparison, restore-as-new, or reviewed-body publication; the extracted module and component reduce complexity in existing large files.


### Integration correction: manually completed Tasks

A Task without any session Run has no Run-primary Distillation producer. It
therefore captures a recorded-only snapshot of the canonical Task/completion,
ordered manual Work Logs (identity plus exact content hash), decisions and actual
reference usage. The same append-time source-manifest CAS applies. It does not
create synthetic semantic nodes, claims or completion snapshots. All newly bound
facts remain reported; final outcomes and verified/decided authority still require
current Distillation. Any Task with a Run remains strictly gated on that current
projection. KnowledgeDraft prompt version 3 makes this boundary explicit.

Validation includes actual registered provider HTTP calls using the packaged
fake, native result validation, immutable Knowledge append and archive proposal
preparation, plus manual log-edit invalidation and failed-Run bypass rejection.


### Integrated dependency receipt

Main includes foundation prompt/job/source contracts, schema-18 current
Distillation topic/completion authority, schema-19 decision/aspect indexing,
schema-20 exact preview references, schema-21 immutable Knowledge/idea versions,
and schema-22 archive journals. KnowledgeDraft is version 3; RunReportDistillation
and TaskJourneyIncrement use their current version-3 evidence contracts.
`knowledge_archive::snapshot` consumes the exact selected private revision and
source hash; its reviewed artifacts serialize canonical decisions/applicability,
selected-only ideas and actual use/counterevidence from the adoption-aware export.
