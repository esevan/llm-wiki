CREATE TABLE task_subtasks (
 child_task_id TEXT PRIMARY KEY REFERENCES tasks(id),
 parent_task_id TEXT NOT NULL REFERENCES tasks(id),
 boundary_reason TEXT NOT NULL, created_at TEXT NOT NULL,
 CHECK(child_task_id != parent_task_id)
);
CREATE INDEX task_subtasks_parent ON task_subtasks(parent_task_id);
CREATE TABLE task_refinements (
 task_id TEXT PRIMARY KEY REFERENCES tasks(id), revision INTEGER NOT NULL
);
INSERT INTO task_refinements(task_id,revision)
 SELECT task_id,MAX(revision) FROM task_revisions WHERE source_reference='refinement' GROUP BY task_id;
UPDATE refinement_sessions SET state='completed' WHERE EXISTS (
 SELECT 1 FROM refinement_proposal_decisions d WHERE d.session_id=refinement_sessions.id AND d.draft_revision=refinement_sessions.current_draft_revision AND d.decision!='reject'
);
CREATE TABLE task_auto_publications (
 task_id TEXT NOT NULL, revision INTEGER NOT NULL, request_json TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'pending', error TEXT NOT NULL DEFAULT '',
 PRIMARY KEY(task_id,revision),
 FOREIGN KEY(task_id,revision) REFERENCES task_knowledge_drafts(task_id,revision)
);
