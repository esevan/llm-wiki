use crate::native::database;
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn job_view(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": row.get::<_, String>(0)?, "task_kind": row.get::<_, String>(1)?,
        "entity_type": row.get::<_, String>(2)?, "entity_id": row.get::<_, String>(3)?,
        "status": row.get::<_, String>(4)?,
        "progress": {"completed": row.get::<_, i64>(5)?, "total": row.get::<_, i64>(6)?},
        "result_interface": row.get::<_, String>(7)?,
        "error": match row.get::<_, String>(8)? { value if value.is_empty() => Value::Null, code => json!({"code":code,"message":row.get::<_,String>(9)?}) },
        "created_at": row.get::<_, String>(10)?, "started_at": row.get::<_, Option<String>>(11)?,
        "finished_at": row.get::<_, Option<String>>(12)?,
    }))
}

const JOB_SELECT: &str = "SELECT id,task_kind,entity_type,entity_id,status,progress_completed,progress_total,result_interface,error_code,error_message,created_at,started_at,finished_at FROM ai_jobs_v2";

pub fn list(db_path: &Path) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let mut statement = connection
        .prepare(&format!("{JOB_SELECT} ORDER BY created_at DESC"))
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
    connection.query_row("SELECT status,result_interface,result_json FROM ai_jobs_v2 WHERE id=?", [job_id], |row| {
        let raw: String = row.get(2)?;
        Ok(json!({"job_id":job_id,"status":row.get::<_,String>(0)?,"result_interface":row.get::<_,String>(1)?,"result":serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)}))
    }).optional().map_err(|e| e.to_string())?.ok_or_else(|| "AI job not found".into())
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

pub fn cancel(db_path: &Path, job_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    if connection.execute("UPDATE ai_jobs_v2 SET status='cancelled',finished_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('queued','running','retryable')", [job_id]).map_err(|e| e.to_string())? == 0 { return Err("AI job cannot be cancelled".into()); }
    get(db_path, job_id)
}

pub async fn enqueue(
    db_path: PathBuf,
    task_kind: String,
    entity_type: String,
    entity_id: String,
) -> Result<Value, String> {
    let job_id = id();
    database::open(&db_path)?.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status) VALUES (?,?,?,?,'queued')", params![job_id,task_kind,entity_type,entity_id]).map_err(|e| e.to_string())?;
    let spawned_id = job_id.clone();
    let spawned_db = db_path.clone();
    tauri::async_runtime::spawn(async move {
        let _ = run(spawned_db, spawned_id).await;
    });
    get_for_id(&job_id, task_kind, entity_type, entity_id)
}

fn get_for_id(
    job_id: &str,
    task_kind: String,
    entity_type: String,
    entity_id: String,
) -> Result<Value, String> {
    Ok(
        json!({"id":job_id,"task_kind":task_kind,"entity_type":entity_type,"entity_id":entity_id,"status":"queued","progress":{"completed":0,"total":1},"result_interface":"inline_preview","error":null}),
    )
}

async fn run(db_path: PathBuf, job_id: String) -> Result<(), String> {
    let (task, entity_type, entity_id, base_url, model) = {
        let connection = database::open(&db_path)?;
        connection.execute("UPDATE ai_jobs_v2 SET status='running',started_at=CURRENT_TIMESTAMP,attempt=attempt+1 WHERE id=? AND status='queued'", [&job_id]).map_err(|e| e.to_string())?;
        connection.query_row("SELECT j.task_kind,j.entity_type,j.entity_id,p.base_url,p.model FROM ai_jobs_v2 j CROSS JOIN provider_settings p WHERE j.id=? AND p.id=1", [&job_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?))).map_err(|e| e.to_string())?
    };
    let (_, _, api_key) = crate::native::settings::provider_credentials(&db_path)?;
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
        "conflict_review" => r#"Return JSON with "conflicts" and summary."#,
        "completion_review" => "Return a completion decision with problem_recommendation.",
        _ => "Return an empty JSON object.",
    };
    let response = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&json!({"model":model,"messages":[{"role":"user","content":prompt}],"stream":false}))
        .send()
        .await;
    let outcome = match response {
        Ok(value) if value.status().is_success() => value
            .json::<Value>()
            .await
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
    let connection = database::open(&db_path)?;
    match outcome {
        Ok(result) => {
            let result = match task.as_str() {
                "workflow_draft" | "workflow_refinement" => {
                    result.get("en").cloned().unwrap_or(result)
                }
                "completion_review" => json!({"report": result}),
                _ => result,
            };
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
