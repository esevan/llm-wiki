# Implementation Plan: Dual-Chat Work Tracking

**Branch**: `feat/mcp-support` | **Date**: 2026-09-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/011-mcp-workbench-bridge/spec.md`

## Summary

Support two first-class conversational inputs over one work-tracking application service: LLM Wiki's in-app Chat and local external Chat hosts connected through MCP. Both can turn a conversation into Capture, Problem, Solution, checkpoints, and Completed Work without opening Workbench; Workbench remains an optional active structured workspace over the same event stream. The current-session AI performs conflict review and other synthesis with separately scoped lexical and semantic Vault retrieval. Completion creates private Completed Work, then the `llm-wiki-workflow` Skill separately offers Knowledge publication. A durable append-only event log acknowledges checkpoints quickly while an idempotent background projector updates workflow, Workbench, and search views. Phase 1 supports Codex and ChatGPT desktop over stdio and excludes ChatGPT web.

## Technical Context

**Language/Version**: Rust 2021 edition for MCP/domain work; TypeScript 5.9, React 19, and bounded compatibility controllers for Chat/Workbench UI changes

**Primary Dependencies**: Existing Tauri 2.8, Tokio with local socket/named-pipe support, rusqlite, serde/serde_json, Tauri Channel/futures-based Chat streaming; add the official `rmcp` 3.x Rust SDK with minimal server/stdio/schema/Elicitation features

**Storage**: One persistence boundary behind application ports: SQLite WAL adapters for the immutable event log, projection cursors, workflow/session projections, connections, proposals, links, and idempotency; `MarkdownVaultAdapter` for bounded search/evidence and reviewed Knowledge draft/publication writes. MCP imports neither SQLite nor Vault adapters directly.

**Testing**: Vitest, runtime structural checks, Rust unit/command/stdio tests, official MCP Inspector CLI, pinned MCP conformance suite, packaged desktop E2E

**Target Platform**: Local packaged Tauri app on macOS and Windows; external MCP host launches the installed executable as a subprocess

**Project Type**: Native desktop application whose GUI process owns one Rust application/persistence layer, with Tauri UI adapters and a persistence-free stdio-to-local-IPC MCP bridge

**Performance Goals**: Existing readiness under 1.5 seconds and durable append p95 under 50 ms remain binding; 95% of accepted checkpoint events become visible in projections within 500 ms; warm lexical search stays under 75 ms; hybrid context contains at most eight passages and 6,000 retrieved-context tokens; MCP-enabled idle regression stays under 15%

**Constraints**: Explicit per-session tracking consent; least-privilege session/topic/current/overview/search/evidence/draft/publish scopes; fast current projection plus stable paginated whole-Workbench overview; MCP 2026-07-28 Elicitation for governed external transitions; explicit native action for in-app/Workbench transitions; the running GUI is the sole application-service and persistence owner; the MCP stdio child only forwards over protected local IPC; authenticated, replay-safe request state; no model-supplied authority; no full transcript or unbounded Vault projection; durable append before acknowledgment; background idempotent projection; trusted occurrence time and per-stream watermark; compare-and-swap revision; refresh-before-safe-retry; completion and publication separated; no generic native-operation tunnel; no TCP/HTTP MCP transport

**Scale/Scope**: One local user and Vault; concurrent Workbench, in-app Chat, and multiple local MCP processes; one lineage branch per opted-in conversation; Codex and ChatGPT desktop in Phase 1; ChatGPT web/remote HTTP/OAuth deferred; English/Korean copy

## Constitution Check

*GATE: Passed before research and re-checked after Phase 1 design.*

- **I. Conversation-first — PASS**: Natural conversation can happen in LLM Wiki or any supported external host; each interface records the same useful work trace.
- **II. Reduce Cognitive Load — PASS**: Exact decisions appear inline in whichever Chat the user chose; no required Workbench navigation or manual copy/paste remains.
- **III. Resume Where You Left Off — PASS**: The Chat refreshes ordered checkpoints and uses bounded lexical plus semantic evidence through the current-session AI, including conservative `insufficient_evidence` handling.
- **IV. Problems, not tasks — PASS**: Tracked Work Session is conversation provenance, not a workflow stage. User-adopted records retain the existing Capture→Problem→Solution→Complete lineage.
- **V. Private Process, Portable Knowledge — PASS**: Completion remains private Completed Work. Only a second, explicit publication decision writes portable Knowledge; raw external conversation is never published.
- **VI. Never Score the Worker — PASS**: Connection/activity data describes work and provenance, never the person.
- **A. Measured Performance — PASS**: Durable log append is the synchronous hot path; projection is background and AI remains outside writes. Readiness, projection lag, search, context, and idle budgets are gates.
- **B. Independent Adapters — PASS**: MCP, in-app Chat, and Workbench call one application service. That service owns workflow/event ports; only adapters know SQLite or Markdown Vault details.
- **C. Human Authority — PASS**: Model proposals carry no authority. External Chat uses exact-payload Elicitation; in-app Chat and Workbench require explicit user actions. All then pass the same existing domain gates.
- **D. Evidence and Consistency — PASS**: Immutable events carry provenance and hashes; revisions reject stale writes; only accepted evidence from either interface can enter Lineage or Completed Work.
- **E. Local and Cross-platform — PASS**: the host-facing transport is stdio; protected Unix domain socket/named-pipe IPC reaches the active GUI without TCP/HTTP. The same signed native executable serves macOS and Windows, while only the GUI process owns SQLite WAL.
- **F. Minimal Complexity — PASS WITH JUSTIFICATION**: Three core work tools separate session open,
  immutable append, and governed advancement; retrieval and Knowledge tools remain separate only
  where their scopes, freshness, or authority differ. In-app Chat reuses the same application
  intents rather than duplicating workflow logic.

## Project Structure

### Documentation (this feature)

```text
specs/011-mcp-workbench-bridge/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── user-flow.md
├── quickstart.md
├── checklists/requirements.md
└── contracts/
    ├── mcp-server.md
    └── native-connection-api.md

