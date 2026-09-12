<!--
Sync Impact Report
Version change: 2.1.0 -> 3.0.0
Modified principles: IV. Organize Around Problems, Not Tasks -> IV. Tasks Carry Work; Problems Carry Context;
C. Human Authority over AI clarifies that optional assistance never gates ordinary Task work;
D. Evidence and Logical Consistency requires exact-revision, nonblocking conflict review.
Added sections: Durable Workflow Migration requirements in Governance.
Removed sections: none.
Follow-up TODOs: none.
-->
# LLM Wiki Constitution

## Core Principles: Product Spirit

### I. You Talk. The Work Organizes Itself.
The product MUST let users begin with natural conversation rather than requiring them to classify,
structure, or rewrite their thoughts first. Refinement MUST organize conversation into clearer
Capture, Task, or Problem records while preserving the user's intent. The system MUST carry the
burden of organization and MUST NOT impose forms or taxonomy decisions that can be inferred safely.

### II. Reduce Cognitive Load
Every surface MUST minimize the state a user holds in mind. Capture MUST remain lightweight and
immediate. In-progress Task shortcuts MUST be visually prominent because they are the most likely
place to resume, while the canonical category list remains available. New fields, statuses, panels,
and choices MUST justify their cognitive cost and remain hidden when irrelevant to the decision.

### III. Resume Where You Left Off
The product MUST preserve enough context for a user to continue without reconstructing work from
memory. Work Log MUST support screenshot-first progress evidence. Refinement MUST retain messages,
drafts, decisions, input, and view position. Conflict review MUST consult portable Knowledge without
forcing manual search and comparison.

### IV. Tasks Carry Work; Problems Carry Context
Task is the independent durable unit for execution, resumption, evidence, and completion. A Task MAY
exist without a Problem and MAY link to multiple exact Problem revisions; a Problem MAY inform many
Tasks and MUST keep its own revision and resolution history. Capture MUST remain a canonical thought
record when it produces Tasks. Refinement, readiness, and conflict review MUST help users understand
Capture or Task records without becoming mandatory workflow stages or approval gates. Work Log MUST
belong to Task, and Task completion MUST NOT resolve a Problem or publish Knowledge automatically.

### V. Private Process, Portable Knowledge
Exploration, unfinished reasoning, chats, drafts, and intermediate work records MUST remain private
local process by default. Only a user-approved completed result MUST become reusable Knowledge.
Knowledge MUST use portable, inspectable formats such as Obsidian-compatible Markdown and MUST not
depend on LLM Wiki to remain useful. The system MUST preserve the boundary between private working
context and deliberately published knowledge.

### VI. Understand the Work, Never Score the Worker
The product MAY explain status, evidence, dependencies, risks, and direction, but MUST NOT turn those
signals into a score, rank, or judgment of a person. Importance and contribution signals MUST
describe the work and its relationship to goals, never individual productivity or performance.
Any future team or organization feature MUST make this distinction explicit in its data model,
language, dashboards, and access controls.

## Engineering Guardrails

### A. Measured Performance
Every capture, workbench, indexing, and search change MUST have a relevant benchmark or profiling
check. Hot paths MUST NOT import or invoke AI code. A regression over 15% of a binding performance
budget fails the gate unless this constitution is amended.

### B. Independent Adapters
The Markdown vault is accessed only through `MarkdownVaultAdapter`; model endpoints are accessed
only through `OpenAICompatibleProvider`. Core workflow code MUST NOT depend on Obsidian,
CLIProxyAPI, provider aliases, or provider-specific configuration.

### C. Human Authority over AI
AI is a required product capability for organizing, refining, comparing, and reporting work, but
users own workflow state, priority, completion, Problem resolution, and Knowledge publication. AI
MUST NOT autonomously create a durable Task or Problem revision, apply a proposal, advance a Task,
resolve a Problem, or publish Knowledge. Optional refinement, readiness, and review results MUST NOT
prevent Task creation, Work Log updates, start, or completion. Provider failure fallbacks MUST
preserve private process and human authority.

