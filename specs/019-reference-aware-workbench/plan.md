# Implementation Plan: Reference-aware workbench

**Branch**: `019-reference-aware-workbench` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

## Summary

Extend the existing Task/Capture refinement workspace with immutable structured work-preview versions, evidence-first preparation, latest-context optional investigation, exact reference usage, and reusable reference/version review controls. Necessary retrieval is part of the requested preview operation and completes before the first useful preview; optional follow-up investigation never blocks conversation. Actual adoption appends a new preview version and uses the shared `ContentRevisionTransition`, while historical reads, mentions, viewing, and rerenders never mutate the Task or imply adoption.

## Technical Context

**Language/Version**: TypeScript/React 19; Rust 2021/Tauri 2

**Primary Dependencies**: Existing React workbench, `applicationClient`/`taskClient`, Tauri commands, rusqlite, serde/serde_json, feature 015 prompt/job/version/reference contracts, feature 016 `ContentRevisionTransition`, and feature 018 aspect-aware retrieval; no new dependency

**Storage**: Existing SQLite in WAL mode. Feature-owned immutable preview versions, current-preview pointer, assumptions, investigation snapshots, mention bindings, and reference-usage facts; existing Task, Work Log, decision, queue, Vault document, evidence-grant, and canonical retrieval stores remain authoritative

**Testing**: Vitest/Testing Library; focused Cargo unit/application-command and migration tests; rendered wide/narrow EN/KO accessibility review; `git diff --check`; selected packaged desktop E2E only when focused checks leave a concrete native/modal lifecycle risk

**Target Platform**: macOS and Windows desktop

**Project Type**: Tauri desktop application with React UI and Rust application services

**Performance Goals**: Stable local filtering/sorting for at least 1,000 reference rows within the existing 100 ms Workbench projection budget; modal navigation and stored-version selection are local and require no model call; preserve feature 018 retrieval budgets and report prompt, retrieval, provider, validation, and application stages separately

**Constraints**: Local-first; no mandatory clarification before supported preview generation; necessary retrieval precedes useful preview; optional investigation is latest-context and has no endless durable retry; exact source/version/section bindings; explicit adoption only; historical reads never apply; restore appends; intervening edits win; input/focus/scroll/order survive async arrivals and modal navigation; reduced motion; no Knowledge/publication behavior

**Scale/Scope**: One existing refinement workspace; structured preview fields for description, background, goal, scope, non-goals, constraints, completion criteria, approach, and assumptions; many paged references; exact document and section mentions; one shared ReferenceViewer and one shared draft-version control surface reusable by features 020 and 021

## Constitution Check

*GATE: Passed before research and after design.*

- **I. Natural conversation**: Preview preparation proceeds from the user request without a classification form or routine clarification gate; conversation remains available during optional investigation.
- **II. Cognitive load**: The primary surface is the current work preview. Investigation state and references occupy a secondary rail, with critical findings promoted only when they affect the next decision.
- **III. Resume**: Immutable versions, exact reference snapshots, message draft, active tab, focused item, modal history, and scroll anchors are explicit state and recover after interruption.
- **IV. Task/Problem boundary**: A work preview structures a Task proposal. Reading, searching, mentioning, investigating, or restoring a preview does not create a Problem, start work, complete a Task, or publish Knowledge.
- **V. Private process**: Preview versions, assumptions, investigations, and reference-use facts stay in private application storage. Feature 020 remains responsible for Knowledge review.
- **VI. No worker scoring**: References rank evidence applicability, never people or productivity.
- **A. Performance**: Feature 018 owns retrieval timing; E adds deterministic local list operations and records stage timing without an unmeasured latency claim. Reference arrivals do not reorder the rows currently being read.
- **B. Adapters**: React calls typed application services only. E consumes feature 018 retrieval and evidence grants and does not call provider, SQLite, or the Vault directly.
- **C. Human authority**: AI may prepare a preview, but only an explicit apply/adopt action changes canonical Task content. Silence, viewing, mention insertion, and source use do not imply adoption.
- **D. Evidence**: Every used preview claim binds exact document ID, revision, and optional section/claim. Assumptions are distinct from evidence. Stale optional results are superseded or retained only as labeled historical context.
- **E. Local/cross-platform**: SQLite transactions, React dialogs, Markdown rendering, and keyboard/focus patterns remain platform-neutral; no external Vault connection is added.
- **F. Minimal complexity**: Reuse refinement sessions, the shared queue, retrieval domain, evidence grants, application client, Markdown renderer, modal-interaction hook, and content transition. Add no duplicate Task, decision, document, or search store.

## Product and UI Direction

