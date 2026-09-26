# Tasks: Multi-aspect Knowledge Retrieval

**Input**: Design documents in `specs/018-multiaspect-knowledge-retrieval/`

**Tests**: Required by the specification's independent tests and retrieval compatibility risks.

## Phase 1: Setup

- [X] T001 Add the bounded YAML front-matter dependency in `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`
- [X] T002 Register the retrieval domain module in `src-tauri/src/domain/mod.rs`

---

## Phase 2: Foundational

- [X] T003 Implement explicit metadata, heading-bounded unit extraction, stable IDs, source/input hashes, and vector compatibility in `src-tauri/src/domain/retrieval.rs`
- [X] T004 Implement independent candidate fusion, aspect/status weighting, deterministic deduplication, limits, and successor safeguards in `src-tauri/src/domain/retrieval.rs`
- [X] T005 Add parser, ranking, identity, vector validation, and lineage unit fixtures in `src-tauri/src/domain/retrieval.rs`

**Checkpoint**: Deterministic retrieval behavior is independently testable without schema or public API changes.

---

## Phase 3: User Story 1 - Find knowledge by when it is useful (Priority: P1)

**Goal**: Applicability-only semantic candidates can enter combined ranking without lexical overlap.

**Independent Test**: Merge disjoint lexical and semantic candidate lists and verify the explicit
applicability passage is returned first with document, revision, section, and aspect metadata.

- [X] T006 [US1] Replace lexical-first native reranking with independent candidate collection, shared fusion, and explicit internal/withdrawn-path exclusion in `src-tauri/src/native/vault.rs`
- [X] T007 [US1] Apply the same shared fusion after scope filtering while preserving evidence grants in `src-tauri/src/adapters/vault/mod.rs`
- [X] T008 [US1] Add shared semantic-only fixtures plus native/adapter lexical fallback, pagination, scope, and evidence-revision fixtures in `src-tauri/src/domain/retrieval.rs` and `src-tauri/tests/work_tracking_search.rs`

---

## Phase 4: User Story 2 - Follow historical choices to final decisions (Priority: P1)

**Goal**: A historical match resolves only to an explicit condition-compatible final successor.

**Independent Test**: Verify valid cross-document resolution and safe missing-target, cycle,
unresolved, withdrawn, and condition-mismatch outcomes.

- [X] T009 [US2] Expose qualified historical/successor metadata from the shared ranker through native and adapter result construction in `src-tauri/src/native/vault.rs` and `src-tauri/src/adapters/vault/mod.rs`
- [X] T010 [US2] Add current, missing, cyclic, and condition-mismatch integration fixtures in `src-tauri/src/domain/retrieval.rs`

---

## Phase 5: User Story 3 - Keep exploratory ideas useful but qualified (Priority: P2)

**Goal**: Explicit ideas remain discoverable but cannot outrank equally relevant applicable final evidence.

**Independent Test**: Rank an unverified idea and confirmed decision with equal candidate ranks;
verify the decision is primary and the idea retains its uncertainty metadata.

- [X] T011 [US3] Preserve additive information type/status/aspect fields in native and adapter hits in `src-tauri/src/native/vault.rs` and `src-tauri/src/adapters/vault/mod.rs`
- [X] T012 [US3] Add idea-status and folder-independence fixtures in `src-tauri/src/domain/retrieval.rs`

---

## Phase 6: Persistent Unit Index and Compatibility

**Dependency gate**: Begin after A/B/C migrations are merged and D's migration number is confirmed.

- [X] T013 Add the allocated persistent search-unit, decision lookup, and embedding-identity schema migration in `src-tauri/src/native/migrations/`
- [X] T014 Update changed/deleted units incrementally, reuse matching input hashes, and apply vectors only against unchanged source revisions in `src-tauri/src/native/vault.rs`
- [X] T015 Query persistent lexical units and compatible semantic units independently from both consumers in `src-tauri/src/native/vault.rs` and `src-tauri/src/adapters/vault/mod.rs`
- [X] T016 Add migration, restart, changed-revision, moved/deleted file, stale-vector, model/version/dimension, and source-hash apply fixtures in `src-tauri/src/native/vault.rs`

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T017 Update completed task state and shipped/deferred boundaries in `specs/018-multiaspect-knowledge-retrieval/tasks.md` and `docs/CONTINUATION.md`
- [X] T018 Run focused Rust tests, warm lexical/structural profiling checks, and whitespace validation from the dedicated worktree
- [X] T019 Create one convention-compliant implementation commit on `018-multiaspect-knowledge-retrieval`

---

## Dependencies & Execution Order

- Setup (T001-T002) blocks foundational implementation.
- Foundational work (T003-T005) blocks all user stories.
- US1 (T006-T008) establishes the real consumer pipeline used by US2 and US3.
- US2 (T009-T010) and US3 (T011-T012) extend the shared output and may proceed after US1.
- Persistent integration (T013-T016) follows A/B/C merge and D's migration allocation, then blocks
  full feature completion and polish.
- A-owned operation registration remains outside D; D integrates with its merged contract rather
  than creating a competing operation surface.

## Implementation Strategy

First land and test the deterministic parser/ranker, then wire it into both existing consumers while
preserving public shapes and scope/evidence grants. Then integrate durable per-unit tables, exact
revision invalidation, and model identity in the allocated shared migration after dependencies merge.
The feature is complete only after persistent integration and profiling pass; do not create a
competing schema or operation surface while waiting for the shared dependencies.
