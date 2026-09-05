use crate::native::database;
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct JobRegistry(Arc<Mutex<HashMap<String, CancellationToken>>>);

impl JobRegistry {
    fn register(&self, job_id: &str) -> Result<CancellationToken, String> {
        let token = CancellationToken::new();
        self.0
            .lock()
            .map_err(|_| "Job registry is unavailable")?
            .insert(job_id.to_owned(), token.clone());
        Ok(token)
    }

    fn finish(&self, job_id: &str) {
        if let Ok(mut active) = self.0.lock() {
            active.remove(job_id);
        }
    }

    fn cancel(&self, job_id: &str) {
        if let Ok(mut active) = self.0.lock() {
            if let Some(token) = active.remove(job_id) {
                token.cancel();
            }
        }
    }
}

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn result_interface(task: &str, stored: String) -> String {
    if stored != "inline_preview" {
        return stored;
    }
    match task {
        "conflict_review" => "conflict_review",
        "completion_review" => "completion_review",
        "completion_report" => "completed_knowledge",
        "image_summary" => "solution_work_summary",
        "knowledge_translation" => "knowledge_document",
        "embedding_refresh" => "embedding_coverage",
        "workbench_organization" => "workbench",
        "lineage_inference" => "solution_lineage",
        _ => "inline_preview",
    }
    .to_owned()
}

fn job_view(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?, "task_kind": row.get::<_, String>(1)?,
        "entity_type": row.get::<_, String>(2)?, "entity_id": row.get::<_, String>(3)?,
        "status": row.get::<_, String>(4)?,
        "progress": {"completed": row.get::<_, i64>(5)?, "total": row.get::<_, i64>(6)?},
        "result_interface": result_interface(&row.get::<_, String>(1)?, row.get(7)?),
        "error": match row.get::<_, String>(8)? { value if value.is_empty() => Value::Null, code => json!({"code":code,"message":row.get::<_,String>(9)?}) },
        "created_at": row.get::<_, String>(10)?, "started_at": row.get::<_, Option<String>>(11)?,
        "finished_at": row.get::<_, Option<String>>(12)?,
    }))
}

const JOB_SELECT: &str = "SELECT id,task_kind,entity_type,entity_id,status,progress_completed,progress_total,result_interface,error_code,error_message,created_at,started_at,finished_at FROM ai_jobs_v2";

pub fn list(db_path: &Path) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut statement = connection
        .prepare(&format!(
            "{JOB_SELECT} ORDER BY created_at DESC, rowid DESC"
        ))
        .map_err(|e| e.to_string())?;
    let jobs = statement
        .query_map([], job_view)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(json!({"jobs":jobs}))
}

pub fn get(db_path: &Path, job_id: &str) -> Result<Value, String> {
    database::open(db_path)?
        .query_row(&format!("{JOB_SELECT} WHERE id=?"), [job_id], job_view)
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "AI job not found".into())
}

pub fn result(db_path: &Path, job_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    connection.query_row("SELECT status,result_interface,result_json,task_kind FROM ai_jobs_v2 WHERE id=?", [job_id], |row| {
        let raw: String = row.get(2)?;
        Ok(json!({"job_id":job_id,"status":row.get::<_,String>(0)?,"result_interface":result_interface(&row.get::<_,String>(3)?,row.get(1)?),"result":serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)}))
    }).optional().map_err(|e| e.to_string())?.ok_or_else(|| "AI job not found".into())
}

