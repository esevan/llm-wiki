# Research: Organized Knowledge and idea archive

## Existing boundaries

The current `task-knowledge.publish` and `task-knowledge.withdraw` paths live in `src-tauri/src/native/task_assistance.rs`. They already validate draft/source hashes and use `MarkdownVaultAdapter` for guarded atomic publication and recoverable withdrawal. The implementation also fixes the path under `Knowledge/Tasks/` and builds YAML in the command handler. G should extract organization and recovery behind a focused application service while preserving these routes as compatibility adapters. Creating another publication source would split authority and make recovery ambiguous.

Feature 018 persists document/search units and treats the SHA-256 of actual file bytes as the authoritative revision. Declared `source_revision` is provenance, not a replacement for that hash. Feature 020 owns immutable private Knowledge/idea revisions and exact reviewed body content. Feature 019 owns `ReferenceViewer`, `DraftVersionControls`, and actual-use interaction facts. Feature 015 owns durable job execution/application state and the `publication_index` operation policy.

## Decisions

### Freeze complete publication artifacts during review

- **Decision**: Deterministically serialize YAML, body, source-links section, and final newline into exact artifact bytes while preparing the proposal. Display and hash those bytes, then write those exact bytes on explicit apply.
- **Why**: “No regeneration at publication” must cover metadata and links as well as prose. Rebuilding YAML after review could change order, escaping, or content even without a model call.
- **Rejected**: Review only the body and assemble front matter at apply time. That weakens byte-level review and external-edit diagnosis.

### Prefer existing taxonomy with a versioned inventory

- **Decision**: Build a bounded inventory from current explicit metadata, directories, aliases/tags, MOCs, and feature 018 overlap results. Hash the normalized inventory. A model may recommend one of the five outcomes and a path through a registry-owned prompt, but deterministic validation requires an existing category or an explicit new-category rationale.
- **Why**: Folder names alone are not semantic truth, while asking users to manually classify every document adds cognitive load. Snapshot hashing prevents a proposal from silently applying after the Wiki changes.
- **Rejected**: A global taxonomy table as a second canonical source. The portable Wiki remains canonical.

### Keep identities independent from paths

- **Decision**: Stable `document_id` identifies a logical Knowledge/idea document. Every publish, move, rename, replacement, supersession, or withdrawal appends a publication revision and a path-history fact. Current path is a pointer, not identity.
- **Why**: Wikis evolve; path identity would break exact references and make rename indistinguishable from a new note.
- **Rejected**: Derive IDs from path or title.

### Store exact source linkage internally and render portable wikilinks

- **Decision**: Persist document ID, authoritative source revision, optional section, role, rationale, and rendered target for each eligible link. Render only actual-use and counterevidence provenance as wikilinks in the artifact.
- **Why**: Wikilinks remain useful outside the app; internal bindings remain exact across rename and duplicate titles.
- **Rejected**: Publish every retrieved/viewed source. Retrieval is not use, and such links would falsely imply evidentiary support.

### Bound MOC ownership with reviewed marker regions

- **Decision**: Give each managed MOC region stable start/end markers. Proposals capture whole-file and region hashes plus exact replacement bytes. Apply rejects malformed or changed markers and preserves every byte outside the region.
- **Why**: A managed block can remain useful while respecting user-authored introductions, ordering, commentary, and unrelated links.
- **Rejected**: Regenerate the whole MOC or search-and-replace arbitrary wikilinks.

### Use a write-ahead journal across filesystem, database, and index

- **Decision**: Persist an operation and exact planned file steps before mutation. Use same-filesystem staging/recovery copies, short database transactions after file IO, and the existing durable `publication_index` job after database recording. Recover by comparing expected pre/post hashes.
- **Why**: SQLite cannot atomically commit with filesystem renames or index computation. A journal makes each boundary observable and idempotent without holding a database write lock during scan, inference, or IO.
- **Rejected**: Keep one SQLite transaction open across file writes. It still cannot roll back filesystem effects and would block ordinary Capture writes.

### Preserve external edits over automatic compensation

- **Decision**: Compensate only when the current file exactly matches the operation-owned post-hash. Otherwise preserve current and recovery bytes and report `repair_required`.
- **Why**: An external editor may win any race. Recovery must never turn its own stale plan into authority over newer user content.
- **Rejected**: Force restore from backup after a mismatch.

### Separate publication success from index freshness

- **Decision**: Treat file application, pending database recording, indexing, and pointer application as distinct phases. Exact files and pending revisions can be durably `index_pending`, but G's current revision and feature 020's published pointer advance only after an exact index receipt. Bounded automatic retries and restart recovery apply to the index job; explicit retry never rewrites unchanged files.
- **Why**: Search is rebuildable, but F's publication pointer must never claim an unindexed revision. Separating execution from application prevents duplicate writes and false completion.
- **Rejected**: Roll back correct files solely because indexing failed.

### Keep ideas separate and lower-trust

- **Decision**: Publish only explicitly selected idea revisions as separate artifacts beneath an `idea/` canonical area. Preserve exact `unverified`, `deferred`, `rejected`, or `out_of_scope` disposition; map top-level retrieval status only for feature 018 compatibility.
- **Why**: Ideas can remain valuable without becoming established conclusions or contaminating final Knowledge.
- **Rejected**: Append an “Ideas” section to final Knowledge.

### Extend metadata for future typed sections without implementing mixed content

- **Decision**: Reserve optional `sections[]` entries with stable section ID, information type, status, and source bindings. G v1 writes whole-document Knowledge or idea files and reads explicit whole-document metadata first.
- **Why**: Future mixed documents need explicit section semantics, while shipping that authoring/indexing model now would expand scope.
- **Rejected**: Infer section type from headings or folder names.

### Reuse the E/F review surfaces

- **Decision**: Add one G-owned archive review component within F's panel. Reuse E's reference viewer and version controls and existing shared tokens. Keep content primary, organization second, provenance/details progressive.
- **Why**: Users need one coherent decision surface, and shared controls already preserve focus, navigation, and exact references.
- **Rejected**: A separate archive screen with duplicate version/reference state.

## Performance and measurement

- Inventory and proposal AI are off hot paths and timed separately.
- Proposal validation is linear in the bounded proposed artifacts/patches/references.
- Apply performs no model call and hashes each affected file once per validation phase where practical.
- Archive projection render is profiled against the 100 ms p95 local projection budget with long paths, multiple ideas, MOC diffs, and 1,000 references.
- Feature 018's 75 ms warm lexical and 3 second structural-index budgets remain unchanged. Index inference time is measured separately.

## Dependency result

Feature 018 and migration 19 are integrated. Feature 019 migration 20 is integrated per the program ledger update, while its reusable UI contract remains a wiring gate. Feature 020 and migration 21 remain a publication-wiring gate. G reserves migration 22 and must verify the integrated schema head is exactly 21 before adding it. Pending feature 017 source-fidelity fixes do not block this design and must not be assumed complete by implementation fixtures.

## Documentation result

The repository documentation guide requires paired English/Korean user documentation when behavior ships. This phase changes only design artifacts, so it intentionally does not add user-facing release documentation.
