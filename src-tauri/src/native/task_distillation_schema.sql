CREATE TABLE task_distillation_revisions (
  owner_type TEXT NOT NULL CHECK(owner_type IN ('work_log','task_journey')),
  owner_id TEXT NOT NULL,
  revision INTEGER NOT NULL CHECK(revision>0),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  run_id TEXT,
  work_log_entry_id TEXT,
  source_set_hash TEXT NOT NULL,
  prompt_id TEXT NOT NULL,
  prompt_version INTEGER NOT NULL CHECK(prompt_version>0),
  rules_version TEXT NOT NULL,
  result_schema_version TEXT NOT NULL,
  locale TEXT NOT NULL,
  result_json TEXT NOT NULL,
  freshness TEXT NOT NULL CHECK(freshness IN ('current','pending','stale','retryable_failure','repair_required','unavailable')),
  job_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(owner_type,owner_id,revision)
);
CREATE TABLE task_distillation_current (
  owner_type TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  revision INTEGER NOT NULL,
  source_set_hash TEXT NOT NULL,
  freshness TEXT NOT NULL CHECK(freshness IN ('current','pending','stale','retryable_failure','repair_required','unavailable')),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(owner_type,owner_id),
  FOREIGN KEY(owner_type,owner_id,revision) REFERENCES task_distillation_revisions(owner_type,owner_id,revision)
);
CREATE TABLE task_distillation_nodes (
  task_id TEXT NOT NULL REFERENCES tasks(id), node_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0),
  kind TEXT NOT NULL, topic_key TEXT, status TEXT NOT NULL, claim_ids_json TEXT NOT NULL,
  detail_json TEXT NOT NULL, source_set_hash TEXT NOT NULL, active INTEGER NOT NULL CHECK(active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(task_id,node_id)
);
CREATE TABLE task_distillation_node_history (
  task_id TEXT NOT NULL, node_id TEXT NOT NULL, revision INTEGER NOT NULL,
  snapshot_json TEXT NOT NULL, projection_revision INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(task_id,node_id,revision)
);
CREATE TABLE task_distillation_relationships (
  task_id TEXT NOT NULL, relationship_id TEXT NOT NULL, kind TEXT NOT NULL,
  from_node_id TEXT NOT NULL, to_node_id TEXT NOT NULL, reason TEXT, sources_json TEXT NOT NULL,
  active INTEGER NOT NULL CHECK(active IN (0,1)), projection_revision INTEGER NOT NULL,
  PRIMARY KEY(task_id,relationship_id)
);
CREATE TABLE task_distillation_redirects (
  task_id TEXT NOT NULL, alias_node_id TEXT NOT NULL, canonical_node_id TEXT NOT NULL,
  projection_revision INTEGER NOT NULL, PRIMARY KEY(task_id,alias_node_id)
);
CREATE TABLE task_distillation_topics (
  task_id TEXT NOT NULL, topic_key TEXT NOT NULL, status TEXT NOT NULL,
  current_node_id TEXT, replacement_node_id TEXT, sources_json TEXT NOT NULL,
  projection_revision INTEGER NOT NULL, PRIMARY KEY(task_id,topic_key)
);
CREATE TABLE task_distillation_completion_snapshots (
  task_id TEXT NOT NULL, snapshot_id TEXT NOT NULL, completion_id TEXT NOT NULL,
  completion_revision TEXT NOT NULL, status TEXT NOT NULL, snapshot_json TEXT NOT NULL,
  projection_revision INTEGER NOT NULL, PRIMARY KEY(task_id,snapshot_id)
);
CREATE TABLE task_distillation_dependencies (
  owner_type TEXT NOT NULL, owner_id TEXT NOT NULL, projection_revision INTEGER NOT NULL,
  source_type TEXT NOT NULL, source_id TEXT NOT NULL, source_revision TEXT NOT NULL, locator TEXT NOT NULL,
  claim_id TEXT NOT NULL DEFAULT '', relationship_id TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(owner_type,owner_id,projection_revision,source_type,source_id,source_revision,locator,claim_id,relationship_id)
);
CREATE INDEX task_distillation_dependency_source ON task_distillation_dependencies(source_type,source_id,source_revision);
CREATE INDEX task_distillation_nodes_current ON task_distillation_nodes(task_id,active,status);
