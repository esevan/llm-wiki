# Feature Specification: Automatic Capture Distillation

**Feature Branch**: `016-capture-auto-distillation`

**Created**: 2026-09-26

**Status**: Draft

**Input**: User description: "Save every new text or image Capture immediately, then perform one automatic background cleanup that derives a readable title, content, context, and explicit requests while preserving the separately inspectable source and protecting later user edits."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Capture First, Organize Automatically (Priority: P1)

A user submits rough text, one or more images, or images without text. The Capture becomes durable immediately with its exact source intact. A compact placeholder such as `Image capture` makes an image-only item identifiable while one background distillation organizes it into a human-readable title and body. The user does not need to classify or rewrite the thought before moving on.

**Why this priority**: Immediate persistence protects the thought; automatic organization delivers the main reduction in cognitive load without slowing Capture.

**Independent Test**: Save text-only, mixed text-and-image, and image-only Captures while distillation is delayed. Confirm each source is immediately inspectable, survives restart, and later receives grounded derived fields on the same identity and in the same position.

**Acceptance Scenarios**:

1. **Given** rough text, **When** the user saves a new Capture, **Then** the exact authored text is durable within the Capture persistence budget and one automatic distillation begins without blocking another input.
2. **Given** images and no text, **When** the user saves a new Capture, **Then** image bytes and metadata are durable, the card has a localized image placeholder, and the background result describes only information supported by the images.
3. **Given** a successful current result, **When** it is applied, **Then** the same Capture identity, lane, position, source, and user-activity time remain while its derived title, readable content, context, and explicit requests become available.
4. **Given** an older Capture that predates this feature, **When** it is listed or opened, **Then** no automatic distillation is created for it.

---

### User Story 2 - Continue While Capture Improves (Priority: P2)

A user can open the Capture conversation immediately, see the original source, keep typing, and understand whether automatic organization is processing, complete, or failed. The card and modal show the same current state. A successful current result changes the existing content in place: the previous title is masked from right to left and the new title is revealed from left to right, while the body uses a subtle transition. The text itself does not slide.

**Why this priority**: The background behavior is useful only if it does not interrupt the next thought or destabilize the current reading and writing context.

**Independent Test**: Open a just-saved Capture before completion, type and scroll in its conversation, then complete or fail the job. Confirm input, focus, selection, modal scroll, page position, and source visibility are preserved and both surfaces agree.

**Acceptance Scenarios**:

1. **Given** a queued or running distillation, **When** the user opens the Capture modal, **Then** the existing job is observed rather than duplicated, the source is immediately visible, and conversation input remains active.
2. **Given** the card and modal are both visible for one Capture, **When** processing succeeds or fails, **Then** both surfaces reconcile to the same status and result without navigation.
3. **Given** a current successful result, **When** derived content replaces its placeholder or prior derived view, **Then** title and body transitions communicate the in-place change without moving the card or its text box.
4. **Given** reduced-motion preference, **When** content changes, **Then** the final content appears without the directional or body transition.
5. **Given** navigation, ordinary rerender, restart, or historical reading, **When** already-current content is rendered, **Then** no content-change animation plays.

---

### User Story 3 - Keep User Changes and Recover Clearly (Priority: P3)

A user may edit a Capture or add refinement input while distillation is running. That later input always wins. An outdated result cannot overwrite it and may be retained as a reviewable proposal when it still offers useful organization. Failures remain visible with a deliberate retry action, and restart recovery continues the original durable work without duplication.

**Why this priority**: Automatic help is trustworthy only when it preserves authorship, exposes failure, and behaves predictably across races and restarts.

**Independent Test**: Delay a job, edit the Capture or submit refinement input, then finish the old attempt. Verify no user-authored field changes, any useful result is labeled as a proposal, failure remains visible, explicit retry is deduplicated, and restart resumes one logical operation.

**Acceptance Scenarios**:

1. **Given** a running distillation, **When** the user directly edits Capture content, **Then** currentness advances, the late result cannot become current, and useful old output may remain a proposal.
2. **Given** a running distillation, **When** the user adds explanatory chat input before initial cleanup settles, **Then** the old attempt is superseded and one bounded successor within the same logical initial operation uses the latest explanation.
3. **Given** a valid but conflicting late result, **When** completion is evaluated, **Then** it is retained as an optional proposal bound to its source revision and does not overwrite the current Capture.
4. **Given** a permanent or exhausted failure, **When** the Capture is viewed later, **Then** failure remains visible with a safe explanation and explicit retry when allowed.
5. **Given** a restart during queued or running work, **When** processing recovers, **Then** the same logical operation resumes or reaches an honest terminal state without a second logical initial cleanup.
6. **Given** repeated modal opens or repeated retry gestures, **When** an equivalent attempt is already active, **Then** no duplicate job or result is created.

