# Research: Automatic Capture Distillation

## Existing Capture and queue path

**Decision**: Extend the existing Capture create transaction, `input_images`, `ai_jobs_v2`, native job worker, job result publication, `/jobs/events` reconciliation, Workbench card, and `RefinementPanel`.

**Rationale**: Current code already persists text and validated raster images, projects Captures in `TaskService::workbench`, operates durable leased jobs, rejects stale results in related workflows, and preserves refinement input/scroll/focus. Reusing these paths produces one lifecycle and one source of truth.

**Alternatives considered**: A frontend request after save can be lost or duplicated. A second queue duplicates leases and recovery. Starting on modal open makes reading mutate state.

The current `prepare_image_capture` path automatically inserts a synthetic refinement message when an older image-only Capture is opened. New distillation-eligible Captures must bypass that compatibility path: their creation-time `capture_distillation` is the sole initial image interpretation, and opening them remains a read/conversation action. Existing pre-feature image-only behavior may remain for compatibility without backfilling distillation.

## Task-kind boundary

**Decision**: Use the foundation's `capture_distillation` durable operation for `captures`.

**Rationale**: `workflow_draft` produces unapplied refinement proposals, while `image_summary` targets Work Log imagery. Initial Capture organization has a distinct trigger, four-field result, and exact automatic-apply rule.

**Alternatives considered**: Overloading either existing kind would conflate result interfaces and authority. Running both would give mixed/image-only Captures two competing states.

## Source and derived data

**Decision**: Snapshot the submitted `captures.text` plus ordered `input_images` identities/hashes as immutable Capture source revision 1; retain existing editable current text, add explicit source/current revision, and store derived distillation separately. Store a useful conflicting result as a proposal while the foundation job remains superseded.

**Rationale**: Existing `item.update` can rewrite `captures.text`, so treating that cell alone as immutable raw would lose authorship. Internal source snapshots preserve initial raw and exact currentness without adding a visible version-history feature.

**Alternatives considered**: In-place replacement and appending generated text both blur source and inference. A general Capture version browser adds out-of-scope cognitive cost.

## Currentness and late output

**Decision**: Bind the job to the foundation's opaque exact `sourceRevision` and local source hash. In one transaction, call `set_job_application_disposition`: `applied` on exact match and `superseded` on every mismatch, as the foundation requires. A useful mismatched result may also create a domain proposal in that same transaction.

**Rationale**: An external preflight still races with user edits. Foundation disposition truthfully says the old result was superseded, while proposal presence says some validated content remains optional for review.

**Alternatives considered**: Last-write-wins can erase newer intent. Always discarding loses useful grounded work. Automatic merging cannot safely resolve semantic overlap.

## Job creation and retry

**Decision**: The successful new-Capture command creates one domain logical operation and submits its first exact-revision job. Reads never submit. Foundation coalesces equivalent active work, recovers expired ownership, and preserves prompt/source revision across explicit retry. If the user adds explanatory chat before cleanup settles, the domain operation supersedes the old job and coalesces at most one successor for the latest context revision; direct Capture edits do not reschedule.

**Rationale**: Creation is the only boundary that knows the record is new and source is committed. A stable domain operation identity preserves once-only initial cleanup, while exact successor jobs let the eventual current result incorporate explanation supplied during processing. Modal open remains read-only.

**Alternatives considered**: A database trigger bypasses registry/provider policy. Infinite automatic retry hides failure and consumes unbounded tokens. A new logical job per gesture fragments status.

## Grounded image-only result

**Decision**: Request structured `title`, `content`, `context`, and `explicitRequests` from only the exact text/images. Each field may be empty; ambiguous image-only source uses a neutral localized placeholder until valid output.

**Rationale**: Partial structured output makes uncertainty honest. Existing multimodal interpretation can organize exact attached images without adding general-purpose OCR/indexing.

**Alternatives considered**: Requiring all fields or inferring likely intent would manufacture completeness. General OCR/indexing exceeds scope.

## Shared projection and motion

**Decision**: Card and modal consume one `CaptureDistillationSummary` and reconcile snapshots after job event hints. A revision-aware shared component animates only semantic live replacement: stationary old title clips right-to-left, stationary new title reveals left-to-right, and body settles subtly.

**Rationale**: Snapshot reconciliation tolerates reconnect/out-of-order events. Revision/cause eligibility prevents mount, locale, navigation, and historical animations and lets later preview/refinement/restore updates reuse the primitive.

**Alternatives considered**: Separate UI state machines can disagree. Direct event payload application trusts order. Generic child-change animation fires for the wrong causes. Text translation destabilizes reading.

## Verification depth

**Decision**: Use focused Rust/React/migration/performance checks plus rendered wide/narrow review. Run one targeted packaged restart scenario only if cheaper checks leave a concrete native lifecycle gap.

**Rationale**: Currentness and UI preservation are deterministic in focused tests. Direct rendering is still needed for masks, wrapping, focus, and reduced motion. No broad risk justifies full E2E.
