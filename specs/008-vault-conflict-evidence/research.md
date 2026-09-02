# Research: Evidence-Rich Vault Conflict Review

## Independent semantic retrieval

**Decision**: Score queries against all current document embeddings in SQLite.
**Rationale**: Lexical-first reranking cannot recover semantically related documents missing from FTS; the expected Vault scale permits a linear scan.
**Alternatives considered**: A vector database adds dependency and portability cost.

## Evidence granularity

**Decision**: Preserve bounded paragraph passages with 1-based line ranges and hashes.
**Rationale**: Paths/snippets are too short to verify contradictions; full documents exceed context budgets.

## Screening safety

**Decision**: Fail open into strong review. Only explicit, complete non-conflicts may be excluded.
**Rationale**: Screening must not create false negatives.

## Clear semantics

**Decision**: Incomplete embedding coverage or zero candidates is insufficient evidence; ultimate policy remains human-reviewed.
**Rationale**: This stops false clear without pretending the final threshold is settled.

## Cancellation

**Decision**: Server-owned events are checked between every phase and provider call.
**Rationale**: Browser abort alone leaves server work running.
