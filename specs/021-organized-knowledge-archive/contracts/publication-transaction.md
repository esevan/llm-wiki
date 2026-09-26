# Contract: Publication transaction and recovery

Publication is a journaled cross-store operation, not a single filesystem/SQLite transaction. All model work, inventory reads, parsing, and hashing finish before a short database write transaction begins.

## Preconditions

Apply requires:

- exact proposal ID/version/hash in `review_needed` state;
- exact feature 020 draft/content/generation snapshot heads;
- unchanged taxonomy snapshot or a recomputed proof that all apply-relevant inputs are identical;
- exact actual pre-hash for every existing target;
- absence of every reviewed-create path;
- exact managed-region and whole-file hashes;
- explicit user publication action and a unique operation ID.

A replay with the same operation ID and payload returns the same operation. A different payload conflicts.

## Ordered phases

1. **Journal**: in one short transaction, insert `PublicationOperation(approved)` and all planned steps with expected pre/post hashes. No file is changed before this commit.
2. **Stage**: through `MarkdownVaultAdapter`, create same-filesystem temporary bytes and recovery copies for existing files. Validate regular files, root containment, and hashes.
3. **Apply files**: rename/replace/create each artifact and managed patch with compare-and-swap semantics; flush files/directories as supported; record each verified step.
4. **Record**: in one short transaction, revalidate journal state, append pending archive revisions/path history/reference links, and enqueue or coalesce the exact `publication_index` job. Mark `recorded/index_pending`; do not move G's current pointer or feature 020's published pointer.
5. **Index**: feature 018 removes stale paths/revisions and upserts exact current bytes. The job's source revision bundle includes document IDs, paths, and file hashes. A save-time CAS rejects changed files.
6. **Complete**: only after index receipts match every expected current file hash, use one short compare-and-swap transaction to move G's current pointer and feature 020's published pointer, mark index state current, and mark the operation `complete/applied`.

An index receipt proves that the exact document revision is present in the structural/lexical
index (or removed for withdrawal). Optional semantic vectors may remain pending or unavailable,
with explicit partial-coverage status and feature 018 lexical fallback; missing model assets alone
do not prevent publication completion. Stale vectors never satisfy a structural receipt.

File success, record success, job execution, and application disposition are reported independently.

## Failure and restart matrix

| Observed state | Recovery action |
| --- | --- |
| Journal only, no staged/applied files | Safely retry staging or mark compensated |
| Staged recovery/temp files, target still pre-hash | Resume apply or remove operation-owned staging |
| Target equals exact post-hash, DB revision missing | Verify all steps and finish short record transaction |
| Some target equals post-hash, another equals pre-hash | Resume remaining exact step; compensate only if requested and all touched files still equal operation hashes |
| Target differs from both pre- and post-hash | Preserve target and recovery copy; mark `repair_required` |
| Pending revision recorded, index missing/failed | Keep applied pointers unchanged, mark `index_pending`/`index_failed`, retry only `publication_index`; compensate unchanged operation-owned bytes after an explicit terminal recovery choice |
| Index receipt hash differs from file | Do not mark current; enqueue fresh scan or require review according to external-edit state |
| Recovery copy differs from journaled hash | Preserve all bytes; mark `repair_required` |

Automatic transient index retries are bounded and restart-safe under feature 015 policy. Terminal failure exposes explicit retry. Optional organization proposal failure leaves the last reviewed Knowledge revision available and does not publish. Retrying indexing never rewrites publication files. Until complete, application projections render the last applied published revision from immutable records rather than treating candidate filesystem bytes as applied.

## Move, rename, supersede, and withdraw

These are new reviewed archive proposals using the same journal:

- **Move/rename**: verify source hash and absent/exact destination, move with stable document ID, append revision/path history, patch reviewed managed MOC/source links, and index remove+upsert.
- **Supersede**: preserve predecessor bytes/history unless a separate reviewed withdrawal is included; record exact successor relationships and refresh both index records.
- **Withdraw**: move exact current bytes into operation recovery storage, mark document withdrawn, repair reviewed managed links, and remove from active search. Recovery remains possible while hashes match.

Unmanaged inbound links are reported as unresolved repairs. The service never scans and rewrites every Markdown file silently.

## Adapter obligations

`MarkdownVaultAdapter` owns root containment, path normalization, symlink/non-file rejection, atomic stage/replace/move/withdraw, exact byte reads/hashes, and recovery-directory durability. Ports expose structured operations rather than filesystem paths to domain code. Platform-specific atomicity differences are contained and tested in the adapter.

## Retention

Recovery copies remain until operation completion plus the configured recovery window. Cleanup only removes operation-owned files whose hashes still equal journal records. `repair_required`, conflict, and externally edited recovery material is never automatically deleted.
