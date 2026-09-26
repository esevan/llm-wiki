# Research: multi-aspect retrieval

## Current paths (inspected 2026-09-26)

- `native/vault.rs::index` writes `vault_documents`, FTS triggers, and one title + first
  4,000-character vector to `vault_document_embeddings`. Translation files are excluded.
- `native/vault.rs::search` first pages FTS results, then reranks only those with semantic similarity.
  This cannot discover a semantic-only candidate and pages before combined ranking.
- `adapters/vault/mod.rs::semantic_search` scans independently across current document embeddings,
  filters visibility, then issues ten-minute connection-scoped evidence grants. It rejects an
  incomplete semantic index instead of claiming freshness.
- `ports/vault_repository.rs` preserves lexical/semantic/evidence-read/publish domain boundaries.
- `native/semantic.rs` lazily loads the verified bundled local model and serializes inference through
  a mutex; no model download is required. It currently exposes no model identity method.
- `schema.sql` has path-keyed documents and embeddings, but no section identity, aspect, epistemic
  status, applicability, or embedding model identity. Foundation A supplies source/reference types;
  C supplies final-decision semantics. Actual implementation must merge those contracts first.

## Decisions

1. Add aspect-aware search units and vector identity rather than interpreting vector dimension as
   business purpose. Keep legacy lexical search usable while new units populate. No unnecessary new
   vector service or external dependency is justified for the current desktop corpus.
2. Parse explicit document metadata and heading-bounded passages deterministically. Missing final
   decisions/applicability are absent, not guessed during indexing. Generated F/G documents will
   emit canonical fields; existing notes retain body fallback. Invalid metadata must not hide body.
3. Independent lexical and semantic candidate collection precedes ranking/pagination. Use bounded
   weighted rank fusion with applicability strongest and exploration downweighted, with deterministic
   tie breaking. Exact coefficients require fixtures, not a product promise.
4. Preserve revision/hash/model/dimension compatibility and reject stale or malformed vectors.
   Reuse unchanged unit vectors by input hash. Compute models outside long SQLite write transactions,
   then apply only if source revision still matches.
5. Keep external evidence visibility and connection grants intact. Internal Workbench retrieval
   must use an adapter method with the same source-boundary protections, not a bypass through UI
   code or raw filesystem paths. Existing public result shapes remain compatible/additive.
6. A historical match resolves a valid successor chain with cycle/missing-target guards; include the
   reason and original matching source. Never relabel a missing successor or contradictory condition
   as a verified current answer. C/G metadata contracts define the relation source.
7. Scope: this feature indexes and retrieves; it does not generate applicability, create final
   decisions, move files, publish, or connect an external personal Vault.

## Validation targets

- Semantic-only applicability match with no lexical overlap; independent candidates and pagination.
- Two compatible model vectors with mismatched dimensions rejected; stale source vector excluded.
- Explicit applicability/decision/idea metadata and heading extraction, including Korean headings,
  fenced code, absent fields, malformed optional metadata, long bodies, and duplicate titles.
- Final successor preferred with source rationale; missing/cyclic successor safely marked unresolved.
- Existing scope/evidence grants and expected revision checks remain valid.
- No-model lexical fallback, unchanged-unit reuse, reindex after move/delete/publication.
- Existing warm lexical 75 ms and structural 1,000-note/10 MB index 3 s budgets; inference timed
  separately. Full inference over all units is not part of structural-index timing.

## Remaining implementation decisions

Assign the migration after A/B/C ownership is settled; define exact canonical metadata field names
with F/G and exported retrieval signatures with E before code. These are inter-feature contracts,
not unanswered user product questions. No general internet research was needed for these local
code decisions.
