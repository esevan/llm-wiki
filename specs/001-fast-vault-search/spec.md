# Feature Specification: Fast Vault Search

**Feature Branch**: `001-fast-vault-search`  
**Created**: 2026-08-18  
**Status**: Implemented and maintained

## User Scenarios & Testing

### User Story 1 — Search an existing vault (Priority: P1)

A person selects a Markdown vault and can immediately search filenames, folders, frontmatter,
headings, tags, links, and note text without starting Obsidian or configuring a model.

**Acceptance**: Changed notes appear after structural indexing; unchanged notes are skipped;
results provide a vault-relative path, matched signals, and a source hash.

### User Story 2 — Preserve Obsidian meaning (Priority: P1)

A person’s links, aliases, path-qualified links, heading and block references, embeds, nested
tags, checkboxes, frontmatter, and code fences remain understood as Markdown corpus signals.

### User Story 3 — Keep search current without blocking the user (Priority: P2)

A person sees search results reflect vault edits without manually rebuilding the whole index and can
continue through a long result set. When Semantic search is requested, its preparation happens in
the background rather than delaying structural search.

**Acceptance**: filesystem changes produce an SSE indexing notification; results paginate with a
stable offset; semantic embedding refresh runs only after it is requested.

## Functional Requirements

- The system MUST use vault-relative POSIX paths.
- The system MUST maintain SQLite FTS5 and dedicated folder/link metadata.
- The system MUST rank matching directory segments before FTS and fall back across directories.
- The system MUST not import AI or embedding libraries on startup or lexical search.
- The system MUST expose health, index, and search endpoints and a local browser shell.
- The system MUST watch the configured vault and notify connected clients after reindexing.
- The system MUST support `limit` and `offset` search pagination.
- The system MUST return result snippets, matched signals, and a source hash for each result.
- The system MUST start semantic embedding work lazily and in a background thread after a user
  explicitly requests semantic search.

## Success Criteria

- Structural search succeeds without a model endpoint or embedding runtime.
- Unchanged files are skipped by modification metadata.
- Warm structural search target is under 75 ms for the 1,000-note reference vault.
- Structural indexing target is under 3 seconds for the reference vault.
- The local server starts and accepts structural search without importing a provider or embedding
  runtime on its request hot path.
- Vault updates become observable to browser clients through the index event stream.

## Assumptions

Plain Markdown files are indexed; attachment content is not indexed. AI is a required product
capability elsewhere in LLM Wiki, while the structural retrieval fallback stays independent so
existing vault context remains available during provider or embedding outages.
