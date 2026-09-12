# Specification Supersession and Impact Map

This document distinguishes historical delivery evidence from current normative behavior. The
Task-centered release does not erase earlier artifacts. Where an earlier requirement or contract
conflicts with this feature, [spec.md](spec.md), [application-api.md](contracts/application-api.md),
[migration-contract.md](contracts/migration-contract.md), and [mcp-task-contract.md](contracts/mcp-task-contract.md)
are authoritative from schema 8 onward.

| Existing feature | Impact | Current rule |
|---|---|---|
| 001 Fast Vault Search | Supporting, not superseded | Lexical/semantic adapter and budgets remain; review consumes its scoped cited evidence through the new review identity. |
| 002 Conflict-Gated Workflow | Fully superseded | Problem approval, required Problem parent, Solution state, and clear-before-start gates are removed; independent Task and nonblocking review replace them. |
| 003 Completion, Writeback, and Archive | Workflow and API superseded | Task completion, Problem resolution, and exact Knowledge publication are separate; reversible write protection remains. |
| 004 Direction Dashboard | Entity projection partially superseded | Existing category/importance and work-direction semantics remain; Problem/Solution board inputs become canonical Capture/Task/refinement items and must not score workers. |
| 005 Refinement Preview Status | Workflow and API superseded | One resumable Capture-or-Task session, multiple proposals, exact draft decisions, and optional application replace Problem/Solution preview. |
| 006 Task-Level AI Model Routing | Target vocabulary partially superseded | Routing/provider settings remain; legacy feature/Solution target kinds map to Capture draft, Task revision, or Task-owned artifact kinds. |
| 007 Korean-English Localization | Entity/API contract partially superseded | Locale persistence and bilingual durable fields remain; Workbench/Task/refinement/review/Knowledge routes and copy replace Problem/Solution workflow terms. |
| 008 Evidence-Rich Vault Conflict Review | Workflow/API superseded | Evidence and citations remain; tagged Capture-draft/Task subjects, exact Vault/grant identity, eight states and nonblocking behavior replace feature review. |
| 009 Background AI Queue | Job substrate partially superseded | Durable queue/retry/cancel behavior remains; new Task/refinement/review targets, cancellation/currentness, and recency exclusions apply. |
| 010 Conflict Resolution Workflow | Fully superseded | Immutable findings and separate decisions remain; resolution cannot change a gate because Task work is never gated. |
| 010 Lineage Knowledge Layer | Entity/API superseded | Evidence/correction preservation remains; Task graph lineage and exact separately approved publication replace completed-Solution lineage. |
| 011 Dual-Chat Work Tracking | Workflow contract superseded | Event log, scopes, cursors, review expiry and replay remain; Task events/resources and the shared Task aggregate replace Problem/Solution proposals and gates. |

## Replacement Rules

1. Legacy `feature` or `Solution` durable execution identity means migrated `Task` with the same ID.
2. Legacy Problem parent or approval requirements have no current equivalent. Problem is optional,
   revisioned context joined many-to-many to exact Task revisions.
3. Legacy `approved`/`in_progress` Solution state maps to `in_progress`; `proposed` maps to `task`;
   `completed` maps to `completed`; archived remains non-active.
4. Any statement that missing/stale/conflicted review blocks start, logging, or completion is retired.
   Current review is advisory and exact-identity bound.
5. Problem-wide completion/writeback is retired. Task completion closes one Task, Problem resolution
   is separate, and Knowledge publication requires exact draft approval.
6. Old paths remain historical evidence only. The removed-contract list in application-api.md is
   exhaustive for runtime compatibility; no alias or dual-write behavior is implied.
