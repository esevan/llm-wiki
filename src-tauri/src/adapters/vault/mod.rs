use crate::domain::work_tracking_state::AppError;
use crate::native::{database, semantic::SemanticEngine, vault};
use crate::ports::vault_repository::VaultRepository;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct MarkdownVaultAdapter {
    db_path: PathBuf,
    vault_path: PathBuf,
    semantic: SemanticEngine,
}

fn app_error(code: &str, message: impl Into<String>) -> AppError {
    AppError::new(code, message)
}

impl MarkdownVaultAdapter {
    pub(crate) fn root_path(&self) -> &Path {
        &self.vault_path
    }

    pub fn withdraw(&self, path: &str, hash: &str, review_id: &str) -> Result<String, AppError> {
        let root = self
            .vault_path
            .canonicalize()
            .map_err(|_| app_error("storage_unavailable", "Vault is unavailable"))?;
        let recovery_relative = format!(
            ".llm-wiki-recovery/{:x}.recovery",
            Sha256::digest(review_id.as_bytes())
        );
        let backup = root.join(&recovery_relative);
        let parent = backup.parent().expect("recovery parent");
        std::fs::create_dir_all(parent)
            .map_err(|_| app_error("storage_unavailable", "Recovery directory is unavailable"))?;
        if std::fs::symlink_metadata(parent).is_ok_and(|m| m.file_type().is_symlink())
            || !parent
                .canonicalize()
                .map_err(|_| app_error("storage_unavailable", "Recovery directory is unavailable"))?
                .starts_with(&root)
        {
            return Err(app_error(
                "publish_conflict",
                "Recovery directory is outside the Vault",
            ));
        }
        let target = root.join(path);
        let digest_file = |p: &Path| -> Result<String, AppError> {
            if std::fs::symlink_metadata(p).is_ok_and(|m| !m.file_type().is_file()) {
                return Err(app_error(
                    "publish_conflict",
                    "Publication or recovery target is not a regular file",
                ));
            }
            let bytes = std::fs::read(p)
                .map_err(|_| app_error("storage_unavailable", "Publication is unavailable"))?;
            Ok(format!("{:x}", Sha256::digest(bytes)))
        };
        if backup.exists() {
            if !target.exists() && digest_file(&backup)? == hash {
                return Ok(recovery_relative);
            }
            return Err(app_error(
                "publish_conflict",
                "Publication changed after withdrawal; the recovery copy is preserved",
            ));
        }
        let target = vault::resolve_markdown(&root, path, true)
            .map_err(|_| app_error("publish_conflict", "Published document is unavailable"))?;
        if digest_file(&target)? != hash {
            return Err(app_error(
                "publish_conflict",
                "Published document changed; withdrawal was blocked",
            ));
        }
        std::fs::rename(&target, &backup).map_err(|_| {
            app_error(
                "storage_unavailable",
                "Publication could not be moved to recovery",
            )
        })?;
        if digest_file(&backup)? != hash {
            // An external writer won the race. Restore without replacing a newer file.
            let _ = std::fs::hard_link(&backup, &target);
            return Err(app_error(
                "publish_conflict",
                format!(
                    "External change preserved in {recovery_relative}; review before continuing"
                ),
            ));
        }
        #[cfg(unix)]
        {
            std::fs::File::open(target.parent().expect("publication parent"))
                .and_then(|f| f.sync_all())
                .map_err(|_| {
                    app_error(
                        "storage_unavailable",
                        "Publication directory could not be flushed",
                    )
                })?;
            std::fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| app_error("storage_unavailable", "Recovery could not be flushed"))?;
        }
        Ok(recovery_relative)
    }

    pub fn new(
        db_path: impl AsRef<Path>,
        vault_path: impl AsRef<Path>,
        semantic: SemanticEngine,
    ) -> Self {
        Self {
            db_path: db_path.as_ref().to_owned(),
            vault_path: vault_path.as_ref().to_owned(),
            semantic,
        }
    }

    fn public_hits(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        raw: &Value,
        semantic: bool,
    ) -> Result<Value, AppError> {
        let connection = database::open(&self.db_path)
            .map_err(|_| app_error("storage_unavailable", "Evidence grants are unavailable"))?;
        let now = chrono::Utc::now();
        let hits=raw.get("results").and_then(Value::as_array).into_iter().flatten().enumerate().map(|(rank,item)|{
            let path=item.get("path").and_then(Value::as_str).unwrap_or_default();
            let revision=item.get("source_hash").and_then(Value::as_str).unwrap_or_default();
            let evidence_id=format!("ev_{}",uuid::Uuid::new_v4());
            connection.execute("INSERT INTO work_tracking_evidence_grants(evidence_id,connection_id,scope_kind,scope_target,path,revision,expires_at,created_at) VALUES (?,?,?,?,?,?,?,?)",params![evidence_id,connection_id,scope_kind,scope_target,path,revision,(now+chrono::Duration::minutes(10)).to_rfc3339(),now.to_rfc3339()]).map_err(|_|app_error("storage_unavailable","Evidence grants are unavailable"))?;
            Ok(json!({
                "evidenceId":evidence_id,"title":item.get("title"),"kind":"knowledge",
                "uri":format!("llm-wiki://evidence/{evidence_id}"),"snippet":item.get("snippet"),
                "sourceIdentity":path,"passage":item.get("passage"),
                "matchedTerms":if semantic{Value::Null}else{item["matchedTerms"].clone()},"rank":rank+1,
                "revision":item.get("source_hash"),"modifiedAt":item.get("modified_at"),
                "score":item.get("score"),"semanticScore":item.get("semantic_score"),
                "documentId":item.get("documentId"),"section":item.get("section"),
                "chunkIndex":item.get("chunkIndex"),"chunkCount":item.get("chunkCount"),
                "aspect":item.get("aspect"),"informationType":item.get("informationType"),
                "status":item.get("status"),"conditions":item.get("conditions"),
                "historicalMatch":item.get("historicalMatch"),"warnings":item.get("warnings")
            }))
        }).collect::<Result<Vec<_>,AppError>>()?;
        Ok(
            json!({"hits":hits,"truncated":raw.get("has_more").and_then(Value::as_bool).unwrap_or(false),"indexState":if semantic{"current"}else{"not_applicable"}}),
        )
    }

    fn path_visible(
        &self,
        connection: &rusqlite::Connection,
        scope_kind: &str,
        scope_target: &str,
        path: &str,
        _title: &str,
        _body: &str,
    ) -> Result<bool, AppError> {
        match scope_kind {
            "workbench"=>Ok(true),
            "topic"=>connection.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_topic_memberships WHERE topic_id=? AND entity_type='vault' AND entity_id=?)",params![scope_target,path],|row|row.get::<_,bool>(0)).map_err(|_|app_error("storage_unavailable","Vault scope is unavailable")),
            "session"=>connection.query_row("SELECT EXISTS(SELECT 1 FROM knowledge_drafts WHERE session_id=? AND published_path=?)",params![scope_target,path],|row|row.get::<_,bool>(0)).map_err(|_|app_error("storage_unavailable","Vault scope is unavailable")),
            _=>Ok(false),
        }
    }

    fn ranked_search(
        &self,
        connection: &rusqlite::Connection,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
        semantic_requested: bool,
    ) -> Result<(Value, i64, bool), AppError> {
        let raw = crate::native::retrieval_index::search(
            connection,
            &self.semantic,
            query,
            limit,
            0,
            semantic_requested,
            |path, title, body| {
                self.path_visible(connection, scope_kind, scope_target, path, title, body)
                    .map_err(|error| error.message)
            },
        )
        .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        let source_revision = raw
            .get("source_revision")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let semantic_complete = raw
            .get("semantic_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok((raw, source_revision, semantic_complete))
    }
}

