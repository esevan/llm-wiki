# Application contract: organized archive

These operations use the existing Tauri `task-command` boundary. Preparation is
an explicit, awaited generation request with visible busy/error/retry in the
Knowledge review panel. It returns the frozen proposal, not HTTP 202 or a draft
that is silently applied. No database transaction remains open during the model
call. A completed request is replayable by operation ID; an interrupted request
can be retried explicitly. Durable publication/index recovery starts on Publish.

| Command | Input | Result |
| --- | --- | --- |
| `task-knowledge.archive-prepare` | `operationId`, `taskId`, `knowledgeRevision`, `expectedKnowledgeContentHash`, `expectedGenerationSnapshotHash`, `selectedIdeaRevisionIds: [{id, revision}]`, `locale` | Immutable proposal |
| `task-knowledge.archive-publish` | `operationId`, `proposalId`, `proposalVersion`, `proposalHash` | Operation status |
| `task-knowledge.archive-status` | `operationId` | Read-only operation status |
| `task-knowledge.archive-retry` | Existing publication `operationId` | Status plus newly queued index job |
| `task-knowledge.archive-recover` | Existing publication `operationId`, `choice: finish|compensate` | Operation status |
| `task-knowledge.archive-organize` | `operationId`, `documentId`, `expectedRevision` (actual file SHA-256), `intent: move|rename|withdraw|repair`, optional `requestedPath` | Immutable proposal |

A proposal contains `proposalId`, `proposalVersion`, `proposalHash`, `state`,
`taskId`, `knowledgeRevision`, `outcome`, `rationale`, frozen `input`,
`taxonomySnapshotHash`, `target: {documentId, path}`, `artifacts`, `mocPatches`,
`referenceLinks`, and `unresolvedConflicts`. Each artifact has `documentId`,
`path`, `kind`, exact `bytes` (null for removal), `sha256`, `expectedHash`, and
optional `ideaId`/`ideaRevision`. MOC patches expose exact before/after text.

Status contains `operationId`, `state`, `error`, `proposalId`, and
`proposalVersion`. States are `approved`, `index_pending`, `complete`,
`index_failed`, `repair_required`, or `compensated`. Only `complete` means exact
structural index receipts and publication pointers were committed together.
Optional semantic enrichment follows separately; lexical retrieval already works.

The legacy `task-knowledge.publish` delegates to the same reviewed proposal
boundary. Its old body/hash-only payload cannot bypass review. Legacy withdrawal
requires an organize proposal. Supersede is an organization generation outcome,
not a move: only matching confirmed decision topics receive successor IDs.

`conflict` proposals cannot publish. Stale source/taxonomy/target revisions,
malformed paths or managed regions, unresolved actual-use references, and external
edits are explicit errors. Missing actual-use source paths, revisions, or rationale
never silently drop a required citation. Compensation is allowed only while all
current bytes match an operation's exact before/after hashes; external bytes remain.