The refinement dialog keeps one visual focus: the structured preview. A quieter right rail contains investigation state and references, and collapses below the preview on narrow windows. The conversation composer stays continuously available. Critical findings appear as a compact alert near the preview and become grounded context for the next assistant reply; supporting and ancillary evidence remains in the reference rail.

Reference rows use stable insertion groups and explicit sort controls rather than silently jumping as results arrive. Status uses text and icons in addition to color. The shared ReferenceViewer presents source title, stable version, section path, safely rendered Markdown, back/forward history, and previous/next result controls. Opening and closing restores the originating control, composer draft, selection, and scroll position. `@` lookup inserts non-sending document/section tokens into the composer.

The compact version control identifies current, historical, and restored versions, supports a field-level two-version diff, and exposes `Restore as new`. Historical mode suppresses content transitions and all apply affordances. A successful explicit apply/restore can use `ContentRevisionTransition` with `preview_adopted` or `version_restore`; navigation and async rerender cannot.

## Research Decisions

Detailed rationale and alternatives are in [research.md](research.md).

- Extend the current refinement session instead of creating a second conversation or preview workflow.
- Split orchestration into `native/reference_aware_workbench.rs`; keep `native/task_assistance.rs` as compatibility routing and transaction integration.
- Register an E-owned preview prompt definition with a typed retrieval-intent and structured-preview output instead of passing caller-authored semantic instructions.
- Treat necessary retrieval as a phase of the user-requested generation and optional follow-up investigation as latest-wins speculative work.
- Persist only immutable preview/reference/usage facts; consume canonical feature 018 documents, chunks, ranking, grants, and redirects.
- Freeze visible reference ordering during a reading session and expose deliberate sort/filter changes.
- Share `ReferenceViewer` and `DraftVersionControls` with features 020/021, while E owns generic behavior and consumers supply feature-specific content.
- Use one append-only restore/apply transaction guarded by exact current preview and Task revisions.

## Data and Interface Design

- [data-model.md](data-model.md) defines immutable preview versions, assumptions, investigations, mentions, usage, view state, and transitions.
- [contracts/application-api.md](contracts/application-api.md) defines read, generation, investigation, reference, mention, apply, compare, and restore operations.
- [contracts/prompt-output.md](contracts/prompt-output.md) defines the registered prompt input/output and retrieval-before-preview boundary.
- [contracts/interaction.md](contracts/interaction.md) defines workspace hierarchy, stable arrivals, modal/mention behavior, version controls, focus, and state semantics.
- [quickstart.md](quickstart.md) defines dependency gates and focused validation scenarios.

## Lifecycle and Freshness

1. A request appends a refinement message and advances the refinement context revision.
2. The registered preview planner returns a retrieval intent. If `needed`, the same requested generation executes feature 018 retrieval and binds exact results before completing a useful preview.
3. Validation appends an immutable preview version with assumptions and an exact reference snapshot. It does not update canonical Task fields.
4. Follow-up conversation may schedule optional investigation keyed by the latest context revision. A newer message cancels or supersedes older optional work. No automatic durable retry loop is required.
5. Findings are partitioned into `critical`, `supporting`, and `ancillary`. Only current critical findings surface promptly; current supporting findings enter the next reply context; ancillary findings update the stable reference rail.
6. An explicit apply checks expected Task revision, expected preview head, and unchanged edited fields in one transaction, updates the canonical Task through the existing application path, records adopted/used reference facts, and marks the job application disposition.
7. Restore reads an immutable old version and appends a new current preview version derived from it. It never rewinds Work Logs, decisions, journey, messages, or Task revisions.

## Failure and Empty-State Semantics

- Requested preview generation exposes queued/running/retryable/failed status and an explicit retry. The last good preview and unsent input remain visible.
- Necessary retrieval `no_suitable_result` completes the preview with labeled assumptions when the prompt contract permits it; it is not presented as a provider failure.
- Optional investigation failure is non-blocking and retryable only by a later relevant request or explicit user action; conversation remains usable.
- `changed_context` means the result executed but cannot affect the current preview/reply. It is never rendered as current evidence.
- A stale apply returns a revision conflict and preserves both the canonical Task and the user’s edited preview text.
- A revoked/expired evidence grant triggers a fresh exact read through feature 018; it never falls back to an unscoped path read.

## Migration and Dependency Gate

The current branch migration head is 16 because B/C/D persistence migrations are not yet integrated here. E tentatively owns migration 20 only after the serialized sequence A16, B17, C18, D19 is present. Implementation task T003 must re-read the integrated head and assigned migration before any schema edit. E must not create migration gaps or ship a `20` migration on top of `16`.

