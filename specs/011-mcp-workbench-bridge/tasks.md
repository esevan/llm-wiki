# Tasks: Dual-Chat Work Tracking

**2026-09-05 checkpoint:** Partial fixes landed for T093, T099, T100, T103 and IPC
hardening associated with T105. See `acceptance-verification.md` for exact executed results.
All 105 task entries are preserved; T089–T104 remain open because their complete workflow
acceptance criteria are not yet met. Do not infer release readiness from component test passes.

**Input**: Design documents from `specs/011-mcp-workbench-bridge/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Tests are included because the specification defines adapter parity, authorization,
out-of-order delivery, crash recovery, performance, and packaged desktop acceptance criteria.
Write the listed tests before their corresponding implementation and confirm they fail for the
intended reason.

**Organization**: Work is grouped by user story. Phase 2 establishes the single application and
persistence boundary that every story must use.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel after its phase prerequisites because it changes different files
- **[Story]**: User story mapping from `spec.md`

## Phase 1: Setup

**Purpose**: Establish dependency, module, and verification entry points without implementing a
second runtime or persistence path.

- [X] T001 Add the minimal pinned `rmcp` server/stdio/schema features and lockfile updates in `src-tauri/Cargo.toml` and `Cargo.lock`
- [X] T002 Add application, domain, port, SQLite-adapter, Vault-adapter, and MCP module declarations in `src-tauri/src/lib.rs` and `src-tauri/src/main.rs`
- [X] T003 [P] Create the work-tracking module skeletons in `src-tauri/src/application/mod.rs`, `src-tauri/src/domain/mod.rs`, `src-tauri/src/ports/mod.rs`, and `src-tauri/src/adapters/mod.rs`
- [X] T004 [P] Add an architecture-boundary verifier that rejects SQL, `rusqlite`, database paths, repositories, search indexes, and Vault adapters from MCP/native UI handlers in `scripts/verify_work_tracking_boundaries.mjs`
- [X] T005 Wire the new architecture verifier into `package.json` without changing the stable test commands in `package.json`

---

## Phase 2: Foundational — Single Authority and Durable Event Projection

**Purpose**: Build the blocking source-of-truth, authorization, ordering, and projection primitives
before any Chat, Workbench, or MCP feature can write work state.

**⚠️ CRITICAL**: No user-story implementation starts until this phase passes.

- [X] T006 [P] Add failing migration tests for connections, sessions, immutable events, idempotency, projection outbox/results, per-stream watermarks, decisions, links, and separate publication state in `src-tauri/tests/work_tracking_migrations.rs`
- [X] T007 [P] Add failing fake-port service tests for authorization, exact idempotency replay, conflicting operation reuse, head CAS, and revocation in `src-tauri/tests/work_tracking_service.rs`
- [X] T008 [P] Add failing projector property tests for in-order, equal-time, late, duplicate, independent-stream, watermark-race, and correction events in `src-tauri/tests/work_tracking_projector.rs`
- [X] T009 Implement additive idempotent work-tracking and publication migrations with required unique keys and foreign keys in `src-tauri/src/native/schema.sql` and `src-tauri/src/native/migrations.rs`
- [X] T010 Define immutable event, trusted order key, stream watermark, projection status, decision, scope, error, and publication state types in `src-tauri/src/domain/work_tracking_state.rs` and `src-tauri/src/domain/knowledge_publication.rs`
- [X] T011 Define `EventLog`, `WorkflowRepository`, `WorkProjection`, `VaultRepository`, `Clock`, `ProjectorWakeup`, and transaction/unit-of-work ports in `src-tauri/src/ports/event_log.rs`, `src-tauri/src/ports/workflow_repository.rs`, `src-tauri/src/ports/work_projection.rs`, and `src-tauri/src/ports/vault_repository.rs`
- [X] T012 Implement the SQLite unit-of-work, event log, idempotency, session-head CAS, outbox, decision, and watermark adapters in `src-tauri/src/adapters/sqlite/mod.rs`
- [X] T013 Refactor existing workflow mutations to accept the shared transaction/repository boundary instead of opening independent connections in `src-tauri/src/native/workflow.rs`, `src-tauri/src/native/workbench.rs`, and `src-tauri/src/native/completion.rs`
- [X] T014 Implement the sole `WorkTrackingApplicationService` command/query boundary with scope checks, trusted `occurred_at`, canonical request hashes, and durable append-before-acknowledgment in `src-tauri/src/application/work_tracking_service.rs`
- [X] T015 Implement the crash-safe leased projector with startup drain, bounded retry, per-stream watermark CAS, and `applied/ignored_late/conflict/failed` results in `src-tauri/src/native/work_tracking_projector.rs`
- [X] T016 Compose one service, SQLite adapter, Vault adapter, and supervised projector for both Tauri and `--mcp` modes in `src-tauri/src/lib.rs`
- [X] T017 Add content-free structured activity logging and safe public error mapping in `src-tauri/src/application/work_tracking_service.rs` and `src-tauri/src/native/job_results.rs`
- [X] T018 Make T006–T008 pass and run the boundary verifier, `cargo test --manifest-path src-tauri/Cargo.toml`, and `git diff --check`

**Checkpoint**: All accepted work enters one durable event log; adapters cannot bypass the
application service; projection is restart-safe and deterministic.

---

## Phase 3: User Story 1 — Start Tracking Without Leaving Chat (Priority: P1) 🎯 MVP

**Goal**: Create or resume one consented tracked session and Capture from either in-app Chat or a
local MCP Chat without opening Workbench.

**Independent Test**: In each Chat interface, approve one Capture, edit/adopt one Problem, and
adopt one Solution; verify one lineage, exact decisions, and no duplicate records on retry.

### Tests for User Story 1

- [X] T019 [P] [US1] Add failing MCP stdio contract tests for initialize, connection ownership, `inbound_work_open`, resume, closed schemas, and idempotent replay in `src-tauri/tests/mcp_stdio.rs`
- [X] T020 [P] [US1] Add failing native command tests for Capture preview/accept/edit/reject and Problem/Solution milestone actions in `src-tauri/tests/application_commands.rs`
- [X] T021 [P] [US1] Add failing in-app Chat card interaction and accessibility tests in `frontend/src/features/chat/WorkTrackingCards.test.tsx`

### Implementation for User Story 1

- [X] T022 [US1] Implement stdio lifecycle, connection resolution, strict tool schemas, and safe errors in `src-tauri/src/mcp.rs`
- [X] T023 [US1] Implement `inbound_work_open` and governed Problem/Solution actions as application-service DTO mappings only in `src-tauri/src/mcp.rs`
- [X] T024 [US1] Implement equivalent allowlisted in-app work-session commands and bind decisions to window/chat/event/payload/revision in `src-tauri/src/native/work_tracking.rs` and `src-tauri/src/lib.rs`
- [X] T025 [P] [US1] Add shared frontend work-session/card types and client calls in `frontend/src/types/application.ts`, `frontend/src/services/applicationClient.ts`, and `frontend/src/services/tauriApplicationClient.ts`
- [X] T026 [US1] Add inline Capture, Problem, and Solution milestone cards without replacing Chat streaming/history in `frontend/src/features/chat/WorkTrackingCards.tsx` and `frontend/src/features/overlays/OverlayLayer.tsx`
- [X] T027 [US1] Make T019–T021 pass and verify the complete no-Workbench start flow through both Chat adapters using `src-tauri/tests/mcp_stdio.rs`

**Checkpoint**: The same consented Capture→Problem→Solution start flow works in both Chats.

---

## Phase 4: User Story 2 — Track and Resume Work in Chat (Priority: P1)

**Goal**: Append durable checkpoints quickly, project them in the background, and resume from
bounded current state after restart or context loss.

**Independent Test**: Append ordered and late checkpoints, restart app/host/projector, then resume
the same session with current Problem, Solution, accepted progress, blockers, projection status,
and next decision.

### Tests for User Story 2

- [X] T028 [P] [US2] Add failing append/resource contract tests for `accepted/queued`, projection status, bounded resume state, corrections, and closed sessions in `src-tauri/tests/mcp_stdio.rs`
- [X] T029 [P] [US2] Add failing crash-point and dual-worker tests for claim recovery and exactly-once Work Log materialization in `src-tauri/tests/work_tracking_projector.rs`
- [X] T030 [P] [US2] Add failing checkpoint/restart tests for the in-app Chat path in `src-tauri/tests/application_commands.rs`

### Implementation for User Story 2

- [X] T031 [US2] Implement `inbound_work_append` with bounded variants, provenance validation, head CAS, and durable queued response in `src-tauri/src/mcp.rs` and `src-tauri/src/application/work_tracking_service.rs`
- [X] T032 [US2] Implement accepted-checkpoint materialization and unique provenance links through the shared workflow repository in `src-tauri/src/native/work_tracking_projector.rs` and `src-tauri/src/native/lineage.rs`
- [X] T033 [US2] Implement the bounded owned-session resource with projection freshness, recent events, accepted evidence, and next actions in `src-tauri/src/application/work_tracking_service.rs` and `src-tauri/src/mcp.rs`
- [X] T034 [P] [US2] Add checkpoint and resume client methods/types in `frontend/src/services/tauriApplicationClient.ts` and `frontend/src/types/application.ts`
- [X] T035 [US2] Add concise checkpoint confirmation, queued/projected state, and resume brief rendering in `frontend/src/features/chat/WorkTrackingCards.tsx`
- [X] T036 [US2] Add startup projector drain and graceful shutdown/lease release behavior in `src-tauri/src/lib.rs` and `src-tauri/src/native/work_tracking_projector.rs`
- [X] T037 [US2] Make T028–T030 pass and benchmark durable append p95 under 50 ms and 95% projection visibility within 500 ms in `src-tauri/tests/work_tracking_projector.rs`

**Checkpoint**: A Chat can be closed and resumed from durable, source-labeled, eventually projected
state without a transcript.

---

## Phase 5: User Story 3 — Complete, Then Choose Whether to Publish (Priority: P1)

**Goal**: Produce Completed Work first, then optionally save and publish one exact Knowledge draft
through a separate decision and permission.

**Independent Test**: Complete work with accepted evidence, verify no Knowledge file exists, defer
the publication offer, then later save/review/publish an exact draft; repeat with external Vault
change and confirm no overwrite.

### Tests for User Story 3

- [X] T038 [P] [US3] Add failing service tests proving completion has no Knowledge side effect and publication has separate state, decision, scope, revision, hash, and idempotency in `src-tauri/tests/work_tracking_service.rs`
- [X] T039 [P] [US3] Add failing MCP contracts for `knowledge_draft_save`, `knowledge_publish`, denied scope, stale evidence, and external-file conflict in `src-tauri/tests/mcp_stdio.rs`
- [X] T040 [P] [US3] Add failing Chat tests for one publication offer per completion revision, defer/no-nag, draft preview, and later publish in `frontend/src/features/chat/WorkTrackingCards.test.tsx`

### Implementation for User Story 3

- [X] T041 [US3] Split completion from Knowledge draft/publication in the domain and existing completion transaction in `src-tauri/src/domain/knowledge_publication.rs` and `src-tauri/src/native/completion.rs`
- [X] T042 [US3] Implement append-versioned Knowledge draft save and exact-draft publication commands through the Vault port in `src-tauri/src/application/work_tracking_service.rs` and `src-tauri/src/adapters/vault/mod.rs`
- [X] T043 [US3] Implement `knowledge_draft_save` and `knowledge_publish` tool mappings with distinct scopes and no content edits during publish in `src-tauri/src/mcp.rs`
- [X] T044 [P] [US3] Add native Knowledge draft/publication commands and frontend client types in `src-tauri/src/native/work_tracking.rs`, `frontend/src/types/application.ts`, and `frontend/src/services/tauriApplicationClient.ts`
- [X] T045 [US3] Add completion result, one-time publication offer, defer state, draft preview, and explicit publish controls in `frontend/src/features/chat/WorkTrackingCards.tsx`
- [X] T046 [US3] Update the bundled Skill's completion/publish behavior and packaging metadata in `.agents/skills/llm-wiki-workflow/SKILL.md` and `.agents/skills/llm-wiki-workflow/agents/openai.yaml`
- [X] T047 [US3] Make T038–T040 pass and verify exact draft revision/hash publication plus atomic reversible Vault conflict handling in `src-tauri/tests/work_tracking_service.rs`

**Checkpoint**: Completion is private and durable; Knowledge exists only after a second exact
publication decision.

---

## Phase 6: User Story 4 — Support Two Chat Input Interfaces (Priority: P1)

**Goal**: Preserve existing in-app Chat while providing equivalent external MCP behavior over the
same intents, gates, and projection semantics.

**Independent Test**: Replay equivalent accepted inputs through both Chat adapters and compare
normalized workflow, decision, Work Log, completion, draft, and publication records.

### Tests for User Story 4

- [X] T048 [P] [US4] Add a cross-adapter canonical parity suite covering start, checkpoint, conflict, completion, defer, and publication in `src-tauri/tests/work_tracking_parity.rs`
- [X] T049 [P] [US4] Add regression tests for existing composer, history, streaming, cancellation, context routing, and completed-work Chat in `frontend/src/app/App.test.tsx` and `frontend/src/services/tauriApplicationClient.test.ts`

### Implementation for User Story 4

- [X] T050 [US4] Consolidate MCP and native request DTO conversion onto shared application commands without adapter-specific domain branching in `src-tauri/src/mcp.rs` and `src-tauri/src/native/work_tracking.rs`
- [X] T051 [US4] Bind AI task results to model identity, source/session revision, context scope, and retrieval snapshot without granting workflow authority in `src-tauri/src/conversation.rs` and `src-tauri/src/native/conversation_context.rs`
- [X] T052 [US4] Integrate inline work cards with the existing Chat Channel/cancellation lifecycle in `frontend/src/features/overlays/OverlayLayer.tsx` and `frontend/src/services/tauriApplicationClient.ts`
- [X] T053 [P] [US4] Add English/Korean Chat, connection, scope, projection, and publication strings using `사용자` in `frontend/public/i18n/en.json` and `frontend/public/i18n/ko.json`
- [X] T054 [US4] Add supported Codex/ChatGPT desktop connection setup, scope disclosure, and revocation UI in `frontend/src/features/settings/SettingsView.tsx`
- [X] T055 [US4] Make T048–T049 pass with zero normalized semantic differences other than allowed provenance and timing fields in `src-tauri/tests/work_tracking_parity.rs`

**Checkpoint**: In-app Chat and external MCP Chat are two inputs to one system, not synchronized
copies.

---

## Phase 7: User Story 5 — Move Between Chat and Workbench (Priority: P1)

**Goal**: Open Workbench at any time, see and edit the same current session, then continue in either
Chat without lost input or duplicate decisions.

**Independent Test**: Start in Chat, edit/approve in Workbench, return to Chat, and exercise a
simultaneous stale save; verify convergence, preserved UI input, and safe retry/review behavior.

### Tests for User Story 5

- [X] T056 [P] [US5] Add failing native concurrency tests for session/entity CAS, stale challenges, refresh-first retry, and overlapping edit rejection in `src-tauri/tests/work_tracking_concurrency.rs`
- [X] T057 [P] [US5] Add failing Workbench refresh tests preserving draft, IME, focus, caret, selection, scroll, disclosures, and streaming output in `frontend/src/features/workbench/WorkbenchView.test.tsx`

### Implementation for User Story 5

- [X] T058 [US5] Route Workbench read/edit/review/progress/conflict/completion actions through `WorkTrackingApplicationService` in `src-tauri/src/native/workbench.rs` and `src-tauri/src/native/work_tracking.rs`
- [X] T059 [US5] Return field-level bounded conflict metadata and separate session, entity, watermark, and projection revisions in `src-tauri/src/application/work_tracking_service.rs`
- [X] T060 [P] [US5] Add incremental refresh, conflict, and retry result types/client methods in `frontend/src/types/application.ts` and `frontend/src/services/tauriApplicationClient.ts`
- [X] T061 [US5] Implement focus/visibility/post-write/bounded-period Workbench refresh and one-shot unchanged-intent retry in `frontend/src/features/workbench/WorkbenchView.tsx`
- [X] T062 [US5] Implement natural pre-read/pre-milestone/pre-save Chat refresh without remounting conversational UI in `frontend/src/features/overlays/OverlayLayer.tsx`
- [X] T063 [US5] Make T056–T057 pass and verify Chat→Workbench→Chat plus Workbench→Chat→Workbench scenarios in `frontend/src/features/workbench/WorkbenchView.test.tsx`

**Checkpoint**: Workbench is optional but fully active over the same authoritative state.

---

## Phase 8: User Story 6 — Resume Through the Representative Skill (Priority: P1)

**Goal**: Let ChatGPT desktop or Codex retrieve the smallest useful Workbench context and present a
natural current-work or complete overview without MCP-shaped output.

**Independent Test**: Invoke the Skill with explicit session, selected current item, ambiguous
items, topic request, whole-Workbench request, stale snapshot, and revoked scopes.

### Tests for User Story 6

- [X] T064 [P] [US6] Add failing resource tests for session, topic, current Workbench, and stable paginated overview scope isolation in `src-tauri/tests/mcp_stdio.rs`
- [X] T065 [P] [US6] Add Skill behavior fixtures for concise resume, ambiguous selection, one-question flow, complete overview, and hidden technical identifiers in `.agents/skills/llm-wiki-workflow/tests/workflow-cases.md`

### Implementation for User Story 6

- [X] T066 [US6] Implement bounded session/topic/current/overview application queries with private cache metadata and snapshot revision cursors in `src-tauri/src/application/work_tracking_service.rs`
- [X] T067 [US6] Expose the four scoped resource families without subscriptions or list-change claims in `src-tauri/src/mcp.rs`
- [X] T068 [US6] Finalize minimal-scope selection and conversational resume rules in `.agents/skills/llm-wiki-workflow/SKILL.md`
- [X] T069 [US6] Finalize stable whole-Workbench pagination and presentation rules in `.agents/skills/llm-wiki-workflow/references/workbench-overview.md`
- [X] T070 [US6] Make T064 pass, run the Skill validator, and manually verify both hosts against `.agents/skills/llm-wiki-workflow/tests/workflow-cases.md`

**Checkpoint**: Ordinary continuation is cheap and conversational; whole-Workbench coverage is
complete only when explicitly requested.

---

## Phase 9: User Story 7 — Review Conflicts with the Current Session AI (Priority: P1)

**Goal**: Give the current Chat AI bounded lexical and semantic Vault evidence for cited conflict
review without invoking a hidden server-side model.

**Independent Test**: Review one topic from each Chat interface with lexical and semantic evidence,
then repeat with semantic lag, no candidates, stale citations, prompt-injection text, and
out-of-scope canaries.

### Tests for User Story 7

- [X] T071 [P] [US7] Add failing MCP search/evidence contract tests for limits, scopes, cursor binding, revisions, semantic lag, and non-disclosing errors in `src-tauri/tests/mcp_stdio.rs`
- [X] T072 [P] [US7] Add failing Vault adapter tests for lexical fallback, semantic source/index revision, exact evidence reads, and citation validation in `src-tauri/tests/work_tracking_search.rs`
- [X] T073 [P] [US7] Add failing in-app current-session AI tests proving zero hidden model calls and rejecting fabricated/stale citations in `src-tauri/tests/application_commands.rs`

### Implementation for User Story 7

- [X] T074 [US7] Implement scoped lexical search, semantic search, and bounded evidence-read ports over existing Vault/index services in `src-tauri/src/adapters/vault/mod.rs`, `src-tauri/src/native/vault.rs`, and `src-tauri/src/native/semantic.rs`
- [X] T075 [US7] Implement search scope authorization, private cursor binding, semantic freshness errors, evidence revision/hash validation, and response caps in `src-tauri/src/application/work_tracking_service.rs`
- [X] T076 [US7] Expose `vault_search_lexical`, `vault_search_semantic`, and `vault_evidence_read` as application-service-only MCP tools in `src-tauri/src/mcp.rs`
- [X] T077 [P] [US7] Add equivalent native Chat search/evidence commands and typed client methods in `src-tauri/src/native/work_tracking.rs`, `frontend/src/types/application.ts`, and `frontend/src/services/tauriApplicationClient.ts`
- [X] T078 [US7] Pass scoped evidence to the current Chat provider and validate structured citations without changing provider selection in `src-tauri/src/conversation.rs` and `src-tauri/src/native/conversation_context.rs`
- [X] T079 [US7] Finalize lexical-plus-semantic comparison, insufficient-evidence, prompt-injection, and user-decision guidance in `.agents/skills/llm-wiki-workflow/references/conflict-review.md`
- [X] T080 [US7] Make T071–T073 pass with zero scope-canary leakage, zero fabricated citations stored, and zero server-side conflict-review model calls in `src-tauri/tests/work_tracking_search.rs`

**Checkpoint**: The current-session AI can perform cited, conservative conflict review using
portable Knowledge without gaining workflow authority.

---

## Phase 10: Polish and Cross-Cutting Verification

**Purpose**: Close performance, security, documentation, cross-platform, and packaging gates after
all selected user stories are complete.

- [X] T081 [P] Add randomized 10,000-event replay/out-of-order determinism coverage and projection-lag metrics in `src-tauri/tests/work_tracking_projector.rs`
- [X] T082 [P] Add permission canaries, malformed/oversized input, revocation race, cancellation, replay, and content-free logging coverage in `src-tauri/tests/mcp_stdio.rs`
- [X] T083 [P] Add full external-Chat, in-app-Chat, and Chat↔Workbench scenarios to `frontend/src/test/desktopScenario.ts` and `src-tauri/src/desktop_e2e.rs`
- [X] T084 Package the representative Skill and local MCP connection metadata for Codex and ChatGPT desktop in `.codex-plugin/plugin.json` and `.agents/skills/llm-wiki-workflow/agents/openai.yaml`
- [X] T085 Add user-facing English/Korean MCP setup and workflow documentation linked to this spec in `docs/features/mcp-workbench-bridge.md`, `docs/features/mcp-workbench-bridge.ko.md`, `docs/features/README.md`, and `docs/features/README.ko.md`
- [X] T086 Run `npm test`, `npm run typecheck`, `npm run lint`, `cargo test --manifest-path src-tauri/Cargo.toml`, the MCP boundary/conformance checks, and `git diff --check` as separate commands
- [X] T087 Run the release build once with `npm run tauri:build`, then run packaged E2E once with `npm run test:desktop` on macOS and record Windows verification expectations in `specs/011-mcp-workbench-bridge/quickstart.md`
- [X] T088 Review `docs/DOCUMENTATION_GUIDE.md`, reconcile final behavior across all feature/spec/Skill documents, and record any remaining risks or intentionally deferred ChatGPT-web work in `docs/CONTINUATION.md`

---

## Dependencies and Execution Order

### Phase dependencies

- **Phase 1 — Setup**: Starts immediately.
- **Phase 2 — Foundation**: Depends on Phase 1 and blocks every user story.
- **US1**: Starts after Phase 2 and is the first independently demonstrable slice.
- **US2**: Depends on US1 session ownership and append contracts.
- **US3**: Depends on US2 accepted evidence and projection status.
- **US4**: Depends on US1–US3 semantics so parity compares the complete lifecycle.
- **US5**: Depends on US4's shared adapter semantics and session/entity revisions.
- **US6**: Depends on US1 session identity and US2 projections; overview work can run in parallel
  with US3–US5 after those prerequisites.
- **US7**: Depends on Phase 2 ports and scope model; it can run in parallel with US2–US6 after the
  current-session query contract is stable.
- **Polish**: Depends on every story selected for Phase 1 release.

### User-story graph

```text
Setup → Foundation → US1 → US2 → US3 → US4 → US5
                         ├──────────────→ US6
                Foundation ────────────→ US7
