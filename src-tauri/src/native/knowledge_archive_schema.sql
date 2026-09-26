CREATE TABLE knowledge_archive_proposals (
 proposal_id TEXT PRIMARY KEY, proposal_version INTEGER NOT NULL DEFAULT 1,
 request_id TEXT NOT NULL UNIQUE, request_hash TEXT NOT NULL,
 task_id TEXT NOT NULL, knowledge_revision INTEGER NOT NULL,
 proposal_hash TEXT NOT NULL, outcome TEXT NOT NULL, payload_json TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'review_needed', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE knowledge_archive_operations (
 operation_id TEXT PRIMARY KEY, proposal_id TEXT NOT NULL REFERENCES knowledge_archive_proposals(proposal_id),
 proposal_version INTEGER NOT NULL, payload_hash TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'approved', error TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX knowledge_archive_one_apply ON knowledge_archive_operations(proposal_id,proposal_version);
CREATE TABLE knowledge_archive_steps (
 operation_id TEXT NOT NULL REFERENCES knowledge_archive_operations(operation_id), ordinal INTEGER NOT NULL,
 document_id TEXT NOT NULL, path TEXT NOT NULL, kind TEXT NOT NULL,
 before_hash TEXT, after_hash TEXT, before_bytes BLOB, after_bytes BLOB,
 state TEXT NOT NULL DEFAULT 'planned', PRIMARY KEY(operation_id,ordinal)
);
CREATE TABLE knowledge_archive_revisions (
 document_id TEXT NOT NULL, revision TEXT NOT NULL, path TEXT NOT NULL, kind TEXT NOT NULL,
 body TEXT NOT NULL, operation_id TEXT NOT NULL REFERENCES knowledge_archive_operations(operation_id),
 withdrawn INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 PRIMARY KEY(document_id,revision)
);
CREATE TABLE knowledge_archive_current (
 document_id TEXT PRIMARY KEY, revision TEXT NOT NULL, path TEXT NOT NULL COLLATE NOCASE UNIQUE,
 kind TEXT NOT NULL, withdrawn INTEGER NOT NULL DEFAULT 0,
 FOREIGN KEY(document_id,revision) REFERENCES knowledge_archive_revisions(document_id,revision)
);
CREATE TABLE knowledge_archive_links (
 operation_id TEXT NOT NULL, source_document_id TEXT NOT NULL, target_document_id TEXT NOT NULL,
 target_revision TEXT NOT NULL, section TEXT NOT NULL, role TEXT NOT NULL, rationale TEXT NOT NULL,
 PRIMARY KEY(operation_id,source_document_id,target_document_id,target_revision,section)
);
