# LLM Wiki — Continuation Handoff

**Updated**: 2026-09-02
**Current status**: The compatibility-first React/Tauri migration on
`feat/react-tauri-migration` passes its Phase 9 audit. React owns primary screens, dialogs, and
navigation; typed HTTP/Tauri application clients, theme tokens, request-ID streaming/cancellation,
a self-contained native bundle, isolated durable-worker runtimes, real-sidecar command tests, and
the release-app desktop scenario all pass. The former 181 KB runtime is now eleven bounded feature
controllers with styles centralized in the theme. See `docs/migrations/react-tauri-report.md` for
the complete matrix and retained compatibility rationale.
**Local URL**: `http://127.0.0.1:8765`  
**Vault**: User-selected local Markdown directory

The current roadmap and acceptance order are maintained in [PROJECT_PLAN.md](PROJECT_PLAN.md).

## What is implemented

### 009 — Background AI Queue

- `serve` supervises one global, memory-only Fast worker plus configurable durable async workers;
  `web`, `fast-worker`, and `async-worker` can also start independently.
- Chat and other immediate AI interactions use the hidden Fast Queue. Durable AI, Knowledge and
  derived translation, Image Summary, Completion Review, organization, Lineage, reporting, and
  embedding work use SQLite Jobs with claims, leases, heartbeats, bounded retry, source checks,
  cancellation, checkpoints, and idempotent notification publication.
- The bottom-right Queue exposes durable status and task-specific result actions. Completion Review
  additionally uses a temporary toast and persisted unread bell alert.
- Knowledge translation resumes by paragraph and moves completed output to the Vault before
  deleting working checkpoints. Closing or switching its reader detaches UI progress without
  cancelling the durable job. Capture and Work Log translations preserve authored source text.
- Concurrent durable workers own separate application/SQLite connections; transient SQLite writer
  contention uses the queue's bounded retry policy. If optional FastEmbed is absent, embedding work
  completes with an explicit lexical-fallback result instead of creating failed queue history.
- AI Queue code follows the enforced layered dependency rules in
  [architecture.md](architecture.md). Web owns composition, controllers receive an assembled
  runtime, Queue domain types are dependency-free, and each durable task has one named handler
  module. The old blocking provider, graph, bundled workflow/localization handler, and AI modules
  were removed.

### 001 — Fast Vault Search

- Obsidian-compatible Markdown parsing: frontmatter, aliases, headings, nested tags,
  wikilinks, heading/block references, embeds, and code-fence exclusion.
- SQLite WAL + FTS5 structural index with changed-file detection, directory routing, link
  graph storage, source hashes, result citations, pagination, filesystem watcher, and SSE.
- The optional FastEmbed semantic runtime can be installed with the `semantic` extra. Embedding
  generation and refresh run as durable document jobs when available; lexical search remains
  available while they complete and is the explicit fallback when the extra is absent.
- The reference 1,000-note structural benchmark is consistently below the 3-second budget.

### 002 — Conflict-Gated Workflow

- Human-managed Capture → Problem → Solution workflow board with Solution-owned Work Log,
  validation checklist, and completion flow.
- Reversible soft deletion removes workbench items (and dependent items) from view without
  deleting stored history or vault files; the restore API remains available for future UI work.
- Every board card has an AI Explore chat and a separate Manual form. Chat streams only
  non-technical outcome/evidence/trade-off discussion and records an `AiRun`; it never changes
  workflow state.
- Explore uses stage-specific prompts and the most recent six persisted conversation turns to
  collect missing information one question at a time. Once a stage is ready, it presents a
  **Create AI draft** action.
- Every workflow draft is AI-assisted and has a required, validated structure: Capture → problem
  statement; Problem → Solution name/outcome/non-goals/validation criteria. The AI proposal is displayed in an editable review dialog
  and becomes an item only after the human chooses **Finalize AI draft**.
- **Explore with AI** refines only the current Capture, Problem, or Solution. **Draft next…**
  begins a separate, next-stage interview, collects its required information, then offers the
  reviewed AI draft; it never skips straight from the current item to creation.
- Captures are an active inbox, not a duplicate workflow stage. Explicit Problem finalization keeps
  the linked Capture for history while removing it from the active inbox.
- Each Solution owns its Work Log and validation checklist, and the optional Workbench Flow view
  displays Problem → Solution relationships.
- The completed local Workbench audit is maintained in [WORKBENCH_CHECKLIST.md](WORKBENCH_CHECKLIST.md).
- Editable stage prompts live in `llm_wiki/prompts/{captures,problems,features}.md`; the
  prompt loader reads the matching package resource only when chat is invoked.
