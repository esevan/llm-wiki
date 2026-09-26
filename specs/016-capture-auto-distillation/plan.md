# Implementation Plan: Automatic Capture Distillation

**Branch**: `016-capture-auto-distillation` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

## Summary

Keep new Capture persistence on the existing fast local path, then submit one exact-source-revision `capture_distillation` operation through the shared durable workflow foundation. Validate a bounded, grounded title/content/context/request result and atomically apply it only when the Capture current revision still matches. Raw text and images remain canonical and separately inspectable. Workbench cards and the existing Capture refinement modal read one projection and reconcile through existing job events. A shared content-transition component animates only explicit live revision replacement and preserves focus, input, and scroll.

## Technical Context

**Language/Version**: TypeScript/React 19; Rust 2021/Tauri 2

**Primary Dependencies**: Existing React, Tauri invoke boundary, rusqlite, serde/serde_json, sha2, `OpenAICompatibleProvider`, durable job worker, and workflow foundation; no new dependency

**Storage**: Existing local SQLite in WAL mode; immutable Capture source snapshots with owned original image bytes, currentness/logical-operation metadata, derived distillation, and optional proposal records; existing `captures`, `input_images`, and `ai_jobs_v2` remain authoritative for current Capture/current attachments and job lifecycle

**Testing**: Vitest/Testing Library; focused Cargo unit/application-command tests; schema migration tests; `git diff --check`; targeted packaged desktop E2E only if final UI/native lifecycle risk remains after cheaper checks

**Target Platform**: macOS and Windows desktop

**Project Type**: Tauri desktop application with React UI and Rust application service

**Performance Goals**: Preserve Capture persistence under 50 ms p95; Workbench projection under 100 ms p95 after local data arrives; card/modal reconciliation within 1 second; transition frames remain visually smooth at 60 Hz on representative content

**Constraints**: Local-first; raw source immutable and inspectable; no provider call on persistence hot path; no backfill; exact-revision compare-and-swap; background writes do not update user recency; no autonomous Task/Problem creation or workflow transition; no navigation/rerender animation; reduced-motion support

**Scale/Scope**: New Captures only; existing Capture input supports text and established validated raster images up to 10 MB each; one logical automatic operation per initial source revision; four bounded derived fields; one Workbench card and one existing modal; shared transition primitive prepared for four semantic update causes

## Constitution Check

*GATE: Passed before research and after design.*

- **I. Natural conversation**: rough text and images persist without a classification or cleanup form; organization happens afterward.
- **II. Cognitive load**: a compact existing card gains only an honest state and readable content; detailed source, failure, and proposal information stays in the modal.
- **III. Resume**: raw source, durable job status, current result, proposal, input draft, focus, and scroll survive interruption.
- **IV. Task/Problem boundary**: Capture remains the same canonical thought and gains no workflow state, Task, Problem, count, or user-facing version history.
- **V. Private process**: source and results stay in local application storage and are not Knowledge or an archived idea.
- **VI. No worker scoring**: no metric, rank, or judgment of a person is introduced.
- **A. Performance**: persistence submits only local metadata and never imports or invokes provider code; focused timing tests guard the 50 ms and 100 ms budgets and fail above the constitutional 15% regression threshold.
- **B. Adapters**: model access remains behind `OpenAICompatibleProvider`; React uses the application client and does not call provider or SQLite code.
- **C. Human authority**: the constitution explicitly permits source-preserving initial cleanup. Exact currentness rejects intervening edits; a useful superseded output may remain a clearly labeled proposal and never implies adoption, execution, or verification.
- **D. Evidence**: result validation requires source-grounded fields and provenance. Job/result identity records exact Capture source revision and image hashes.
- **E. Local/cross-platform**: SQLite transactions, durable leases, CSS masks, and browser motion preference are platform-neutral; private source is not external storage.
- **F. Minimal complexity**: reuse current queue, provider, image storage, event stream, modal, tokens, and focus/scroll machinery; add no dependency or second worker path.

## Product Spirit Assessment

The feature serves “You Talk. The Work Organizes Itself,” “Reduce Cognitive Load,” and “Resume Where You Left Off.” It does not weaken the other principles: Capture identity and raw source remain canonical; automatic organization changes no Task/Problem lifecycle; private process is not published; and no worker assessment is introduced.

## Research Decisions

Detailed rationale and rejected alternatives are in [research.md](research.md).

- Use the foundation's registered durable `capture_distillation` kind rather than overloading `workflow_draft` or `image_summary`.
- Submit once from the successful new-Capture command, never from reads or modal lifecycle.
- Bypass the existing modal-open `prepare_image_capture` synthetic refinement for new distillation-eligible image-only Captures so one creation-time interpretation owns the initial cleanup and the item stays in Inbox.
- Snapshot initial raw source separately from editable current Capture text, current derived presentation, and conflicting proposal.
- Publish through `set_job_application_disposition` plus domain writes in one transaction: `applied` for an exact head and `superseded` for every head mismatch; useful superseded output may atomically store a domain proposal.
- Reconcile UI from the Workbench/Capture snapshot after event hints.
- Animate semantic live replacement through a reusable revision-transition component, not component mount.

## Data and Interface Design