B’s full Capture wiring and D’s persistence/application boundary gate E implementation. B's initial cleanup remains an internal derived presentation and never appears in E's user draft history. E's full preview is generated separately and performs necessary retrieval. Feature 020 may consume E’s shared ReferenceViewer/version controls only after E’s public component/type contract is integrated.

## Project Structure

### Documentation

```text
specs/019-reference-aware-workbench/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── application-api.md
│   ├── interaction.md
│   └── prompt-output.md
└── tasks.md
```

### Source code planned for implementation

```text
src-tauri/src/
├── native/
│   ├── reference_aware_workbench.rs
│   ├── task_assistance.rs
│   ├── jobs.rs
│   ├── migrations.rs
│   └── schema.sql
├── workflow_foundation/prompts/
└── domain/retrieval.rs

frontend/src/
├── components/
│   ├── ReferenceViewer.tsx
│   ├── DraftVersionControls.tsx
│   └── ContentRevisionTransition.tsx
├── features/workbench/
│   ├── RefinementPanel.tsx
│   ├── ReferenceAwarePreview.tsx
│   ├── ReferenceList.tsx
│   ├── reference-aware-workbench.css
│   └── taskWorkbenchText.ts
├── services/taskClient.ts
└── types/taskWorkbench.ts
```

**Structure Decision**: Native orchestration and feature-owned persistence live in one focused module, with minimal routing hooks in existing shared files. Generic reference/version review UI lives under `components`; Workbench-specific composition remains under `features/workbench`. No user-facing guide is added during planning because the feature is not shipped.

## Post-design Constitution Re-check

Passed. The design keeps explicit adoption, separate job/application/content/UI state, exact evidence provenance, local-first adapters, append-only restore, focus/input preservation, and Task/Knowledge boundaries. No exception or complexity waiver is required.


## Native runtime integration checkpoint (2026-09-26)

Integrated dependencies: D persistence `1e9c80c`, contiguous E migration 20 `c3302c6`,
E migration test `bde8703`, and domain checkpoint `43b484c`. The current migration head
is 20; the earlier tentative-head discussion is historical.

The consumer seam is `jobs::run_inner` for `refinement_preview`, automatically queued
by the existing refinement response flow. New work records `work_preview_reply`,
`work_preview_planner`, `work_preview_finalizer`, and `work_preview_investigation`
version 1. Old stored non-reference-aware jobs retain their compatibility path.

Generated previews, actual claim use, source/context/preview-head validation and job
review disposition commit in one immediate transaction. Explicit version apply reuses
`apply_proposal_tx` and checks canonical Task revision, hierarchy, source revisions,
preview head/content hash and latest conversation. It never adopts automatically.
Native version edit/restore and apply replay an operation identity transactionally.

Reference retrieval consumes the local desktop authority, D's persisted search index,
and exact revision reads inside the Vault. It does not reuse MCP grants belonging to
another connection. Search precedes finalization; optional investigation is a separate,
non-durable latest-context operation. Preview generation and optional failures remain
separate; critical/supporting findings can ground one later reply, and only provider-
declared actual finding use records reply provenance. Ancillary findings stay out of
unsolicited conversation input.


## Consumer completion decisions (2026-09-26)

The native boundary retains strictly validated JSON values rather than adding unused
named Rust facade types (T007). The registry schema, `validate_planner`,
`runtime::validate_output`, exact-reference validation, canonical hashes and immediate
transactions remain the runtime contract. This is an implementation-location/type
representation resolution, not permission to accept unknown planner/provider fields.

The many-reference projection (T035/T038) lives in `ReferenceList.tsx`, derived from the
persisted exact workspace snapshot and usage facts. It does not copy or own canonical
search/document storage. The component owns its reading epoch, accepted exact keys,
combined metadata/use filters, document-version grouping and 20-document paging.
An arrival cannot insert or reorder a row until explicit acceptance or filter/sort.
Native workspace reads now expose persisted assumption status and exact interaction
facts; retrieval retains information type in each saved reference binding.

Mention token positioning and atomic editing live in `referenceMentions.ts`, with
lookup/batch selection in `RefinementPanel.tsx`. Saved mentions retain offsets in the
existing draft payload; older drafts resolve their ordered visible tokens without
creating bindings from newly pasted text. The schema does not change.

Canonical adoption emits `TaskApplicationEvent` from the confirmed native result with
`taskId`, exact canonical `revision`, and `cause: preview_adopted`. Polling reads and
start/reopen callbacks cannot synthesize this event. TaskCard/TaskDetail consume this
optional event in the separate integrated application owner.
