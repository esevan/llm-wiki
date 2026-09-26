# Implementation Plan: Multi-aspect Knowledge Retrieval

**Branch**: `018-multiaspect-knowledge-retrieval` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

## Summary

Replace document-level, lexical-first retrieval with explicit aspect extraction and a shared ranking
pipeline. Markdown remains the source of truth. The index records independently searchable
applicability, decision, content, and exploration units; lexical and semantic candidate collection
run independently before fusion, successor resolution, deduplication, and pagination. Missing or
invalid metadata never creates a decision and never removes the existing body fallback.

## Technical Context

**Language/Version**: Rust 2021 edition; TypeScript 5 for unchanged callers

**Primary Dependencies**: rusqlite/SQLite FTS5, fastembed, serde/serde_json, sha2, walkdir, and one
bounded YAML parser for standard Markdown front matter

**Storage**: Markdown Vault plus SQLite/WAL derived search index; schema allocation is deferred until
the shared A/B/C migration sequence is merged (D provisionally owns migration 19)

**Testing**: focused Rust unit and native integration tests; `cargo test --manifest-path
src-tauri/Cargo.toml`; `git diff --check`

**Target Platform**: packaged macOS and Windows desktop application, offline capable

**Project Type**: Tauri desktop application with Rust application/adapters and modular web UI

**Performance Goals**: preserve warm lexical retrieval below 75 ms and structural indexing of a
1,000-note/10 MB Vault below 3 seconds; time embedding inference separately

**Constraints**: at most eight returned passages and 6,000 retrieved-context tokens; no model work on
the lexical hot path; source revision is always the computed file-content hash; semantic failure
falls back to lexical results without claiming completeness

**Scale/Scope**: local personal Vault, initially about 1,000 Markdown notes; no broad attachment
indexing, external Vault connection, Knowledge generation, or publication UI

## Constitution Check

### Pre-design gate

- **Product Spirit III / V**: PASS. Results retain exact source identity, revision, passage, status,
  and historical lineage so users can resume with portable evidence. Exploration remains labelled.
- **A. Measured Performance**: PASS. Candidate collection is bounded; lexical behavior has a focused
  timing check; model inference is lazy and measured separately.
- **B. Independent Adapters**: PASS. Vault access remains behind `MarkdownVaultAdapter`; shared
  parsing and ranking are domain-neutral Rust modules.
- **C. Human Authority**: PASS. Retrieval never adopts, verifies, supersedes, or publishes content.
- **D. Evidence and Logical Consistency**: PASS. Only explicit metadata can create decision state;
  every result retains an exact computed revision and passage.
- **E. Local and Cross-Platform**: PASS. Paths remain normalized without platform-specific metadata
  behavior; SQLite remains the derived local index.
- **F. Minimal Complexity**: PASS with one dependency. A YAML parser avoids a false claim that a
  handwritten subset accepts ordinary YAML. It is limited to front matter deserialization and adds
  no service or runtime boundary.

### Post-design gate

PASS. The data model and contracts make invalidation, vector compatibility, fallback, conditions,
and limits explicit. No new visible workflow state, provider dependency, or publishing authority is
introduced. Shared schema work and A-owned API integration remain dependency-gated rather than
duplicated locally.

## Project Structure

### Documentation (this feature)

```text
specs/018-multiaspect-knowledge-retrieval/
├── spec.md
├── research.md
├── plan.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── knowledge-metadata.md
│   └── retrieval.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/retrieval.rs             # metadata, units, ranking, lineage safeguards
├── native/vault.rs                 # local indexing and internal search consumer
├── adapters/vault/mod.rs           # scoped retrieval and evidence grants
├── ports/vault_repository.rs       # stable repository boundary
└── native/schema.sql               # later shared migration integration only

src-tauri/tests/
└── retrieval.rs                    # contract and behavior fixtures
```

**Structure Decision**: Keep extraction and ranking as deterministic domain behavior shared by the
native and adapter entry points. Storage and scope/evidence enforcement remain in their current
native and adapter owners.

## Design

### Index and invalidation

The parser derives heading-bounded `SearchUnit` values from each file. Stable unit IDs hash the
document ID, aspect, and normalized section locator. `source_revision` is the actual SHA-256 content
hash; a declared revision is provenance only. Unit input hashes permit unchanged vectors to be
reused. Stored embeddings include model ID, model version, dimension, and content hash; incompatible
or stale rows are excluded.

Embedding inference happens outside long write transactions. The apply step checks that the current
document revision still matches the revision used to produce each vector. Deletion removes derived
units. A move preserves identity only when explicit stable `document_id` metadata is present.

### Candidate collection and ranking

Lexical FTS and semantic vector scans each produce their own bounded candidate set. Weighted
reciprocal-rank fusion combines those sets before deduplication or pagination. Applicability receives
the strongest aspect weight; decision and body content remain neutral; exploration is downweighted
from explicit information type/status, independent of folder names. Deterministic ties use document
ID, section, and unit ID. One best passage per document is primary while additional passages remain
bounded supporting evidence.

### Decision lineage

Only explicit confirmed-final/completion metadata yields a final decision. Decision nodes use C's
`adopted`, `superseded`, `withdrawn`, and `unresolved` states. A superseded match follows stable
successor IDs across documents. Missing targets, cycles, withdrawn/unresolved successors, or
condition mismatch stop redirection and return qualified historical evidence. Contradictory current
conclusions remain separate with their conditions.

### Public compatibility

Existing result fields and lexical/semantic operations remain available. Additive evidence fields
expose document ID, source revision, section, aspect, status, conditions, and matched historical
source. Adapter retrieval applies current visibility checks before ranking and issues the existing
connection-scoped evidence grants after ranking. Evidence reads continue to reject changed revisions.

## Implementation sequence

1. Land contracts, parser, unit identity, fusion, lineage resolution, and deterministic unit tests.
2. Wire shared candidate fusion into existing native and adapter reads without changing A-owned
   public operation registration or the shared schema.
3. After A/C integration, add allocated migration 19, unit/vector persistence, model identity, and
   exact-revision apply checks; then complete performance fixtures and public additive fields.

## Performance and token budgets

- Lexical candidate query: existing 75 ms warm budget.
- Structural extraction/index bookkeeping: existing 3 s for 1,000 notes / 10 MB, excluding model
  inference and reported separately from it.
- Candidate pools: bounded before materializing bodies; ranking is deterministic in memory.
- Output: no more than eight passages and an estimated 6,000 retrieved-context tokens.
- No AI prompt or remote token use occurs in retrieval or indexing.

## Complexity Tracking

No constitution violations require exceptions.