pub fn conflict_review_status(db_path: &Path, identifier: &str) -> Result<Value, String> {
    if let Ok(report) = crate::native::workflow::conflict_review(db_path, identifier) {
        return Ok(report);
    }
    let connection = database::open(db_path)?;
    let (status, raw): (String, String) = connection
        .query_row(
            "SELECT status,result_json FROM ai_jobs_v2 WHERE id=? AND task_kind='conflict_review'",
            [identifier],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Conflict review not found")?;
    if matches!(status.as_str(), "completed" | "awaiting_review") {
        return serde_json::from_str(&raw).map_err(|error| error.to_string());
    }
    Ok(json!({
        "run_id":identifier,"status":status,"phase":status,"progress":0.0,
        "recommended_state":"reviewing","findings":[],"conflicts":[],"candidates":[]
    }))
}

pub fn notifications(db_path: &Path, unread_only: bool) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let sql = if unread_only {
        "SELECT id,job_id,kind,title,target_json,read_at,dismissed_at FROM notifications WHERE read_at IS NULL AND dismissed_at IS NULL ORDER BY created_at DESC"
    } else {
        "SELECT id,job_id,kind,title,target_json,read_at,dismissed_at FROM notifications WHERE dismissed_at IS NULL ORDER BY created_at DESC"
    };
    let mut statement = connection.prepare(sql).map_err(|e| e.to_string())?;
    let items = statement.query_map([], |row| { let raw:String=row.get(4)?; Ok(json!({"id":row.get::<_,String>(0)?,"job_id":row.get::<_,String>(1)?,"kind":row.get::<_,String>(2)?,"title":row.get::<_,String>(3)?,"target":serde_json::from_str::<Value>(&raw).unwrap_or(json!({})),"read_at":row.get::<_,Option<String>>(5)?,"dismissed_at":row.get::<_,Option<String>>(6)?})) }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string())?;
    let unread_count = items
        .iter()
        .filter(|item| item["read_at"].is_null() && item["dismissed_at"].is_null())
        .count();
    Ok(json!({"notifications":items,"unread_count":unread_count}))
}

pub fn update_notification(
    db_path: &Path,
    notification_id: &str,
    dismiss: bool,
) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let column = if dismiss { "dismissed_at" } else { "read_at" };
    if connection
        .execute(
            &format!("UPDATE notifications SET {column}=CURRENT_TIMESTAMP WHERE id=?"),
            [notification_id],
        )
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("Notification not found".into());
    }
    Ok(json!({"id":notification_id}))
}

pub fn cancel(db_path: &Path, registry: &JobRegistry, job_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    if connection.execute("UPDATE ai_jobs_v2 SET status='cancelled',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('queued','running','retryable')", [job_id]).map_err(|e| e.to_string())? == 0 { return Err("AI job cannot be cancelled".into()); }
    registry.cancel(job_id);
    get(db_path, job_id)
}

pub async fn enqueue(
    db_path: PathBuf,
    settings_path: PathBuf,
    vault: PathBuf,
    registry: JobRegistry,
    semantic: crate::native::semantic::SemanticEngine,
    input: Value,
) -> Result<Value, String> {
    let required = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("{key} is required"))
    };
    let task_kind = required("taskKind")?;
    let entity_type = required("entityType")?;
    let entity_id = required("entityId")?;
    if !matches!(
        task_kind.as_str(),
        "workflow_draft"
            | "workflow_refinement"
            | "image_summary"
            | "completion_review"
            | "knowledge_translation"
            | "derived_translation"
            | "embedding_refresh"
            | "conflict_review"
            | "workbench_organization"
            | "lineage_inference"
            | "completion_report"
    ) {
        return Err("Unsupported AI job type".into());
    }
    let idempotency_key = format!(
        "{}:{}:{}:{:x}",
        task_kind,
        entity_type,
        entity_id,
        Sha256::digest(input.to_string().as_bytes())
    );
    let connection = database::open(&db_path)?;
    let existing = connection.query_row(
        "SELECT id FROM ai_jobs_v2 WHERE idempotency_key=? AND status IN ('queued','running','retryable') ORDER BY created_at DESC LIMIT 1",
        [&idempotency_key], |row| row.get::<_,String>(0),
    ).optional().map_err(|error| error.to_string())?;
    if let Some(existing) = existing {
        return get(&db_path, &existing);
    }
    let job_id = id();
    connection.execute(
        "INSERT INTO ai_jobs_v2(
           id,task_kind,entity_type,entity_id,status,input_json,execution_mode,idempotency_key,
           result_interface,progress_total,available_at,created_at
         ) VALUES (?,?,?,?,'queued',?,'native',?,'inline_preview',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        params![
            job_id,
            task_kind,
            entity_type,
            entity_id,
            input.to_string(),
            idempotency_key
        ],
    )
    .map_err(|error| error.to_string())?;
    let spawned_id = job_id.clone();
    let spawned_db = db_path.clone();
    let spawned_settings = settings_path.clone();
    let token = registry.register(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let _ = run(
            spawned_db,
            spawned_settings,
            vault,
            registry,
            semantic,
            token,
            spawned_id,
        )
        .await;
    });
    get_for_id(&job_id, task_kind, entity_type, entity_id)
}

