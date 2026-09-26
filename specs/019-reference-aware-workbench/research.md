# Research: Reference-aware workbench

## Extend the existing refinement workspace

**Decision**: Add preview/reference orchestration to the current refinement session and extract the new native behavior into `reference_aware_workbench.rs`.

**Rationale**: The current session already owns messages, saved input, task/capture identity, preview jobs, decision application, queue polling, and focus/scroll recovery. A focused module prevents further growth in `task_assistance.rs` while keeping one user conversation and one compatibility boundary.

**Alternatives considered**: A separate research chat would split context and resumption state. Rewriting refinement as a new subsystem would duplicate canonical session and Task behavior.

## Retrieval before the first useful preview

**Decision**: The registered preview planner emits the foundation `RetrievalIntent`. Necessary retrieval executes inside the same user-requested generation before a complete preview is appended.

**Rationale**: A preview cannot claim evidence grounding when the model says retrieval is necessary but retrieval is still pending. Binding exact feature 018 results before validation gives one auditable generation snapshot and one visible requested-job status.

**Alternatives considered**: Always retrieving wastes time and tokens. Rendering a complete-looking preview before required retrieval would weaken provenance. Mandatory clarification contradicts the approved no-routine-question workflow.

## Optional investigation beside conversation

**Decision**: Follow-up investigation uses feature 015’s speculative, latest-wins policy keyed by refinement context revision. It is cancellable, non-blocking, and has no automatic durable retry requirement.

**Rationale**: The user can continue talking while current evidence arrives. Freshness checks prevent revision-1 findings from advising revision 2. A later relevant message or explicit action can request another investigation.

**Alternatives considered**: A durable endless retry queue would consume resources for stale context. Blocking the composer would recreate a clarification/research gate.

## Finding priority and reply integration

**Decision**: Validate findings into `critical`, `supporting`, and `ancillary`. Current critical contradictions surface immediately; supporting evidence becomes bounded context for the next reply; ancillary results update only the reference list.

**Rationale**: This matches the approved attention hierarchy without flooding the conversation. Each finding keeps exact source attribution and context revision.

**Alternatives considered**: Posting every result as chat creates noise and unexpected scroll movement. Keeping every contradiction in a side rail can hide a decision-changing fact.

## Immutable preview versions and restore

**Decision**: Store immutable structured preview versions with a current pointer. Editing, generation, apply preparation, and restoration append versions. Restore copies the selected content/reference snapshot into a new head.

**Rationale**: Append-only versions support exact comparison and provenance. They prevent historical navigation from changing canonical Task state and keep actual Work Logs, decisions, and journey history intact.

**Alternatives considered**: In-place preview updates erase provenance. Moving a pointer backward makes an old read look like a new decision and complicates concurrent edits.

## Exact apply boundary

**Decision**: Explicit apply uses one transaction guarded by expected Task revision, expected preview head, and the user-edited field hashes. It calls the existing canonical Task mutation path and records application disposition/reference use atomically.

**Rationale**: User edits win over late AI output. Foundation metadata alone cannot prove the domain mutation occurred, so the domain write and disposition share the transaction.

**Alternatives considered**: Applying after a separate freshness read has a race. Treating generated preview creation as adoption violates human authority.

## Reference identity, ordering, and state

**Decision**: Consume feature 018 stable document ID, content revision, section/chunk, aspect, status, redirect, and grant metadata. Deduplicate document rows while retaining exact passages. Freeze arrival order for the active reading session; user-selected sorts are deterministic with stable identity as the tie-breaker.

**Rationale**: Exact identity survives duplicate titles and path changes. Stable rows prevent the item under the pointer from moving when a background result arrives.

**Alternatives considered**: Path/title identity is ambiguous. Continuous score re-sorting disrupts reading. Copying retrieval rows into an E search store creates a second source of truth.

## Viewed, mentioned, used, adopted, and excluded

**Decision**: Record separate append-only events/facts. `viewed` is local interaction, `mentioned` is an exact composer binding, `used` requires a claim/reply/preview use reason, `adopted` is recorded only by explicit preview application, and `excluded` records deliberate non-use with scope/reason. Publication lineage consumes actual `used`/counterevidence facts, not discovery or clicks.

**Rationale**: The states answer different questions and prevent inflated provenance.

**Alternatives considered**: A single selected flag conflates attention, evidence use, and decision authority.

## Shared reference and version controls

**Decision**: E owns generic `ReferenceViewer` and `DraftVersionControls`; feature-specific panels supply typed content/actions. The viewer reuses `KnowledgeMarkdown` and `useModalInteraction` patterns.

**Rationale**: E, F, and G need the same exact source/version navigation and safe Markdown semantics. Sharing behavior prevents accessibility and focus drift while preserving distinct preview, Knowledge, and archive content. Published links remain limited to actual-used sources and useful counterevidence.

**Alternatives considered**: Separate E/F/G modals duplicate keyboard, history, and source-version behavior. A global browser-style route would add navigation complexity for a contained workbench task.

## UI hierarchy and responsive behavior

**Decision**: Use a preview-first two-column workspace on wide windows and a preview/conversation/reference stack on narrow windows. References remain secondary, status is textual, and the composer stays reachable.

**Rationale**: This follows the existing warm Workbench surfaces and keeps one main decision visible. It accommodates long English/Korean labels, many references, and accessibility states without competing cards.

**Alternatives considered**: Equal-width card grids obscure the primary action. A references-only full-screen view interrupts the conversation and loses local context.

## Migration sequencing

**Decision**: Reserve E20 tentatively, but create it only after integrated B17/C18/D19 exist and the parent confirms ownership.

**Rationale**: The current branch is at schema 16. Shipping a numbered gap would break serialized migration integration.

**Alternatives considered**: Creating migration 20 now or renumbering downstream work independently would produce collisions or missing predecessors.
