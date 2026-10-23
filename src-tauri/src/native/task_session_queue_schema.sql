ALTER TABLE task_work_sessions ADD COLUMN settings_epoch INTEGER NOT NULL DEFAULT 0;
ALTER TABLE task_work_sessions ADD COLUMN reasoning_effort TEXT;
ALTER TABLE task_work_sessions ADD COLUMN effective_reasoning_effort TEXT;
ALTER TABLE task_work_session_runs ADD COLUMN reasoning_effort TEXT;
-- Existing conversations must confirm the additive settings contract once.
UPDATE task_work_sessions SET effective_settings_revision=NULL;
CREATE TABLE task_session_queue_control (
 session_id TEXT PRIMARY KEY REFERENCES task_work_sessions(id),
 mode TEXT NOT NULL DEFAULT 'active' CHECK(mode IN ('active','paused','stop_then_drain','reconcile_needed')),
 stop_run_id TEXT REFERENCES task_work_session_runs(id),
 reason TEXT NOT NULL DEFAULT '', revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE task_session_queue_items (
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), session_id TEXT NOT NULL REFERENCES task_work_sessions(id),
 operation_id TEXT NOT NULL, payload_hash TEXT NOT NULL,
 instruction TEXT NOT NULL, attachment_json TEXT, settings_json TEXT NOT NULL,
 position INTEGER NOT NULL, revision INTEGER NOT NULL DEFAULT 1,
 state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','linked_run','cancelled')),
 run_id TEXT UNIQUE REFERENCES task_work_session_runs(id),
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(session_id,operation_id)
);
CREATE INDEX task_session_queue_pending ON task_session_queue_items(session_id,state,position,id);