.agents/skills/llm-wiki-workflow/
├── SKILL.md
├── agents/openai.yaml
└── references/
    ├── conflict-review.md
    └── workbench-overview.md
```

### Source Code (repository root)

```text
frontend/src/
├── features/overlays/OverlayLayer.tsx       # keep Chat and add inline work review cards
├── features/chat/WorkTrackingCards.tsx      # shared in-app milestone/decision UI
├── features/settings/SettingsView.tsx       # connection setup/revocation
├── features/workbench/ExternalWorkReview.tsx # active shared-session editor/review
├── services/{applicationClient,tauriApplicationClient}.ts
└── types/{application,mcp}.ts

frontend/public/runtime/
├── explore.js                               # retain streaming Chat and Draft/Refine review
├── completed-workspace.js                   # retain completed-work Chat and details
├── solution-work.js                         # existing non-chat detail and Work Log
├── workbench.js
├── jobs.js
└── foundation.js

frontend/public/i18n/{en,ko}.json

src-tauri/src/
├── main.rs                                  # Tauri or --mcp mode
├── lib.rs                                   # composition root and native handlers
├── mcp.rs                                   # stdio/schema/Elicitation adapter; no SQL/Vault access
├── application/
│   └── work_tracking_service.rs             # sole workflow/search/write use-case boundary
├── domain/
│   ├── work_tracking_state.rs               # event ordering, gates, state transitions
│   └── knowledge_publication.rs             # completion/publish separation
├── ports/
│   ├── event_log.rs
│   ├── workflow_repository.rs
│   ├── vault_repository.rs
│   └── work_projection.rs
├── adapters/
│   ├── sqlite/                              # log, cursors, workflow projections
│   └── vault/                               # MarkdownVaultAdapter-backed search/write
└── native/
    ├── work_tracking.rs                     # Tauri adapter to application service
    ├── work_tracking_projector.rs           # supervised background projector
    ├── workflow.rs                          # existing entity/Work Log integration
    ├── refinement.rs                        # current item + jobs + accepted external evidence
    ├── lineage.rs                           # accepted external evidence projection
    ├── completion.rs                        # provenance in completed evidence
    ├── migrations.rs
    └── schema.sql

src-tauri/tests/
├── application_commands.rs
├── work_tracking.rs
└── mcp_stdio.rs

