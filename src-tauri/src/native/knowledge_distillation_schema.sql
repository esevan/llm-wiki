CREATE TABLE knowledge_evidence_snapshots (
  id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id), task_revision INTEGER NOT NULL,
  generation_snapshot_hash TEXT NOT NULL UNIQUE, journey_projection_revision INTEGER NOT NULL,
  journey_source_set_hash TEXT NOT NULL, completion_snapshot_id TEXT NOT NULL,
  completion_snapshot_revision TEXT NOT NULL, manifest_json TEXT NOT NULL,
  freshness TEXT NOT NULL CHECK(freshness IN ('current','historical')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE knowledge_applicability (
  id TEXT PRIMARY KEY, summary TEXT NOT NULL, representative_questions_json TEXT NOT NULL,
  helps_with_json TEXT NOT NULL, conditions_json TEXT NOT NULL, exclusions_json TEXT NOT NULL
);
CREATE TABLE knowledge_draft_versions (
  task_id TEXT NOT NULL REFERENCES tasks(id), revision INTEGER NOT NULL CHECK(revision>0),
  parent_revision INTEGER, derived_from_revision INTEGER,
  derivation_kind TEXT NOT NULL CHECK(derivation_kind IN ('generated','edited','restored','regenerated','legacy')),
  article_type TEXT NOT NULL CHECK(article_type IN ('concept','guide','comparison_decision','research_result')),
  title TEXT NOT NULL, body_markdown TEXT NOT NULL, content_hash TEXT NOT NULL,
  generation_snapshot_id TEXT REFERENCES knowledge_evidence_snapshots(id),
  applicability_id TEXT NOT NULL REFERENCES knowledge_applicability(id),
  result_json TEXT NOT NULL, quality_state TEXT NOT NULL CHECK(quality_state IN ('valid','blocked_unsupported','blocked_omission')),
  model_status TEXT NOT NULL, model_error TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(task_id,revision),
  FOREIGN KEY(task_id,parent_revision) REFERENCES knowledge_draft_versions(task_id,revision),
  FOREIGN KEY(task_id,derived_from_revision) REFERENCES knowledge_draft_versions(task_id,revision)
);
CREATE TABLE knowledge_pointers (
  task_id TEXT PRIMARY KEY REFERENCES tasks(id), current_private_revision INTEGER,
  published_revision INTEGER, publication_document_id TEXT, publication_path TEXT,
  publication_content_hash TEXT, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(task_id,current_private_revision) REFERENCES knowledge_draft_versions(task_id,revision),
  FOREIGN KEY(task_id,published_revision) REFERENCES knowledge_draft_versions(task_id,revision)
);
CREATE TABLE knowledge_idea_revisions (
  id TEXT NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), revision INTEGER NOT NULL CHECK(revision>0),
  knowledge_revision INTEGER NOT NULL, title TEXT NOT NULL, body_markdown TEXT NOT NULL,
  disposition TEXT NOT NULL CHECK(disposition IN ('unverified','deferred','out_of_scope','rejected')),
  reconsideration_conditions_json TEXT NOT NULL, source_refs_json TEXT NOT NULL,
  related_topic_keys_json TEXT NOT NULL, content_hash TEXT NOT NULL,
  publication_state TEXT NOT NULL DEFAULT 'private' CHECK(publication_state IN ('private','published','withdrawn')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY(id,revision),
  FOREIGN KEY(task_id,knowledge_revision) REFERENCES knowledge_draft_versions(task_id,revision)
);
CREATE TABLE knowledge_quality_findings (
  task_id TEXT NOT NULL, knowledge_revision INTEGER NOT NULL, id TEXT NOT NULL,
  kind TEXT NOT NULL, severity TEXT NOT NULL CHECK(severity IN ('blocking','warning')),
  locator TEXT NOT NULL, source_refs_json TEXT NOT NULL, message TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(task_id,knowledge_revision,id),
  FOREIGN KEY(task_id,knowledge_revision) REFERENCES knowledge_draft_versions(task_id,revision) ON DELETE CASCADE
);
CREATE INDEX knowledge_versions_snapshot ON knowledge_draft_versions(generation_snapshot_id);
CREATE INDEX knowledge_ideas_task ON knowledge_idea_revisions(task_id,knowledge_revision);

-- Preserve every existing reviewed byte and hash as an immutable legacy version.
INSERT INTO knowledge_applicability(id,summary,representative_questions_json,helps_with_json,conditions_json,exclusions_json)
SELECT 'legacy:'||task_id||':'||revision,'Legacy draft; structured applicability was not recorded.','[]','[]','[]','[]'
FROM task_knowledge_drafts;
INSERT INTO knowledge_draft_versions(task_id,revision,parent_revision,derived_from_revision,derivation_kind,article_type,title,body_markdown,content_hash,generation_snapshot_id,applicability_id,result_json,quality_state,model_status,model_error,created_at)
SELECT k.task_id,k.revision,
  (SELECT MAX(p.revision) FROM task_knowledge_drafts p WHERE p.task_id=k.task_id AND p.revision<k.revision),
  NULL,'legacy','concept',COALESCE(NULLIF(r.title,''),'Knowledge'),k.body_markdown,k.content_hash,NULL,
  'legacy:'||k.task_id||':'||k.revision,
  json_object('legacyLineage',json(k.lineage_json),'state',k.state,'path',k.path,'publishedHash',k.published_hash),
  'valid',k.model_status,k.model_error,k.created_at
FROM task_knowledge_drafts k JOIN task_revisions r ON r.task_id=k.task_id AND r.revision=k.task_revision;
INSERT INTO knowledge_pointers(task_id,current_private_revision,published_revision,publication_path,publication_content_hash)
SELECT task_id,
  MAX(revision),
  MAX(CASE WHEN state IN ('published','withdrawn') THEN revision END),
  (SELECT path FROM task_knowledge_drafts p WHERE p.task_id=k.task_id AND p.state IN ('published','withdrawn') ORDER BY revision DESC LIMIT 1),
  (SELECT published_hash FROM task_knowledge_drafts p WHERE p.task_id=k.task_id AND p.state IN ('published','withdrawn') ORDER BY revision DESC LIMIT 1)
FROM task_knowledge_drafts k GROUP BY task_id;