- Features ordinarily require an approved Problem, a cited `clear` conflict evaluation, and explicit
  user approval. `unknown` and `conflicted` states are blocked unless the user invokes the supported
  skip-with-reason resilience override.
- OpenAI-compatible provider has only base URL, API key, model, `/v1/models`, JSON completion,
  and streaming behavior. No provider-specific behavior exists outside the adapter.
- LangGraph is installed but lazy; it runs only after an AI enrichment request.
- AI Setup stores endpoint/model in the local database and the API key only in the OS keyring.
  The default endpoint is `http://127.0.0.1:8317/v1`; users can replace it in the app.
- AI Setup stores exactly two model choices: Default and Advanced. Advanced options select the
  model tier for each named AI task, rather than for Capture, Problem, or Solution stages. An
  enabled task uses the Advanced model; if it is blank, it falls back to the Default model.
- Discussion and refinement, drafting, conflict review, image summary, completion review, and
  completion report use the Advanced tier by default. Workbench organization, completed-Solution
  discussion, and Problem enrichment use the Default tier by default.
- Copyable handoff contains outcomes/done criteria and prevents technical implementation steps.

### 003 — Completion, Writeback, Archive

- Completion evidence/report gate, explicit no-update reason, reviewed structured patch proposal,
  source-hash conflict block, atomic adapter-owned write, undo preimage, projection and archive.
- Generated projections use Obsidian-compatible Markdown frontmatter. External edits block
  automatic replacement through mirrored hashes.

### 004 — Direction Dashboard

- Compass goals, evidence-backed importance assessment, immutable score-event ledger, and
  precomputed time-period totals are implemented and surfaced in the Compass screen.

## Environment and startup

| Item | Value |
|---|---|
| Platform | macOS on Apple Silicon (`arm64`) |
| Project path | Repository checkout directory |
| Python | 3.12.14 (uv-managed) |
| Package manager | `uv` 0.12.5 |
| Local database | OS application-data directory, `LLM Wiki/llm-wiki.sqlite3` |
| Service | `com.llm-wiki` LaunchAgent |
| Service logs | `~/Library/Application Support/LLM Wiki/logs/` |

The service command is `.venv/bin/llm-wiki serve --vault <vault> --no-browser`; the LaunchAgent
is installed at `~/Library/LaunchAgents/com.llm-wiki.plist`. It is local-only (`127.0.0.1`).

## Verification record

- The core/dev environment is synchronized; use `uv sync --all-extras` when semantic and LangGraph
  extras are required.
- Latest migration run: **195 Python + 10 React + 12 Rust tests, 0 skipped**, plus two consecutive
  release-app desktop E2E passes. The macOS/Windows/Linux acceptance matrix remains in
  `.github/workflows/cross-platform.yml`.
- Ruff lint and 120-column format checks passed across all Python source and tests.
- Browser JavaScript syntax is separately validated by Node and passed.
- Latest 1,000-note structural-index benchmark: **1,002.13 ms** (budget: <3,000 ms).

## Remaining work

These are deliberate continuation tasks, not blockers for the currently running local product:

1. Add a real 1,000-note multilingual retrieval fixture and measure FTS, semantic reranking,
   capture, dashboard, startup, and memory budgets individually.
2. Add a user-visible patch-review surface showing base/current/proposed content and a
   three-way merge for non-overlapping edits (current implementation blocks changed files).
3. Add Windows-native lock/move integration tests and a cross-platform file-lock implementation
   (`portalocker`) before claiming Windows release acceptance.
4. Implement confirmed SpecKit gaps: independent semantic corpus search; locally bundled fonts;
   deprecated compatibility API markers; removal of the unused report-language setting; Knowledge
   translation tier UI; checklist-edit retranslation; state-changing Knowledge translation request;
   Lineage inference-failure indicator; Conflict Review progress and document deduplication; hard
   cancellation; and Chat-close abort.
5. Define the durable Queue history TTL (`TD-006`).
6. Select the Queue, toast, and notification accessibility acceptance scope (`TD-020`).

The Lineage Knowledge Layer specification migrated from duplicate prefix `006` to `010`.
Current and pending behavior-driven characterization cases are cataloged in
[`tests/CHARACTERIZATION.md`](../tests/CHARACTERIZATION.md).

## First actions for the next agent

1. Read `.specify/memory/constitution.md` and this file.
2. Read the feature `tasks.md` matching the continuation task.
3. Run `uv sync --all-extras` and `uv run pytest -q` from the project root.
4. Confirm service health at `/api/health`; use LaunchAgent logs if unavailable.
5. Preserve adapter boundaries: only `MarkdownVaultAdapter` touches vault paths; only
   `OpenAICompatibleProvider` knows endpoint details; no AI imports in capture, board, or
   structural search paths.