### D. Evidence and Logical Consistency
Claims MUST cite source passages. Conflict review MUST bind its result to the exact material draft or
Task revision, Vault revision, and evidence scope; failed, cancelled, insufficient, or stale review
MUST NOT appear clear. Unresolved findings MAY produce a nonblocking warning and an explicit user
decision. Knowledge-file writes MUST be reviewed structured patches, atomic, reversible, and guarded
against external changes.

### E. Local and Cross-Platform
The application MUST run independently on macOS and Windows. SQLite WAL, platform-specific data
paths, file locks, and atomic operations MUST be used where applicable. External model endpoints
MUST NOT become storage for the user's private process.

### F. Minimal Complexity
Dependencies require a measured justification. The supported desktop distribution MUST use a thin
Tauri shell, a modular web UI, and one supervised local application boundary; packaging MUST NOT
duplicate domain behavior across JavaScript, Rust, Python, or HTTP handlers. Python MAY remain as a
packaged sidecar while it owns substantial stable domain behavior, provided it is never contacted
directly by the web UI, is bound to loopback only, is lifecycle-managed by the desktop shell, and
has a documented domain-by-domain removal path. Version one continues to exclude sync,
collaborative users, OCR, attachment indexing, and Obsidian application integration.

## Performance Standards

Backend capture readiness is under 1.5 seconds; capture and direct-Task persistence p95 is under
50 ms; warm FTS search is under 75 ms; structural indexing of a 1,000-note/10 MB vault is under
3 seconds. Workbench local projections render within 100 ms p95 after data arrives. Context packs
contain at most eight passages and 6,000 retrieved-context tokens. AI libraries and semantic models
are lazy loaded, and lexical search remains operational during provider or model failure.

## Product Spirit Review Gate

Every specification, plan, implementation, and review MUST state how the change serves at least one
Product Spirit principle and MUST check that it does not weaken any other principle. A proposal MUST
be revised before implementation when it:

- makes the user organize information that conversation or context can organize safely;
- adds visible state without reducing a larger cognitive burden;
- loses context required to resume work;
- makes optional refinement, readiness, or review a gate for ordinary Task work;
- publishes unfinished private process as Knowledge; or
- scores, ranks, or judges a worker rather than explaining the work.

## Development Workflow

Each vertical feature follows Constitution, Specify, Clarify, Checklist, Plan, Tasks, Analyze,
Implement, and Converge. Plans MUST include a Product Spirit assessment plus performance/token
budgets, adapter boundaries, cross-platform behavior, conflict invalidation, benchmarks, and
dependency cost. Feature work may proceed only after its artifacts resolve critical and high
inconsistencies and pass the Product Spirit Review Gate.

## Governance

This constitution supersedes local implementation preferences. The Product Spirit governs product
direction; Engineering Guardrails govern how that direction is built safely. Amendments require a
written reason, semantic-version update, and migration note for affected behavior. Reviews MUST
explicitly cite the applicable Product Spirit principle and verify relevant guardrails and budgets.

Breaking durable-workflow changes MUST preserve existing user records through a verified backup and
one-time migration. The migration MUST preserve stable identities, revision provenance, Work Log
text and attachment bytes, comments, checklists, decisions, completion evidence, and lineage. It MUST
validate counts, identifiers, relationships, hashes, and foreign keys before committing the new
schema, and MUST provide a verified restore path without deleting source records automatically.

Migration note for 3.0.0: existing Solution records become Tasks with their identifiers and evidence
preserved; existing Problems become revisioned context and exact Task links; orphan Capture records
remain canonical; Problem-only records remain discoverable as resumable refinement items. The old
Problem approval and Solution conflict gates are retired at one release boundary without a runtime
compatibility shim.

**Version**: 3.0.0 | **Ratified**: 2026-08-18 | **Last Amended**: 2026-09-05