impl VaultRepository for MarkdownVaultAdapter {
    fn lexical_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError> {
        if query.trim().is_empty() || query.len() > 512 {
            return Err(app_error("invalid_input", "Query must be 1–512 characters"));
        }
        let connection = database::open(&self.db_path)
            .map_err(|_| app_error("storage_unavailable", "Vault scope is unavailable"))?;
        let (raw, _, _) =
            self.ranked_search(&connection, scope_kind, scope_target, query, limit, false)?;
        let mut result = self.public_hits(connection_id, scope_kind, scope_target, &raw, false)?;
        result["query"] = json!(query);
        result["searchRevision"] = json!(chrono::Utc::now().timestamp_millis());
        Ok(result)
    }

    fn semantic_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError> {
        if query.trim().is_empty() || query.len() > 512 {
            return Err(app_error("invalid_input", "Query must be 1–512 characters"));
        }
        let connection = database::open(&self.db_path)
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        let (raw, source_revision, semantic_complete) = self.ranked_search(
            &connection,
            scope_kind,
            scope_target,
            query,
            limit.clamp(1, 10),
            true,
        )?;
        let mut result = self.public_hits(connection_id, scope_kind, scope_target, &raw, true)?;
        result["query"] = json!(query);
        result["sourceRevision"] = json!(source_revision);
        result["indexRevision"] = json!(source_revision);
        result["indexState"] = json!(if semantic_complete {
            "current"
        } else {
            "partial"
        });
        Ok(result)
    }