- [data-model.md](data-model.md) defines Capture source/current revision, distillation, proposal, and transition event.
- [contracts/application-api.md](contracts/application-api.md) defines save/read/retry, foundation registration, publication, and projection contracts.
- [contracts/interaction.md](contracts/interaction.md) defines card/modal synchronization, motion eligibility, accessibility, and context preservation.
- [quickstart.md](quickstart.md) defines focused validation and the E2E decision gate.

## Performance, Token, and Dependency Budgets

- The Capture transaction stores source, images, revision metadata, and one idempotent local workflow submission only. No provider/model code is loaded on that path.
- Benchmark text-only, image-only, and mixed saves against the existing 50 ms p95 budget; benchmark a 1,000-item Workbench projection against 100 ms p95. A regression over 15% fails.
- Provider input includes only the exact Capture text and its validated images. It performs no Vault retrieval. Output is bounded to a 120-character title, 4,000-character readable content, 2,000-character context, and at most eight explicit requests of 500 characters each.
- One logical operation receives existing bounded technical attempts and at most one coalesced active successor for latest explanatory chat context. Modal open, event reconnect, and UI remount consume zero model tokens.
- No dependency is added. Shared transition uses React and CSS primitives already present.

## Conflict Invalidation and Recovery

- The initial job input carries `taskKind`, `entityType`, `entityId`, opaque exact `sourceRevision`, locale, source hash, and ordered image identities/hashes. Foundation preserves registered prompt ID/version and source revision across retry.
- Existing `item.update` Capture edits append source provenance and advance source/currentness without rescheduling cleanup. Explanatory refinement input advances context currentness, supersedes the old attempt, and coalesces one latest-context successor under the same logical operation. Publication compares bound and actual revisions inside the same transaction that would write derived fields.
- Exact match records `applied`; mismatch records foundation `superseded`, while a useful validated mismatch may store a proposal in the same transaction; deleted/missing/incompatible source stores no proposal. No branch partially writes current fields.
- Automatic application never updates `last_user_activity_at`. Proposal acceptance is outside automatic apply.
- Expired leases recover through the existing durable queue. Active equivalent work coalesces. Permanent/exhausted failure persists until explicit retry or a newer applicable result.

## UI Design Direction

**Primary task**: let the user keep capturing or continue conversation while automatic organization clarifies the same card in place.

**Information hierarchy**: raw source and active input are primary in the modal; derived title/body are primary on the compact card; status is a quiet labeled line; retry/proposal details appear only when relevant. The current Capture remains in its lane and position.

**Visual characteristic**: a short stationary-text mask communicates “this content was replaced here.” The old title disappears right-to-left and the new title appears left-to-right. Body opacity/color settles more subtly. Existing cream, white, ink, restrained pink, spacing, radius, shadow, and motion tokens are reused.

The shared `ContentRevisionTransition` receives stable entity identity, displayed revision, next revision, semantic cause (`preview_adopted`, `automatic_apply`, `refinement_apply`, `version_restore`), and reduced-motion state. It snapshots only old/new rendered content during an eligible change. Initial mount, hydration, navigation, locale-only render, historical mode, and equal/unknown revisions render statically. The wrapper preserves document geometry; text never translates.

Implementation review will render and inspect wide/narrow, Korean/English, long/empty/error content, active input, failure/retry, proposal, and reduced-motion states. Keyboard focus and scroll will be exercised. This design phase records that requirement but does not claim rendered validation.

## Project Structure

```text
specs/016-capture-auto-distillation/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md, tasks.md
├── checklists/requirements.md
└── contracts/{application-api.md,interaction.md}

frontend/src/components/ContentRevisionTransition.tsx
frontend/src/features/workbench/
├── WorkbenchView.tsx, RefinementPanel.tsx
├── WorkbenchView.test.tsx, RefinementPanel.test.tsx
├── taskWorkbenchText.ts
└── task-workbench.css
frontend/src/services/{taskClient.ts,tauriApplicationClient.ts}
frontend/src/types/taskWorkbench.ts

src-tauri/src/workflow_foundation.rs
src-tauri/src/native/workflow_foundation.rs
src-tauri/src/application/task_service.rs
src-tauri/src/native/workflow.rs
src-tauri/src/native/{jobs.rs,job_results.rs,migrations.rs,capture_distillation.rs}
src-tauri/tests/application_commands.rs

docs/features/{capture-auto-distillation.md,capture-auto-distillation.ko.md}
docs/features/{README.md,README.ko.md}
docs/{product-spirit.md,product-spirit.ko.md}
```

**Structure Decision**: Extend the established Capture vertical slice and shared workflow foundation. Keep orchestration/result publication in Rust, provider calls behind the existing adapter, and presentation/reconciliation in the existing React Workbench.

## Delivery Sequence and Integration Gate

1. Merge the foundation branch that supplies workflow registry/types, shared job metadata, prompt/version contract, provenance/reference APIs, and exact-revision application disposition.
2. Reconcile paths/names in these artifacts with the merged code before implementation.
3. Add Capture revision/derived schema and register/use the new kind.
4. Add handler/result validation and exact publication outcomes.
5. Add card/modal status, source presentation, retry, shared transition, and new-Capture bypass of legacy modal-open image refinement.
6. Add localized documentation and focused verification; run the smallest packaged scenario only if UI/native restart behavior remains materially unverified.

No implementation begins before step 1 is available on this branch.

## Complexity Tracking

No constitution violations. The new result table and shared transition component are the smallest separations that keep raw source immutable, make exact currentness auditable, and avoid duplicating queue or animation rules.
