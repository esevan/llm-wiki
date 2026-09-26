# Quickstart: Organized Knowledge and idea archive

This is the implementation and acceptance guide. Native checks below were run in the dedicated G worktree; final UI/integrated checks remain separately recorded.

## Dependency gate

1. Confirm feature 018 migration 19 and persistent search/index contracts are present.
2. Confirm feature 019 migration 20 plus reusable `ReferenceViewer` and `DraftVersionControls` are present.
3. Confirm feature 020 migration 21, immutable Knowledge/idea revisions, exact publication handoff, and `KnowledgeReviewPanel` are present.
4. Read SQLite `user_version`; add G as migration 22 only when it is exactly 21. Never create a migration gap.
5. Confirm feature 017 source-fidelity fixes used by feature 020 are integrated; do not substitute inferred evidence in fixtures.

## Focused verification scenarios

### Existing taxonomy and overlap

Seed an existing current article with the same decision and conditions plus a relevant MOC. Prepare an archive proposal from a final Knowledge revision. Verify the outcome is `update`, `merge`, or `conflict` as the evidence requires, rather than a duplicate `new`; the existing category is preferred; and any proposed new category has a visible rationale. Change the target file after preparation and verify apply returns conflict without modifying it.

The organization prompt receives at most eight ranked overlap passages and 6,000
estimated passage tokens, using the shared four-characters-per-token estimate. The
bounded context retains each passage and its exact document, revision, path, and section
identity without truncation. Complete document bodies used by the source viewer do not
enter the proposal prompt.

### Exact final and idea artifacts

Prepare one final revision and four selected idea revisions covering unverified, deferred, rejected, and out-of-scope states. Verify the final artifact body is byte-equivalent to the reviewed final body and contains no idea passage. Verify each selected idea is a separate artifact with exact disposition and lower-trust metadata; unselected ideas produce no file. Apply and compare actual full-file SHA-256 to the proposal artifact hashes.

### Source eligibility and wikilinks

Provide viewed, mentioned, used-support, used-counterevidence, adopted, and excluded facts. Verify only used-support and counterevidence become portable wikilinks, each with rationale. Verify internal rows retain exact document ID/revision/section and remain resolvable after a reviewed source rename. Duplicate titles must not alter the binding.

### Managed MOC preservation

Create a MOC with user text before/after one valid managed region. Apply an exact patch and verify every byte outside the region is unchanged and the link is not duplicated. Repeat with changed region bytes, malformed/nested/duplicate markers, and a changed whole-file prefix; each must preserve the current file and return conflict.

### Restart and partial recovery

Inject process interruption after: journal commit, first file write, all file writes, database record, and index enqueue. Reopen the application and verify exact pre/post hashes cause an idempotent resume. Introduce an external edit after a partial write and verify it is preserved, the recovery copy remains, and state becomes `repair_required`. Replaying the same operation never duplicates documents, revisions, paths, links, or jobs.

### Index lag and model outage

Force `publication_index` transient and terminal failures. Verify durable files and database revisions remain published as `index_pending`/`index_failed`, bounded retries survive restart, and explicit retry performs only indexing. Lexical search remains available and never reports “no knowledge” solely because semantic/model service is unavailable. A successful retry indexes the exact authoritative file hash.

### Move, rename, supersede, and withdraw

For each action, review and apply a proposal. Verify stable document ID, immutable new revision/path history, managed inbound link and MOC repair, index remove/upsert, and exact external-edit guards. Arbitrary links outside managed regions must remain unchanged and appear as unresolved repair items. Withdrawal moves bytes to recovery and removes active search visibility without permanent deletion.

### Portable metadata and typed-section compatibility

Open final and idea files without the application and verify readable YAML/Markdown and Obsidian wikilinks. Move a typed file outside its expected folder and verify search trust follows metadata, not the folder. Parse an artifact containing future-compatible typed `sections[]` and verify unknown supported fields are preserved without inferring types from headings.

### Migration

Test fresh bootstrap through 22, populated schema 21 to 22, second open, and migration rollback. Preserve existing Knowledge publication paths/hashes/body bytes and all row counts/foreign keys. First-access adoption reads and hashes the file without rewriting it; a mismatch becomes repair-required.

### UI and accessibility

Render wide and narrow layouts in English and Korean with long paths/titles, multiple idea artifacts, MOC/source diffs, conflict, stale, writing, index-pending, index-failed, repair-required, and complete states. Verify keyboard order, visible focus, semantic headings, dialog reuse, status text independent of color, reduced motion, and stable version/reference/focus/scroll state during async updates. Confirm only explicit Publish applies the proposal.

## Performance evidence

Use representative 1,000-note/10 MB fixtures and log separate elapsed values for taxonomy inventory, model proposal, deterministic validation, file apply, database record, feature 018 structural index, and UI projection render. Keep existing structural index below 3 seconds, warm lexical search below 75 ms, and local projection render below 100 ms p95. Model/inference timing is reported separately and must not weaken those gates.

## Commands

Run focused commands separately so failures remain attributable:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib knowledge_archive
cargo test --manifest-path src-tauri/Cargo.toml --lib archive
cargo test --manifest-path src-tauri/Cargo.toml --test work_tracking_migrations
npm exec vitest -- run frontend/src/features/workbench/KnowledgeReviewPanel.test.tsx
npm run typecheck
git diff --check
```

Run only the smallest packaged desktop scenario if adapter-level crash/restart or native/UI lifecycle risk remains after focused tests. Record the exact residual risk and selected scenario. A full E2E suite or release build is not a default completion check.

## Documentation

After implementation, update `docs/features/knowledge-archive.md` and `docs/features/knowledge-archive.ko.md`, link them from both feature indexes, and describe explicit publication, idea separation, exact hash conflicts, recovery, and index-pending behavior. This design phase makes no shipped-behavior claim.


## Native checkpoint evidence

`cargo test --manifest-path src-tauri/Cargo.toml --lib archive -- --nocapture`:
16 passed; includes migration 21→22 preservation/rollback and real provider HTTP
request/response preparation. The archive overlap regression verifies 8-passage/6,000
estimated-token enforcement, exact source preservation, and removal of full viewer bodies
from provider input. 1,001-note/~10 MB inventory: 1,569 ms. No model latency is included
in that inventory number. Recovery tests cover each partial move-file boundary and
external edits before/after publication; actual process kill and packaged native/UI
continuity remain final integration checks.

F owns the archive review consumer. Main is not yet merged; no release build,
installation, or packaged E2E has been claimed for this checkpoint. Prior raw
body-only publish API tests must be updated to require the reviewed proposal.

### Final selected packaged receipt

`task-publication` passed on signed main `3b07837`; artifacts: `.tmp/desktop-e2e-artifacts-qZOCVD`. Exact reviewed artifact/index receipt, explicit withdrawal, retained private history and reopen passed. The archive budget regression also passed in the 16-test native archive suite.

The three selected flows address concrete native background, modal/adoption and
file/index risks; the full suite was not needed. Later source changes were
refinement-harness-only. The installed app and personal Vault were not changed.
Latest direct visual review remains unavailable on the locked Mac; mounted or
packaged assertions do not substitute for that visual state matrix.
