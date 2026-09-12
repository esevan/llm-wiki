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

fn cosine(left: &[f32], bytes: &[u8]) -> f32 {
    if bytes.len() != left.len() * 4 {
        return -1.0;
    }
    let mut dot = 0.0;
    let mut right_norm = 0.0;
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let (chunks, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() {
        return -1.0;
    }
    for (left_value, chunk) in left.iter().zip(chunks) {
        let right = f32::from_le_bytes(*chunk);
        dot += left_value * right;
        right_norm += right * right;
    }
    let denominator = left_norm * right_norm.sqrt();
    if denominator == 0.0 {
        -1.0
    } else {
        dot / denominator
    }
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
                "revision":item.get("source_hash"),"modifiedAt":item.get("modified_at")
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
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = connection.prepare(
            "SELECT d.path,d.title,d.body,d.source_hash,d.modified_at,snippet(vault_documents_fts,2,'<mark>','</mark>',' … ',24) FROM vault_documents_fts JOIN vault_documents d ON d.rowid=vault_documents_fts.rowid WHERE vault_documents_fts MATCH ? ORDER BY bm25(vault_documents_fts),d.path"
        ).map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        let mut rows = statement
            .query([terms])
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        let limit = limit.clamp(1, 20);
        let mut visible = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?
        {
            let get = |column| {
                row.get::<_, String>(column)
                    .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))
            };
            let (path, title, body) = (get(0)?, get(1)?, get(2)?);
            if self.path_visible(&connection, scope_kind, scope_target, &path, &title, &body)? {
                let lower = body.to_lowercase();
                let matched = query
                    .split_whitespace()
                    .filter(|term| {
                        lower.contains(&term.to_lowercase())
                            || title.to_lowercase().contains(&term.to_lowercase())
                    })
                    .collect::<Vec<_>>();
                let line = body
                    .lines()
                    .position(|line| {
                        matched
                            .iter()
                            .any(|term| line.to_lowercase().contains(&term.to_lowercase()))
                    })
                    .map(|line| line + 1);
                visible.push(json!({"path":path,"title":title,"source_hash":get(3)?,"modified_at":row.get::<_,i64>(4).map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?,"snippet":get(5)?,"matchedTerms":matched,"passage":{"kind":"document","firstMatchingLine":line}}));
                if visible.len() > limit {
                    break;
                }
            }
        }
        let has_more = visible.len() > limit;
        visible.truncate(limit);
        let raw = json!({"results":visible,"has_more":has_more});
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
        if !self.semantic.available() {
            return Err(app_error(
                "semantic_index_not_ready",
                "Semantic index is unavailable; use lexical search",
            ));
        }
        let query_vector = self
            .semantic
            .embed(vec![query.to_owned()])
            .map_err(|_| {
                app_error(
                    "semantic_index_not_ready",
                    "Semantic index is unavailable; use lexical search",
                )
            })?
            .remove(0);
        let connection = database::open(&self.db_path)
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        let mut statement=connection.prepare("SELECT d.path,d.title,d.body,d.source_hash,d.modified_at,e.vector FROM vault_documents d LEFT JOIN vault_document_embeddings e ON e.path=d.path AND e.source_hash=d.source_hash").map_err(|_|app_error("storage_unavailable","Vault search is unavailable"))?;
        let mut rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<Vec<u8>>>(5)?,
                ))
            })
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| app_error("storage_unavailable", "Vault search is unavailable"))?;
        rows.retain(|row| {
            self.path_visible(
                &connection,
                scope_kind,
                scope_target,
                &row.0,
                &row.1,
                &row.2,
            )
            .unwrap_or(false)
        });
        let source_revision = rows.iter().map(|row| row.4).max().unwrap_or(0);
        if rows.iter().any(|row| row.5.is_none()) {
            return Err(app_error(
                "semantic_index_not_ready",
                "Semantic index is behind in the requested scope; use lexical search",
            ));
        }
        rows.sort_by(|a, b| {
            cosine(&query_vector, b.5.as_deref().unwrap_or_default())
                .partial_cmp(&cosine(&query_vector, a.5.as_deref().unwrap_or_default()))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let now = chrono::Utc::now();
        let truncated = rows.len() > limit.clamp(1, 10);
        let hits=rows.into_iter().take(limit.clamp(1,10)).enumerate().map(|(rank,(path,title,body,hash,modified,embedding))|{
            let evidence_id=format!("ev_{}",uuid::Uuid::new_v4());
            connection.execute("INSERT INTO work_tracking_evidence_grants(evidence_id,connection_id,scope_kind,scope_target,path,revision,expires_at,created_at) VALUES (?,?,?,?,?,?,?,?)",params![evidence_id,connection_id,scope_kind,scope_target,path,hash,(now+chrono::Duration::minutes(10)).to_rfc3339(),now.to_rfc3339()]).map_err(|_|app_error("storage_unavailable","Evidence grants are unavailable"))?;
            Ok(json!({"evidenceId":evidence_id,"title":title,"kind":"knowledge","uri":format!("llm-wiki://evidence/{evidence_id}"),"sourceIdentity":path,"passage":{"kind":"document","startLine":1},"score":cosine(&query_vector,embedding.as_deref().unwrap_or_default()),"snippet":body.chars().take(500).collect::<String>(),"rank":rank+1,"revision":hash,"modifiedAt":modified}))
        }).collect::<Result<Vec<_>,AppError>>()?;
        Ok(
            json!({"query":query,"sourceRevision":source_revision,"indexRevision":source_revision,"indexState":"current","hits":hits,"truncated":truncated}),
        )
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
