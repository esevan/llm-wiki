# LLM Wiki — Continuation Handoff

**Updated**: 2026-08-19  
**Current status**: Local application is running as a per-user macOS `launchd` agent.  
**Local URL**: `http://127.0.0.1:8765`  
**Vault**: User-selected local Markdown directory

The current roadmap and acceptance order are maintained in [PROJECT_PLAN.md](PROJECT_PLAN.md).

## What is implemented

### 001 — Fast Vault Search

- Obsidian-compatible Markdown parsing: frontmatter, aliases, headings, nested tags,
  wikilinks, heading/block references, embeds, and code-fence exclusion.
- SQLite WAL + FTS5 structural index with changed-file detection, directory routing, link
  graph storage, source hashes, result citations, pagination, filesystem watcher, and SSE.
- Optional FastEmbed semantic runtime is installed. It is invoked only after the user enables
  Semantic search; initial embedding generation happens in a background thread.
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
- Features require an approved Problem, a cited `clear` conflict evaluation, and explicit human
  approval. `unknown` and `conflicted` Features are blocked.
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
| Platform | macOS on Intel (`x86_64`) |
| Project path | Repository checkout directory |
| Python | 3.12.13 (`/usr/local/opt/python@3.12/bin/python3.12`) |
| Package manager | `uv` 0.12.2 |
| Local database | OS application-data directory, `LLM Wiki/llm-wiki.sqlite3` |
| Service | `com.llm-wiki` LaunchAgent |
| Service logs | `~/Library/Application Support/LLM Wiki/logs/` |

The service command is `.venv/bin/llm-wiki serve --vault <vault> --no-browser`; the LaunchAgent
is installed at `~/Library/LaunchAgents/com.llm-wiki.plist`. It is local-only (`127.0.0.1`).

## Verification record

- `uv sync --all-extras` completed with semantic, AI, and test dependencies.
- Latest automated run: **24 passed, 1 skipped**. The skipped test is a real-browser menu test;
  Playwright’s Chromium archive could not be downloaded locally because the machine’s TLS chain
  could not validate the CDN certificate. The test and a macOS/Windows/Linux CI matrix are in
  `.github/workflows/cross-platform.yml`.
- Browser JavaScript syntax is separately validated by Node and passed.
- Latest 1,000-note structural-index benchmark: **1,002.13 ms** (budget: <3,000 ms).

## Remaining work

These are deliberate continuation tasks, not blockers for the currently running local product:

1. Run the Playwright menu test locally after the machine trust store/CDN certificate issue is
   corrected; retain the cross-platform CI matrix as the acceptance source.
2. Add a real 1,000-note multilingual retrieval fixture and measure FTS, semantic reranking,
   capture, dashboard, startup, and memory budgets individually.
3. Add a user-visible patch-review surface showing base/current/proposed content and a
   three-way merge for non-overlapping edits (current implementation blocks changed files).
4. Add Windows-native lock/move integration tests and a cross-platform file-lock implementation
   (`portalocker`) before claiming Windows release acceptance.
5. Add remaining workflow UI surfaces: conflict findings (claim/severity/citation), completion
   reporting, patch review, archive, and importance assessment. Their
   API/service layer is present; the UI is intentionally still compact.

## First actions for the next agent

1. Read `.specify/memory/constitution.md` and this file.
2. Read the feature `tasks.md` matching the continuation task.
3. Run `uv sync --all-extras` and `uv run pytest -q` from the project root.
4. Confirm service health at `/api/health`; use LaunchAgent logs if unavailable.
5. Preserve adapter boundaries: only `MarkdownVaultAdapter` touches vault paths; only
   `OpenAICompatibleProvider` knows endpoint details; no AI imports in capture, board, or
   structural search paths.
