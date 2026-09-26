# Implementation Plan: Organized Knowledge and idea archive

**Branch**: `021-organized-knowledge-archive` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/021-organized-knowledge-archive/spec.md`

## Summary

Add a reviewed archive proposal and recoverable publication boundary around the existing Knowledge publication flow. The service inventories the current Wiki and proposes `new`, `update`, `merge`, `conflict`, or `supersede` before choosing one canonical path. It freezes exact portable Markdown artifacts, selected idea files, managed MOC patches, and eligible source links for review. Explicit publication then executes those bytes through the native archive filesystem boundary with authoritative file-hash compare-and-swap, records stable document revisions in SQLite, and queues the existing durable `publication_index` operation. A durable journal makes file/database/index partial states idempotently recoverable without overwriting external edits.

## Technical Context

**Language/Version**: Rust 2021, TypeScript 5, React 19
**Primary Dependencies**: Existing Tauri command/application boundary, rusqlite, serde/serde_json, sha2, existing `MarkdownVaultAdapter`, feature 015 job/prompt/provenance contracts, feature 018 persisted retrieval index, feature 019 shared reference/version controls, and feature 020 Knowledge/idea revisions; no new dependency
**Storage**: Obsidian-compatible YAML + Markdown files and SQLite WAL; migration 22 is tentative and may be added only after integrated migrations 20 and 21 exist
**Testing**: Focused Rust unit/integration/migration tests, Vitest/React Testing Library, static contract checks, `git diff --check`; selected packaged desktop E2E only if cheaper implementation checks leave a concrete file/database lifecycle risk
**Target Platform**: Tauri desktop on macOS and Windows
**Project Type**: Desktop application with React UI and Rust native/application/domain layers
**Performance Goals**: Preserve warm search under 75 ms and 1,000-note/10 MB structural indexing under 3 seconds; render archive review projections within 100 ms p95 after data arrives; proposal AI and filesystem inventory are measured separately and never run on capture or search hot paths
**Constraints**: Offline-first, explicit publication only, exact reviewed bytes, actual file SHA-256 as revision authority, no write transaction during filesystem scan or AI work, no broad note rewrite, portable documents remain useful without the app, lexical search remains available when model services fail; at most eight retrieved passages and 6,000 retrieved-context tokens enter one proposal prompt
**Scale/Scope**: One local Wiki with at least 1,000 notes, one final Knowledge artifact plus explicitly selected idea artifacts per publication, multiple MOCs and source links, resumable operations across restart

## Constitution Check

*GATE: Passed before Phase 0 and re-checked after Phase 1 design.*

- **Product Spirit I–III**: Existing taxonomy and retrieved overlap are proposed automatically, while one review surface shows paths, patches, sources, and conflicts so users do not reconstruct context or classify every note manually.
- **Principles IV–V / authority**: Task completion never publishes. Final Knowledge and explicitly selected ideas remain separate, and only an explicit publish action applies a frozen proposal. Drafts and exploration remain private.
- **Principle VI**: The feature classifies information and provenance, never people or productivity.
- **Measured performance**: Inventory, proposal, publication, and indexing timings are instrumented separately. No archive code enters Capture or search hot paths. Existing search and index budgets remain binding.
- **Independent adapters**: Archive filesystem operations stay in the native adapter boundary; domain contracts do not perform IO. The existing `vault::replace_file` implementation provides platform replacement.
- **Evidence and consistency**: Only actual-use and counterevidence references become links. Every mutation is a reviewed structured patch guarded by exact hashes; source revision, declared provenance, and actual file hash remain distinct.
- **Local and cross-platform**: Relative POSIX-style logical paths are validated and converted by the adapter. Atomic replacement, directory sync, and recovery behavior receive macOS and Windows fixtures.
- **Minimal complexity**: Reuse feature 015 jobs/references, feature 018 identity/index, feature 019 controls, feature 020 revisions, and the existing publication routes. Migration 22 adds only archive identity/revision/provenance/journal records; it does not duplicate drafts or index units.
- **Migration safety**: Migration 22 is additive, preserves existing `knowledge_drafts` publication data, validates counts/foreign keys, and backfills lazily from actual files rather than rewriting them.

Post-design re-check: the contracts preserve all gates. The journal increases state only to make cross-store file/database/index operations reversible and externally editable without pretending they are one transaction.

## Design

### Proposal preparation

`native::knowledge_archive::prepare` receives an exact feature 020 Knowledge revision, explicit idea revision IDs, and their feature 015 reference facts. It reads the Wiki through the native archive filesystem boundary, uses feature 018 persisted units to locate overlapping current documents, and captures a taxonomy snapshot hash from existing directories, explicit metadata, aliases, tags, and MOCs. A dedicated versioned `knowledge_archive_proposal` prompt under the `user_generation` policy may recommend an outcome and canonical placement; `publication_index` remains the distinct post-write indexing operation. The prompt receives at most eight retrieved passages and 6,000 retrieved-context tokens, with taxonomy represented as bounded structured summaries. Deterministic validation owns paths, identities, eligible references, and patches.

The immutable proposal records one action per artifact, the existing document/hash when relevant, stable target identity, exact final bytes and their hash, and every managed MOC/link patch with before and after hashes. Existing taxonomy is preferred. A proposed new category requires a rationale and remains visible at review. `conflict` has no applicable artifact until the user resolves it. The proposal becomes stale when its source revision, taxonomy snapshot, target file hash, or managed-region hash changes.

### Exact portable artifact

Feature 020 hands over reviewed body and structured metadata. G deterministically serializes the entire publication artifact during proposal preparation: YAML front matter followed by the exact reviewed body bytes. The UI reviews that frozen artifact, not a reconstruction. Publication writes the stored bytes byte-for-byte and never invokes a model. YAML includes stable `document_id`, declared `source_revision`, `information_type`, status/applicability/decisions, tags, aliases, provenance, and an extension-compatible `sections` field only when typed sections later exist. The computed SHA-256 of the complete file is the authoritative revision.

Final Knowledge contains no exploration. Each selected unverified, deferred, rejected, or out-of-scope item becomes a separate `idea/` artifact with exact disposition and lower-trust retrieval metadata. Folder placement never substitutes for `information_type` or status.

### Links and MOCs

Only feature 019 actual-use facts whose source role is `support` or `counterevidence` produce source links. Each published link has a readable rationale and an Obsidian wikilink such as `[[relative/path#section|label]]`; SQLite retains exact source document ID, authoritative revision, optional section, role, and rationale. Viewed, mentioned, candidate, adopted-without-use, and excluded references do not become publication links.

MOC writes are bounded by unique markers:

```markdown
<!-- llm-wiki:document-id:start -->
...reviewed managed links and reading guidance...
<!-- llm-wiki:document-id:end -->
```

The proposal stores the whole-file hash and managed-region hash. Apply replaces only the reviewed region and verifies all bytes outside it remain identical. Missing markers may be inserted only at a reviewed location. Malformed, nested, duplicated, or changed markers cause conflict. Duplicate detection uses stable document identity/normalized target. User-authored text and arbitrary wikilinks outside managed regions are never rewritten silently.

### Publication and recovery

The existing feature 020 publish route remains the public boundary and delegates to `KnowledgeArchiveService`. Explicit apply verifies the proposal/source heads and creates a durable operation journal before filesystem mutation. File reads, hashing, proposal generation, and model inference occur outside a database write transaction. Each journaled step contains expected pre-hash, exact post-hash, target/recovery path, and action.

The adapter stages same-filesystem temporary files and preserves prior bytes under `.llm-wiki-recovery/<operation-id>/`. It applies artifacts and managed patches with compare-and-swap. A new file is removable during compensation only while its hash equals the journaled post-hash; an updated file is restorable only while the current hash equals that post-hash. Any intervening edit is preserved and marks `repair_required`. Once files are verified, a short database transaction records pending document revisions, path history, exact reference links, and `files_recorded` disposition; it does not advance G's current revision or feature 020's published pointer.

Indexing is a separate existing durable `publication_index` job with bounded automatic transient retries, restart recovery, and explicit retry after terminal failure. `index_pending` means exact candidate bytes and pending database revisions are durable, while the last applied published pointers remain unchanged. After feature 018 records the exact file revision, one short transaction advances G's current revision and feature 020's published pointer and marks the operation `complete/applied`. A terminal index failure retains explicit retry and may compensate operation-owned bytes only while their hashes are unchanged; it never claims publication was applied. File success, database recording, index execution, and application disposition remain independently observable.

On restart, recovery compares journal hashes with actual files and database revisions. It idempotently finishes recording/indexing when exact post-state exists, compensates only unchanged operation-owned bytes, or reports `repair_required` with preserved recovery copies. Move, rename, replacement, and withdrawal keep stable document ID, append a revision/path-history event, update managed inbound links through reviewed patches, and enqueue remove/upsert index work. Unmanaged inbound links are reported for review rather than broadly rewritten.

### UI ownership

G adds a focused `KnowledgeArchiveReview` region inside F's `KnowledgeReviewPanel` after a user selects an exact draft version. It shows outcome, canonical path, taxonomy/new-category rationale, final and selected idea artifact previews, tags/aliases, MOC diff, source-link rationales, unresolved link repairs, and publication/recovery/index state. It reuses E's `ReferenceViewer` and `DraftVersionControls`; it does not create another modal, version picker, or reference store. F owns generation and edit/version selection. E owns shared reference/version interaction. G owns archive proposal, explicit publish, path/MOC changes, and recovery UI. Integration is sequenced to avoid simultaneous edits to F's panel.

The visual hierarchy keeps the exact final artifact primary; path and organization form a compact summary, while MOC/source details disclose on demand. Conflict and `repair_required` states receive a persistent inline alert and explicit next action. `index_pending` states that candidate files are safely recorded but publication is not yet applied while indexing finishes. Async updates never steal focus, change the selected version, collapse an open diff, or scroll the review. Wide layouts may use a secondary review rail; narrow layouts stack content, organization, then provenance. All status meaning has text as well as color.

## Project Structure

### Documentation (this feature)

```text
specs/021-organized-knowledge-archive/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── application-api.md
│   ├── archive-proposal.md
│   ├── portable-markdown.md
│   └── publication-transaction.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/src/
├── domain/knowledge_archive.rs
├── application/knowledge_archive_service.rs
├── native/knowledge_archive.rs
├── native/migrations.rs
├── native/mod.rs
├── adapters/vault/mod.rs
├── ports/vault_repository.rs
└── workflow_foundation/
    ├── prompts/archive.rs
    └── prompts/mod.rs

src-tauri/tests/
├── knowledge_archive.rs
└── work_tracking_migrations.rs

frontend/src/
├── features/knowledge/
│   ├── KnowledgeArchiveReview.tsx
│   ├── KnowledgeArchiveReview.test.tsx
│   └── archiveTypes.ts
├── features/workbench/RefinementPanel.tsx
├── services/applicationClient.ts
└── styles.css

docs/features/
├── knowledge-archive.md
└── knowledge-archive.ko.md
```

**Structure Decision**: Extract publication organization and recovery from the large native task-assistance module into domain/application/native archive modules. Keep compatibility command dispatch thin, route all file access through the existing adapter, and add one G-owned UI component that F integrates after its panel lands.

## Dependency and migration gates

1. Feature 018 persistence/index contract and migration 19 are integrated before implementation begins.
2. E's shared `ReferenceViewer`/`DraftVersionControls` contract and migration 20 must exist before G UI integration; backend proposal work may proceed without the UI.
3. F's immutable Knowledge/idea revision contract and migration 21 must exist before final publication wiring and before migration 22 is added.
4. Migration 22 is added only after confirming the integrated schema head is exactly 21. A missing 20/21 blocks migration work rather than creating a gap.
5. F's panel integration is serialized: G owns `KnowledgeArchiveReview`; the F owner performs or coordinates the one import/render change in `KnowledgeReviewPanel` after both branches are integrated.

## Documentation impact

Implementation updates paired `docs/features/knowledge-archive.md` and `docs/features/knowledge-archive.ko.md` with the review/publish boundary, idea separation, portable metadata, conflict/recovery states, and index-pending behavior. This design phase does not claim the behavior has shipped and therefore does not add user-facing documentation.

## Complexity Tracking

No constitution violation requires an exception. The durable journal is the minimum mechanism that makes cross-store file/database/index operations reversible and externally editable without pretending they are one transaction.


## Resolved implementation boundary (2026-09-26)

C source-fidelity checkpoints and F schema 21 are integrated in this worktree;
G adds schema 22 additively. Native archive orchestration lives in
`src-tauri/src/native/knowledge_archive.rs`, typed artifacts/organization in
`domain/knowledge_archive.rs`, and full prompt semantics in
`workflow_foundation/prompts/knowledge_archive.rs`. The existing native command
boundary, database adapter, retrieval index, and cross-platform vault replacement
are reused. The draft plan's extra application service and port facade are not
needed to provide a second identical service boundary. Tests exercise real SQLite,
filesystem, and HTTP provider dispatch inside the native archive module.

Preparation is awaited explicit user generation, with visible errors and explicit
retry, rather than a new durable AI queue type. Essential publication is journaled
before writes and uses durable `publication_index`; startup recovery invokes no
model. This applies the program's different policies for requested generation and
important file/index work. Expensive taxonomy inventory is outside database write
locks. Targets are checked again for journaling and immediately before file apply.

Every matching superseded topic retains predecessor evidence and points to a real
new decision ID. Unrelated current topics survive. New MOCs receive portable stable
IDs; updates preserve user sections. Actual-use source blocks use the same managed
ID markers as MOCs so later moves can repair links without rewriting user text.
Implementation checkpoints are not final UI/convergence completion declarations.


## Resolved frontend integration

The archive region is implemented directly inside the F-owned
`KnowledgeReviewPanel.tsx`, with canonical types in `types/taskWorkbench.ts` and
commands in `services/taskClient.ts`. This avoids a second modal or duplicate
proposal owner. It reuses E's exact `ReferenceViewer`; exact full artifact bytes,
computed byte count, MOC before/after text, source hyperlinks, organization and
repair controls are part of the same persisted review. Corresponding mounted
coverage lives in `KnowledgeReviewPanel.test.tsx`. The earlier proposed
`features/knowledge/KnowledgeArchiveReview` paths describe the original split,
not additional missing UI. Final rendered and packaged checks remain separate.
