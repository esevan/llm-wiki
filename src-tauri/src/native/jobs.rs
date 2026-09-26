use crate::native::database;
use crate::workflow_foundation::{
    build_prompt, operation_policy, prompt_definition, validate_prompt_output,
    ApplicationDisposition, PromptId,
};
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct JobRegistry(Arc<Mutex<HashMap<String, (String, CancellationToken)>>>);

impl JobRegistry {
    fn register(&self, job_id: &str) -> Result<(String, CancellationToken), String> {
        let token = CancellationToken::new();
        let generation = id();
        if let Some(previous) = self
            .0
            .lock()
            .map_err(|_| "Job registry is unavailable")?
            .insert(job_id.to_owned(), (generation.clone(), token.clone()))
        {
            previous.1.cancel();
        }
        Ok((generation, token))
    }

    fn finish(&self, job_id: &str, generation: &str) {
        if let Ok(mut active) = self.0.lock() {
            if active.get(job_id).is_some_and(|entry| entry.0 == generation) {
                active.remove(job_id);
            }
        }
    }

    pub(crate) fn cancel(&self, job_id: &str) {
        if let Ok(mut active) = self.0.lock() {
            if let Some(token) = active.remove(job_id) {
                token.1.cancel();
            }
        }
    }

    #[allow(dead_code)]
    pub fn register_latest(&self, scope: &str) -> Result<CancellationToken, String> {
        let (_, token) = self.register(&format!("ephemeral:{scope}"))?;
        Ok(token)
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
        "knowledge_draft" => "task_knowledge_draft",
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
        "prompt": {"id":row.get::<_,String>(13)?,"version":row.get::<_,i64>(14)?},
        "source_revision":row.get::<_,String>(15)?,
        "execution_outcome":row.get::<_,String>(16)?,
        "application_disposition":row.get::<_,String>(17)?,
    }))
}

const JOB_SELECT: &str = "SELECT id,task_kind,entity_type,entity_id,status,progress_completed,progress_total,result_interface,error_code,error_message,created_at,started_at,finished_at,prompt_id,prompt_version,source_revision,execution_outcome,application_disposition FROM ai_jobs_v2";

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
    connection.query_row("SELECT status,result_interface,result_json,task_kind,prompt_id,prompt_version,source_revision,execution_outcome,application_disposition FROM ai_jobs_v2 WHERE id=?", [job_id], |row| {
        let raw: String = row.get(2)?;
        Ok(json!({"job_id":job_id,"status":row.get::<_,String>(0)?,"result_interface":result_interface(&row.get::<_,String>(3)?,row.get(1)?),"result":serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),"prompt":{"id":row.get::<_,String>(4)?,"version":row.get::<_,i64>(5)?},"source_revision":row.get::<_,String>(6)?,"execution_outcome":row.get::<_,String>(7)?,"application_disposition":row.get::<_,String>(8)?}))
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
    if connection.execute("UPDATE ai_jobs_v2 SET status='cancelled',execution_outcome='cancelled',application_disposition=CASE WHEN application_disposition='pending' THEN 'not_applicable' ELSE application_disposition END,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('queued','running','retryable')", [job_id]).map_err(|e| e.to_string())? == 0 { return Err("AI job cannot be cancelled".into()); }
    registry.cancel(job_id);
    get(db_path, job_id)
}

pub(super) fn image_source(connection: &rusqlite::Connection, entity_type: &str, entry_id: &str,
) -> Result<(String, String, String), String> {
    let sql = match entity_type {
        "task_work_log_entries" => "SELECT e.task_id,a.data,a.media_type FROM task_work_log_entries e JOIN task_attachments a ON a.entry_id=e.id WHERE e.id=? AND a.data<>'' AND a.media_type LIKE 'image/%' AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=e.task_id) ORDER BY a.rowid LIMIT 1",
        "solution_progress_entries" => "SELECT feature_id,image_data,image_media_type FROM solution_progress_entries WHERE id=? AND image_data<>''",
        _ => return Err("Unsupported image summary target".into()),
    };
    connection.query_row(sql, [entry_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|_| "Work Log image is no longer available".to_string())
}

pub(super) fn image_source_hash(data: &str, media_type: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{media_type}\0{data}").as_bytes()))
}

pub async fn enqueue(
    db_path: PathBuf,
    settings_path: PathBuf,
    vault: PathBuf,
    registry: JobRegistry,
    semantic: crate::native::semantic::SemanticEngine,
    mut input: Value,
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
    if task_kind == "conflict_review" {
        return Err("Conflict review uses the current Chat session; start it from Chat".into());
    }
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
            | "knowledge_draft"
            | "capture_distillation"
            | "work_log_distillation"
            | "run_report_distillation"
            | "task_journey_increment"
            | "publication_index"
            | "user_generation"
    ) {
        return Err("Unsupported AI job type".into());
    }
    let image_hash = if task_kind == "image_summary" {
        let (_, data, media_type) = image_source(&database::open(&db_path)?, &entity_type, &entity_id)?;
        Some(image_source_hash(&data, &media_type))
    } else { None };
    let idempotency_key = format!(
        "{}:{}:{}:{:x}",
        task_kind,
        entity_type,
        entity_id,
        Sha256::digest(input.to_string().as_bytes())
    );
    let idempotency_key = image_hash.map(|hash| format!("image_summary:{entity_type}:{entity_id}:{hash}")).unwrap_or(idempotency_key);
    let prompt_id = PromptId::for_job(&task_kind, &entity_type);
    let prompt_version = prompt_id.map(|id| prompt_definition(id).version).unwrap_or(0);
    let source_revision = input
        .get("sourceRevision")
        .or_else(|| input.get("sourceHash"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut connection = database::open(&db_path)?;
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let automatic_image = task_kind == "image_summary" && input.get("automatic").and_then(Value::as_bool).unwrap_or(false);
    let existing = tx.query_row(
        "SELECT id FROM ai_jobs_v2 WHERE idempotency_key=? AND (status IN ('queued','running','retryable') OR ?) ORDER BY created_at DESC LIMIT 1",
        params![idempotency_key, automatic_image], |row| row.get::<_,String>(0),
    ).optional().map_err(|error| error.to_string())?;
    if let Some(existing) = existing {
        drop(tx);
        return get(&db_path, &existing);
    }
    let stale_ids = if source_revision.is_empty() {
        Vec::new()
    } else {
        let mut statement = tx.prepare(
            "SELECT id FROM ai_jobs_v2
             WHERE task_kind=? AND entity_type=? AND entity_id=?
               AND source_revision<>? AND status IN ('queued','running','retryable')",
        ).map_err(|error| error.to_string())?;
        let ids = statement.query_map(
            params![task_kind,entity_type,entity_id,source_revision],
            |row| row.get::<_,String>(0),
        ).map_err(|error| error.to_string())?
         .collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
        drop(statement);
        for stale_id in &ids {
            tx.execute(
                "UPDATE ai_jobs_v2 SET status='stale',execution_outcome='cancelled',
                 application_disposition='superseded',finished_at=CURRENT_TIMESTAMP WHERE id=?",
                [stale_id],
            ).map_err(|error| error.to_string())?;
        }
        ids
    };
    let job_id = id();
    if task_kind == "knowledge_draft" && input.get("operationId").is_none() {
        input["operationId"] = Value::String(format!("knowledge-job-{job_id}"));
    }
    tx.execute(
        "INSERT INTO ai_jobs_v2(
           id,task_kind,entity_type,entity_id,status,input_json,execution_mode,idempotency_key,
           result_interface,progress_total,available_at,created_at,prompt_id,prompt_version,
           source_revision,execution_outcome,application_disposition
         ) VALUES (?,?,?,?,'queued',?,'native',?,?,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,?,?,?,'pending','pending')",
        params![
            job_id,
            task_kind,
            entity_type,
            entity_id,
            input.to_string(),
            idempotency_key,
            if task_kind == "image_summary" && entity_type == "task_work_log_entries" { "task_work_summary" }
            else if task_kind == "lineage_inference" && entity_type == "tasks" { "task_journey" }
            else { "inline_preview" },
            prompt_id.map(PromptId::as_str).unwrap_or(""),
            prompt_version,
            source_revision,
        ],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    for stale_id in stale_ids { registry.cancel(&stale_id); }
    let spawned_id = job_id.clone();
    let spawned_db = db_path.clone();
    let spawned_settings = settings_path.clone();
    let (generation, token) = registry.register(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let _ = run(
            spawned_db,
            spawned_settings,
            vault,
            registry,
            semantic,
            token,
            generation,
            spawned_id,
        )
        .await;
    });
    get(&db_path, &job_id)
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
    let (prompt,prompt_version,task_kind,entity_id,source_revision,status): (String,u32,String,String,String,String) = connection.query_row(
        "SELECT prompt_id,prompt_version,task_kind,entity_id,source_revision,status FROM ai_jobs_v2 WHERE id=?", [job_id],
        |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))
    ).optional().map_err(|error| error.to_string())?.ok_or("AI job not found")?;
    if !prompt.is_empty() {
        let id: PromptId = prompt.parse().map_err(|error: crate::workflow_foundation::ContractError| error.to_string())?;
        if prompt_definition(id).version != prompt_version {
            return Err(format!("Prompt version {prompt}:{prompt_version} is not registered"));
        }
    }
    if task_kind == "capture_distillation" {
        if matches!(status.as_str(),"queued"|"running"|"retryable") {
            return get(db_path,job_id);
        }
        let current = crate::native::capture_distillation::projection(&connection,&entity_id,"en")?
            .ok_or("Capture is no longer available")?;
        if current["sourceRevision"].as_str() != Some(source_revision.as_str()) {
            connection.execute("UPDATE ai_jobs_v2 SET application_disposition='superseded' WHERE id=?",[job_id]).map_err(|error|error.to_string())?;
            return Err("source_stale: Capture source changed; the old result cannot be retried".into());
        }
        let active: Option<String> = connection.query_row(
            "SELECT id FROM ai_jobs_v2 WHERE task_kind='capture_distillation' AND entity_id=? AND source_revision=? AND id<>? AND status IN ('queued','running','retryable') ORDER BY created_at DESC,rowid DESC LIMIT 1",
            params![entity_id,source_revision,job_id], |row| row.get(0),
        ).optional().map_err(|error|error.to_string())?;
        if let Some(active) = active { return get(db_path,&active); }
    }
    let changed = connection.execute(
        "UPDATE ai_jobs_v2 SET status='queued',execution_outcome='pending',application_disposition='pending',error_code='',error_message='',started_at=NULL,finished_at=NULL,available_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('failed','cancelled','stale')",
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
    let (generation, token) = registry.register(job_id)?;
    tauri::async_runtime::spawn(async move {
        let _ = run(
            spawned_db,
            spawned_settings,
            spawned_vault,
            spawned_registry,
            spawned_semantic,
            token,
            generation,
            spawned_id,
        )
        .await;
    });
    get(db_path, job_id)
}

