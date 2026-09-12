# Research: Task-Centered Workbench

## Decision 1: Replace the workflow at one release boundary

**Decision**: Remove legacy Problem/Solution write routes and gates after a preserving migration; do
not ship a compatibility shim.

**Rationale**: The UI, native routes, work tracking, and MCP currently encode the same parent/gate
semantics. Parallel compatibility would duplicate domain behavior and allow records to diverge.

**Alternatives considered**: Aliased `/features` routes and dual-write tables were rejected because
they obscure revision ownership and lengthen the high-risk migration window.

## Decision 2: Rebuild constrained SQLite tables in migration v8

**Decision**: Rebuild affected tables in a transaction after a separately verified SQLite backup.
Remove `features.problem_id NOT NULL` by mapping features to Tasks; remove `problems.capture_id UNIQUE`
so revisions/links allow many relationships. Confirm foreign keys and query plans in a spike.

**Rationale**: SQLite cannot relax these constraints in place safely. Existing migrations remain
immutable, and the new version can validate exact source-to-target mappings before commit.

**Alternatives considered**: Editing baseline schema alone would miss installed databases; nullable
legacy feature links would retain obsolete semantics.

## Decision 3: One Task aggregate service

**Decision**: A Rust application service owns Task mutations and is consumed by native routes and MCP.
Frontend types mirror the contract; routers do not duplicate rules.

**Rationale**: This preserves the one native application boundary and makes revision conflict,
idempotency, activity, and human approval uniform across surfaces.

## Decision 4: Review subjects are exact drafts or exact Task revisions

**Decision**: Use a tagged review subject: Capture refinement draft identity or persisted Task revision.
Material hash includes title/outcome/scope/criteria and linked Problem revision hashes; Vault and
evidence-grant revisions complete the identity.

**Rationale**: Capture drafts need review before Task persistence. Tagged subjects prevent temporary
Task creation and enable the same orchestration rules on both paths.

## Decision 5: Preserve the last current review separately from attempts

**Decision**: Persist attempts and immutable findings; compute currentness from identity. A failed
new attempt cannot overwrite the last current result.

**Rationale**: Failure is information about an attempt, not evidence that previous exact input is
clear or conflicted.

## Decision 6: Recency comes from explicit user operations

**Decision**: Store allowlisted user activity events and derive `last_user_activity_at`; provider,
index, translation, projection, and job status updates do not emit them.

**Rationale**: Generic table timestamps currently conflate useful user resumption order with
background churn.

## Decision 7: Replace only Workbench legacy ownership

**Decision**: React owns Workbench projection, input, category cards, and Task detail. Disable the old
`#board` render/load/click bindings while retaining unrelated legacy Search, Settings, and queue code.

**Rationale**: A full runtime rewrite expands risk; leaving board handlers attached creates duplicate
requests and state ownership.

## Decision 8: No new dependency

**Decision**: Use rusqlite transactions, existing Tauri routing, React state, and existing job/provider
abstractions.

**Rationale**: Current libraries cover the feature; dependency cost would not improve correctness.

## Decision 9: Fixed final-provider rubric

**Decision**: Review 12 publications and 24 conflict cases with fixed expectations. Record exact sample
counts, median/p95 latency, citation coverage, false-clear count, classification accuracy, and 1–5
scores for fidelity, evidence, structure, and readability.

**Rationale**: A fixed corpus is repeatable and does not require subjective multi-reviewer adjudication.
If provider configuration is absent, the report is explicitly blocked.
