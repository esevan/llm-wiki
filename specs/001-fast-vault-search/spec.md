# Feature Specification: Fast Vault Search

**Feature Branch**: `001-fast-vault-search`
**Created**: 2026-08-18
**Last Reconciled**: 2026-09-02
**Status**: Current behavior reconciled — confirmed search and font changes pending

## User Scenarios & Testing

### User Story 1 — Search an existing Vault (Priority: P1)

A user searches Markdown notes by path, title, headings, tags, aliases, and text without opening
Obsidian or configuring an AI provider.

**Independent Test**: Index a Vault, search for content stored in different Markdown signals, and
verify that each result identifies the matching document and its current source revision.

**Acceptance Scenarios**:

1. **Given** indexed Markdown notes, **When** the user searches for a matching path, heading, tag,
   alias, or body term, **Then** matching notes are returned with a snippet, match signals, and source
   hash.
2. **Given** no match in the highest-ranked directory, **When** matching content exists elsewhere,
   **Then** search falls back across directories.
3. **Given** a long result set, **When** the user requests another offset, **Then** the next bounded
   page is returned.

### User Story 2 — Preserve Markdown and Obsidian meaning (Priority: P1)

Links, aliases, headings, nested tags, embeds, heading and block references, checkboxes,
frontmatter, and code fences remain usable as retrieval signals.

### User Story 3 — Keep search current (Priority: P2)

Vault changes become searchable without a full manual rebuild. Structural search remains usable
while semantic coverage is incomplete or unavailable.

**Acceptance Scenarios**:

1. **Given** a changed or deleted note, **When** the Vault watcher or an explicit index request runs,
   **Then** the structural index and source hashes reflect the current Vault.
2. **Given** a structural index refresh, **When** semantic coverage is missing or stale, **Then**
   embedding work is scheduled in the background and lexical results remain available.
3. **Given** the user requests semantic search, **When** current embeddings exist, **Then** the
   complete semantic corpus is searched independently of lexical candidates; if semantic search is
   unavailable, structural search remains usable.

### Edge Cases

- Empty queries are rejected; result count and offset are bounded.
- Unchanged files are skipped by modification metadata.
- Deleted documents are removed from structural and semantic coverage.
- Derived Korean reading files and `.obsidian` content are excluded from the canonical index.
- Missing semantic dependencies do not disable structural search.
- Attachment contents are not indexed.

## Functional Requirements

- **FR-001**: The system MUST represent indexed documents with Vault-relative paths.
- **FR-002**: Search MUST consider directory segments, filenames, titles, text, headings, tags,
  aliases, frontmatter-derived content, and Markdown links.
- **FR-003**: Search MUST rank matching directory segments before full-text results and MUST fall
  back across directories when the routed search finds no result.
- **FR-004**: Results MUST expose a title, Vault-relative path, snippet, matching signals, and source
  revision hash.
- **FR-005**: Search MUST support bounded result count and non-negative offset pagination.
- **FR-006**: Startup, explicit refresh, and detected Vault changes MUST update the structural index
  without reparsing unchanged notes.
- **FR-007**: Connected clients MUST be able to observe a completed watcher-driven index refresh.
- **FR-008**: Missing or stale semantic representations MUST be scheduled as visible background work
  after startup, explicit indexing, and detected Vault changes.
- **FR-009**: Lexical search MUST remain usable before semantic coverage completes and when semantic
  preparation fails.
- **FR-010**: User-selected semantic search MUST search the complete current semantic corpus
  independently of lexical candidates and MUST return readable source-bound results.
- **FR-011**: Canonical indexing MUST exclude generated Korean reading files, application metadata,
  and non-Markdown attachments.
- **FR-012**: The local application MUST expose a browser shell, health state, explicit index
  refresh, structural/semantic search, and an index-change event stream without requiring an AI
  provider for structural operation.

### Key Entities

- **Indexed document**: One canonical Markdown note, its searchable signals, modification identity,
  and source hash.
- **Search result**: One matching document with its readable snippet, match provenance, score, and
  source revision.
- **Semantic coverage**: The number of current indexed documents with a valid semantic
  representation.

## Success Criteria

- **SC-001**: All structural search acceptance tests pass without an AI provider or semantic
  runtime.
- **SC-002**: A changed or deleted note is reflected after the next completed index refresh.
- **SC-003**: Lexical results remain available in every tested state of semantic preparation,
  failure, and partial coverage.
- **SC-004**: Result pagination returns no more than the requested bounded page size and preserves
  the requested offset.
- **SC-005**: The 1,000-note reference Vault completes structural indexing within the project
  performance budget.

## Assumptions

- The application operates on a local Markdown Vault.
- Semantic search supplements structural retrieval; it is not required for a usable lexical search
  result.
- The watcher reports completed index refreshes rather than every raw filesystem event.

## Confirmed Implementation Gaps

- **IG-010 — Independent semantic search**: The current Search screen reranks only lexical
  candidates. It must be changed to search the complete semantic corpus when Semantic is selected.
- **IG-021 — Local font bundle**: The current browser shell requests Google Fonts. Required font
  files and used weights must be bundled with the application and loaded locally without a runtime
  font-network request. System fonts are fallback faces only. Bundled font licenses must permit
  redistribution.
