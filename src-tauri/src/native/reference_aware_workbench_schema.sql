CREATE TABLE work_previews (
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL UNIQUE REFERENCES refinement_sessions(id) ON DELETE CASCADE,
  subject_type TEXT NOT NULL CHECK(subject_type IN ('capture','task')), subject_id TEXT NOT NULL,
  current_version INTEGER NOT NULL DEFAULT 0 CHECK(current_version>=0), context_revision INTEGER NOT NULL DEFAULT 0,
  task_revision INTEGER, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE work_preview_versions (
  preview_id TEXT NOT NULL REFERENCES work_previews(id) ON DELETE CASCADE, version INTEGER NOT NULL CHECK(version>0),
  derivation_kind TEXT NOT NULL CHECK(derivation_kind IN ('generated','edited','restored','reconciled')),
  derived_from_version INTEGER, generation_job_id TEXT, task_revision INTEGER, context_revision INTEGER NOT NULL,
  locale TEXT NOT NULL CHECK(locale IN ('en','ko')), fields_json TEXT NOT NULL, content_hash TEXT NOT NULL,
  prompt_id TEXT NOT NULL DEFAULT '', prompt_version INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(preview_id,version), FOREIGN KEY(preview_id,derived_from_version) REFERENCES work_preview_versions(preview_id,version)
);
CREATE TABLE work_preview_assumptions (
  preview_id TEXT NOT NULL, version INTEGER NOT NULL, id TEXT NOT NULL, text TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('open','confirmed','rejected','superseded')),
  basis TEXT NOT NULL CHECK(basis IN ('user_context','source','model_inference')), source_claim_ids_json TEXT NOT NULL DEFAULT '[]',
  supersedes_id TEXT, PRIMARY KEY(preview_id,version,id),
  FOREIGN KEY(preview_id,version) REFERENCES work_preview_versions(preview_id,version) ON DELETE CASCADE
);
CREATE TABLE work_preview_references (
  id TEXT PRIMARY KEY, preview_id TEXT NOT NULL, version INTEGER NOT NULL, retrieval_request_hash TEXT NOT NULL,
  source_bundle_hash TEXT NOT NULL, document_id TEXT NOT NULL, document_version TEXT NOT NULL, section TEXT NOT NULL DEFAULT '',
  chunk_index INTEGER, metadata_json TEXT NOT NULL DEFAULT '{}', role TEXT NOT NULL, claim_ids_json TEXT NOT NULL DEFAULT '[]',
  FOREIGN KEY(preview_id,version) REFERENCES work_preview_versions(preview_id,version) ON DELETE CASCADE
);
CREATE TABLE work_preview_investigations (
  id TEXT PRIMARY KEY, preview_id TEXT NOT NULL REFERENCES work_previews(id) ON DELETE CASCADE, context_revision INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('speculative_search','conflict_review')), state TEXT NOT NULL,
  request_json TEXT NOT NULL, source_bundle_hash TEXT NOT NULL DEFAULT '', safe_error TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, finished_at TEXT
);
CREATE TABLE work_preview_findings (
  id TEXT PRIMARY KEY, investigation_id TEXT NOT NULL REFERENCES work_preview_investigations(id) ON DELETE CASCADE,
  priority TEXT NOT NULL CHECK(priority IN ('critical','supporting','ancillary')), summary TEXT NOT NULL,
  reference_ids_json TEXT NOT NULL, delivered_in_message_id TEXT, surfaced_at TEXT
);
CREATE TABLE refinement_document_mentions (
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES refinement_sessions(id) ON DELETE CASCADE, draft_revision INTEGER NOT NULL,
  document_id TEXT NOT NULL, document_version TEXT NOT NULL, section TEXT NOT NULL DEFAULT '', display_label TEXT NOT NULL,
  range_anchor_json TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE reference_interactions (
  id TEXT PRIMARY KEY, preview_id TEXT NOT NULL REFERENCES work_previews(id) ON DELETE CASCADE, context_revision INTEGER NOT NULL,
  document_id TEXT NOT NULL, document_version TEXT NOT NULL, section TEXT NOT NULL DEFAULT '', kind TEXT NOT NULL,
  use_scope TEXT NOT NULL DEFAULT '', reason TEXT NOT NULL DEFAULT '', preview_version INTEGER, message_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX work_preview_reference_owner ON work_preview_references(preview_id,version);
CREATE INDEX work_preview_investigation_current ON work_preview_investigations(preview_id,kind,context_revision,state);
CREATE INDEX refinement_mentions_draft ON refinement_document_mentions(session_id,draft_revision);