pub(crate) fn start_stored(
    db_path: &Path,
    settings_path: &Path,
    vault: &Path,
    registry: &JobRegistry,
    semantic: &crate::native::semantic::SemanticEngine,
    job_id: &str,
) -> Result<(), String> {
    let (generation, token) = registry.register(job_id)?;
    let (db, settings, vault, registry, semantic, job) = (
        db_path.to_owned(),
        settings_path.to_owned(),
        vault.to_owned(),
        registry.clone(),
        semantic.clone(),
        job_id.to_owned(),
    );
    tauri::async_runtime::spawn(async move {
        let _ = run(db, settings, vault, registry, semantic, token, generation, job,
        ).await;
    });
    Ok(())
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
    generation: String,
    job_id: String,
) -> Result<(), String> {
    let outcome = loop {
        let outcome = run_inner(&db_path, &settings_path, &vault, &semantic, &token, &generation, &job_id,
        ).await;
        if token.is_cancelled() || outcome.is_ok() { break outcome; }
        let error = outcome.as_ref().unwrap_err();
        let connection = database::open(&db_path)?;
        let (attempt,prompt_id):(i64,String) = connection.query_row(
            "SELECT attempt,prompt_id FROM ai_jobs_v2 WHERE id=?", [&job_id],
            |row| Ok((row.get(0)?,row.get(1)?)),
        ).map_err(|database_error| database_error.to_string())?;
        let retry_limit = prompt_id.parse::<PromptId>().ok()
            .map(|id| {
                operation_policy(prompt_definition(id).operation_kind).automatic_retry_limit as i64
            })
            .unwrap_or(0);
        if attempt <= retry_limit && transient_error(error) {
            connection.execute(
                "UPDATE ai_jobs_v2 SET status='queued',execution_outcome='pending',worker_id='',
                 error_code='transient_retry',error_message=?,available_at=CURRENT_TIMESTAMP
                 WHERE id=? AND status='running' AND worker_id=?",
                params![error,job_id,generation],
            ).map_err(|database_error| database_error.to_string())?;
            tokio::time::sleep(Duration::from_millis(50 * (1_u64 << attempt.min(4)))).await;
            continue;
        }
        let error_code = if error.contains("invalid_result") || error.contains("invalid_prompt_output") {
            "invalid_result"
        } else if prompt_id == PromptId::CaptureDistillation.as_str() && attempt > retry_limit {
            "retry_exhausted"
        } else if prompt_id == PromptId::CaptureDistillation.as_str() {
            "provider_unavailable"
        } else {
            "application_error"
        };
        connection.execute(
            "UPDATE ai_jobs_v2 SET status='failed',execution_outcome='failed',error_code=?,error_message=?,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?",
            params![error_code,error,job_id,generation],
        ).map_err(|database_error| database_error.to_string())?;
        break outcome;
    };
    if token.is_cancelled() {
        registry.finish(&job_id, &generation);
        return Ok(());
    }
    registry.finish(&job_id, &generation);
    outcome
}

fn transient_error(error: &str) -> bool {
    !["invalid_prompt", "invalid_result", "prompt version", "missing", "required", "stale", "cancelled", "not valid JSON"]
        .iter().any(|marker| error.to_ascii_lowercase().contains(marker))
}

