# Task-centered MCP remediation implementation plan

**Date:** 2026-09-12
**Status:** Implementation, final checks, signed macOS release, full packaged E2E and local
app/plugin installation complete after GPT-6 Astra/high planning review. See the
[verification record](../testing/mcp-task-workflow-verification.md) for actual results.

## Outcome and scope

Make local MCP and desktop expose the same Task-centered work: lightweight Capture, independent
refinement, Work Log, exact Problem revisions, Task relationships, advisory review, independent
completion, and separate Knowledge draft/publication decisions. Fix the workflow skill and installed
plugin delivery as well as server queries and governed actions. Preserve existing grants, identifiers,
historical evidence and user-owned files. Do not restore the retired Python/browser delivery.

The prerequisite DB review and migration design are in
[DB Migration Plan](mcp-task-db-migration-plan.md). The approved v9 change only makes the existing
session Capture reference nullable. All existing records remain unchanged; the remaining corrections
are application-service, projection, DTO, runtime, skill and plugin-delivery changes.

## Decisions incorporated from Astra review

- V8 removes the old Workbench mutation triggers. Do not reintroduce SQL event triggers or add
  another event/session store. Use one transaction-scoped mutation path for desktop, MCP, refinement
  adoption and projectors, with genuine operation hashes and exactly-once event dispositions.
- Canonical discovery remains sessionless. Explicit continuation reviews the existing Task and
  atomically creates a connection-owned captureless session, binding event, link and idempotency
  result. Rejection creates nothing. Existing Capture sessions continue to work. Other connections'
  session details/ownership are never exposed or adopted.
- Continuation needs `session:write` plus the authorized discovery scope. Preserve grants and explicit
  topic membership, and recheck them at acceptance. Binding does not authorize other Tasks/Problems;
  each additional target needs independently authorized context and exact review.
- Current/overview/session projections read their complete dependencies in one SQLite read
  transaction. Overview currently hashes board and attention; current/link review uses workspace
  revision. Shared writes must invalidate workspace freshness and return material target hashes that
  cover state, links, relationships, Work Log, checklists, review and Knowledge as well as revisions.
  Extend the overview snapshot hash to every field returned; retain bounded paging and expiry rules.
- New Task Knowledge uses `task_knowledge_drafts` and one application service for both inputs.
  Capture immutable lineage in the same read snapshot: exact Task/completion/Problem revisions,
  historical links, relationships and evidence. Verify immutable referenced records and reviewed
  body/source hashes; later active-link changes do not invalidate valid historical lineage. Bind a
  separate lineage/source hash in review JSON because the body hash alone does not cover it.
- Preserve legacy events, drafts, publication decisions and files as labelled history. Reject new
  legacy approval funnels and grouped completion; unresolved legacy proposals require fresh canonical
  review. Recover already-approved legacy publication jobs only through the existing handler using
  their original reviewed draft/hash and file guards. Other unresolved legacy mutation jobs/proposals
  stay safely blocked with fresh canonical review guidance; preserve every record. Never silently
  discard them or replay legacy domain writes during projection startup.
- MCP conflict synthesis stays in the current Chat AI. Shared assistance persistence/decisions must
  not introduce a hidden provider conflict job when converging with native assistance modules.

## Required action and review matrix

Each row must have an explicit MCP schema/action, authorized targets, transaction-scoped service
entry point and behavioral acceptance test. Extend the existing tool surface where possible; do not
expose an arbitrary native-operation passthrough. Review bindings include connection, operation,
action, exact targets, payload hash, expected revisions/material hashes, and source proposal identity
where applicable. Task content revision alone cannot detect child/state changes.

