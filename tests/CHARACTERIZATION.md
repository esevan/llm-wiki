# Behavior-Driven Characterization Cases

**Purpose**: Preserve the application's externally observable baseline before confirmed behavior
changes are implemented. These cases describe outcomes rather than internal call structure. Paths
to removed Python tests below refer to the final parity snapshot `caef236`; current native evidence
is mapped in `docs/migrations/react-tauri-inventory.md` and `src-tauri/tests/`.

## How to use this catalog

- `CB-*` records behavior preserved by the native suite or the archived parity evidence.
- `PA-*` records a confirmed product decision that is not yet fully implemented. It is an acceptance
  case for the future change, not a passing characterization test today.
- When a `PA-*` case is implemented, add or update the referenced automated test and move the case
  into the `CB-*` section.
- Test names should describe the same Given/When/Then outcome and avoid asserting private payloads,
  helper calls, or framework-specific behavior unless those details form a public contract.

## Current behavior characterization

### CB-001 — Structural search remains provider-independent

- **Given** a Vault with indexed Markdown and no configured AI provider,
- **When** the user searches paths, headings, tags, aliases, or content,
- **Then** readable source-bound lexical results remain available.
- **Automated evidence**: `tests/test_retrieval.py`, `tests/test_api.py`, `tests/test_performance.py`

### CB-002 — Semantic preparation is durable background work

- **Given** current Markdown documents without current embeddings,
- **When** the application starts, explicitly indexes, or observes a Vault change,
- **Then** embedding work is enqueued while lexical search remains available.
- **Automated evidence**: `tests/test_embedding_jobs.py`, `tests/test_async_workers.py`

### CB-003 — Current Search semantic mode reranks lexical candidates

- **Given** a lexical result page and available embeddings,
- **When** the current Search semantic option is used,
- **Then** only those lexical candidates are semantically reranked.
- **Automated evidence**: `tests/test_retrieval.py`
- **Transition note**: `PA-001` replaces this behavior with independent semantic corpus search.

### CB-004 — Capture promotion preserves lineage and leaves the inbox

- **Given** a saved Capture,
- **When** the user promotes it to a Problem,
- **Then** one linked Problem exists and the Capture leaves the active inbox without being deleted.
- **Automated evidence**: `tests/test_workbench_flow.py`, `tests/test_transitions.py`

### CB-005 — Conflict review can be bypassed with a recorded reason

- **Given** a proposed Solution and an unavailable or unreliable remote review,
- **When** the user selects the explicit skip path and supplies a reason,
- **Then** the reason is retained as the conflict decision basis and the Solution is approved.
- **Automated evidence**: `tests/test_transitions.py`

### CB-006 — Direct Problem completion is an intentional override

- **Given** a Problem without a verified Solution and an unavailable or unwanted AI review,
- **When** the user invokes direct completion,
- **Then** the Problem is completed, the optional reason is retained, and any applicable
  deterministic completed-work document is published without a second approval.
- **Automated evidence**: `tests/test_api.py`, `tests/test_completion_dashboard.py`

### CB-007 — Workbench organization applies metadata automatically

- **Given** active work without organization metadata,
- **When** Workbench organization completes,
- **Then** category and attention metadata are applied without a separate result approval while
  explicit user overrides remain preserved.
- **Automated evidence**: `tests/test_workflow.py`

### CB-008 — Locale switching does not regenerate stored content

- **Given** stored content and an open primary workflow surface,
- **When** the user switches between Korean and English,
- **Then** packaged and stored locale views are selected without an AI request or identity change.
- **Automated evidence**: `tests/test_localization.py`, `tests/test_browser_menu.py`

### CB-009 — Managed Knowledge translation is request-driven

- **Given** English-canonical managed Knowledge without a current Korean reading,
- **When** Korean reading or explicit translation is requested,
- **Then** canonical content remains immediately readable and one source-bound paragraph job is
  enqueued; unrelated managed documents are not proactively translated. Closing or switching the
  reader detaches presentation updates without cancelling that durable job.
- **Automated evidence**: `tests/test_api.py`, `tests/test_localization.py`,
  `tests/test_browser_menu.py`

### CB-010 — New authored work schedules derived translation

- **Given** a new Capture, Work Log body, comment, or checklist item,
- **When** the authored source is saved,
- **Then** the save returns without waiting for AI and derived translation work is enqueued without
  replacing the authored source.
- **Automated evidence**: `tests/test_api.py`, `tests/test_localization.py`
- **Transition note**: existing checklist body edits are covered by `PA-005`.

### CB-011 — Durable Queue state is visible and payload-safe

- **Given** durable AI or embedding work,
- **When** the user reads Queue state,
- **Then** task, target, status, progress, result destination, timestamps, and safe error information
  are available without exposing private input or provider secrets.
- **Automated evidence**: `tests/test_jobs_api.py`, `tests/test_ai_jobs.py`

### CB-012 — Fast Queue requests remain ephemeral

- **Given** an interactive Chat or enrichment request,
- **When** it passes through the Fast Queue,
- **Then** it is served by the single shared throttle and creates no durable job, result, retry,
  history, or notification record.
- **Automated evidence**: `tests/test_fast_queue.py`, `tests/test_async_provider.py`

### CB-013 — Completion Review creates the only routine completion notification

- **Given** a successful Completion Review,
- **When** its durable result is published,
- **Then** one unread review-ready notification exists and read or dismissal updates the unread
  count; routine jobs remain silent.