pub fn retry(
    db_path: &Path,
    settings_path: &Path,
    vault: &Path,
    registry: &JobRegistry,
    semantic: &crate::native::semantic::SemanticEngine,
    job_id: &str,
) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let changed = connection.execute(
        "UPDATE ai_jobs_v2 SET status='queued',error_code='',error_message='',started_at=NULL,finished_at=NULL,available_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('failed','cancelled','stale')",
        [job_id],
    ).map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("AI job cannot be retried".into());
    }
    let spawned_id = job_id.to_owned();
    let spawned_db = db_path.to_owned();
    let spawned_settings = settings_path.to_owned();
    let spawned_vault = vault.to_owned();
    let spawned_registry = registry.clone();
    let spawned_semantic = semantic.clone();
    let token = registry.register(job_id)?;
    tauri::async_runtime::spawn(async move {
        let _ = run(
            spawned_db,
            spawned_settings,
            spawned_vault,
            spawned_registry,
            spawned_semantic,
            token,
            spawned_id,
        )
        .await;
    });
    get(db_path, job_id)
}

fn get_for_id(
    job_id: &str,
    task_kind: String,
    entity_type: String,
    entity_id: String,
) -> Result<Value, String> {
    Ok(
        json!({"id":job_id,"task_kind":task_kind,"entity_type":entity_type,"entity_id":entity_id,"status":"queued","progress":{"completed":0,"total":1},"result_interface":result_interface(&task_kind,"inline_preview".into()),"error":null}),
    )
}

async fn run(
    db_path: PathBuf,
    settings_path: PathBuf,
    vault: PathBuf,
    registry: JobRegistry,
    semantic: crate::native::semantic::SemanticEngine,
    token: CancellationToken,
    job_id: String,
) -> Result<(), String> {
    let outcome = run_inner(&db_path, &settings_path, &vault, &semantic, &token, &job_id).await;
    if let Err(error) = &outcome {
        database::open(&db_path)?.execute(
            "UPDATE ai_jobs_v2 SET status='failed',error_code='application_error',error_message=?,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",
            params![error,job_id],
        ).map_err(|database_error| database_error.to_string())?;
    }
    registry.finish(&job_id);
    outcome
}

