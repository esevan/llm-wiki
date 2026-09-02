# Implementation Plan: Fast Vault Search

## Architecture

`MarkdownVaultAdapter` is the only vault-path owner. `RetrievalEngine` builds SQLite FTS5,
folder, and link metadata. The FastAPI layer only calls services. The static client is an
AI-free shell.

## Budget Impact

Structural indexing uses file modification metadata and content hashes; FTS queries are narrowed
by matched folder segments. No LLM context is assembled in this feature, so its context budget is
zero. Benchmarks cover index elapsed time; a 1,000-note fixture remains required.

## Boundary and Platform Check

No Obsidian API/plugin or provider-specific behavior is used. SQLite WAL and `platformdirs`
support independent macOS/Windows installations. Semantic dependencies remain absent until a
background-only module is added.

## Conflict and Invalidation

This search-only feature creates no workflow approvals or conflict reports. Source hashes are
returned to make later context invalidation possible.