    fn evidence_read(
        &self,
        connection_id: &str,
        evidence_id_value: &str,
        expected_revision: Option<&str>,
    ) -> Result<Value, AppError> {
        let connection = database::open(&self.db_path)
            .map_err(|_| app_error("storage_unavailable", "Evidence is unavailable"))?;
        let grant:Option<(String,String,String)>=connection.query_row("SELECT path,revision,expires_at FROM work_tracking_evidence_grants WHERE evidence_id=? AND connection_id=?",params![evidence_id_value,connection_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_|app_error("storage_unavailable","Evidence is unavailable"))?;
        let (path, granted_revision, expires) = grant
            .ok_or_else(|| app_error("not_found_or_not_visible", "Evidence is unavailable"))?;
        if expires < chrono::Utc::now().to_rfc3339() {
            return Err(app_error(
                "not_found_or_not_visible",
                "Evidence is unavailable",
            ));
        }
        let raw = vault::read(&self.vault_path, &path, "en")
            .map_err(|_| app_error("not_found_or_not_visible", "Evidence is unavailable"))?;
        let revision = raw
            .get("source_hash")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if revision != granted_revision {
            return Err(app_error(
                "evidence_revision_changed",
                "Evidence changed; refresh before continuing",
            ));
        }
        if expected_revision.is_some_and(|expected| expected != revision) {
            return Err(app_error(
                "evidence_revision_changed",
                "Evidence changed; refresh before continuing",
            ));
        }
        let content = raw
            .get("content")
            .or_else(|| raw.get("markdown"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(
            json!({"evidenceId":evidence_id_value,"title":path.trim_end_matches(".md"),"uri":format!("llm-wiki://evidence/{evidence_id_value}"),"revision":revision,"contentHash":revision,"mimeType":"text/markdown","content":content.chars().take(8000).collect::<String>(),"citation":{"uri":format!("llm-wiki://evidence/{evidence_id_value}"),"revision":revision},"truncated":content.chars().count()>8000}),
        )
    }

    fn publish(
        &self,
        relative_path: &str,
        expected_hash: Option<&str>,
        markdown: &str,
    ) -> Result<String, AppError> {
        let target = vault::resolve_markdown(&self.vault_path, relative_path, false)
            .map_err(|_| app_error("invalid_input", "Knowledge target is invalid"))?;
        let parent = target
            .parent()
            .ok_or_else(|| app_error("invalid_input", "Knowledge target is invalid"))?;
        let root = self
            .vault_path
            .canonicalize()
            .map_err(|_| app_error("storage_unavailable", "Vault is unavailable"))?;
        if parent.exists()
            && !parent
                .canonicalize()
                .map_err(|_| app_error("publish_conflict", "Knowledge directory is unavailable"))?
                .starts_with(&root)
        {
            return Err(app_error(
                "publish_conflict",
                "Knowledge directory is outside the Vault",
            ));
        }
        if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(app_error(
                "publish_conflict",
                "Knowledge target is a symbolic link",
            ));
        }
        if target.is_file() {
            let current = std::fs::read(&target)
                .map_err(|_| app_error("storage_unavailable", "Knowledge target is unavailable"))?;
            let current_hash = format!("{:x}", Sha256::digest(&current));
            let requested_hash = format!("{:x}", Sha256::digest(markdown.as_bytes()));
            if current_hash == requested_hash {
                return Ok(current_hash);
            }
            if expected_hash.is_none_or(|expected| expected != current_hash) {
                return Err(app_error(
                    "publish_conflict",
                    "Knowledge changed outside LLM Wiki; refresh before publishing",
                ));
            }
        } else if expected_hash.is_some() {
            return Err(app_error(
                "publish_conflict",
                "Knowledge target changed; refresh before publishing",
            ));
        }
        if expected_hash.is_some() {
            return Err(app_error(
                "publish_conflict",
                "Replacing existing Knowledge requires a separate reviewed patch",
            ));
        }
        std::fs::create_dir_all(parent)
            .map_err(|_| app_error("storage_unavailable", "Knowledge directory is unavailable"))?;
        let temporary = parent.join(format!(".publish-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(markdown.as_bytes())?;
            file.sync_all()?;
            // Atomic no-clobber publication: an external file appearing during
            // the write wins. The approved durable job can retry after a crash.
            std::fs::hard_link(&temporary, &target)?;
            #[cfg(unix)]
            std::fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        let _ = std::fs::remove_file(&temporary);
        result.map_err(|error| {
            app_error(
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    "publish_conflict"
                } else {
                    "storage_unavailable"
                },
                "Knowledge could not be published without replacing another file",
            )
        })?;
        Ok(format!("{:x}", Sha256::digest(markdown.as_bytes())))
    }
}