| Capability | Canonical entry points / behavior | Decision boundary |
| --- | --- | --- |
| Discovery and continuation | current/overview/session DTOs; explicit existing-Task continuation | Reads retain scope; exact binding review, no fabricated Capture. |
| Capture and refinement | `capture.create`, `task-refinement.*` messages/workspace/proposals/decision | Capture acceptance and proposal adoption are separate; no automatic Task/Problem adoption. |
| Task and Problem revisions | `task.create`, `task.revision`, `problem.create`, `problem.revision` | Exact immutable revision proposal adoption. |
| Problem links | `task.problem-link.create/delete` | Exact Problem revision and historical link identity; unlink leaves history intact. |
| Task relationships | `task.relationship.create/delete` | Review both authorized endpoints; sorted related endpoints, split inverse, uniqueness, no self-link/prerequisite cycle. |
| Readiness and work evidence | readiness get/decision, Work Log get/create, comments, attachment evidence, checklist create/update, Task decisions | Meaningful checkpoint policy remains; explicit governed choices retain exact evidence/target hashes. |
| Advisory review | review create/get/history/cancel/decision, cited current-Chat findings | Advisory, cancellable and stale-aware; never a mandatory completion gate or hidden conflict model. |
| State and completion | `task.transition`, reopen, `task.completion.create` | Exact current aggregate/evidence review; independent from Problem resolution and publication. |
| Problem resolution | `problem.resolution.create` | Exact Problem revision, authorized scope and explicit evidence; completion does not resolve it. |
| Lineage and Knowledge | `task.lineage`, draft/correction/regenerate/publish/withdraw | Shared Task draft store; exact revision + body/source hashes; published edits create new draft; separate publication/withdrawal. |

Attachment handling uses permitted evidence references or approved host-supported input; MCP must
not request arbitrary absolute filesystem paths. Capture/refinement and ordinary checkpoints retain
their separate existing policies rather than turning each conversational turn into a review dialog.

## Execution order

1. Prerequisite completed: source-based DB review, DB Migration Plan and one focused Astra review.
   Use the incorporated decisions above and the DB plan's exact v9 contract before application changes.
2. Create a dedicated worktree with `scripts/create_task_worktree.sh mcp-task-workflow
   fix/mcp-task-workflow`. Preserve all primary-checkout changes, including AGENTS.md, the commit
   skill, CONTINUATION, the original Task-centered plan, and both UI/UX plans. Carry applicable current
   instructions into the worktree deliberately. Implement and verify there.
3. Add failing behavior tests for desktop-created Tasks visible through MCP, exact Problem links,
   Task relationships, pending decisions, stable overview paging and stale review rejection. Include
   mixed legacy/current fixtures; test source/installed skill contracts alongside tool schemas.
4. Implement the nullable-Capture v9 migration before dependent queries/actions. Preserve all other
   columns, FKs, uniqueness, rows, child identities and surviving session triggers/indexes. Use the documented SQLite replacement/copy/drop/rename pattern with connection-local FK
   enforcement disabled before the v9 transaction and verified restoration on every outcome,
   verify child and parent-session chains, and set
   user_version inside the transaction before commit. Prove backup/recovery on disposable fixtures.
   Fix every Capture-required join/DTO/open path for both legacy and captureless sessions.
5. Correct current/session/overview projections through bounded application-service reads. Preserve
   connection scope, explicit topic membership and stable snapshot paging. Include Task state/revision,
   exact Problem links, Task relationships, meaningful activity and pending decisions. Make direct
   desktop Tasks readable/resumable without forcing them through the old workflow.
6. Complete every row of the action/review matrix through canonical Task services. Reject new
   `adopt_problem`, `approve_problem`, `adopt_solution`, `approve_solution`, mandatory conflict gates
   and legacy grouped `verify_and_complete`; retain auditable migrated history and canonical continuation.
   Preserve operation idempotency and head/target revision conflicts. Completion must not implicitly
   resolve a Problem or publish Knowledge; advisory review must not become a mandatory approval gate.
   Converge all new Knowledge operations onto the Task draft service with immutable source snapshots,
   and preserve old jobs/history under the compatibility policy above before updating consumers.
