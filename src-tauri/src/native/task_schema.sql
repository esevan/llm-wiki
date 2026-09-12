-- Schema 8 Task aggregate.  This is intentionally separate from the immutable v1 schema.
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY, origin_capture_id TEXT REFERENCES captures(id), current_revision INTEGER NOT NULL DEFAULT 1,
  state TEXT NOT NULL DEFAULT 'task' CHECK(state IN ('task','in_progress','completed')),
  archived_at TEXT, category TEXT NOT NULL DEFAULT 'General', created_at TEXT NOT NULL,
  last_user_activity_at TEXT NOT NULL, started_at TEXT, completed_at TEXT, reopened_at TEXT
);
CREATE TABLE IF NOT EXISTS task_revisions (
  task_id TEXT NOT NULL REFERENCES tasks(id), revision INTEGER NOT NULL,
  title TEXT NOT NULL, detail TEXT NOT NULL DEFAULT '', outcome TEXT NOT NULL DEFAULT '', scope TEXT NOT NULL DEFAULT '',
  non_goals TEXT NOT NULL DEFAULT '', validation_criteria TEXT NOT NULL DEFAULT '', content_hash TEXT NOT NULL,
  author TEXT NOT NULL DEFAULT 'user', source_reference TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL,
  PRIMARY KEY(task_id,revision)
);
CREATE TABLE IF NOT EXISTS problem_revisions (
  problem_id TEXT NOT NULL REFERENCES problems(id), revision INTEGER NOT NULL, statement TEXT NOT NULL,
  detail TEXT NOT NULL DEFAULT '', content_hash TEXT NOT NULL, author TEXT NOT NULL DEFAULT 'user', created_at TEXT NOT NULL,
  PRIMARY KEY(problem_id,revision)
);
CREATE TABLE IF NOT EXISTS task_problem_links (
  id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), problem_id TEXT NOT NULL REFERENCES problems(id),
  problem_revision INTEGER NOT NULL, relationship TEXT NOT NULL DEFAULT 'context', note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL, unlinked_at TEXT, CHECK(relationship IN ('context','contributes_to','blocks'))
);
CREATE UNIQUE INDEX IF NOT EXISTS active_task_problem_link ON task_problem_links(task_id,problem_id,problem_revision,relationship) WHERE unlinked_at IS NULL;
CREATE TABLE IF NOT EXISTS task_relationships (
  id TEXT PRIMARY KEY, source_task_id TEXT NOT NULL REFERENCES tasks(id), target_task_id TEXT NOT NULL REFERENCES tasks(id),
  kind TEXT NOT NULL CHECK(kind IN ('prerequisite','split_from','related')), note TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL, unlinked_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS active_task_relationship ON task_relationships(source_task_id,target_task_id,kind) WHERE unlinked_at IS NULL;
CREATE TABLE IF NOT EXISTS task_work_log_entries (
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), body TEXT NOT NULL DEFAULT '', image_data TEXT NOT NULL DEFAULT '', image_media_type TEXT NOT NULL DEFAULT '', image_summary TEXT NOT NULL DEFAULT '', source_legacy_id TEXT, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS task_work_log_comments (id TEXT PRIMARY KEY, entry_id TEXT NOT NULL REFERENCES task_work_log_entries(id), body TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_attachments (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), entry_id TEXT REFERENCES task_work_log_entries(id), name TEXT NOT NULL DEFAULT '', media_type TEXT NOT NULL DEFAULT '', data TEXT NOT NULL DEFAULT '', byte_hash TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_checklist_items (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), body TEXT NOT NULL, checked INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_decisions (id TEXT PRIMARY KEY, task_id TEXT REFERENCES tasks(id), task_revision INTEGER, kind TEXT NOT NULL, payload_json TEXT NOT NULL DEFAULT '{}', operation_id TEXT NOT NULL, created_at TEXT NOT NULL, UNIQUE(operation_id));
CREATE TABLE IF NOT EXISTS task_completions (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), task_revision INTEGER NOT NULL, evidence TEXT NOT NULL, report TEXT NOT NULL DEFAULT '', operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS problem_resolution_decisions (id TEXT PRIMARY KEY, problem_id TEXT NOT NULL REFERENCES problems(id), problem_revision INTEGER NOT NULL, rationale TEXT NOT NULL, evidence_refs_json TEXT NOT NULL DEFAULT '[]', operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_readiness_decisions (id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), task_revision INTEGER NOT NULL, field_key TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('not_applicable','calculated')), reason TEXT NOT NULL, evidence_refs_json TEXT NOT NULL DEFAULT '[]', provenance TEXT NOT NULL DEFAULT 'user', operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS user_activity_events (id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, operation TEXT NOT NULL, operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_operation_results (operation_id TEXT PRIMARY KEY, operation TEXT NOT NULL, payload_hash TEXT NOT NULL, result_json TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS legacy_task_migration_records (
  source_table TEXT NOT NULL, source_id TEXT NOT NULL, task_id TEXT REFERENCES tasks(id),
  problem_id TEXT REFERENCES problems(id), payload_json TEXT NOT NULL, source_hash TEXT NOT NULL,
  created_at TEXT NOT NULL, PRIMARY KEY(source_table,source_id)
);
CREATE TABLE IF NOT EXISTS refinement_items (
  id TEXT PRIMARY KEY, problem_id TEXT NOT NULL UNIQUE REFERENCES problems(id),
  capture_id TEXT REFERENCES captures(id), problem_revision INTEGER NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'legacy_problem', created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS task_activity_index ON tasks(state,last_user_activity_at DESC);
CREATE INDEX IF NOT EXISTS legacy_task_migration_task ON legacy_task_migration_records(task_id,source_table);
