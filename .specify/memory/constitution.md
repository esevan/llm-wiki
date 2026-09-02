<!--
Sync Impact Report
Version change: 2.0.0 -> 2.1.0
Modified principles: Engineering Guardrail F now permits a narrowly scoped Tauri desktop package
with one supervised local application boundary while continuing to exclude unrelated platform scope.
Added sections: none.
Removed sections: none.
Follow-up TODOs: none.
-->
# LLM Wiki Constitution

## Core Principles: Product Spirit

### I. You Talk. The Work Organizes Itself.
The product MUST let people begin with natural conversation rather than requiring them to classify,
structure, or rewrite their thoughts first. Refinement MUST turn conversation into a clearer Problem
or Solution while preserving the speaker's intent. The system MUST carry the burden of organizing
the work; it MUST NOT transfer that burden back to the user through form-heavy intake or taxonomy
decisions that can be inferred safely.

### II. Reduce Cognitive Load
Every surface MUST minimize the amount of state a person has to hold in their head. Capture MUST
remain lightweight, immediate, and separate from the structured workflow. In-progress work MUST be
visually prominent because it is the most likely place to resume. New fields, statuses, panels, and
choices MUST justify the cognitive load they add and MUST remain hidden when they are not relevant
to the current decision.

### III. Resume Where You Left Off
The product MUST preserve enough context for a person to continue without reconstructing the work
from memory. Work Log MUST support screenshot-first progress evidence because visual state is often
the fastest reliable resume point. Refinement MUST retain the current item's prior decisions,
evidence, constraints, and trade-offs. Conflict review MUST consult portable Knowledge so earlier
work can inform the present decision without forcing the user to search and compare it manually.

### IV. Organize Around Problems, Not Tasks
Every durable workflow MUST originate from a Problem and preserve its lineage through Solutions and
Solutions. Problems define why the work matters; Solutions define both the intended outcome and the
execution boundary through their Work Log and validation checklist. The product MUST NOT add another
workflow stage below Solution. Execution ambiguity MUST be resolved in its parent Solution or Problem
so the workflow stays organized around the reason for the work.

### V. Private Process, Portable Knowledge
Exploration, unfinished reasoning, chats, drafts, and intermediate work records MUST remain private
local process by default. Only a human-approved completed result MUST become reusable Knowledge.
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
Every capture, dashboard, indexing, and search change MUST have a relevant benchmark or profiling
check. Hot paths MUST NOT import or invoke AI code. A regression over 15% of a binding performance
budget fails the gate unless this constitution is amended.

### B. Independent Adapters
The Markdown vault is accessed only through `MarkdownVaultAdapter`; model endpoints are accessed
only through `OpenAICompatibleProvider`. Core workflow code MUST NOT depend on Obsidian,
CLIProxyAPI, provider aliases, or provider-specific configuration.

### C. Human Authority over AI
AI is a required product capability for organizing, refining, comparing, and reporting work, but
humans own workflow state, approval, priority, completion, and Knowledge publication. AI MUST NOT
autonomously advance a workflow or produce technical implementation directions in user-facing task
handoffs. Provider failure fallbacks MUST preserve private process and human authority.

### D. Evidence and Logical Consistency
Claims MUST cite source passages. Solutions with missing, stale, unknown, or conflicted context MUST
NOT be approved. Knowledge-file writes MUST be reviewed structured patches, atomic, reversible, and
guarded against external changes.

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

Backend capture readiness is under 1.5 seconds; capture persistence p95 is under 50 ms; warm FTS
search is under 75 ms; structural indexing of a 1,000-note/10 MB vault is under 3 seconds. Context
packs contain at most eight passages and 6,000 retrieved-context tokens. AI libraries and semantic
models are lazy loaded, and lexical search remains operational as a fallback during provider or
model failure.

## Product Spirit Review Gate

Every specification, plan, implementation, and review MUST state how the change serves at least one
Product Spirit principle and MUST check that it does not weaken any other principle. A proposal MUST
be revised before implementation when it:

- makes the user organize information that conversation or existing context can organize safely;
- adds visible state without reducing a larger cognitive burden;
- loses the context required to resume work;
- creates an independent workflow or refinement path below Solution;
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
explicitly cite the applicable Product Spirit principle and verify all relevant guardrails and
performance budgets before acceptance.

**Version**: 2.1.0 | **Ratified**: 2026-08-18 | **Last Amended**: 2026-09-02
