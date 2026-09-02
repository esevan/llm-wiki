# Implementation Plan: Lineage Knowledge Layer

**Branch**: `feature/lineage-knowledge-layer` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-lineage-knowledge-layer/spec.md`

## Summary

Replace the completed-Solution Lineage tab's current Problem-plus-chat-context presentation with an evidence-first Capture → Problem → Solution → Complete knowledge layer. At completion, build a deterministic snapshot from SQLite source records, optionally add strictly cited AI interpretations, and use the selected Lineage snapshot as the input boundary for the final report narrative before rendering the Markdown Playbook. Persist snapshot/revision history, source drill-down, Problem navigation, decision/conflict semantics, and append-only user corrections without changing the completed-work Detail modal.

## Technical Context

**Language/Version**: Python 3.12+; browser-native JavaScript, HTML, and CSS

**Primary Dependencies**: FastAPI, SQLite standard library, `OpenAICompatibleProvider`, `MarkdownVaultAdapter`; no new runtime dependency

**Storage**: Existing SQLite workflow database for source records and lineage snapshots/revisions; Obsidian-compatible Markdown for completed Knowledge and Raw Data

**Testing**: pytest, FastAPI TestClient, Playwright browser tests, exact browser-script parse command

**Target Platform**: Local macOS and Windows web application

**Project Type**: Single-package local web application with FastAPI backend and a static browser client

**Performance Goals**: Deterministic lineage assembly p95 under 100 ms for a completed Problem with up to 20 decisions/conflicts and 100 evidence records; cached lineage read p95 under 50 ms; no additional model call on Workbench, search, or Lineage read paths

**Constraints**: Completion must remain human-controlled; deterministic lineage must survive provider failure; AI context stays within the existing 6,000 retrieved-context-token ceiling and output is capped to a concise structured interpretation; vault writes remain atomic and adapter-owned; no source record may be overwritten by correction or regeneration; completed-work Detail modal is out of scope

**Scale/Scope**: One direct Capture ancestry, one Problem, one selected completed Solution, up to 20 material decision/conflict entries in the primary experience, with additional evidence retained in drill-down and Raw Data

## Constitution Check

*GATE: Passed before Phase 0 and re-checked after Phase 1.*

- **I. You Talk. The Work Organizes Itself — PASS**: lineage is assembled from existing conversation and workflow records; the user is not asked to reconstruct the history at completion.
- **II. Reduce Cognitive Load — PASS**: the primary graph is fixed to four lifecycle cards and only material conflicts appear inline; secondary records are progressively disclosed.
- **III. Resume Where You Left Off — PASS**: source-backed snapshots, decisions, conflicts, and completion evidence make completed work understandable without replaying private chat.
- **IV. Organize Around Problems, Not Tasks — PASS**: Capture → Problem → Solution → Complete remains the only lineage. Conflict is transition context, never a workflow stage.
- **V. Private Process, Portable Knowledge — PASS**: only already human-completed work becomes Markdown Knowledge. Private chat is excluded unless an excerpt has been promoted into a retained decision/evidence record.
- **VI. Understand the Work, Never Score the Worker — PASS**: provenance and confidence describe claims, never people.
- **A. Measured Performance — PASS**: add deterministic assembly/read benchmarks and keep model work off hot read paths.
- **B. Independent Adapters — PASS**: vault writes remain in `MarkdownVaultAdapter`; model access remains in `OpenAICompatibleProvider`; the workflow service receives validated data rather than provider objects.
- **C. Human Authority over AI — PASS**: completion and corrections are human actions. AI can only contribute visibly inferred interpretations and cannot advance workflow state.
- **D. Evidence and Logical Consistency — PASS**: every assertion carries validated evidence references or an explicit absence/inference label; external document changes retain current hash protections.
- **E. Local and Cross-Platform — PASS**: SQLite and adapter-owned atomic writes preserve the current macOS/Windows boundary.
- **F. Minimal Complexity — PASS**: use existing dependencies and a small set of append-only lineage tables; do not introduce a graph database, job queue, or frontend framework.

## Project Structure

### Documentation (this feature)

```text
specs/006-lineage-knowledge-layer/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── api.md
└── checklists/
    └── requirements.md
```

### Source Code (repository root)

```text
llm_wiki/
├── api/
│   └── app.py                 # completion orchestration and lineage endpoints
├── services/
│   ├── workflow.py            # schema, source events, snapshots, corrections
│   ├── lineage.py             # deterministic assembly, validation, Markdown rendering
│   ├── provider.py            # existing model adapter for optional inference
│   └── vault.py               # existing atomic Markdown/file writes
└── static/
    └── index.html             # four-stage graph, drill-down, corrections, Problem navigation