- **Automated evidence**: `tests/test_jobs_api.py`, `tests/test_completion_dashboard.py`

### CB-014 — Current cancellation prevents late publication

- **Given** queued or running durable work,
- **When** the user requests cancellation,
- **Then** source content and prior valid results remain unchanged and a cancelled result cannot be
  published as current.
- **Automated evidence**: `tests/test_ai_jobs.py`, `tests/test_api.py`
- **Transition note**: immediate termination of active provider computation is covered by `PA-008`.

### CB-015 — Conflict Review publishes a conservative terminal report

- **Given** a Solution, its Problem, and the current indexed Vault,
- **When** Conflict Review reaches terminal completion,
- **Then** evidence-backed findings and a conservative `potential_conflict`, `clear`, or
  `insufficient_evidence` recommendation are published without changing workflow state.
- **Automated evidence**: `tests/test_api.py`, `tests/test_jobs_api.py`, `tests/test_browser_menu.py`

### CB-016 — Deterministic Lineage survives optional inference failure

- **Given** retained completed-work evidence,
- **When** optional Lineage inference is absent or fails,
- **Then** deterministic Capture→Problem→Solution→Complete lineage remains available.
- **Automated evidence**: `tests/test_workflow.py`, `tests/test_api.py`

### CB-017 — Direction progress uses milestone evidence

- **Given** assessed importance,
- **When** Problem approval, Solution approval, and completion verification occur,
- **Then** immutable contribution events allocate 10%, 20%, and 70% respectively.
- **Automated evidence**: `tests/test_completion_dashboard.py`, `tests/test_workflow.py`

### CB-018 — Refinement Preview remains source-bound and unapplied

- **Given** a Problem or Solution Explore surface,
- **When** bounded context and a Draft or Refine proposal are prepared,
- **Then** no more than five context entries and 500 visible characters are shown, stale results are
  rejected, and durable content changes only after explicit apply or create.
- **Automated evidence**: `tests/test_workflow.py`, `tests/test_browser_menu.py`

### CB-019 — Compatibility APIs remain reachable

- **Given** a current client using a direct workflow, completion, archive, or translation route,
- **When** it submits a valid request,
- **Then** the compatibility contract remains operational during the current backend lifecycle.
- **Automated evidence**: `tests/test_api.py`, `tests/test_transitions.py`
- **Transition note**: deprecation markers are covered by `PA-003` and `PA-006`.

### CB-020 — Required fonts are bundled

- **Given** the application is running without internet access,
- **When** the React shell loads,
- **Then** Nunito, DM Mono, and Noto Sans KR load from verified packaged WOFF2 assets and system
  fonts are fallback faces only.
- **Automated evidence**: `scripts/verify_bundled_fonts.mjs`, `npm run build`

### CB-021 — Native provider work is promptly cancellable

- **Given** active native durable or streamed provider computation,
- **When** the user cancels the job or aborts the conversation,
- **Then** its cancellation token stops active work, no late result is published, and prior valid
  content remains unchanged.
- **Automated evidence**: `src-tauri/tests/application_commands.rs`,
  `frontend/src/services/tauriApplicationClient.test.ts`

## Pending acceptance scenarios

### PA-001 — Independent semantic corpus search

- **Given** a semantically relevant document that is absent from the lexical candidate page,
- **When** the user selects Semantic search,
- **Then** the document can be returned from the complete current semantic corpus.

### PA-003 — Deprecated direct API disclosure

- **Given** a client uses a direct workflow, completion, or archive compatibility route,
- **When** the response is returned,
- **Then** the contract exposes a consistent deprecation signal and remains usable until the Tauri
  backend migration removes it.

### PA-004 — Provider configuration cleanup and Knowledge tier control

- **Given** the user opens AI Setup,
- **When** configuration is read or saved,
- **Then** no unused report-language field is exposed and Knowledge translation has a visible
  Default/Advanced task preference.

### PA-005 — Checklist edit retranslation

- **Given** an existing checklist item with a derived translation,
- **When** its authored body changes,
- **Then** the old derived reading becomes stale and one replacement translation is enqueued after
  the source update succeeds.

### PA-006 — State-changing Knowledge translation request

- **Given** managed Knowledge that needs translation,
- **When** the user explicitly requests translation,
- **Then** a state-changing request enqueues the job and the former read-shaped request is either
  rejected or consistently marked deprecated during migration.

### PA-007 — Lineage inference failure indicator

- **Given** a usable Lineage snapshot and failed optional inference work,
- **When** the completed Lineage view opens,
- **Then** the usable snapshot remains visible and a retryable failure icon exposes a readable
  tooltip.

### PA-008 — Browser-delivery prompt cancellation and Chat-close abort

- **Given** active browser-delivery durable or Fast Queue provider computation,
- **When** the user cancels the work or closes its Chat surface,
- **Then** active computation is promptly aborted, no late result is delivered or published, and
  prior valid content remains unchanged.

### PA-009 — Progressive Conflict Review with deduplicated evidence

- **Given** an active Conflict Review with repeated documents across claims,
- **When** claims, search, screening, detailed review, and finalization advance,
- **Then** safe phases and available counts are visible without hidden reasoning, and terminal
  evidence contains one document entry with all related claim associations.

## Remaining decision-dependent cases

- **TD-006 — Durable history TTL**: Add expiration boundary cases after the retention period is
  selected.
- **TD-020 — Queue accessibility**: Add keyboard, focus, screen-reader announcement, contrast, and
  reduced-motion journeys after the acceptance scope is selected.
