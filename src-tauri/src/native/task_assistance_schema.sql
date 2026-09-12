CREATE TABLE IF NOT EXISTS refinement_sessions (
 id TEXT PRIMARY KEY, capture_id TEXT REFERENCES captures(id), task_id TEXT REFERENCES tasks(id),
 state TEXT NOT NULL DEFAULT 'active', current_draft_revision INTEGER NOT NULL DEFAULT 0,
 active_tab TEXT NOT NULL DEFAULT 'refinement', scroll_anchor TEXT NOT NULL DEFAULT '',
 input_draft TEXT NOT NULL DEFAULT '', last_user_activity_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 CHECK ((capture_id IS NOT NULL) != (task_id IS NOT NULL)), UNIQUE(capture_id), UNIQUE(task_id)
);
CREATE TABLE IF NOT EXISTS refinement_messages (
 id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES refinement_sessions(id),
 role TEXT NOT NULL CHECK(role IN ('user','assistant')), content TEXT NOT NULL,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS refinement_drafts (
 session_id TEXT NOT NULL REFERENCES refinement_sessions(id), revision INTEGER NOT NULL,
 material_hash TEXT NOT NULL, payload_json TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 PRIMARY KEY(session_id,revision)
);
CREATE TABLE IF NOT EXISTS refinement_proposal_decisions (
 id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES refinement_sessions(id),
 draft_revision INTEGER NOT NULL, proposal_id TEXT NOT NULL, decision TEXT NOT NULL,
 result_json TEXT NOT NULL, operation_id TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(session_id,draft_revision,proposal_id)
);
CREATE TABLE IF NOT EXISTS task_assistance_operations (
 operation_id TEXT PRIMARY KEY, payload_hash TEXT NOT NULL, result_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS task_assistance_jobs (
 id TEXT PRIMARY KEY, kind TEXT NOT NULL, subject_id TEXT NOT NULL, status TEXT NOT NULL,
 input_json TEXT NOT NULL, result_json TEXT NOT NULL DEFAULT '{}', error TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, started_at TEXT, finished_at TEXT,
 CHECK(status IN ('queued','running','completed','failed','cancelled'))
);
CREATE TABLE IF NOT EXISTS task_conflict_review_runs (
 id TEXT PRIMARY KEY, subject_kind TEXT NOT NULL, subject_id TEXT NOT NULL, subject_revision INTEGER NOT NULL,
 material_hash TEXT NOT NULL, vault_revision TEXT NOT NULL, scope_revision TEXT NOT NULL,
 trigger_kind TEXT NOT NULL, status TEXT NOT NULL, subject_json TEXT NOT NULL,
 findings_json TEXT NOT NULL DEFAULT '[]', evidence_json TEXT NOT NULL DEFAULT '[]',
 error TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 started_at TEXT, first_evidence_at TEXT, finished_at TEXT, cancel_requested_at TEXT,
 superseded_by_run_id TEXT
);
CREATE INDEX IF NOT EXISTS task_review_subject ON task_conflict_review_runs(subject_kind,subject_id,created_at);
CREATE TABLE IF NOT EXISTS task_conflict_decisions (
 id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES task_conflict_review_runs(id), finding_id TEXT NOT NULL,
 disposition TEXT NOT NULL, rationale TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS task_knowledge_drafts (
 task_id TEXT NOT NULL REFERENCES tasks(id), revision INTEGER NOT NULL, task_revision INTEGER NOT NULL,
 completion_id TEXT NOT NULL, body_markdown TEXT NOT NULL, content_hash TEXT NOT NULL,
 lineage_json TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'draft', path TEXT, published_hash TEXT,
 model_status TEXT NOT NULL DEFAULT 'deterministic', model_error TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 PRIMARY KEY(task_id,revision), CHECK(state IN ('draft','published','withdrawn','conflicted','failed'))
);
CREATE INDEX IF NOT EXISTS task_assistance_job_subject ON task_assistance_jobs(kind,subject_id,created_at);
CREATE INDEX IF NOT EXISTS task_knowledge_latest ON task_knowledge_drafts(task_id,revision DESC);