tests/
├── test_api.py
├── test_completion_dashboard.py
├── test_browser_menu.py
├── test_transitions.py
└── test_workflow.py
```

**Structure Decision**: Keep the current single-package architecture. Add one pure lineage service to prevent `workflow.py` and `app.py` from absorbing rendering and evidence-validation rules; persist data through `WorkflowEngine`, call models only from API orchestration, and write Markdown only through the vault adapter.

## Implementation Strategy

### 1. Preserve decision-ready source events

- Add append-only Solution decision events for creation, applied refinement/manual edits, approval, conflict address, and completion. Store before/after values only for fields that changed and reference the human action, approval, AI run, conflict report, or work evidence that caused the event.
- Extend conflict resolution recording so a previously detected conflict can be marked Addressed only with `explicit_decision` or `implementation_evidence`. Record its Preserved/Modified/Superseded/Rejected disposition. Existing ambiguous `clear` citations remain `unclear`, never retroactively Addressed.
- Keep current mutable Solution fields as the live projection; the event stream is the audit source for Decision Changes.

### 2. Build an evidence-first completion snapshot

- On human completion, synchronously assemble a four-stage deterministic snapshot from Capture, Problem, current Solution fields, approvals, decision events, conflict reports/addresses, Work Log, checklist, completion review, and human completion decision.
- Copy concise source excerpts plus stable entity/field references and hashes into the snapshot so final Knowledge remains understandable if live records later change or disappear.
- Emit `Not explicitly recorded` when no decision reason exists. Do not infer a conflict address or disposition.
- Persist the deterministic snapshot before Playbook rendering. Snapshot creation is idempotent for the same source hash.

### 3. Add bounded, validated AI interpretation

- Completion and regeneration first build deterministic Lineage, then optionally submit a compact evidence bundle with opaque evidence IDs to the dedicated Lineage interpretation task. Provider failure leaves deterministic Lineage available and does not control the human completion decision.
- Accept only schema-valid `Inferred` claims whose cited evidence IDs exist in that bundle. Reject unknown citations, unsupported Addressed/Resolved conflict claims, and attempts to label content Observed or Decided.
- Store provider failure or validation failure on the snapshot while leaving the deterministic lineage ready and retryable. Reading Lineage never invokes the provider.

### 4. Persist corrections as revisions

- Treat generated claims as immutable revisions. A user correction appends a `user` revision, points to the prior revision, and becomes current without deleting the AI text or evidence links.
- Permit correction of interpretation text and classification only within evidence rules. Source excerpts, source IDs, human decisions, conflict facts, and completion evidence are immutable.
- Regeneration rebuilds current Lineage from source records and carries forward applicable user corrections by stable claim key; internal audit snapshots and unmatched corrections remain preserved but are not exposed as document Version Control.

### 5. Project one model into UI and Markdown

- Return a single Lineage API representation consumed by the completed workspace and the Playbook renderer.
- Render four cards with three transition arrows. Show provenance badges on claims and a compact conflict state only on affected transitions. Keep additional decisions/conflicts in expandable sections.
- Selecting Problem opens the existing Problem exploration surface in read-only completed context. Evidence selection opens an in-workspace source panel, with live-record navigation when available.
- Render final Markdown sections in this order: Detail, Lineage, Decision Changes, Conflicts & Addresses, Completion Evidence, followed by a Raw Data link. Do not change the completed-work Detail modal.

### 6. Generate the final document from Lineage

- Enforce a one-way generation graph: source records → validated Lineage snapshot → optional final-report narrative → final Markdown. The report output never feeds back into the snapshot that supplied it.
- Replace the current raw-content-prefix prompt input with a compact report context built from the selected Lineage projection and the immutable evidence excerpts referenced by its claims.
- Ask the existing completion-report model to organize the executive summary and detailed narrative from this context only. Validate that returned headings are present and that no source IDs outside the context are cited; provider failure falls back to deterministic sections rendered from Lineage.
- Record internal snapshot, schema, source-hash, and report-input metadata with the completed-work projection for safety and audit without presenting it as user-facing document versioning.
- A normal Playbook regeneration performs external-file checks, rebuilds current Lineage with applicable user corrections, and then regenerates the report and Markdown from that same Lineage.

### 7. Integrate completion and regeneration safely

- Update both human completion paths to ensure a deterministic `ready_without_inference` or fully enriched `ready` snapshot exists before final-report generation and `write_completion_playbook` rendering.
- Preserve the existing behavior where workflow completion is not rolled back by a vault I/O failure. Return an explicit retryable archive/lineage status.
- Make Playbook regeneration rebuild Lineage and the document together after source-hash and external-file checks.

### 8. Verify and document

- Add unit tests for provenance rules, conflict status/basis/disposition validation, source hashing, idempotency, correction revision chains, and provider-failure fallback.
- Add tests proving the final-report prompt receives the selected Lineage projection and referenced evidence rather than an arbitrary Raw Data prefix, and that report output cannot become its own source evidence.
- Add API tests for read, regenerate, correction, completion integration, and missing/deleted live records.
- Add Playwright coverage at desktop and mobile widths for graph readability, Problem selection, evidence drill-down, correction audit, material-conflict disclosure, and absence of horizontal page scrolling.
- Benchmark deterministic assembly and cached reads against the stated budgets.
- Update paired English/Korean feature guides and completion/archive documentation when implementation changes user-visible behavior.

## Post-Design Constitution Re-check

The design passes all gates. The evidence-first deterministic layer is the authoritative backbone; optional AI output is constrained to cited, labeled interpretation. The approach adds no workflow stage, dependency, worker scoring, hot-path model call, or private-chat publication. The normalized audit tables are justified by correction history and referential traceability that a mutable JSON summary alone cannot safely provide.

## Complexity Tracking

No constitution violations require justification.

## Human-readable document evidence addendum

Keep opaque evidence IDs in the private Lineage inference request because response validation must bind inferred claims to exact database evidence. Build a separate completion-report projection that removes every internal ID, deduplicates evidence by retained source identity, assigns deterministic human labels by evidence kind and order, and replaces claim links with semantic claim keys plus those labels. The final-report prompt may cite only these labels.