### Edge Cases

- Empty text with unsupported, corrupt, or unreadable image data fails before creating a misleading Capture.
- Legible and ambiguous images appear together; the result distinguishes observed content from absent context and does not infer unsupported intent.
- The provider returns empty, malformed, overlong, or invented fields; validation fails visibly and the raw Capture stays usable.
- The provider is unavailable at save time; source persistence succeeds and the durable job remains recoverable or visibly failed.
- The user edits while result publication is committing; exact-revision comparison makes either the user edit or derived apply win atomically, never a partial mix.
- A Capture is deleted before completion; late output becomes stale and cannot recreate or reorder it.
- Multiple queue events arrive out of order or after reconnect; surfaces reconcile from the durable snapshot rather than trusting event payload order.
- A title or body wraps across lines or expands under Korean/English localization; masking does not clip glyphs or change layout unexpectedly.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST commit every valid new Capture's exact authored text and image sources before automatic distillation can affect what the user sees.
- **FR-002**: Capture persistence MUST remain independent of provider availability and MUST complete without waiting for distillation.
- **FR-003**: Exactly one logical initial distillation MUST belong to each newly created Capture. Listing, opening, rerendering, reconnecting, and restarting MUST NOT create another logical operation; added explanatory chat MAY supersede its old exact-revision attempt with one coalesced successor inside that same operation.
- **FR-004**: Existing Captures MUST NOT be automatically backfilled by this feature.
- **FR-005**: Text-only, mixed text-and-image, and image-only Captures MUST be supported; an image-only Capture MUST have a localized temporary label and MUST NOT require invented text.
- **FR-006**: Distillation MUST produce separately addressable derived title, readable content, context, and explicit-request fields, allowing any unsupported field to remain empty.
- **FR-007**: Derived claims MUST be grounded in the Capture's exact source text and images; the result MUST NOT invent intent, people, dates, actions, or context that the source does not support.
- **FR-008**: The exact raw source text, image metadata, and image bytes MUST remain separately inspectable after successful, failed, stale, proposed, or retried distillation.
- **FR-009**: Applying a result MUST retain the Capture ID, lane, category, position, workflow meaning, and user-activity ordering; automatic work MUST NOT create a Task or Problem, enter a refining/refined state, or create visible version-count semantics.
- **FR-010**: Every job and result MUST bind to an exact Capture source/current revision and source identity. Publication MUST atomically reject a mismatch.
- **FR-011**: A direct user edit after job creation MUST take precedence over the job. A useful conflicting result MAY remain as an explicitly labeled proposal but MUST NOT overwrite current content automatically.
- **FR-011a**: Explanatory chat input added before initial cleanup settles MUST advance context currentness, supersede older work, and coalesce a bounded successor using the latest raw source plus explanation within the same logical initial operation.
- **FR-012**: Card and modal MUST derive processing, success, failure, retry, stale, and proposal presentation from one durable current state.
- **FR-013**: Opening the modal MUST show source and current distillation state immediately and MUST NOT schedule work.
- **FR-014**: Background state changes MUST preserve the active editor's value, selection, composition, focus, and enabled state; modal scroll, page scroll, selected view, and input draft MUST also remain stable.
- **FR-015**: Current successful title changes MUST erase the old visible title from right to left and reveal the new title from left to right by masking stationary text; body changes MUST use a subtle transition.
- **FR-016**: The content-transition behavior MUST be a shared semantic component usable by later preview adoption, automatic application, refinement application, and version restoration without encoding Capture-specific queue behavior.
- **FR-017**: The shared transition MUST play only for a confirmed change from one current content revision to a newer current content revision. It MUST NOT play for navigation, mount, ordinary rerender, restart hydration, historical reading, or a changed presentation locale alone.
- **FR-018**: Reduced-motion preference MUST suppress directional masks and nonessential body transitions while preserving immediate content and status updates.
- **FR-019**: Queued/running work MUST survive restart through the durable job lifecycle. Equivalent submissions, latest-context successor scheduling, and explicit retries MUST be idempotent while an equivalent attempt is active.
- **FR-020**: Terminal failure MUST remain visible on the Capture until an explicit retry begins or a newer applicable result supersedes it; the system MUST NOT silently report failure as success.
- **FR-021**: Retry MUST be user initiated after visible failure, reuse the same current source binding, and expose whether retry is unavailable or becomes stale.
- **FR-022**: The system MUST expose safe, localized status and errors without prompts, credentials, binary image payloads, or private provider details.
- **FR-023**: Automatic distillation and its status updates MUST NOT change `last user activity`, card recency, or active-work shortcuts.
- **FR-024**: The UI MUST provide keyboard-visible focus, non-color status labels, descriptive controls, and layouts that tolerate narrow windows, long content, and Korean/English text.
- **FR-025**: Automated validation MUST cover immediate source persistence, all three source modes, one-job deduplication, restart recovery, current apply, stale/proposal behavior, visible failure/retry, synchronized surfaces, focus/input/scroll preservation, transition eligibility, and reduced motion.