pub(crate) fn recover_stored(
    db_path: &Path,
    settings_path: &Path,
    vault: &Path,
    registry: &JobRegistry,
    semantic: &crate::native::semantic::SemanticEngine,
) -> Result<usize, String> {
    let connection = database::open(db_path)?;
    connection.execute(
        "UPDATE ai_jobs_v2 SET status='queued',worker_id='',started_at=NULL
         WHERE status IN ('running','retryable')",
        [],
    ).map_err(|error| error.to_string())?;
    let mut statement = connection.prepare(
        "SELECT id FROM ai_jobs_v2 WHERE status='queued' ORDER BY created_at,rowid"
    ).map_err(|error| error.to_string())?;
    let ids = statement.query_map([], |row| row.get::<_,String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    drop(statement);
    drop(connection);
    for job_id in &ids {
        start_stored(db_path,settings_path,vault,registry,semantic,job_id)?;
    }
    Ok(ids.len())
}

async fn run_inner(
    db_path: &Path,
    settings_path: &Path,
    vault: &Path,
    semantic: &crate::native::semantic::SemanticEngine,
    token: &CancellationToken,
    generation: &str,
    job_id: &str,
) -> Result<(), String> {
    if token.is_cancelled() { return Ok(()); }
    let (task, entity_type, entity_id, input, prompt_id, prompt_version) = {
        let connection = database::open(db_path)?;
        let claimed = connection.execute("UPDATE ai_jobs_v2 SET status='running',started_at=CURRENT_TIMESTAMP,attempt=attempt+1,worker_id=?,error_code='',error_message='' WHERE id=? AND status='queued'", params![generation,job_id]).map_err(|e| e.to_string())?;
        if claimed == 0 {
            return Ok(());
        }
        connection
            .query_row(
                "SELECT task_kind,entity_type,entity_id,input_json,prompt_id,prompt_version FROM ai_jobs_v2 WHERE id=?",
                [&job_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, u32>(5)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?
    };
    if !prompt_id.is_empty() {
        let registered: PromptId = prompt_id.parse().map_err(|error: crate::workflow_foundation::ContractError| error.to_string())?;
        if prompt_definition(registered).version != prompt_version {
            return Err(format!("Prompt version {prompt_id}:{prompt_version} is not registered"));
        }
    }
    let input = serde_json::from_str::<Value>(&input).unwrap_or_else(|_| json!({}));
    if task == "task_journey_increment" {
        return apply_task_journey_increment(db_path, job_id, &input, generation);
    }
    if task == "run_report_distillation" && distillation_input(&input)?.primary_source.is_none() {
        return finalize_run_distillation(
            db_path,
            job_id,
            prompt_version,
            &input,
            Ok(json!({})),
            generation,
        );
    }
    if task == "refinement_preview" {
        let attempt: i64 = database::open(db_path)?
            .query_row(
                "SELECT attempt FROM ai_jobs_v2 WHERE id=?",
                [job_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let prepared = tokio::select! {
            biased;
            _ = token.cancelled() => return Ok(()),
            prepared = async {
                if input["referenceAware"] == true {
                    crate::native::reference_aware_workbench::prepare(db_path, settings_path, vault, &semantic, &input).await
                } else { crate::native::task_assistance::prepare_refinement_preview(settings_path, &input).await }
            } => prepared?,
        };
        if token.is_cancelled() {
            return Ok(());
        }
        if let Some(revision) = crate::native::task_assistance::finalize_refinement_preview(
            db_path, job_id, attempt, &input, &prepared,
        )? {
            if input["referenceAware"] == true {
                crate::native::reference_aware_workbench::schedule_investigation(db_path, settings_path, vault, &semantic, &input, &prepared, revision)?;
            }
            if prepared["proposals"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
            {
                let _ = crate::native::task_assistance::review_refinement_preview(
                    db_path,
                    settings_path,
                    vault,
                    semantic.clone(),
                    &input,
                    revision,
                )
                .await;
            }
        }
        return Ok(());
    }
    if task == "publication_index" && entity_type == "knowledge_archive" {
        if token.is_cancelled() {
            return Ok(());
        }
        let operation_id = input["archiveOperationId"]
            .as_str()
            .ok_or("archiveOperationId is required")?;
        let result = crate::native::knowledge_archive::finish_index(
            db_path,
            vault,
            &semantic,
            operation_id,
        )?;
        if semantic.available() {
            let db = db_path.to_path_buf();
            let root = vault.to_path_buf();
            let engine = semantic.clone();
            tokio::task::spawn_blocking(move || {
                crate::native::vault::index(&db, &root, &engine, true, false)
            });
        }
        database::open(db_path)?.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition='applied',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?", params![result.to_string(),job_id,generation]).map_err(|error|error.to_string())?;
        return Ok(());
    }
    if task == "knowledge_draft" {
        let prepared = tokio::select! {
            biased;
            _ = token.cancelled() => return Ok(()),
            prepared = crate::native::task_assistance::prepare_knowledge_draft(db_path, settings_path, &input) => prepared?,
        };
        if token.is_cancelled() { return Ok(()); }
        return finalize_knowledge_draft(db_path, job_id, &input, &prepared);
    }
    if task == "lineage_inference" && entity_type == "tasks" {
        return run_task_journey(db_path, settings_path, token, generation, job_id, &entity_id, &input,
        ).await;
    }
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
    let image = if task == "image_summary" {
        Some(image_source(&database::open(db_path)?, &entity_type, &entity_id,
        )?)
    } else { None };
    let source_hash = match task.as_str() {
        "image_summary" => {
            let (_, data, media_type) = image.as_ref().ok_or("Image unavailable")?;
            image_source_hash(data, media_type)
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
        "capture_distillation" => input
            .get("sourceHash")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        _ => String::new(),
    };
    database::open(db_path)?
        .execute(
            "UPDATE ai_jobs_v2 SET source_hash=? WHERE id=? AND status='running' AND worker_id=?",
            params![source_hash, job_id, generation],
        )
        .map_err(|error| error.to_string())?;
    if task == "embedding_refresh" {
            let indexing_db = db_path.to_owned();
            let indexing_vault = vault.to_owned();
            let indexing_semantic = semantic.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::native::vault::index(&indexing_db, &indexing_vault, &indexing_semantic, true, true,
            )
            })
            .await
            .map_err(|error| format!("Embedding refresh task failed: {error}"))??;
            return complete_without_provider(db_path, job_id, result);
    }
    let context = if task == "knowledge_translation" {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(&entity_id);
        let canonical = crate::native::vault::read(vault, path, "en")?;
        if canonical["canonical_locale"] != "en" {
            return Err("Knowledge document is not managed English canonical content".into());
        }
        canonical["markdown"].clone()
    } else if task == "lineage_inference" {
        crate::native::lineage::get(db_path, &entity_id)?
    } else if task == "derived_translation" {
        input.get("source").cloned().unwrap_or(Value::Null)
    } else if task == "completion_review" {
        let feature = crate::native::workflow::item(db_path, "features", &entity_id)?;
        let progress = crate::native::workflow::progress(db_path, &entity_id)?;
        json!({"solution":feature,"progress":progress})
    } else if matches!(task.as_str(), "workflow_draft" | "workflow_refinement") {
        json!({
            "entityType":entity_type,
            "savedWork":crate::native::workflow::item(db_path, &entity_type, &entity_id)?,
        })
    } else if task == "workbench_organization" {
        let locale = input.get("locale").and_then(Value::as_str).unwrap_or("en");
        json!({"locale":locale,"items":crate::native::workbench::organization_items(db_path, locale)?})
    } else {
        input.clone()
    };
    let prompt_id = PromptId::for_job(&task, &entity_type);
    let prompt = if let Some(prompt_id) = prompt_id {
        build_prompt(prompt_id, &json!({"context":context}))
            .map_err(|error| error.to_string())?
            .content
    } else {
        return Err(format!("No registered prompt for {task}"));
    };
    let message_content = if task == "image_summary" {
        let (_, image_data, media_type) = image.as_ref().ok_or("Image unavailable")?;
        json!([
            {"type":"text","text":prompt},
            {"type":"image_url","image_url":{"url":format!("data:{};base64,{}",if media_type.is_empty() { "image/png" } else { media_type },image_data)}}
        ])
    } else if task == "capture_distillation" {
        crate::native::capture_distillation::provider_content(
            &database::open(db_path)?,
            &entity_id,
            input.get("sourceNumber").and_then(Value::as_i64).unwrap_or(1),
            prompt,
        )?
    } else {
        Value::String(prompt)
    };
    let request = Client::builder()
        .timeout(Duration::from_secs(120))
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
        _ = token.cancelled() => return Ok(()),
        response = request => response,
    };
    let outcome = match response {
        Ok(value) if value.status().is_success() => tokio::select! {
            _ = token.cancelled() => return Ok(()),
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
    if task == "run_report_distillation" {
        return finalize_run_distillation(db_path, job_id, prompt_version, &input, outcome,
            generation,
        );
    }
    if task == "image_summary" {
        let outcome = outcome.and_then(|value| {
            validate_prompt_output(PromptId::ImageSummary, &value)
                .map_err(|error| error.to_string())?;
            Ok(value)
        });
        return finalize_image_summary(db_path, vault, job_id, generation, &entity_id, &input, &model, &source_hash, outcome,
        );
    }
    let connection = database::open(db_path)?;
    let outcome = outcome.and_then(|result| {
        if let Some(prompt_id) = prompt_id {
            validate_prompt_output(prompt_id, &result).map_err(|error| error.to_string())?;
        }
        crate::native::job_results::prepare_result(
            crate::native::job_results::JobContext {
                job_id,
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
            let disposition = prompt_id.map(|id| {
                    operation_policy(prompt_definition(id).operation_kind).default_disposition.as_str()
                }).unwrap_or(ApplicationDisposition::NotApplicable.as_str());
            connection.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition=?,result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'", params![disposition,result.to_string(),job_id]).map_err(|e| e.to_string())?;
            if task == "completion_review" {
                connection.execute("INSERT OR IGNORE INTO notifications(id,job_id,kind,title,target_json) VALUES (?,?,'completion_review','Completion review ready',?)", params![id(),job_id,json!({"entity_type":entity_type,"entity_id":entity_id}).to_string()]).map_err(|e| e.to_string())?;
            }
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

fn distillation_input(value:&Value,
) -> Result<crate::application::task_distillation_service::DistillationInput,String> {
    serde_json::from_value(value.get("distillationInput").cloned().ok_or("distillationInput is required")?,
    ).map_err(|error|format!("invalid distillation input: {error}"))
}

fn finalize_run_distillation(
    db_path:&Path, job_id:&str, prompt_version:u32, input:&Value, outcome:Result<Value,String>,
    generation: &str,
) -> Result<(),String> {
    use crate::application::task_distillation_service::{canonicalize_semantic_ids,factual_fallback,DistillationResult,
    };
    let manifest=distillation_input(input)?;
    let mut candidate=if manifest.primary_source.is_none() {
        factual_fallback(&manifest,input.get("runStatus").and_then(Value::as_str).unwrap_or("unknown"),
        )
    } else {
        let value=outcome?;
        validate_prompt_output(PromptId::RunReportDistillation,&value).map_err(|error|error.to_string())?;
        serde_json::from_value::<DistillationResult>(value).map_err(|error|format!("invalid Distillation result: {error}"))?
    };
    canonicalize_semantic_ids(&manifest,&mut candidate)?;
    let mut connection=database::open(db_path)?;
    let work_log=crate::native::task_distillation::publish_for_worker(&mut connection,job_id,&manifest,&candidate,PromptId::RunReportDistillation,prompt_version,
        generation,
    )?;
    connection.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",params![serde_json::to_string(&candidate).map_err(|e|e.to_string())?,job_id]).map_err(|error|error.to_string())?;
    let expected= manifest.expected_journey_revision;
    let mut journey_manifest=manifest.clone();
    journey_manifest.projection_kind="task_journey".into();
    journey_manifest.expected_projection_revision=expected;

    let mut journey_id=id();
    let journey_input=json!({"distillationInput":journey_manifest,"validatedResult":candidate,"sourceRunJobId":job_id});
    connection.execute("INSERT OR IGNORE INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,prompt_id,prompt_version,source_revision,execution_outcome,application_disposition) VALUES(?,'task_journey_increment','tasks',?,'queued',?,?, 'task_journey_increment',3,?,'pending','pending')",params![&journey_id,&manifest.owner.task_id,journey_input.to_string(),format!("task-journey-increment:{}:{}",manifest.owner.task_id,manifest.source_set_hash),&manifest.source_set_hash]).map_err(|error|error.to_string())?;
    journey_id = connection
        .query_row(
            "SELECT id FROM ai_jobs_v2 WHERE idempotency_key=?",
            [format!(
                "task-journey-increment:{}:{}",
                manifest.owner.task_id, manifest.source_set_hash
            )],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    apply_task_journey_increment(db_path, &journey_id, &journey_input, generation)?;
    connection.execute("UPDATE ai_jobs_v2 SET result_json=json_set(COALESCE(NULLIF(result_json,''),'{}'),'$.workLogPublication',json(?),'$.journeyJobId',?) WHERE id=?",params![work_log.to_string(),journey_id,job_id]).map_err(|error|error.to_string())?;
    Ok(())
}

fn apply_task_journey_increment(db_path:&Path,job_id:&str,input:&Value,
    generation: &str,
)->Result<(),String>{
    use crate::application::task_distillation_service::DistillationResult;
    let manifest=distillation_input(input)?;
    let candidate:DistillationResult=serde_json::from_value(input.get("validatedResult").cloned().ok_or("validatedResult is required")?,
    ).map_err(|error|format!("invalid validated Distillation result: {error}"))?;
    let mut connection=database::open(db_path)?;
    connection.execute("UPDATE ai_jobs_v2 SET status='running',worker_id=?,attempt=CASE WHEN attempt=0 THEN 1 ELSE attempt END,started_at=COALESCE(started_at,CURRENT_TIMESTAMP) WHERE id=? AND status='queued'",params![generation,job_id]).map_err(|error|error.to_string())?;
    let publication=crate::native::task_distillation::publish_for_worker(&mut connection,job_id,&manifest,&candidate,PromptId::TaskJourneyIncrement,
        3,
        generation,
    )?;
    connection.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",params![json!({"projection":publication,"distillation":candidate}).to_string(),job_id]).map_err(|error|error.to_string())?;
    Ok(())
}

fn journey_fallback_title(event_type: &str, locale: &str) -> String {
    let (en, ko) = match event_type {
        "origin_capture" => ("Idea", "아이디어"), "task_created" => ("Created", "생성"),
        "refinement_input" => ("Refinement", "정제"), "refinement_applied" => ("Applied", "적용"),
        "task_updated" => ("Updated", "수정"), "execution_started" => ("Started", "시작"),
        "work_recorded" => ("Work log", "작업 기록"), "decision_recorded" => ("Decision", "결정"),
        "task_completed" => ("Completed", "완료"), _ => ("Activity", "활동"),
    };
    if locale.starts_with("ko") { ko } else { en }.to_owned()
}

fn compact_title(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return fallback.to_owned();
    }
    let mut result = value.chars().take(48).collect::<String>();
    if value.chars().count() > 48 {
        result.pop();
        result.push('…');
    }
    result
}

fn event_contains_quote(event: &Value, quote: &str) -> bool {
    fn contains_narrative(value: &Value, quote: &str, payload: bool) -> bool {
        match value {
            Value::String(text) => text.contains(quote),
            Value::Array(values) => values
                .iter()
                .any(|value| contains_narrative(value, quote, payload)),
            Value::Object(values) => values.iter().any(|(key, value)| {
                (payload
                    || matches!(
                        key.as_str(),
                        "summary"
                            | "title"
                            | "report"
                            | "before"
                            | "after"
                            | "payload"
                            | "rationale"
                    ))
                    && contains_narrative(value, quote, payload || key == "payload")
            }),
            _ => false,
        }
    }
    (4..=280).contains(&quote.chars().count()) && contains_narrative(&event["detail"], quote, false)
}

fn validated_journey_relationships(recorded: &Value, suggested: Option<&Value>) -> Vec<Value> {
    let events = recorded["events"].as_array().cloned().unwrap_or_default();
    let positions = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| event["id"].as_str().map(|id| (id.to_owned(), index)))
        .collect::<HashMap<_, _>>();
    let by_id = events
        .iter()
        .filter_map(|event| event["id"].as_str().map(|id| (id.to_owned(), event)))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    suggested.and_then(Value::as_array).into_iter().flatten().filter_map(|relationship| {
        let from = relationship["from"].as_str()?.trim();
        let to = relationship["to"].as_str()?.trim();
        let kind = relationship["kind"].as_str()?;
        let rationale = relationship["rationale"].as_str()?.trim();
        if !matches!(kind, "supersedes" | "derived_from" | "depends_on")
            || from == to || rationale.is_empty() || rationale.chars().count() > 500
            || positions.get(from)? <= positions.get(to)?
        { return None; }
        let evidence = relationship["evidence"].as_array()?;
        let mut grounded = Vec::new();
        let mut covered = HashSet::new();
        for item in evidence {
            let event_id = item["eventId"].as_str()?.trim();
            let quote = item["quote"].as_str()?.trim();
            if event_id != from && event_id != to { return None; }
            let event = by_id.get(event_id)?;
            if !event_contains_quote(event, quote) { return None; }
            covered.insert(event_id);
            grounded.push(json!({"eventId":event_id,"quote":quote}));
        }
        if !covered.contains(from) || !covered.contains(to) { return None; }
        if !seen.insert((from.to_owned(), to.to_owned(), kind.to_owned())) { return None; }
        Some(json!({"from":from,"to":to,"kind":kind,"provenance":"inferred","rationale":rationale,"evidence":grounded}))
    }).collect()
}

fn journey_graph(recorded: &Value, locale: &str, inference: Option<&Value>) -> Value {
    let mut graph = recorded.clone();
    let title_map = inference.and_then(|value| value["titles"].as_object());
    let localized = graph["events"]
        .as_array()
        .map(|events| {
            events
                .iter()
                .map(|event| {
                    let fallback =
                        journey_fallback_title(event["type"].as_str().unwrap_or(""), locale);
                    let suggested = title_map
                        .and_then(|map| map.get(event["id"].as_str().unwrap_or("")))
                        .and_then(Value::as_object)
                        .and_then(|value| {
                            value.get(if locale.starts_with("ko") { "ko" } else { "en" })
                        })
                        .and_then(Value::as_str)
                        .unwrap_or(&fallback);
                    json!({"id":event["id"],"title":compact_title(suggested,&fallback)})
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    graph["titles"] = Value::Array(localized);
    graph["relationships"] = Value::Array(validated_journey_relationships(
        recorded,
        inference.and_then(|value| value.get("relationships")),
    ));
    graph
}

pub(crate) const TASK_JOURNEY_SCHEMA_VERSION: u32 = 2;

pub(crate) fn task_journey_source_hash(recorded: &Value) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            json!({"schemaVersion":TASK_JOURNEY_SCHEMA_VERSION,"journey":recorded})
                .to_string()
                .as_bytes()
        )
    )
}

async fn prepare_task_journey(
    settings_path: &Path,
    recorded: &Value,
    locale: &str,
    token: Option<&CancellationToken>,
) -> Result<Option<Value>, String> {
    let mut model_error = String::new();
    let mut model_status = "fallback";
    let mut suggested: Option<Value> = None;
    match crate::native::settings::provider_credentials_for(settings_path, "lineage_inference") {
        Ok((base_url, model, api_key)) => {
            let prompt = build_prompt(PromptId::TaskJourney, &json!({"context":{"locale":locale,"events":recorded}}),
            )
                .map_err(|error| error.to_string())?.content;
            let request = Client::builder().timeout(Duration::from_secs(30)).build()
                .map_err(|error|error.to_string())?
                .post(format!("{}/chat/completions",base_url.trim_end_matches('/')))
                .bearer_auth(api_key)
                .json(&json!({"model":model,"messages":[{"role":"user","content":prompt}],"stream":false}))
                .send();
            let response = if let Some(token) = token {
                tokio::select! { _ = token.cancelled() => return Ok(None), response = request => response }
            } else {
                request.await
            };
            match response {
                Ok(response) if response.status().is_success() => {
                    let body = if let Some(token) = token {
                        tokio::select! { _ = token.cancelled() => return Ok(None), body = response.json::<Value>() => body.ok() }
                    } else {
                        response.json::<Value>().await.ok()
                    };
                    match body.and_then(|body| {
                        body.pointer("/choices/0/message/content")
                            .and_then(Value::as_str)
                            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                    }) {
                        Some(value) if validate_prompt_output(PromptId::TaskJourney, &value).is_ok()
                            && value["titles"].is_object() && value["relationships"].is_array() => {
                            suggested = Some(value);
                            model_status = "ai";
                        }
                        _ => model_error = "Provider returned no usable journey inference".into(),
                    }
                }
                Ok(response) => {
                    model_error = format!("Provider request failed ({})", response.status())
                }
                Err(error) => model_error = error.to_string(),
            }
        }
        Err(error) => model_error = error,
    }
    if token.is_some_and(CancellationToken::is_cancelled) {
        return Ok(None);
    }
    Ok(Some(json!({
        "journey":journey_graph(recorded, locale, suggested.as_ref()),
        "modelStatus":model_status,
        "modelError":model_error,
    })))
}

pub(crate) async fn ensure_task_journey(
    db_path: &Path,
    settings_path: &Path,
    task_id: &str,
    locale: &str,
) -> Result<Value, String> {
    let available: bool = database::open(db_path)?.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?))",
        params![task_id,task_id], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if !available {
        return Err("Task not found".into());
    }
    let snapshot =
        crate::native::task_assistance::task_lineage_for_locale(db_path, task_id, locale)?;
    let source_hash = snapshot["journeySourceHash"]
        .as_str()
        .ok_or("Task journey source is unavailable")?
        .to_owned();
    if !snapshot["journey"].is_null() {
        return Ok(
            json!({"sourceHash":source_hash,"journey":snapshot["journey"],
            "modelStatus":snapshot["modelStatus"],"modelError":snapshot["modelError"]}),
        );
    }
    let prepared = prepare_task_journey(settings_path, &snapshot["recordedJourney"], locale, None)
        .await?
        .ok_or("Task journey generation was cancelled")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let deleted: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?)",
        [task_id], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if deleted {
        return Err("Task was deleted while journey inference was running".into());
    }
    let fresh = crate::native::task_assistance::task_lineage_tx_locale(&tx, task_id, locale)?;
    if fresh["journeySourceHash"].as_str() != Some(source_hash.as_str()) {
        return Err("Task journey evidence changed while inference was running".into());
    }
    if !fresh["journey"].is_null() {
        let cached = json!({"sourceHash":source_hash,"journey":fresh["journey"],
            "modelStatus":fresh["modelStatus"],"modelError":fresh["modelError"]});
        tx.commit().map_err(|error| error.to_string())?;
        return Ok(cached);
    }
    let graph = &prepared["journey"];
    let model_status = prepared["modelStatus"].as_str().unwrap_or("fallback");
    let model_error = prepared["modelError"].as_str().unwrap_or("");
    tx.execute("INSERT INTO task_journey_graphs(task_id,locale,source_hash,graph_json,model_status,model_error,created_at) VALUES(?,?,?,?,?,?,CURRENT_TIMESTAMP) ON CONFLICT(task_id,locale) DO UPDATE SET source_hash=excluded.source_hash,graph_json=excluded.graph_json,model_status=excluded.model_status,model_error=excluded.model_error,created_at=CURRENT_TIMESTAMP", params![task_id,locale,source_hash,graph.to_string(),model_status,model_error]).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(json!({"sourceHash":source_hash,"journey":graph,
        "modelStatus":model_status,"modelError":model_error}))
}

async fn run_task_journey(
    db_path: &Path,
    settings_path: &Path,
    token: &CancellationToken,
    generation: &str,
    job_id: &str,
    task_id: &str,
    input: &Value,
) -> Result<(), String> {
    let locale = input.get("locale").and_then(Value::as_str).unwrap_or("en");
    let snapshot =
        crate::native::task_assistance::task_lineage_for_locale(db_path, task_id, locale)?;
    let source_hash = snapshot["journeySourceHash"]
        .as_str()
        .ok_or("Task journey source is unavailable")?
        .to_owned();
    if input.get("sourceHash").and_then(Value::as_str) != Some(source_hash.as_str()) {
        database::open(db_path)?.execute("UPDATE ai_jobs_v2 SET status='stale',execution_outcome='succeeded',application_disposition='superseded',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?", params![job_id,generation]).map_err(|error| error.to_string())?;
        return Ok(());
    }
    let Some(prepared) = prepare_task_journey(
        settings_path,
        &snapshot["recordedJourney"],
        locale,
        Some(token),
    )
    .await?
    else {
        return Ok(());
    };
    finalize_task_journey(
        db_path,
        generation,
        job_id,
        task_id,
        locale,
        &source_hash,
        &prepared,
    )
}

/// Commit the graph and Queue result atomically only for the current worker and
/// unchanged evidence. A cancelled or superseded worker cannot replace the cache.
fn finalize_task_journey(
    db_path: &Path,
    generation: &str,
    job_id: &str,
    task_id: &str,
    locale: &str,
    source_hash: &str,
    prepared: &Value,
) -> Result<(), String> {
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let running = tx.execute(
        "UPDATE ai_jobs_v2 SET source_hash=? WHERE id=? AND status='running' AND worker_id=? AND entity_type='tasks' AND entity_id=? AND task_kind='lineage_inference'",
        params![source_hash,job_id,generation,task_id],
    ).map_err(|error| error.to_string())?;
    if running == 0 {
        return Ok(());
    }
    let deleted: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?)",
        [task_id], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    let fresh = crate::native::task_assistance::task_lineage_tx_locale(&tx, task_id, locale)?;
    if deleted || fresh["journeySourceHash"].as_str() != Some(source_hash) {
        tx.execute("UPDATE ai_jobs_v2 SET status='stale',execution_outcome='succeeded',application_disposition='superseded',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?", params![job_id,generation]).map_err(|error| error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let graph = &prepared["journey"];
    let model_status = prepared["modelStatus"].as_str().unwrap_or("fallback");
    let model_error = prepared["modelError"].as_str().unwrap_or("");
    tx.execute("INSERT INTO task_journey_graphs(task_id,locale,source_hash,graph_json,model_status,model_error,created_at) VALUES(?,?,?,?,?,?,CURRENT_TIMESTAMP) ON CONFLICT(task_id,locale) DO UPDATE SET source_hash=excluded.source_hash,graph_json=excluded.graph_json,model_status=excluded.model_status,model_error=excluded.model_error,created_at=CURRENT_TIMESTAMP", params![task_id,locale,source_hash,graph.to_string(),model_status,model_error]).map_err(|error| error.to_string())?;
    tx.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition='applied',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?", params![json!({"taskId":task_id,"sourceHash":source_hash,"journey":graph,"modelStatus":model_status,"modelError":model_error}).to_string(),job_id,generation]).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

fn complete_without_provider(db_path: &Path, job_id: &str, result: Value) -> Result<(), String> {
    database::open(db_path)?.execute(
        "UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition='not_applicable',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'",
        params![result.to_string(),job_id],
    ).map_err(|error| error.to_string())?;
    Ok(())
}

/// Commits the draft and its durable Queue result together.  A cancellation that
/// wins before this transaction starts leaves neither a private draft nor a result.
pub(crate) fn finalize_knowledge_draft(
    db_path: &Path,
    job_id: &str,
    input: &Value,
    prepared: &Value,
) -> Result<(), String> {
    let expected_source = prepared
        .get("sourceHash")
        .and_then(Value::as_str)
        .ok_or("Prepared Knowledge source is unavailable")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let running = tx
        .execute(
            "UPDATE ai_jobs_v2 SET source_hash=? WHERE id=? AND status='running'",
            params![expected_source, job_id],
        )
        .map_err(|error| error.to_string())?;
    if running == 0 {
        return Err("Knowledge draft job was cancelled before it could be saved".into());
    }
    let result =
        crate::native::task_assistance::save_prepared_knowledge_draft_tx(&tx, input, prepared)?;
    if result.get("sourceHash").and_then(Value::as_str) != Some(expected_source) {
        return Err("Task evidence changed while the Knowledge draft was running".into());
    }
    tx.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition='review_needed',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running'", params![result.to_string(),job_id]).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]
    fn old_worker_cannot_remove_retried_workers_cancellation() {
        let registry = JobRegistry::default();
        let (old, old_token) = registry.register("preview").unwrap();
        let (_new, new_token) = registry.register("preview").unwrap();
        assert!(old_token.is_cancelled());
        registry.finish("preview", &old);
        registry.cancel("preview");
        assert!(new_token.is_cancelled());
    }

    #[test]
    fn latest_wins_ephemeral_scope_cancels_the_previous_request() {
        let registry = JobRegistry::default();
        let first = registry.register_latest("task:one:search").unwrap();
        let second = registry.register_latest("task:one:search").unwrap();
        assert!(first.is_cancelled());
        assert!(!second.is_cancelled());
    }
}

#[cfg(test)]
mod foundation_queue_tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_coalesces_equivalent_active_job_with_prompt_metadata() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let input = json!({"taskKind":"user_generation","entityType":"tasks","entityId":"task-1","sourceRevision":"r1","payload":"same"});
        let key = format!("user_generation:tasks:task-1:{:x}",Sha256::digest(input.to_string().as_bytes()));
        database::open(&db).unwrap().execute(
            "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,prompt_id,prompt_version,source_revision)
             VALUES('existing','user_generation','tasks','task-1','queued',?,?,?,1,'r1')",
            params![input.to_string(),key,PromptId::UserGeneration.as_str()],
        ).unwrap();
        let result = enqueue(
            db, root.path().join("settings.json"), root.path().join("vault"),
            JobRegistry::default(), crate::native::semantic::SemanticEngine::new(None), input,
        ).await.unwrap();
        assert_eq!(result["id"],"existing");
        assert_eq!(result["prompt"]["id"],PromptId::UserGeneration.as_str());
        assert_eq!(result["source_revision"],"r1");
    }

    #[tokio::test]
    async fn newer_source_marks_active_older_job_superseded_and_cancels_it() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let registry = JobRegistry::default();
        let (_, old_token) = registry.register("old").unwrap();
        database::open(&db).unwrap().execute(
            "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,prompt_id,prompt_version,source_revision)
             VALUES('old','user_generation','tasks','task-1','running','{}','old-key',?,1,'r1')",
            [PromptId::UserGeneration.as_str()],
        ).unwrap();
        let _ = enqueue(
            db.clone(), root.path().join("settings.json"), root.path().join("vault"),
            registry, crate::native::semantic::SemanticEngine::new(None),
            json!({"taskKind":"user_generation","entityType":"tasks","entityId":"task-1","sourceRevision":"r2","payload":"new"}),
        ).await.unwrap();
        assert!(old_token.is_cancelled());
        let stored:(String,String,String)=database::open(&db).unwrap().query_row(
            "SELECT status,execution_outcome,application_disposition FROM ai_jobs_v2 WHERE id='old'",[],
            |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        ).unwrap();
        assert_eq!(stored,("stale".into(),"cancelled".into(),"superseded".into()));
    }
}

fn finalize_image_summary(db_path: &Path, vault: &Path, job_id: &str, generation: &str, entity_id: &str, input: &Value, model: &str, source_hash: &str, outcome: Result<Value, String>,
) -> Result<(), String> {
    let mut connection = database::open(db_path)?;
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let running: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM ai_jobs_v2 WHERE id=? AND status='running' AND worker_id=?)", params![job_id,generation], |r| r.get(0)).map_err(|e| e.to_string())?;
    if !running { return Ok(()); }
    let result = outcome.and_then(|value| { crate::native::job_results::prepare_result(crate::native::job_results::JobContext {
        job_id,
        connection: &tx, db_path, task: "image_summary", entity_id, input, vault, model, source_hash,
    }, value,
        )
    })?;
    tx.execute("UPDATE ai_jobs_v2 SET status='completed',execution_outcome='succeeded',application_disposition='applied',result_json=?,progress_completed=1,finished_at=CURRENT_TIMESTAMP WHERE id=? AND status='running' AND worker_id=?", params![result.to_string(),job_id,generation]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod image_summary_tests {
    use super::*;

    #[test]
    fn finishing_an_old_attempt_keeps_the_retry_cancellable() {
        let registry = JobRegistry::default();
        let (old, _) = registry.register("job").unwrap();
        registry.cancel("job");
        let (_, retry) = registry.register("job").unwrap();
        registry.finish("job", &old);
        registry.cancel("job");
        assert!(retry.is_cancelled());
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, String, String) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let service = crate::application::task_service::TaskApplicationService::new(&db);
        let task = service.execute("task.create", &json!({"operationId":"task","title":"Image evidence","inputText":"Image evidence"})).unwrap();
        let task_id = task["id"].as_str().unwrap().to_owned();
        let entry = service.execute("task.work-log.create", &json!({"operationId":"image","taskId":task_id,"expectedTaskRevision":1,"body":"","attachment":{"name":"capture.png","mediaType":"image/png","data":"aGVsbG8="}})).unwrap();
        let entry_id = entry["id"].as_str().unwrap().to_owned();
        database::open(&db).unwrap().execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,worker_id) VALUES('image-job','image_summary','task_work_log_entries',?,'running','{}','image-job','image-worker')", [&entry_id]).unwrap();
        (root, db, task_id, entry_id)
    }

    fn finish(db: &Path, entry: &str, hash: &str, result: Value) -> Result<(), String> {
        finalize_image_summary(db, db.parent().unwrap(), "image-job", "image-worker", entry, &json!({"entityType":"task_work_log_entries","locale":"ko"}), "test", hash, Ok(result),
        )
    }

    fn versions() -> Value { json!({"ko":{"summary":"화면의 작업 근거"},"en":{"summary":"Evidence in the screenshot"}}) }

    #[test]
    fn task_image_summary_saves_both_languages_and_queue_result_together() {
        let (_root, db, task, entry) = fixture();
        finish(&db, &entry, &image_source_hash("aGVsbG8=", "image/png"), versions(),
        ).unwrap();
        let service = crate::application::task_service::TaskApplicationService::new(&db);
        let saved = service.execute("task.get", &json!({"taskId":task})).unwrap();
        assert_eq!(saved["workLog"][0]["imageSummary"], "Evidence in the screenshot");
        assert_eq!(saved["workLog"][0]["imageSummaryVersions"]["ko"]["image_summary"], "화면의 작업 근거");
        assert_eq!(saved["workLog"][0]["imageSummaryVersions"]["en"]["image_summary"], "Evidence in the screenshot");
        assert_eq!(saved["workLog"][0]["imageSummaryJob"]["status"], "completed");
        assert_eq!(result(&db, "image-job").unwrap()["result"]["taskId"], task);
    }

    #[test]
    fn invalid_stale_cancelled_and_deleted_image_jobs_write_no_summaries() {
        for case in ["missing-language", "blank-language", "stale", "cancelled", "retried", "deleted",
        ] {
            let (_root, db, task, entry) = fixture();
            let connection = database::open(&db).unwrap();
            let mut output = versions();
            let mut hash = image_source_hash("aGVsbG8=", "image/png");
            match case {
                "missing-language" => output = json!({"en":{"summary":"Only English"}}),
                "blank-language" => output["ko"]["summary"] = json!("  "),
                "stale" => hash = "outdated".into(),
                "cancelled" => { connection.execute("UPDATE ai_jobs_v2 SET status='cancelled'", []).unwrap(); }
                "retried" => { connection.execute("UPDATE ai_jobs_v2 SET attempt=attempt+1,worker_id='retry-worker'", [],
                        )
                        .unwrap();
                }
                "deleted" => { crate::application::task_service::TaskApplicationService::new(&db).execute("task.delete", &json!({"taskId":task})).unwrap(); }
                _ => unreachable!(),
            }
            let outcome = finish(&db, &entry, &hash, output);
            assert_eq!(outcome.is_ok(), matches!(case, "cancelled" | "retried"), "{case}: {outcome:?}");
            let count: i64 = connection.query_row("SELECT count(*) FROM localized_content WHERE entity_type='task_work_log_entries'", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 0, "{case}");
            let summary: String = connection.query_row("SELECT image_summary FROM task_work_log_entries WHERE id=?", [&entry], |r| r.get(0),
                ).unwrap();
            assert!(summary.is_empty(), "{case}");
        }
    }

    #[test]
    fn task_journey_titles_are_bounded_and_never_change_recorded_events() {
        let recorded = json!({"events":[
            {"id":"work","type":"work_recorded","detail":{"summary":"A complete, deliberately long recorded Work Log entry"}},
            {"id":"decision","type":"decision_recorded","detail":{"payload":{"rationale":"Keep original evidence"}}
        }],"edges":[{"from":"work","to":"decision","kind":"followed_by"}]});
        let inference = json!({"titles":{"work":{"en":"This is a deliberately overlong graph node label that must be bounded"}},"relationships":[]});
        let graph = journey_graph(&recorded, "en", Some(&inference));
        assert_eq!(graph["events"], recorded["events"]);
        assert_eq!(graph["edges"], recorded["edges"]);
        assert!(graph["titles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|title| title["title"].as_str().unwrap().chars().count() <= 48));
        assert_eq!(
            graph["titles"][0]["title"],
            "This is a deliberately overlong graph node labe…"
        );
        let fallback = journey_graph(&recorded, "ko", None);
        assert_eq!(fallback["titles"][0]["title"], "작업 기록");
        assert_eq!(fallback["relationships"], json!([]));
    }

    #[test]
    fn task_journey_keeps_only_ordered_relationships_grounded_in_both_endpoints() {
        let recorded = json!({"events":[
            {"id":"decision-old","type":"decision_recorded","detail":{"payload":{"rationale":"Use CSV export"}}},
            {"id":"work","type":"work_recorded","detail":{"summary":"The API must exist before the UI can ship"}},
            {"id":"decision-new","type":"decision_recorded","detail":{"payload":{"rationale":"Replace CSV export with JSON export"}}}
        ],"edges":[]});
        let inference = json!({"titles":{},"relationships":[
            {"from":"decision-new","to":"decision-old","kind":"supersedes","rationale":"Malformed duplicate first.","evidence":[{"eventId":"decision-new","quote":"not recorded"},{"eventId":"decision-old","quote":"Use CSV export"}]},
            {"from":"decision-new","to":"decision-old","kind":"supersedes","rationale":"The later choice explicitly replaces the old format.","evidence":[{"eventId":"decision-new","quote":"Replace CSV export"},{"eventId":"decision-old","quote":"Use CSV export"}]},
            {"from":"work","to":"decision-old","kind":"invented","rationale":"Unsupported kind","evidence":[{"eventId":"work","quote":"API must exist"},{"eventId":"decision-old","quote":"Use CSV"}]},
            {"from":"decision-old","to":"decision-new","kind":"depends_on","rationale":"Forward link","evidence":[{"eventId":"decision-old","quote":"Use CSV"},{"eventId":"decision-new","quote":"JSON export"}]},
            {"from":"decision-new","to":"work","kind":"derived_from","rationale":"Hallucinated evidence","evidence":[{"eventId":"decision-new","quote":"Replace CSV"},{"eventId":"work","quote":"a database migration"}]},
            {"from":"decision-new","to":"decision-new","kind":"supersedes","rationale":"Self link","evidence":[{"eventId":"decision-new","quote":"JSON export"}]},
            {"from":"missing","to":"work","kind":"depends_on","rationale":"Missing endpoint","evidence":[]}
        ]});
        let graph = journey_graph(&recorded, "en", Some(&inference));
        assert_eq!(graph["relationships"].as_array().unwrap().len(), 1);
        assert_eq!(graph["relationships"][0]["kind"], "supersedes");
        assert_eq!(graph["relationships"][0]["provenance"], "inferred");
        assert_eq!(graph["relationships"][0]["from"], "decision-new");
        assert_eq!(graph["relationships"][0]["to"], "decision-old");
    }

    #[test]
    fn task_journey_cache_and_active_job_are_locale_bound() {
        let (_root, db, task, _) = fixture();
        let before =
            crate::native::task_assistance::task_lineage_for_locale(&db, &task, "en").unwrap();
        let source = before["journeySourceHash"].as_str().unwrap();
        let connection = database::open(&db).unwrap();
        connection.execute("INSERT INTO task_journey_graphs(task_id,locale,source_hash,graph_json) VALUES(?,?,?,?)", params![task,"en",source,json!({"events":[],"titles":[{"id":"x","title":"English"}]}).to_string()]).unwrap();
        connection.execute("INSERT INTO task_journey_graphs(task_id,locale,source_hash,graph_json) VALUES(?,?,?,?)", params![task,"ko",source,json!({"events":[],"titles":[{"id":"x","title":"한국어"}]}).to_string()]).unwrap();
        connection.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key) VALUES('journey','lineage_inference','tasks','task-placeholder','running',?,'journey-key')", [json!({"sourceHash":source,"locale":"en"}).to_string()]).unwrap();
        connection
            .execute(
                "UPDATE ai_jobs_v2 SET entity_id=? WHERE id='journey'",
                [&task],
            )
            .unwrap();
        let en = crate::native::task_assistance::task_lineage_for_locale(&db, &task, "en").unwrap();
        let ko = crate::native::task_assistance::task_lineage_for_locale(&db, &task, "ko").unwrap();
        assert_eq!(en["journey"]["titles"][0]["title"], "English");
        assert_eq!(ko["journey"]["titles"][0]["title"], "한국어");
        assert_eq!(en["journeyStatus"]["jobId"], "journey");
        assert!(ko["journeyStatus"].is_null());
    }

    #[test]
    fn task_journey_source_hash_versions_the_inference_contract() {
        let recorded = json!({"events":[],"edges":[]});
        let legacy_hash = format!("{:x}", Sha256::digest(recorded.to_string().as_bytes()));
        assert_ne!(task_journey_source_hash(&recorded), legacy_hash);
    }

    #[tokio::test]
    async fn ensure_task_journey_persists_and_reuses_grounded_fallback() {
        let (root, db, task, _) = fixture();
        let settings = root.path().join("missing-settings.json");
        let first = ensure_task_journey(&db, &settings, &task, "en")
            .await
            .unwrap();
        assert_eq!(first["modelStatus"], "fallback");
        assert_eq!(first["journey"]["relationships"], json!([]));
        let second = ensure_task_journey(&db, &settings, &task, "en")
            .await
            .unwrap();
        assert_eq!(second["sourceHash"], first["sourceHash"]);
        assert_eq!(second["journey"], first["journey"]);
        let count: i64 = database::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM task_journey_graphs WHERE task_id=? AND locale='en'",
                [&task],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn ensure_task_journey_does_not_create_cache_for_deleted_task() {
        let (root, db, task, _) = fixture();
        database::open(&db)
            .unwrap()
            .execute(
                "INSERT INTO deleted_entities(entity_type,entity_id) VALUES('tasks',?)",
                [&task],
            )
            .unwrap();
        let result =
            ensure_task_journey(&db, &root.path().join("missing-settings.json"), &task, "en").await;
        assert_eq!(result.unwrap_err(), "Task not found");
        let count: i64 = database::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM task_journey_graphs WHERE task_id=?",
                [&task],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn task_journey_finalizer_preserves_cache_on_stale_cancelled_and_superseded_jobs() {
        for case in ["fresh", "stale", "cancelled", "superseded", "deleted"] {
            let (_root, db, task, entry) = fixture();
            let snapshot =
                crate::native::task_assistance::task_lineage_for_locale(&db, &task, "en").unwrap();
            let source = snapshot["journeySourceHash"].as_str().unwrap();
            let prepared = json!({"journey":journey_graph(&snapshot["recordedJourney"],"en",None),"modelStatus":"fallback","modelError":"No provider configured"});
            let connection = database::open(&db).unwrap();
            connection.execute("INSERT INTO task_journey_graphs(task_id,locale,source_hash,graph_json) VALUES(?,'en','prior',?)", params![task,json!({"retained":true}).to_string()]).unwrap();
            connection.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,worker_id) VALUES('journey-finalize','lineage_inference','tasks',?,'running',?,'journey-finalize','worker')", params![task,json!({"sourceHash":source,"locale":"en"}).to_string()]).unwrap();
            match case {
                "stale" => {
                    connection.execute("UPDATE task_work_log_entries SET body='New evidence after snapshot' WHERE id=?", [&entry]).unwrap();
                }
                "cancelled" => {
                    connection
                        .execute(
                            "UPDATE ai_jobs_v2 SET status='cancelled' WHERE id='journey-finalize'",
                            [],
                        )
                        .unwrap();
                }
                "superseded" => {
                    connection.execute("UPDATE ai_jobs_v2 SET worker_id='new-worker' WHERE id='journey-finalize'", []).unwrap();
                }
                "deleted" => {
                    connection
                        .execute(
                            "INSERT INTO deleted_entities(entity_type,entity_id) VALUES('tasks',?)",
                            [&task],
                        )
                        .unwrap();
                }
                _ => {}
            }
            finalize_task_journey(
                &db,
                "worker",
                "journey-finalize",
                &task,
                "en",
                source,
                &prepared,
            )
            .unwrap();
            let (hash, graph): (String,String) = connection.query_row("SELECT source_hash,graph_json FROM task_journey_graphs WHERE task_id=? AND locale='en'", [&task], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
            let job = result(&db, "journey-finalize").unwrap();
            if case == "fresh" {
                assert_eq!(hash, source);
                assert_eq!(
                    serde_json::from_str::<Value>(&graph).unwrap(),
                    prepared["journey"]
                );
                assert_eq!(job["status"], "completed");
                assert_eq!(job["result"]["modelStatus"], "fallback");
                assert_eq!(job["result"]["journey"], prepared["journey"]);
            } else {
                assert_eq!(hash, "prior", "{case}");
                assert_eq!(
                    serde_json::from_str::<Value>(&graph).unwrap(),
                    json!({"retained":true})
                );
                assert!(job["result"].get("journey").is_none(), "{case}");
                if case == "stale" || case == "deleted" {
                    assert_eq!(job["status"], "stale");
                }
            }
        }
    }
}