scripts/verify_mcp_conformance.mjs
```

Retained implementation paths include `conversation.rs`, `native/conversation_context.rs`, the
frontend Channel/cancellation adapter, chat composer/history, and completed-work Chat routing.
Existing `ai_runs`/`workflow_chat` history remains available to in-app Chat but is not exposed through
MCP or automatically projected into workflow records.

**Structure Decision**: Keep MCP wire/Elicitation handling in `mcp.rs`, but prohibit it from importing `rusqlite`, SQL/schema modules, filesystem paths, or Vault adapters. MCP and native handlers map requests to `application::work_tracking_service`; domain state machines depend only on ports. SQLite and Markdown implementations live behind adapters composed in `lib.rs`. The same service appends authoritative events, reads projections, performs scoped retrieval, and invokes controlled draft/publication operations. Both Chat interfaces are primary inputs; Workbench is an optional active structured interface over the same session.

## Delivery Sequence

1. **Application boundary and event foundation**: Define typed ports and one work-tracking service; add connection/session/event/idempotency models, trusted occurrence ordering, stream watermark, durable append, and projector cursor/status migrations. Prove MCP/native modules cannot bypass the service.
2. **Background projection and concurrency**: Add a supervised idempotent projector with restart recovery, `queued/applied/ignored_late/failed` status, CAS heads, and deterministic out-of-order tests. Add incremental read APIs and one-shot safe retry for unchanged non-overlapping intents.
3. **Scoped retrieval and Knowledge boundary**: Route lexical and semantic Vault search, bounded evidence reads, reviewed Knowledge draft save, and publication through application ports. Keep current-session AI responsible for synthesis and validate returned citations. Split completion from publication state/events.
4. **MCP transport and Skill**: Add local stdio mode for Codex/ChatGPT desktop, work tools/resources, context-aware scopes, search/evidence/draft/publish tools, Elicitation, signed request state, capability failure, cancellation, and revocation. Validate the Skill's conflict-review and post-completion publication offer.
5. **Native Chat, Workbench, and packaging**: Add equivalent in-app cards and passive incremental refresh without disturbing input/focus/scroll/streaming. Keep Workbench directly editable on the same projections. Finish English/Korean copy, architecture tests, parity/load/restart tests, release build, and packaged E2E.

## Complexity Tracking

| Choice | Why Needed | Simpler Alternative Rejected Because |
| --- | --- | --- |
| Official Rust MCP SDK | Current/legacy protocol lifecycles, stdio framing, schema, cancellation, and conformance are security-sensitive | Hand-written JSON-RPC creates more custom protocol code; TypeScript adds another packaged runtime |
| Append-only Tracked Work Session model | Retries, out-of-order events, provenance, review, and restart-safe resumption from either Chat cannot safely mutate workflow tables directly | Calling existing generic workflow CRUD loses consent, idempotency, source trust, and stale-write protection |
| Separate governed advancement tool | Preserves human authority while allowing the lifecycle to finish in the current Chat | A generic append tool obscures state changes; Workbench-only review forces the context switch the feature is meant to eliminate |
| Signed single-use Elicitation state | A Chat/model can retry or alter tool input, so the server must prove which exact user-reviewed action is being applied | Caller-provided `confirmed:true` and tool annotations are not consent or authorization |
| Two input adapters over one domain service | In-app and external Chat solve different entry contexts but must produce identical work semantics | Removing in-app Chat discards a useful native interface; duplicating workflow logic would cause drift |
| Active Workbench on the same session head | Users may prefer structured overview/editing midway through a conversation | A read-only mirror prevents continuation; a separate Workbench copy creates synchronization and duplication problems |
| Fast current view plus paginated overview | Ordinary resume should be cheap, while an explicit whole-Workbench request needs complete, revision-consistent coverage | Always loading the whole board wastes context; returning only top items makes “전체” misleading |
| Chat-friendly Skill | Agents must translate structured state into concise dialogue and clean full-board summaries | Tool schemas alone produce mechanical dumps and do not teach safe conversational continuation |
| Durable log plus background projector | Fast accepted checkpoints, crash safety, deterministic late-event handling, and multiple readers require a clear source of truth | A memory queue can lose acknowledged work; direct table writes create divergent ordering and concurrency rules |
| Separate lexical and semantic tools | The current-session AI needs inspectable exact-match and conceptual evidence with explicit fallback/coverage | A hidden hybrid score obscures missing semantic coverage; a server-side model would split conversational judgment |
| Separate completion and publication | Private completion and portable Knowledge have different authority and conflict consequences | One atomic action publishes as an unintended side effect and makes deferral impossible |
| Fine-grained context scopes | Session continuation, topic review, and whole-board overview have materially different disclosure and token costs | Always loading the Workbench or Vault overexposes private context and wastes tokens |

## Post-Design Constitution Re-check

The revised design lets the user complete work in the chosen Chat or switch to Workbench at any time, then decide separately whether to publish. The current-session AI performs evidence-backed synthesis while the application supplies bounded retrieval. All surfaces share one durable event log, application service, domain gates, projector, and provenance model; CAS and watermark rules prevent overwrite and late-event regression. Workbench is optional, in-app Chat remains supported, and Phase 1 adds neither a new workflow stage nor remote/web infrastructure. No Constitution amendment or unresolved clarification is required.