### Key Entities

- **Capture Source Revision**: Exact user-authored text and image identities for one Capture at a point in time; immutable input to a distillation attempt and separately inspectable from derived presentation.
- **Capture Distillation**: Current derived title, readable content, context, and explicit requests with source binding, generated locale, result revision, and application state.
- **Distillation Job**: Durable automatic work identity, attempts, lifecycle, safe failure, source binding, and retry/recovery information for one initial Capture source revision.
- **Distillation Proposal**: A validated result whose source is no longer current, retained for optional review without automatic application.
- **Content Transition**: Presentation event binding an entity, previous content revision, next content revision, semantic cause, and motion preference; it contains no workflow mutation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At least 95% of valid Capture saves make the exact source available in under 50 ms at p95 on the supported local benchmark, independent of provider latency.
- **SC-002**: Across text-only, mixed, and image-only acceptance fixtures, 100% of saved sources survive restart byte-for-byte and 0 derived results replace or delete the raw source.
- **SC-003**: Across duplicate-open, reconnect, restart, and repeated-retry tests, each new Capture has one logical automatic operation and at most one active equivalent attempt.
- **SC-004**: Across edit/result race tests, stale results overwrite newer user input in 0 cases and partial result application occurs in 0 cases.
- **SC-005**: In all card/modal synchronization tests, both surfaces show the same terminal status and content within 1 second of durable reconciliation while typed input, focus, and scroll remain unchanged.
- **SC-006**: In transition tests, 100% of eligible current-revision replacements use the specified stationary-text mask/body treatment, and 0 navigation, hydration, rerender, historical-reading, locale-only, or reduced-motion cases animate incorrectly.
- **SC-007**: In grounded fixture review, every nonempty derived claim maps to visible source evidence and unsupported intent is invented in 0 cases.
- **SC-008**: Every simulated permanent failure remains discoverable after restart and offers an accurate retry state; no failure is presented as a completed cleanup.

## Assumptions

- The existing local durable AI queue, provider routing, Capture image storage, refinement modal, localization system, and Workbench event reconciliation are extended rather than replaced.
- The foundation work supplies registered workflow-job kinds, exact source/current revision checks, attempt recovery, and typed apply outcomes. This feature owns Capture-derived storage, Capture projections, endpoint wiring, prompt/result validation, and UI behavior.
- Automatic cleanup applies only to Captures created after the feature is enabled. Migration may add neutral fields and revisions to old rows but does not enqueue them.
- Distilled content is a derived presentation of the canonical Capture, not a workflow state, refinement decision, user-authored revision history, Task, Problem, or Knowledge publication.
- The initial supported source is existing Capture text and already-supported raster images; OCR as a separately indexed capability remains out of scope.
- One automatic attempt means one logical durable operation. The queue may perform bounded technical retries for transient failure without creating additional user-visible operations.

## Product Spirit Assessment

This feature directly advances Principle I, “You Talk. The Work Organizes Itself,” and Principle II, “Reduce Cognitive Load,” by accepting rough input immediately and organizing it afterward. It advances Principle III by keeping source and processing state resumable. It preserves Principle IV because the Capture remains the same canonical thought rather than becoming a Task, Problem, or workflow stage; Principle V because source and derived private process stay local and unpublished; and Principle VI because no personal score or productivity signal is introduced.