US1–US7 → Polish
```

### Within each story

- Write the story's tests first and verify the expected failure.
- Implement domain/service behavior before exposing MCP/native/UI adapters.
- Keep MCP and UI handlers as DTO/transport adapters; never add direct persistence access.
- Finish the independent test before starting a dependent story.

## Parallel Opportunities

- T003 and T004 can run in parallel after module naming is fixed.
- T006–T008 cover independent migration, service, and projector concerns.
- Within each story, `[P]` contract/native/frontend tests can be authored concurrently.
- US6 resource/Skill work and US7 search/evidence work can proceed in parallel after US1/Phase 2.
- Cross-cutting randomized, security, and desktop scenario tests T081–T083 touch different files.

## Parallel Examples

### User Story 2

```text
T028: MCP append/resource contract tests in src-tauri/tests/mcp_stdio.rs
T029: projector crash/exactly-once tests in src-tauri/tests/work_tracking_projector.rs
T030: in-app checkpoint/restart tests in src-tauri/tests/application_commands.rs
```

### User Story 7

```text
T071: MCP search/evidence contracts in src-tauri/tests/mcp_stdio.rs
T072: Vault/search adapter tests in src-tauri/tests/work_tracking_search.rs
T073: current-session AI/citation tests in src-tauri/tests/application_commands.rs
```

## Implementation Strategy

### MVP first

1. Complete Setup and Foundation.
2. Complete US1 and demonstrate one consented Capture→Problem→Solution flow in each Chat.
3. Stop and validate the shared application boundary before adding background progress or Knowledge.

### Phase 1 release increment

1. Add US2 for durable checkpointing/resume.
2. Add US3 for separate completion and publication.
3. Add US4 and US5 for input parity and Workbench switching.
4. Add US6 and US7 for context-efficient Skill behavior and current-session conflict review.
5. Complete all Polish gates once, including the release build and packaged E2E.

### Commit boundaries

- Use one English gitmoji commit for the completed Phase 1 release branch after tasks are squashed,
  following `.agents/skills/commit-convention/SKILL.md` when committing.
- Do not commit generated caches, shared worktree links, temporary test data, Vault fixtures with
  personal content, or provider transcripts.

## Notes

- Every task uses an exact repository path and is intended to be executable without reopening the
  architecture decision.
- `occurred_at` is trusted application/approved-adapter time; caller/model timestamps are only
  descriptive `observed_at` provenance.
- Late events remain in the immutable audit log but never change current projection, Lineage, Work
  Log, completion evidence, or watermark.
- A durable event append is synchronous; projection/materialization is background work.
- Whole-Workbench scope is explicit and does not become the silent default for later Chat turns.
- ChatGPT web, remote HTTP/OAuth, team sync, arbitrary Vault writes, and transcript synchronization
  remain outside Phase 1.

---

## Phase 11: Convergence

**Purpose**: Close the user-scenario gaps found by revalidating every acceptance scenario against
the shipped UI, MCP stdio path, shared application service, persistence behavior, and packaged E2E.

- [X] T089 CRITICAL Change external and in-app session opening to persist only a server-issued Capture preview until an exact accept/edit/reject decision creates the Capture and tracked session per US1/AC1 and FR-002/FR-004
- [X] T090 CRITICAL Wire exact-payload Capture, Problem, Solution, checkpoint, conflict, and completion cards to real in-app Chat state and callbacks; support accept/edit/reject and bind native decisions to window, Chat session, source event, payload hash, and revision per US1/AC2-3 and FR-006/FR-007/FR-023
- [X] T091 CRITICAL Enforce accepted completion-proposal, existing completion checks, selected fresh accepted evidence, and complete Capture→Problem→Solution→completion Lineage before `verify_and_complete` can close work per US3/AC3 and FR-013/FR-015
- [X] T092 CRITICAL Put Knowledge draft save and exact-draft publication behind distinct MCP Elicitation or native inline decisions, reject direct model-only publication calls, and verify the one-time post-completion offer per US3/AC4-6 and FR-041/FR-042
- [X] T093 Correct native versus external source-interface/session/idempotency provenance and expand canonical parity coverage through Capture, Problem, Solution, checkpoints, conflict, completion, deferral, and publication per US4/AC3 and FR-024/FR-046
- [X] T094 CRITICAL Route Workbench tracked-state rendering and every linked edit/review/progress/conflict/completion/follow-up action through the shared tracked session head, showing provenance, recent checkpoints, pending decisions, and already-completed decisions per US4/AC4, US5, and FR-025/FR-026/FR-028
- [X] T095 Implement end-to-end stale-save recovery that preserves unsaved Chat and Workbench input, refreshes bounded current state, compares material fields, retries once only for unchanged non-overlapping intent, and otherwise requests renewed review per US5/AC3 and FR-027/FR-050/FR-051
- [X] T096 Add a real active-selection source, enrich current projection with linked Problem/Solution/progress/blockers/next decision, complete overview attention/active/recent-completion coverage across stable pages, and automate representative Skill behavior cases per US6 and FR-033–FR-039
- [X] T097 Bind cross-interface `link_current_work` challenges to exact workspace and entity revisions in addition to the session revision, and reject stale or handle-only linking per US6/AC4 and FR-035
- [X] T098 CRITICAL Replace the hidden server-side conflict-review model job with an orchestrated current-Chat lexical-plus-semantic evidence flow for both Chat interfaces, including cited agreement/contradiction/missing/inference output and explicit user resolution per US7/AC1-2 and FR-043
- [X] T099 Replace topic substring filtering with authoritative topic membership, compute truncation only after scope filtering, and return required source identity, passage location, match/score metadata, and content revision without scope-canary leakage per US7/AC3 and FR-044
- [X] T100 Preserve and render checkpoint decisions, evidence, artifacts, validation results, origin, and timestamps in the existing Solution Work Log while keeping pre-Solution progress pending per US2/AC1 and FR-010/FR-012
- [X] T101 Add executable coverage and state handling for challenge expiry, cancellation, replay/edit, cross-principal binding, revocation races, topic changes, two-connection identity collisions, database failure, supersession, pending projection, and content-free activity logs for every operation per FR-014/FR-019/FR-020/FR-029 and the specified edge cases
- [X] T102 CRITICAL Make reviewed Knowledge publication crash-consistent across Vault file creation and publication-decision persistence, with recoverable compensation/undo and external-change protection per Constitution D and FR-041/FR-045
- [X] T103 Route all new Chat, connection, scope, projection, error, provenance, and publication UI copy through English/Korean localization resources and retain `사용자` terminology per FR-031
- [ ] T104 Run equivalent packaged Windows acceptance coverage for external MCP Chat, in-app Chat, Capture/draft/checkpoint/completion, restart, and connection revocation, and record evidence beside the macOS result per SC-011 (missing)
- [X] T105 Make the active GUI process the sole application-service and persistence owner by routing the stdio MCP child through protected Unix-domain-socket/Windows-named-pipe IPC, failing closed without the GUI, and proving the child ignores DB/Vault environment paths
