ALTER TABLE task_work_sessions ADD COLUMN codex_thread_id TEXT;
ALTER TABLE task_work_sessions ADD COLUMN approvals_reviewer TEXT NOT NULL DEFAULT 'user' CHECK(approvals_reviewer IN ('user','auto_review'));
ALTER TABLE task_work_sessions ADD COLUMN effective_model TEXT;
ALTER TABLE task_work_sessions ADD COLUMN effective_cwd TEXT;
ALTER TABLE task_work_sessions ADD COLUMN effective_approval_policy TEXT;
ALTER TABLE task_work_sessions ADD COLUMN effective_sandbox TEXT;
ALTER TABLE task_work_sessions ADD COLUMN effective_structured_input INTEGER NOT NULL DEFAULT 0 CHECK(effective_structured_input IN (0,1));
ALTER TABLE task_work_sessions ADD COLUMN effective_settings_revision TEXT;
ALTER TABLE task_work_sessions ADD COLUMN execution_revision INTEGER NOT NULL DEFAULT 0 CHECK(execution_revision>=0);
ALTER TABLE task_work_sessions ADD COLUMN context_hash TEXT;
CREATE UNIQUE INDEX task_work_sessions_codex_thread ON task_work_sessions(codex_thread_id) WHERE codex_thread_id IS NOT NULL;

CREATE TABLE task_execution_runtime (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1),
  connection_generation INTEGER NOT NULL DEFAULT 0 CHECK(connection_generation>=0)
);
INSERT INTO task_execution_runtime(singleton,connection_generation) VALUES(1,0);

CREATE TABLE task_work_session_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  session_id TEXT NOT NULL REFERENCES task_work_sessions(id),
  submission_key TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  instruction TEXT NOT NULL,
  user_entry_id TEXT NOT NULL UNIQUE REFERENCES task_work_session_entries(id),
  work_log_entry_id TEXT NOT NULL UNIQUE REFERENCES task_work_log_entries(id),
  retry_of_run_id TEXT REFERENCES task_work_session_runs(id),
  provider TEXT NOT NULL DEFAULT 'codex' CHECK(provider='codex'),
  model TEXT NOT NULL,
  workspace_path TEXT NOT NULL,
  context_hash TEXT NOT NULL,
  provider_thread_id TEXT,
  provider_turn_id TEXT,
  status TEXT NOT NULL CHECK(status IN ('queued','running','awaiting_response','succeeded','failed','cancelled','interrupted','needs_attention')),
  dispatch_state TEXT NOT NULL DEFAULT 'not_dispatched' CHECK(dispatch_state IN ('not_dispatched','dispatch_recorded','accepted','uncertain')),
  stop_requested INTEGER NOT NULL DEFAULT 0 CHECK(stop_requested IN (0,1)),
  final_report TEXT,
  error_code TEXT,
  error_message TEXT,
  observed_evidence_json TEXT NOT NULL DEFAULT '[]',
  work_log_sync_state TEXT NOT NULL DEFAULT 'pending' CHECK(work_log_sync_state IN ('pending','synced','failed')),
  work_log_sync_error TEXT,
  revision INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(session_id,submission_key),
  UNIQUE(provider_thread_id,provider_turn_id)
);
CREATE INDEX task_work_session_runs_session ON task_work_session_runs(session_id,created_at DESC,id);
CREATE UNIQUE INDEX task_work_session_runs_active ON task_work_session_runs(session_id) WHERE status IN ('queued','running','awaiting_response');

CREATE TABLE task_work_session_run_items (
  run_id TEXT NOT NULL REFERENCES task_work_session_runs(id),
  provider_item_id TEXT NOT NULL,
  provider_order INTEGER NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL,
  content_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  completed_at TEXT NOT NULL,
  PRIMARY KEY(run_id,provider_item_id)
);
CREATE INDEX task_work_session_run_items_order ON task_work_session_run_items(run_id,provider_order,provider_item_id);

CREATE TABLE task_work_session_formal_requests (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES task_work_session_runs(id),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  session_id TEXT NOT NULL REFERENCES task_work_sessions(id),
  provider_thread_id TEXT NOT NULL,
  provider_turn_id TEXT NOT NULL,
  connection_generation INTEGER NOT NULL,
  provider_request_id TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('command_approval','file_change_approval','permissions_approval','user_input')),
  is_blocking INTEGER NOT NULL DEFAULT 1 CHECK(is_blocking IN (0,1)),
  request_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('pending','submitting','answered','stale','error')),
  proposed_response_json TEXT,
  response_json TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  answered_at TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(connection_generation,provider_request_id)
);
CREATE INDEX task_work_session_formal_requests_run ON task_work_session_formal_requests(run_id,created_at,id);

CREATE TABLE task_work_session_run_logs (
  run_id TEXT PRIMARY KEY REFERENCES task_work_session_runs(id),
  work_log_entry_id TEXT NOT NULL UNIQUE REFERENCES task_work_log_entries(id),
  projection_revision INTEGER NOT NULL DEFAULT 1,
  sync_state TEXT NOT NULL DEFAULT 'pending' CHECK(sync_state IN ('pending','synced','failed')),
  sync_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TRIGGER task_execution_run_insert_revision AFTER INSERT ON task_work_session_runs BEGIN
  UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=NEW.session_id;
END;
CREATE TRIGGER task_execution_run_update_revision AFTER UPDATE ON task_work_session_runs BEGIN
  UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=NEW.session_id;
END;
CREATE TRIGGER task_execution_item_insert_revision AFTER INSERT ON task_work_session_run_items BEGIN
  UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=(SELECT session_id FROM task_work_session_runs WHERE id=NEW.run_id);
END;
CREATE TRIGGER task_execution_request_insert_revision AFTER INSERT ON task_work_session_formal_requests BEGIN
  UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=NEW.session_id;
END;
CREATE TRIGGER task_execution_request_update_revision AFTER UPDATE ON task_work_session_formal_requests BEGIN
  UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=NEW.session_id;
END;
