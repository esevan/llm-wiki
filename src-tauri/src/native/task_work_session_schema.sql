CREATE TABLE IF NOT EXISTS task_work_sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  title TEXT NOT NULL,
  provider TEXT NOT NULL DEFAULT 'codex' CHECK(provider IN ('codex')),
  model TEXT NOT NULL DEFAULT 'gpt-5.6-sol',
  approval_mode TEXT NOT NULL DEFAULT 'ask' CHECK(approval_mode IN ('ask','auto')),
  workspace_path TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS task_work_sessions_task ON task_work_sessions(task_id,updated_at DESC);
CREATE TABLE IF NOT EXISTS task_work_session_entries (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES task_work_sessions(id),
  author TEXT NOT NULL CHECK(author IN ('user','assistant','system')),
  kind TEXT NOT NULL CHECK(kind IN ('note','ai_output','execution_result')),
  body TEXT NOT NULL DEFAULT '',
  attachment_json TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS task_work_session_entries_session ON task_work_session_entries(session_id,created_at,id);
