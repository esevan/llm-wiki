CREATE TABLE IF NOT EXISTS mcp_connections (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  scopes_json TEXT NOT NULL,
  allowed_topics_json TEXT NOT NULL DEFAULT '[]',
  checkpoint_policy TEXT NOT NULL DEFAULT 'confirm_each',
  state TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_used_at TEXT,
  revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS work_tracking_evidence_grants (
  evidence_id TEXT PRIMARY KEY,
  connection_id TEXT NOT NULL,
  scope_kind TEXT NOT NULL,
  scope_target TEXT NOT NULL DEFAULT '',
  path TEXT NOT NULL,
  revision TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(connection_id) REFERENCES mcp_connections(id)
);
CREATE INDEX IF NOT EXISTS idx_work_tracking_evidence_grants_owner
  ON work_tracking_evidence_grants(connection_id, expires_at);

CREATE TABLE IF NOT EXISTS work_tracking_sessions (
  id TEXT PRIMARY KEY,
  connection_id TEXT,
  source_interface TEXT NOT NULL,
  conversation_ref_hash TEXT NOT NULL,
  capture_id TEXT NOT NULL UNIQUE,
  head_event_id TEXT NOT NULL,
  head_revision INTEGER NOT NULL,
  state TEXT NOT NULL DEFAULT 'active',
  publication_state TEXT NOT NULL DEFAULT 'not_requested',
  publication_offer_revision INTEGER,
  parent_session_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(connection_id, conversation_ref_hash),
  FOREIGN KEY(connection_id) REFERENCES mcp_connections(id),
  FOREIGN KEY(capture_id) REFERENCES captures(id),
  FOREIGN KEY(parent_session_id) REFERENCES work_tracking_sessions(id)
);

CREATE TABLE IF NOT EXISTS work_tracking_events (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  previous_event_id TEXT,
  stream_id TEXT NOT NULL,
  source_sequence INTEGER NOT NULL,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  source_turn_key_hash TEXT,
  supersedes_event_id TEXT,
  occurred_at TEXT NOT NULL,
  observed_at TEXT,
  ingested_at TEXT NOT NULL,
  UNIQUE(session_id, revision),
  UNIQUE(session_id, stream_id, source_sequence),
  FOREIGN KEY(session_id) REFERENCES work_tracking_sessions(id),
  FOREIGN KEY(previous_event_id) REFERENCES work_tracking_events(id),
  FOREIGN KEY(supersedes_event_id) REFERENCES work_tracking_events(id)
);

CREATE TABLE IF NOT EXISTS work_tracking_idempotency_records (
  source_interface TEXT NOT NULL,
  source_owner_hash TEXT NOT NULL,
  operation_name TEXT NOT NULL,
  operation_id TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  session_id TEXT,
  event_id TEXT,
  response_json TEXT NOT NULL,
  committed_at TEXT NOT NULL,
  PRIMARY KEY(source_interface, source_owner_hash, operation_name, operation_id)
);

CREATE TABLE IF NOT EXISTS work_tracking_projection_jobs (
  event_id TEXT NOT NULL,
  projection_name TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'pending',
  lease_owner TEXT,
  lease_expires_at TEXT,
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at TEXT,
  safe_error_code TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(event_id, projection_name),
  FOREIGN KEY(event_id) REFERENCES work_tracking_events(id)
);

CREATE TABLE IF NOT EXISTS work_tracking_projection_results (
  event_id TEXT NOT NULL,
  projection_name TEXT NOT NULL,
  state TEXT NOT NULL,
  result_revision INTEGER,
  result_entity_id TEXT,
  safe_error_code TEXT,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(event_id, projection_name),
  FOREIGN KEY(event_id) REFERENCES work_tracking_events(id)
);

CREATE TABLE IF NOT EXISTS work_tracking_stream_watermarks (
  session_id TEXT NOT NULL,
  stream_id TEXT NOT NULL,
  projection_name TEXT NOT NULL,
  last_occurred_at TEXT NOT NULL,
  last_source_sequence INTEGER NOT NULL,
  last_event_id TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(session_id, stream_id, projection_name)
);

CREATE TABLE IF NOT EXISTS work_tracking_decisions (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  event_id TEXT NOT NULL,
  decision TEXT NOT NULL,
  accepted_payload_hash TEXT,
  result_entity_type TEXT,
  result_entity_id TEXT,
  challenge_id TEXT UNIQUE,
  decision_channel TEXT NOT NULL,
  client_capability TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  UNIQUE(session_id, event_id, decision_channel)
);

CREATE TABLE IF NOT EXISTS work_tracking_links (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  source_event_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  relationship TEXT NOT NULL,
  decision_id TEXT,
  created_at TEXT NOT NULL,
  UNIQUE(source_event_id, relationship)
);

CREATE TABLE IF NOT EXISTS mcp_elicitation_challenges (
  id TEXT PRIMARY KEY,
  connection_id TEXT NOT NULL,
  session_id TEXT NOT NULL,
  action TEXT NOT NULL,
  source_event_id TEXT NOT NULL,
  proposed_payload_hash TEXT NOT NULL,
  expected_head_revision INTEGER NOT NULL,
  nonce_hash TEXT NOT NULL UNIQUE,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS knowledge_drafts (
  id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  session_id TEXT NOT NULL,
  completion_event_id TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  body_markdown TEXT NOT NULL,
  evidence_refs_json TEXT NOT NULL DEFAULT '[]',
  content_hash TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'draft',
  published_path TEXT,
  created_at TEXT NOT NULL,
  PRIMARY KEY(id, revision)
);

CREATE TABLE IF NOT EXISTS knowledge_publication_decisions (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  draft_revision INTEGER NOT NULL,
  content_hash TEXT NOT NULL,
  decision_channel TEXT NOT NULL,
  published_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(draft_id, draft_revision)
);

CREATE TABLE IF NOT EXISTS work_tracking_activity_events (
  id TEXT PRIMARY KEY,
  source_interface TEXT NOT NULL,
  connection_id TEXT,
  session_id TEXT,
  operation TEXT NOT NULL,
  outcome TEXT NOT NULL,
  affected_record_id TEXT,
  safe_error_code TEXT,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_work_tracking_events_session
  ON work_tracking_events(session_id, revision DESC);
CREATE INDEX IF NOT EXISTS idx_projection_jobs_state
  ON work_tracking_projection_jobs(state, next_attempt_at, created_at);
CREATE INDEX IF NOT EXISTS idx_work_tracking_activity_created
  ON work_tracking_activity_events(created_at DESC);

-- Existing Workbench mutations join the same event history atomically. These triggers only fire
-- for entities already linked to a tracked session; untracked legacy work remains unchanged.
CREATE TRIGGER IF NOT EXISTS track_linked_problem_update
AFTER UPDATE OF statement,detail,state ON problems
WHEN OLD.statement!=NEW.statement OR OLD.detail!=NEW.detail OR OLD.state!=NEW.state
BEGIN
  INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
  SELECT lower(hex(randomblob(16))),s.id,s.head_revision+1,s.head_event_id,'workbench',COALESCE((SELECT MAX(source_sequence)+1 FROM work_tracking_events WHERE session_id=s.id AND stream_id='workbench'),1),'workflow_link',json_object('entityType','problems','entityId',NEW.id,'state',NEW.state),lower(hex(randomblob(32))),strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
  FROM work_tracking_sessions s JOIN work_tracking_links l ON l.session_id=s.id WHERE l.entity_type='problems' AND l.entity_id=NEW.id;
  UPDATE work_tracking_sessions SET head_revision=head_revision+1,head_event_id=(SELECT id FROM work_tracking_events WHERE session_id=work_tracking_sessions.id ORDER BY revision DESC LIMIT 1),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='problems' AND entity_id=NEW.id);
  INSERT OR IGNORE INTO work_tracking_projection_results(event_id,projection_name,state,updated_at) SELECT id,'workflow','applied',strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM work_tracking_events WHERE stream_id='workbench' AND payload_json=json_object('entityType','problems','entityId',NEW.id,'state',NEW.state);
END;

CREATE TRIGGER IF NOT EXISTS track_linked_solution_update
AFTER UPDATE OF title,outcome,non_goals,validation_criteria,conflict_state,state ON features
WHEN OLD.title!=NEW.title OR OLD.outcome!=NEW.outcome OR OLD.non_goals!=NEW.non_goals OR OLD.validation_criteria!=NEW.validation_criteria OR OLD.conflict_state!=NEW.conflict_state OR OLD.state!=NEW.state
BEGIN
  INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
  SELECT lower(hex(randomblob(16))),s.id,s.head_revision+1,s.head_event_id,'workbench',COALESCE((SELECT MAX(source_sequence)+1 FROM work_tracking_events WHERE session_id=s.id AND stream_id='workbench'),1),'workflow_link',json_object('entityType','features','entityId',NEW.id,'state',NEW.state,'conflictState',NEW.conflict_state),lower(hex(randomblob(32))),strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
  FROM work_tracking_sessions s JOIN work_tracking_links l ON l.session_id=s.id WHERE l.entity_type='features' AND l.entity_id=NEW.id;
  UPDATE work_tracking_sessions SET head_revision=head_revision+1,head_event_id=(SELECT id FROM work_tracking_events WHERE session_id=work_tracking_sessions.id ORDER BY revision DESC LIMIT 1),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id=NEW.id);
  INSERT OR IGNORE INTO work_tracking_projection_results(event_id,projection_name,state,updated_at) SELECT id,'workflow','applied',strftime('%Y-%m-%dT%H:%M:%fZ','now') FROM work_tracking_events WHERE stream_id='workbench' AND payload_json=json_object('entityType','features','entityId',NEW.id,'state',NEW.state,'conflictState',NEW.conflict_state);
END;

CREATE TRIGGER IF NOT EXISTS track_linked_progress_insert
AFTER INSERT ON solution_progress_entries
WHEN EXISTS(SELECT 1 FROM work_tracking_links WHERE entity_type='features' AND entity_id=NEW.feature_id)
BEGIN
  INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
  SELECT lower(hex(randomblob(16))),s.id,s.head_revision+1,s.head_event_id,'workbench',COALESCE((SELECT MAX(source_sequence)+1 FROM work_tracking_events WHERE session_id=s.id AND stream_id='workbench'),1),'work_log_checkpoint',json_object('summary','Workbench progress updated','entityId',NEW.id),lower(hex(randomblob(32))),strftime('%Y-%m-%dT%H:%M:%fZ','now'),strftime('%Y-%m-%dT%H:%M:%fZ','now')
  FROM work_tracking_sessions s JOIN work_tracking_links l ON l.session_id=s.id WHERE l.entity_type='features' AND l.entity_id=NEW.feature_id;
  UPDATE work_tracking_sessions SET head_revision=head_revision+1,head_event_id=(SELECT id FROM work_tracking_events WHERE session_id=work_tracking_sessions.id ORDER BY revision DESC LIMIT 1),updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id IN (SELECT session_id FROM work_tracking_links WHERE entity_type='features' AND entity_id=NEW.feature_id);
END;
