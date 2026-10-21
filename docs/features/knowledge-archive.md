# Reviewed Knowledge archive

Publication review is available from both Workbench and the completed Task list in Compass. Preparing a review only proposes exact local vault files; publishing requires a separate explicit approval. No external storage connection is required.

Knowledge review focuses on the article, final outcomes, and future use. Sources,
ideas, version history, and draft tools are available in collapsed sections. The
Task review tab no longer runs a separate conflict review. Historical records are
preserved. Verification notes are private records of the exact version and do not
automatically revise the article or verify its claims.

Choose **Publish Knowledge**, check the prepared destinations, then explicitly
confirm publication. Cancel closes the confirmation without writing files. File
conflicts still block unsafe writes; this protection is independent of the removed
conflict review UI. Legacy drafts without evidence snapshots remain readable and
require a new draft before publication.

The expandable publication details show the exact
Markdown/YAML files, canonical paths, tags/aliases, separately selected idea files,
source links, and managed MOC changes. Generating the proposal does not write files.
Publish applies the exact reviewed bytes; a conflict or changed source requires a
fresh review. Existing categories are preferred, and new categories need a reason.

Final decisions stay in Knowledge. Deferred, rejected, unverified, and out-of-scope
ideas are separate `idea/` documents with conditions for revisiting them. Source
links retain the exact referenced version, section, and reason for using it. Merely
finding or viewing a reference does not make it a publication source.

After files are written, `index_pending` means the previous published pointer is
still authoritative. Publication becomes complete only after the exact files are
indexed. Index retry never regenerates the document. Recovery material is retained
locally; restarting can finish a journaled operation without calling a model.
External edits are preserved and may require repair. Compensation restores only
unchanged operation-owned bytes.

Reviewed move, rename, withdrawal, and repair preserve document identity/history
and update managed links. Unmanaged user links are reported instead of rewritten.
Supersession applies per decision topic; unrelated final decisions remain current.
Markdown and YAML remain readable without the app. Connecting a personal external
Obsidian Vault is outside this feature.

[Specification](../../specs/021-organized-knowledge-archive/spec.md)

Reopening review shows the stored operation state. An explicit index retry moves the same publication back to `index_pending`; enqueue failure remains `index_failed`. An operation requiring file repair must first be resolved through a recovery choice.

The compatibility publication command uses the same archive indexing lifecycle. Replaying a repaired or completed operation does not launch legacy translation or a second whole-vault indexing job.