7. Align in-app tracking cards, MCP schemas/action descriptions, workflow skill and overview/conflict
   references. Document concrete supported actions and payloads. Verify that the host can represent
   arbitrary JSON patches and evidence entries rather than assuming server DTOs alone prove integration.
8. Update paired feature guides and current API/migration contracts without rewriting historical v8
   semantics. Use the plugin development
   workflow to refresh the local plugin rather than editing generated installed-cache files. Preserve
   configured connection IDs, settings and grants. Test fresh-task plugin loading; report elicitation or
   external-host limits separately from successful source tests.
9. After final source/document changes, run required checks and one final release build/packaged E2E.
   Record the exact artifact and fixture identities, fresh MCP/plugin evidence, migration results and
   remaining external gates. Do not label source coverage as installed-platform acceptance.

## Verification and completion

- Run `npm test`, `npm run lint`, `npm run typecheck`, and `npm run build`.
- Run Rust format check, strict all-target clippy, and
  `cargo test --manifest-path src-tauri/Cargo.toml`.
- Run `npm run tauri:build`, then `npm run test:desktop` against the final release artifact; retain
  evidence under the task worktree using the documented `--full` mode. Extend packaged scenarios for
  the changed MCP/desktop behavior.
- Run `git diff --check`. Full lint currently has 21 reported pre-existing findings; record the actual
  final result and touched-scope status without claiming a clean full run if that baseline remains.
- Cover empty/latest/legacy migration fixtures, failed/interrupted migration and retry/restore where
  required by the reviewed DB plan; prove exact references and unchanged historical event identities.
- Cover direct desktop Task creation → scoped MCP read/resume → Task modification → desktop refresh,
  cancellation, exact/stale reviews, completion independent of Problem resolution and publication,
  EN/KO tracking copy, revoke/deny scope, and concurrent desktop/Chat changes.
- Keep installed Windows/Linux, OS reduced motion, real-provider corpus and host elicitation evidence
  explicit when unavailable on this host. Do not reset user data, keychain or TCC state.

Completion requires all confirmed MCP gaps fixed or a concrete, justified compatibility decision
with coverage. Commit conventions apply when composing a commit; installation/push must be supported
by the user's session authorization and a verified artifact. This plan does not claim those actions
have already occurred.

## Stage execution models

| Stage | Model / effort | Reason |
| --- | --- | --- |
| DB and shared mutation implementation | GPT-5.6 Terra / high | SQLite FK rebuild, scope, transactional CAS and publication recovery can affect data integrity. |
| Remaining service/DTO/runtime integration | GPT-5.6 Terra / medium | Implement the reviewed action matrix with controlled cross-module integration. |
| Skill, paired docs and mechanical metadata | GPT-5.6 Luna / low | Defined terminology and contract changes; independent files only. |
| Behavioral verification and release evidence | GPT-5.6 Terra / medium | Validate migration, concurrency and packaged behavior against explicit invariants. |

Do not modify the same files concurrently. Reuse the implementing agent across dependent stages,
lower effort for routine follow-up, and escalate only a demonstrated unresolved correctness bottleneck.
The Astra consultation is complete; do not keep Astra on routine implementation/testing.

## Review record

On 2026-09-12 GPT-6 Astra/high reviewed the DB and implementation drafts against source. Conditional
approval required correcting trigger activity/version-commit facts and adopting the precise v9,
scoped continuation, consistent projection snapshots, shared mutation path, canonical Knowledge
authority and immutable lineage decisions. Those corrections are incorporated in both plans.
This is design review evidence, not migration/runtime/release acceptance.

Execution integrity corrections bind all referenced Task/Problem targets to scoped access and complete child-aware snapshots, recheck them within the acceptance transaction, and apply governed non-publication mutations in that same transaction. Mutation failure leaves the review pending. Publication retains its accepted durable job before file writes. Completed operation retries return their stored outcome without another approval.
