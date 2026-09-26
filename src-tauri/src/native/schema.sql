-- Migration 1 baseline. Keep immutable; append migrations in migrations.rs.
CREATE TABLE IF NOT EXISTS captures (
  id TEXT PRIMARY KEY,
  text TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS problems (
  id TEXT PRIMARY KEY,
  capture_id TEXT UNIQUE,
  statement TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL DEFAULT 'draft',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS features (
  id TEXT PRIMARY KEY,
  problem_id TEXT NOT NULL,
  title TEXT NOT NULL,
  outcome TEXT NOT NULL,
  non_goals TEXT NOT NULL DEFAULT '',
  conflict_state TEXT NOT NULL DEFAULT 'unknown',
  validation_criteria TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL DEFAULT 'proposed',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS solution_progress_entries (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  body TEXT NOT NULL DEFAULT '',
  image_data TEXT NOT NULL DEFAULT '',
  image_media_type TEXT NOT NULL DEFAULT '',
  image_summary TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS solution_progress_comments (
  id TEXT PRIMARY KEY,
  entry_id TEXT NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS solution_checklist_items (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  body TEXT NOT NULL,
  checked INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS approvals (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS conflict_reports (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  state TEXT NOT NULL,
  citation TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS conflict_review_runs (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  status TEXT NOT NULL,
  query TEXT NOT NULL,
  candidates_json TEXT NOT NULL DEFAULT '[]',
  report_json TEXT NOT NULL DEFAULT '{}',
  error TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS conflict_review_conflicts (
  storage_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  feature_id TEXT NOT NULL,
  conflict_id TEXT NOT NULL,
  target_id TEXT NOT NULL DEFAULT '',
  target_title TEXT NOT NULL DEFAULT '',
  severity TEXT NOT NULL DEFAULT 'medium',
  category TEXT NOT NULL DEFAULT 'Conflicting requirement',
  summary TEXT NOT NULL DEFAULT '',
  current_claim TEXT NOT NULL DEFAULT '',
  existing_claim TEXT NOT NULL DEFAULT '',
  impact TEXT NOT NULL DEFAULT '',
  recommendation TEXT NOT NULL DEFAULT '',
  evidence_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(run_id, conflict_id)
);
CREATE TABLE IF NOT EXISTS conflict_resolutions (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  feature_id TEXT NOT NULL,
  conflict_id TEXT NOT NULL,
  action TEXT NOT NULL,
  rationale TEXT NOT NULL DEFAULT '',
  resolved_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(run_id, conflict_id)
);
CREATE TABLE IF NOT EXISTS completions (
  id TEXT PRIMARY KEY,
  feature_id TEXT UNIQUE NOT NULL,
  evidence TEXT NOT NULL,
  report TEXT NOT NULL,
  knowledge_status TEXT NOT NULL DEFAULT 'pending',
  no_update_reason TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL DEFAULT 'draft',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS completion_reviews (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'ready',
  report_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS problem_completion_decisions (
  id TEXT PRIMARY KEY,
  problem_id TEXT NOT NULL,
  review_id TEXT NOT NULL DEFAULT '',
  reason TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS follow_up_links (
  problem_id TEXT PRIMARY KEY, source_feature_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS completion_playbooks (
  problem_id TEXT PRIMARY KEY, path TEXT NOT NULL, source_hash TEXT NOT NULL,
  lineage_snapshot_id TEXT NOT NULL DEFAULT '', lineage_version INTEGER NOT NULL DEFAULT 0,
  lineage_schema_version INTEGER NOT NULL DEFAULT 0, report_input_hash TEXT NOT NULL DEFAULT '',
  report_generation_status TEXT NOT NULL DEFAULT 'deterministic_fallback',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS lineage_snapshots (
  id TEXT PRIMARY KEY, feature_id TEXT NOT NULL, version INTEGER NOT NULL,
  schema_version INTEGER NOT NULL, source_hash TEXT NOT NULL, status TEXT NOT NULL,
  document_json TEXT NOT NULL DEFAULT '{}', inference_error TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(feature_id,version)
);
CREATE TABLE IF NOT EXISTS lineage_claims (
  id TEXT PRIMARY KEY, snapshot_id TEXT NOT NULL, claim_key TEXT NOT NULL,
  section TEXT NOT NULL, subject_type TEXT NOT NULL, subject_id TEXT NOT NULL,
  classification TEXT NOT NULL, confidence TEXT, material INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(snapshot_id,claim_key)
);
CREATE TABLE IF NOT EXISTS lineage_evidence (
  id TEXT PRIMARY KEY, claim_id TEXT NOT NULL, source_type TEXT NOT NULL,
  source_id TEXT NOT NULL, field_name TEXT NOT NULL, excerpt TEXT NOT NULL,
  source_hash TEXT NOT NULL, live_entity_type TEXT NOT NULL DEFAULT '',
  captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS lineage_revisions (
  id TEXT PRIMARY KEY, claim_id TEXT NOT NULL, supersedes_id TEXT,
  author_type TEXT NOT NULL, text TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '',
  is_current INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_lineage_snapshots_feature ON lineage_snapshots(feature_id,version DESC);
CREATE INDEX IF NOT EXISTS idx_lineage_claims_snapshot ON lineage_claims(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_lineage_evidence_claim ON lineage_evidence(claim_id);
CREATE INDEX IF NOT EXISTS idx_lineage_revisions_claim ON lineage_revisions(claim_id,is_current);
CREATE TABLE IF NOT EXISTS compass_goals (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS score_events (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  goal_id TEXT,
  points REAL NOT NULL,
  event_type TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS score_periods (
  period TEXT NOT NULL,
  goal_id TEXT,
  points REAL NOT NULL,
  PRIMARY KEY(period, goal_id)
);
CREATE TABLE IF NOT EXISTS importance_assessments (
  id TEXT PRIMARY KEY,
  problem_id TEXT UNIQUE NOT NULL,
  alignment INTEGER NOT NULL,
  impact INTEGER NOT NULL,
  urgency INTEGER NOT NULL,
  leverage INTEGER NOT NULL,
  evidence TEXT NOT NULL,
  importance INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS mirror_files (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  path TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  PRIMARY KEY(entity_type, entity_id)
);
CREATE TABLE IF NOT EXISTS patch_proposals (
  id TEXT PRIMARY KEY,
  feature_id TEXT NOT NULL,
  path TEXT NOT NULL,
  operation TEXT NOT NULL,
  heading TEXT NOT NULL,
  content TEXT NOT NULL,
  base_hash TEXT NOT NULL,
  before_text TEXT NOT NULL,
  proposed_text TEXT NOT NULL,
  reverse_text TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'proposed',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS deleted_entities (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  deleted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(entity_type, entity_id)
);
CREATE TABLE IF NOT EXISTS workbench_priorities (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  category TEXT NOT NULL DEFAULT 'General',
  attention_rank INTEGER NOT NULL DEFAULT 0,
  rationale TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(entity_type, entity_id)
);
CREATE TABLE IF NOT EXISTS workbench_priority_overrides (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  manual_priority INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(entity_type, entity_id)
);
CREATE TABLE IF NOT EXISTS workbench_category_overrides (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  category TEXT NOT NULL,
  PRIMARY KEY(entity_type, entity_id)
);
CREATE TABLE IF NOT EXISTS vault_documents (
  path TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  modified_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS vault_document_embeddings (
  path TEXT PRIMARY KEY,
  source_hash TEXT NOT NULL,
  dimensions INTEGER NOT NULL,
  vector BLOB NOT NULL,
  FOREIGN KEY(path) REFERENCES vault_documents(path) ON DELETE CASCADE
);
CREATE VIRTUAL TABLE IF NOT EXISTS vault_documents_fts USING fts5(
  path,
  title,
  body,
  content='vault_documents',
  content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS vault_documents_ai AFTER INSERT ON vault_documents BEGIN
  INSERT INTO vault_documents_fts(rowid,path,title,body)
  VALUES (new.rowid,new.path,new.title,new.body);
END;
CREATE TRIGGER IF NOT EXISTS vault_documents_ad AFTER DELETE ON vault_documents BEGIN
  INSERT INTO vault_documents_fts(vault_documents_fts,rowid,path,title,body)
  VALUES ('delete',old.rowid,old.path,old.title,old.body);
END;
CREATE TRIGGER IF NOT EXISTS vault_documents_au AFTER UPDATE ON vault_documents BEGIN
  INSERT INTO vault_documents_fts(vault_documents_fts,rowid,path,title,body)
  VALUES ('delete',old.rowid,old.path,old.title,old.body);
  INSERT INTO vault_documents_fts(rowid,path,title,body)
  VALUES (new.rowid,new.path,new.title,new.body);
END;
CREATE TABLE IF NOT EXISTS vault_search_units (
  unit_id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL,
  path TEXT NOT NULL REFERENCES vault_documents(path) ON DELETE CASCADE,
  title TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  declared_revision TEXT,
  section TEXT NOT NULL,
  chunk_index INTEGER NOT NULL CHECK(chunk_index >= 0),
  chunk_count INTEGER NOT NULL CHECK(chunk_count > 0),
  aspect TEXT NOT NULL CHECK(aspect IN ('applicability','decision','content','exploration')),
  information_type TEXT NOT NULL CHECK(information_type IN ('knowledge','idea')),
  status TEXT CHECK(status IS NULL OR status IN ('current','historical','unverified','deferred','rejected')),
  input_hash TEXT NOT NULL,
  text TEXT NOT NULL,
  unit_json TEXT NOT NULL,
  indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS vault_search_units_document
  ON vault_search_units(document_id,source_revision);
CREATE INDEX IF NOT EXISTS vault_search_units_path
  ON vault_search_units(path,source_revision);
CREATE INDEX IF NOT EXISTS vault_search_units_aspect
  ON vault_search_units(aspect,information_type,status);
CREATE VIRTUAL TABLE IF NOT EXISTS vault_search_units_fts USING fts5(
  unit_id UNINDEXED,
  document_id UNINDEXED,
  path,
  title,
  section,
  text,
  content='vault_search_units',
  content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS vault_search_units_ai AFTER INSERT ON vault_search_units BEGIN
  INSERT INTO vault_search_units_fts(rowid,unit_id,document_id,path,title,section,text)
  VALUES(new.rowid,new.unit_id,new.document_id,new.path,new.title,new.section,new.text);
END;
CREATE TRIGGER IF NOT EXISTS vault_search_units_ad AFTER DELETE ON vault_search_units BEGIN
  INSERT INTO vault_search_units_fts(vault_search_units_fts,rowid,unit_id,document_id,path,title,section,text)
  VALUES('delete',old.rowid,old.unit_id,old.document_id,old.path,old.title,old.section,old.text);
END;
CREATE TRIGGER IF NOT EXISTS vault_search_units_au AFTER UPDATE ON vault_search_units BEGIN
  INSERT INTO vault_search_units_fts(vault_search_units_fts,rowid,unit_id,document_id,path,title,section,text)
  VALUES('delete',old.rowid,old.unit_id,old.document_id,old.path,old.title,old.section,old.text);
  INSERT INTO vault_search_units_fts(rowid,unit_id,document_id,path,title,section,text)
  VALUES(new.rowid,new.unit_id,new.document_id,new.path,new.title,new.section,new.text);
END;
CREATE TABLE IF NOT EXISTS vault_search_unit_embeddings (
  unit_id TEXT NOT NULL REFERENCES vault_search_units(unit_id) ON DELETE CASCADE,
  source_revision TEXT NOT NULL,
  input_hash TEXT NOT NULL,
  model_id TEXT NOT NULL,
  model_version TEXT NOT NULL,
  dimensions INTEGER NOT NULL CHECK(dimensions > 0),
  vector BLOB NOT NULL,
  indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(unit_id,model_id,model_version)
);
CREATE INDEX IF NOT EXISTS vault_search_unit_embedding_identity
  ON vault_search_unit_embeddings(model_id,model_version,dimensions,source_revision,input_hash);
CREATE TABLE IF NOT EXISTS localized_content (
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  field_name TEXT NOT NULL,
  locale TEXT NOT NULL,
  value TEXT NOT NULL,
  origin TEXT NOT NULL DEFAULT 'ai',
  source_hash TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(entity_type, entity_id, field_name, locale)
);
CREATE INDEX IF NOT EXISTS idx_localized_content_entity
  ON localized_content(entity_type,entity_id,locale);
CREATE TABLE IF NOT EXISTS ai_runs (
  id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
  kind TEXT NOT NULL, input_text TEXT NOT NULL, output_text TEXT NOT NULL DEFAULT '',
  model TEXT NOT NULL DEFAULT '', source_revision TEXT NOT NULL DEFAULT '',
  context_scope TEXT NOT NULL DEFAULT '', retrieval_snapshot_hash TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS ai_jobs_v2 (
  id TEXT PRIMARY KEY, task_kind TEXT NOT NULL, entity_type TEXT NOT NULL DEFAULT '',
  entity_id TEXT NOT NULL DEFAULT '', status TEXT NOT NULL, input_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}', source_hash TEXT NOT NULL DEFAULT '', model TEXT NOT NULL DEFAULT '',
  execution_mode TEXT NOT NULL DEFAULT 'native', idempotency_key TEXT NOT NULL DEFAULT '',
  result_interface TEXT NOT NULL DEFAULT 'inline_preview', notification_policy TEXT NOT NULL DEFAULT 'none',
  progress_completed INTEGER NOT NULL DEFAULT 0, progress_total INTEGER NOT NULL DEFAULT 1,
  attempt INTEGER NOT NULL DEFAULT 0, worker_id TEXT NOT NULL DEFAULT '', lease_token TEXT NOT NULL DEFAULT '',
  lease_expires_at TEXT, heartbeat_at TEXT, available_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  error_code TEXT NOT NULL DEFAULT '', error_message TEXT NOT NULL DEFAULT '',
  prompt_id TEXT NOT NULL DEFAULT '', prompt_version INTEGER NOT NULL DEFAULT 0,
  source_revision TEXT NOT NULL DEFAULT '',
  execution_outcome TEXT NOT NULL DEFAULT 'pending'
    CHECK(execution_outcome IN ('pending','succeeded','failed','cancelled')),
  application_disposition TEXT NOT NULL DEFAULT 'pending'
    CHECK(application_disposition IN ('pending','review_needed','applied','superseded','rejected','not_applicable')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, started_at TEXT, finished_at TEXT
);
CREATE TABLE IF NOT EXISTS workflow_version_provenance (
  owner_type TEXT NOT NULL, owner_id TEXT NOT NULL, owner_version TEXT NOT NULL,
  source_owner_type TEXT, source_owner_id TEXT, source_owner_version TEXT,
  prompt_id TEXT NOT NULL DEFAULT '', prompt_version INTEGER NOT NULL DEFAULT 0,
  operation_id TEXT NOT NULL DEFAULT '', restored_from_version TEXT,
  source_references_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(owner_type,owner_id,owner_version),
  CHECK((source_owner_type IS NULL AND source_owner_id IS NULL AND source_owner_version IS NULL)
     OR (source_owner_type IS NOT NULL AND source_owner_id IS NOT NULL AND source_owner_version IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS workflow_provenance_operation
  ON workflow_version_provenance(operation_id) WHERE operation_id<>'';
CREATE TABLE IF NOT EXISTS workflow_document_references (
  id TEXT PRIMARY KEY, owner_type TEXT NOT NULL, owner_id TEXT NOT NULL,
  owner_version TEXT NOT NULL, document_id TEXT NOT NULL, document_version TEXT NOT NULL,
  section TEXT NOT NULL DEFAULT '', excerpt TEXT NOT NULL DEFAULT '', claim_id TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(owner_type,owner_id,owner_version,document_id,document_version,section,claim_id)
);
CREATE INDEX IF NOT EXISTS workflow_reference_owner
  ON workflow_document_references(owner_type,owner_id,owner_version);
CREATE TABLE IF NOT EXISTS notifications (
  id TEXT PRIMARY KEY, job_id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL,
  target_json TEXT NOT NULL, read_at TEXT, dismissed_at TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(job_id, kind)
);
