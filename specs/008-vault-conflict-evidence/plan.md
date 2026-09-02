# Implementation Plan: Evidence-Rich Vault Conflict Review

**Branch**: `feat/vault-conflict-evidence` | **Date**: 2026-09-01 | **Spec**: [spec.md](spec.md)

## Summary

Extend SQLite retrieval with independent all-document semantic search and exact passages. Run conflict review as a cancellable background job that extracts claims, merges evidence, conservatively screens candidates, performs evidence-validated strong review, publishes progress/findings, caches by hashes, and records phase timings.

## Technical Context

**Language/Version**: Python 3.11+, existing browser JavaScript
**Dependencies**: FastAPI, SQLite FTS5, optional fastembed/onnxruntime, watchfiles
**Storage**: Existing SQLite and Markdown Vault via `MarkdownVaultAdapter`
**Testing**: pytest, TestClient, Playwright
**Platform**: macOS and Windows local service
**Performance**: Preserve warm lexical search under 75 ms and 1,000-note indexing under 3 seconds; record model timings
**Constraints**: lazy AI imports; no vector store; maximum eight passages/6,000 retrieved tokens per strong call; human-owned workflow state

## Constitution Check

- Product Spirit III: exposes portable Knowledge evidence needed to resume decisions.
- Product Spirit II: detail appears only during active review and is grouped by scope, progress, evidence, and meaning.
- Private/portable boundary: approved Vault Knowledge is evidence; private Workbench records are not.
- Measured performance: structural indexing stays AI-free; search/screen/review timings and retrieval tests are added.
- Adapter boundaries: Vault access remains through `MarkdownVaultAdapter`; model traffic through `OpenAICompatibleProvider`.
- Human authority: recommendations never mutate conflict or approval state.
- Evidence consistency: clear requires completed review, full semantic coverage, candidates, and validated citations.
- Cross-platform/minimal complexity: reuse SQLite, threads, and existing optional semantic dependencies.

No gate violations are introduced.

## Project Structure

```text
llm_wiki/api/app.py
llm_wiki/services/conflict_review.py
llm_wiki/services/provider.py
llm_wiki/services/retrieval.py
llm_wiki/services/workflow.py
llm_wiki/static/index.html
tests/test_conflict_review.py
tests/test_retrieval.py
tests/test_browser_menu.py
```

## Design

1. Index changes remove obsolete embeddings and schedule lazy refresh on startup and watcher changes.
2. Semantic search embeds a query once and scores every current embedding independently of lexical hits.
3. Deterministic claims come from title, outcome, non-goals, validation criteria, and parent Problem. Per-claim candidates retain exact line-numbered passages.
4. A review manager owns cancellation events and snapshots. Start, poll, and cancel endpoints expose partial findings.
5. One fast batch screen may exclude only complete, evidence-grounded `non_conflict` decisions. All others receive strong review.
6. Findings reference supplied evidence IDs and inherit exact citations. Invalid citations force insufficient evidence.
7. Terminal cache keys include Solution and indexed Vault manifest hashes.

## Complexity Tracking

No runtime dependency or workflow stage is added. Threads are bounded by one semantic refresh and one active review per Solution.