async fn run_inner(
    db_path: &Path,
    settings_path: &Path,
    vault: &Path,
    semantic: &crate::native::semantic::SemanticEngine,
    token: &CancellationToken,
    job_id: &str,
) -> Result<(), String> {
    let (task, entity_type, entity_id, input) = {
        let connection = database::open(db_path)?;
        let claimed = connection.execute("UPDATE ai_jobs_v2 SET status='running',started_at=CURRENT_TIMESTAMP,attempt=attempt+1 WHERE id=? AND status='queued'", [&job_id]).map_err(|e| e.to_string())?;
        if claimed == 0 {
            return Ok(());
        }
        connection
            .query_row(
                "SELECT task_kind,entity_type,entity_id,input_json FROM ai_jobs_v2 WHERE id=?",
                [&job_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?
    };
    let input = serde_json::from_str::<Value>(&input).unwrap_or_else(|_| json!({}));
    let model_task = match (task.as_str(), entity_type.as_str()) {
        ("workflow_draft", "captures") => "problem_drafting",
        ("workflow_draft", "problems") => "solution_drafting",
        ("workflow_refinement", "captures") => "capture_assistance",
        ("workflow_refinement", "problems") => "problem_assistance",
        ("workflow_refinement", "features") => "solution_assistance",
        (other, _) => other,
    };
    let (base_url, model, api_key) =
        crate::native::settings::provider_credentials_for(settings_path, model_task)?;
    let source_hash = match task.as_str() {
        "image_summary" => {
            let image: String = database::open(db_path)?
                .query_row(
                    "SELECT image_data FROM solution_progress_entries WHERE id=? AND image_data<>''",
                    [&entity_id],
                    |row| row.get(0),
                )
                .map_err(|_| "Progress image is no longer available".to_string())?;
            format!("{:x}", Sha256::digest(image.as_bytes()))
        }
        "knowledge_translation" => {
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or(&entity_id);
            crate::native::vault::read(vault, path, "en")?["source_hash"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        }
        "workbench_organization" => {
            let locale = input.get("locale").and_then(Value::as_str).unwrap_or("en");
            let items = crate::native::workbench::organization_items(db_path, locale)?;
            crate::native::workbench::source_hash(&items)
        }
        "lineage_inference" => {
            let lineage = crate::native::lineage::create(
                db_path,
                &entity_id,
                input.get("force").and_then(Value::as_bool).unwrap_or(false),
            )?;
            lineage["source_hash"].as_str().unwrap_or("").to_owned()
        }
        "derived_translation" => {
            let source = input
                .get("source")
                .and_then(Value::as_str)
                .ok_or("source is required")?;
            format!("{:x}", Sha256::digest(source.as_bytes()))
        }
        _ => String::new(),
    };
    database::open(db_path)?
        .execute(
            "UPDATE ai_jobs_v2 SET source_hash=? WHERE id=? AND status='running'",
            params![source_hash, job_id],
        )
        .map_err(|error| error.to_string())?;
    let prompt = match task.as_str() {
        "workflow_draft" if entity_type == "captures" => {
            "Return a clear problem statement as JSON with localized title and detail."
        }
        "workflow_draft" => {
            r#"Return JSON with "validation_criteria":string and localized solution fields."#
        }
        "workflow_refinement" if entity_type == "problems" => {
            r#"Return JSON shaped as "ko":{"title":string,"detail":string} and en."#
        }
        "workflow_refinement" => r#"Return JSON with "title":"refined capture"."#,
        "image_summary" => {
            r#"Return JSON only with exactly this shape: {"ko":{"summary":string},"en":{"summary":string}}."#
        }
        "conflict_review" => r#"Return JSON with "conflicts" and summary."#,
        "completion_review" => "Return a completion decision with problem_recommendation.",
        "workbench_organization" => {
            r#"Return JSON with "entries" containing entity_type, entity_id, category, attention_rank, and rationale. Do not change workflow states."#
        }
        "knowledge_translation" => "",
        "derived_translation" => {
            r#"Return JSON with "ko":string,"en":string preserving the source meaning, code, URLs, paths, identifiers, and quoted text without adding facts."#
        }
        "lineage_inference" => r#"Return JSON with "claims" and evidence_ids."#,
        "completion_report" => {
            r#"Return JSON with executive_summary_markdown and report_body_markdown."#
        }
        "embedding_refresh" => {
            let indexing_db = db_path.to_owned();
            let indexing_vault = vault.to_owned();
            let indexing_semantic = semantic.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::native::vault::index(&indexing_db, &indexing_vault, &indexing_semantic, true)
            })
            .await
            .map_err(|error| format!("Embedding refresh task failed: {error}"))??;
            return complete_without_provider(db_path, job_id, result);
        }
        _ => "Return an empty JSON object.",
    };
    let prompt = if task == "knowledge_translation" {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(&entity_id);
        let canonical = crate::native::vault::read(vault, path, "en")?;
        if canonical["canonical_locale"] != "en" {
            return Err("Knowledge document is not managed English canonical content".into());
        }
        format!(
            "Return JSON with \"markdown\":string containing a faithful Korean Markdown translation. Preserve code, identifiers, citations, URLs, and wiki-link targets exactly.\n\n{}",
            canonical["markdown"].as_str().unwrap_or("")
        )
    } else if task == "lineage_inference" {
        let lineage = crate::native::lineage::get(db_path, &entity_id)?;
        format!(
            "{prompt} Every claim must cite one or more evidence_ids from this snapshot.\n\n{}",
            lineage
        )
    } else if task == "derived_translation" {
        format!(
            "{prompt}\n\n{}",
            input.get("source").and_then(Value::as_str).unwrap_or("")
        )
    } else if task == "conflict_review" {
        let connection = database::open(db_path)?;
        let query = crate::native::workflow::conflict_query(&connection, &entity_id)?;
        let mut statement = connection
            .prepare("SELECT path,title,substr(body,1,2400) FROM vault_documents ORDER BY modified_at DESC LIMIT 20")
            .map_err(|error| error.to_string())?;
        let evidence = statement
            .query_map([], |row| {
                Ok(json!({
                    "path":row.get::<_,String>(0)?,
                    "title":row.get::<_,String>(1)?,
                    "excerpt":row.get::<_,String>(2)?
                }))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        format!(
            "{prompt} Compare the proposed Solution only with the supplied Vault evidence. Every conflict must include target_id/path, target_title, severity, category, summary, current_claim, existing_claim, impact, recommendation, and evidence. Do not invent evidence.\n\n{}",
            json!({"solution_query":query,"vault_evidence":evidence})
        )
    } else if task == "completion_review" {
        let feature = crate::native::workflow::item(db_path, "features", &entity_id)?;
        let progress = crate::native::workflow::progress(db_path, &entity_id)?;
        format!(
            "{prompt} Review only the supplied saved Solution, Work Log, comments, checklist, and completion record. Return resolution, executive_summary, what_changed, criteria_review, remaining_checklist, decision_rationale, problem_recommendation, and capture_recommendation without inventing facts.\n\n{}",
            json!({"solution":feature,"progress":progress})
        )
    } else if matches!(task.as_str(), "workflow_draft" | "workflow_refinement") {
        let current = crate::native::workflow::item(db_path, &entity_type, &entity_id)?;
        format!(
            "{prompt} Preserve the saved facts and do not advance state.\n\n{}",
            current
        )
    } else {
        prompt.to_owned()
    };
    let message_content = if task == "image_summary" {
        let (image_data, media_type): (String, String) = database::open(db_path)?
            .query_row(
                "SELECT image_data,image_media_type FROM solution_progress_entries WHERE id=?",
                [&entity_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| "Progress image is no longer available".to_string())?;
        json!([
            {"type":"text","text":prompt},
            {"type":"image_url","image_url":{"url":format!("data:{};base64,{}",if media_type.is_empty() { "image/png" } else { &media_type },image_data)}}
        ])
    } else if task == "workbench_organization" {
        let locale = input.get("locale").and_then(Value::as_str).unwrap_or("en");
        let items = crate::native::workbench::organization_items(db_path, locale)?;
        Value::String(format!("{prompt}\n\n{}", json!({"items":items})))
    } else {
        Value::String(prompt)
    };
    let request = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&json!({"model":model,"messages":[{"role":"user","content":message_content}],"stream":false}))
        .send();
    let response = tokio::select! {
        _ = token.cancelled() => return mark_cancelled(db_path, job_id),
        response = request => response,
    };
    let outcome = match response {
        Ok(value) if value.status().is_success() => tokio::select! {
            _ = token.cancelled() => return mark_cancelled(db_path, job_id),
            body = value.json::<Value>() => body,
        }
        .map_err(|e| e.to_string())
        .and_then(|body| {
            body.pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .ok_or_else(|| "Provider response did not include content".into())
                .and_then(|raw| serde_json::from_str::<Value>(raw).map_err(|e| e.to_string()))
        }),
        Ok(value) => Err(format!("Provider request failed ({})", value.status())),
        Err(error) => Err(error.to_string()),
    };
    let connection = database::open(db_path)?;
    let outcome = outcome.and_then(|result| {
        crate::native::job_results::prepare_result(
            crate::native::job_results::JobContext {
                connection: &connection,
                db_path,
                task: &task,
                entity_id: &entity_id,
                input: &input,
                vault,
                model: &model,
                source_hash: &source_hash,
            },
            result,
        )
    });
    match outcome {
        Ok(result) => {
            connection.execute("UPDATE ai_jobs_v2 SET status='completed',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'", params![result.to_string(),job_id]).map_err(|e| e.to_string())?;
            if task == "completion_review" {
                connection.execute("INSERT OR IGNORE INTO notifications(id,job_id,kind,title,target_json) VALUES (?,?,'completion_review','Completion review ready',?)", params![id(),job_id,json!({"entity_type":entity_type,"entity_id":entity_id}).to_string()]).map_err(|e| e.to_string())?;
            }
        }
        Err(error) => {
            connection.execute("UPDATE ai_jobs_v2 SET status='failed',error_code='provider_error',error_message=?,finished_at=CURRENT_TIMESTAMP WHERE id=?", params![error,job_id]).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn mark_cancelled(db_path: &Path, job_id: &str) -> Result<(), String> {
    database::open(db_path)?
        .execute(
            "UPDATE ai_jobs_v2 SET status='cancelled',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",
            [job_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn complete_without_provider(db_path: &Path, job_id: &str, result: Value) -> Result<(), String> {
    database::open(db_path)?.execute(
        "UPDATE ai_jobs_v2 SET status='completed',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",
        params![result.to_string(),job_id],
    ).map_err(|error| error.to_string())?;
    Ok(())
}
