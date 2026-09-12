use crate::adapters::sqlite::task_repository::{
    create_problem_revision_tx, create_task_tx, SqliteTaskRepository,
};
use crate::native::{database, semantic::SemanticEngine, settings, vault};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use walkdir::WalkDir;

static REVIEW_TOKENS: OnceLock<Mutex<HashMap<String, CancellationToken>>> = OnceLock::new();
static REVIEW_LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn id() -> String {
    Uuid::new_v4().to_string()
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn required<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("invalid_input: {key} is required"))
}

fn operation_replay(tx: &Transaction<'_>, input: &Value) -> Result<Option<Value>, String> {
    let operation_id = required(input, "operationId")?;
    let payload_hash = digest(&input.to_string());
    let stored = tx
        .query_row(
            "SELECT payload_hash,result_json FROM task_assistance_operations WHERE operation_id=?",
            [operation_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match stored {
        Some((stored_hash, result)) if stored_hash == payload_hash => serde_json::from_str(&result)
            .map(Some)
            .map_err(|error| error.to_string()),
        Some(_) => Err("operation_conflict".into()),
        None => Ok(None),
    }
}

fn record_operation(tx: &Transaction<'_>, input: &Value, result: &Value) -> Result<(), String> {
    tx.execute(
        "INSERT INTO task_assistance_operations(operation_id,payload_hash,result_json) VALUES(?,?,?)",
        params![
            required(input, "operationId")?,
            digest(&input.to_string()),
            result.to_string()
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn safe_async_error(error: &str) -> &'static str {
    if error.contains("Configure") {
        "provider_not_configured"
    } else if error == "cancelled" {
        "cancelled"
    } else if error.contains("Provider") || error.contains("provider") {
        "provider_unavailable"
    } else {
        "assistance_failed"
    }
}

async fn provider_json(
    settings_path: &Path,
    task_kind: &str,
    prompt: String,
    token: Option<&CancellationToken>,
) -> Result<Value, String> {
    let (base_url, model, api_key) = settings::provider_credentials_for(settings_path, task_kind)?;
    if model.trim().is_empty() {
        return Err("Configure a model in AI setup before using AI".into());
    }
    let request = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| error.to_string())?
        .post(format!(
            "{}/chat/completions",
            base_url.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&json!({
            "model": model,
            "messages": [{"role":"user","content":prompt}],
            "stream": false
        }));
    let response = if let Some(token) = token {
        tokio::select! {
            _ = token.cancelled() => return Err("cancelled".into()),
            response = request.send() => response,
        }
    } else {
        request.send().await
    }
    .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Provider request failed ({})", response.status()));
    }
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())?;
    let raw = payload
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or("Provider response did not include content")?;
    serde_json::from_str(raw)
        .map_err(|error| format!("Provider response was not valid JSON: {error}"))
}

pub(crate) async fn execute(
    db_path: &Path,
    settings_path: &Path,
    vault_root: &Path,
    semantic: SemanticEngine,
    name: &str,
    input: &Value,
) -> Result<Value, String> {
    match name {
        "task-refinement.open" => refinement_open(db_path, input),
        "task-refinement.get" => refinement_get_for_subject(db_path, input),
        "task-refinement.message" => {
            refinement_message(db_path, settings_path, vault_root, semantic, input).await
        }
        "task-refinement.workspace" => refinement_workspace(db_path, input),
        "task-refinement.proposals" => refinement_proposals(db_path, required(input, "sessionId")?),
        "task-refinement.decision" => refinement_decision(db_path, input),
        "task-review.create" => {
            review_create(db_path, settings_path, vault_root, semantic, input).await
        }
        "task-review.get" => review_get(db_path, vault_root, input),
        "task-review.history" => review_history(db_path, vault_root, input),
        "task-review.cancel" => review_cancel(db_path, required(input, "runId")?),
        "task-review.decision" => review_decision(db_path, input),
        "task-knowledge.draft" => knowledge_draft(db_path, settings_path, input).await,
        "task-knowledge.publish" => knowledge_publish(db_path, vault_root, input),
        "task-knowledge.correction" => knowledge_correction(db_path, input),
        "task-knowledge.regenerate" => knowledge_regenerate(db_path, settings_path, input).await,
        "task-knowledge.withdraw" => knowledge_withdraw(db_path, vault_root, input),
        "task.lineage" => task_lineage(db_path, required(input, "taskId")?),
        _ => Err(format!(
            "Native task assistance operation is not implemented: {name}"
        )),
    }
}

fn subject(input: &Value) -> Result<(&'static str, &str), String> {
    match (
        input.get("captureId").and_then(Value::as_str),
        input.get("taskId").and_then(Value::as_str),
    ) {
        (Some(capture), None) if !capture.trim().is_empty() => Ok(("capture", capture)),
        (None, Some(task)) if !task.trim().is_empty() => Ok(("task", task)),
        _ => Err("invalid_input: supply exactly one captureId or taskId".into()),
    }
}

fn refinement_open(db_path: &Path, input: &Value) -> Result<Value, String> {
    let (kind, subject_id) = subject(input)?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let table = if kind == "capture" {
        "captures"
    } else {
        "tasks"
    };
    let exists: bool = tx
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?)"),
            [subject_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("{kind} not found"));
    }
    let column = if kind == "capture" {
        "capture_id"
    } else {
        "task_id"
    };
    let session_id = tx
        .query_row(
            &format!("SELECT id FROM refinement_sessions WHERE {column}=?"),
            [subject_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .unwrap_or_else(id);
    let initial_draft = if kind == "capture" {
        tx.query_row(
            "SELECT text FROM captures WHERE id=?",
            [subject_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| error.to_string())?
    } else {
        String::new()
    };
    tx.execute(
        &format!(
            "INSERT OR IGNORE INTO refinement_sessions(id,{column},input_draft) VALUES(?,?,?)"
        ),
        params![session_id, subject_id, initial_draft],
    )
    .map_err(|error| error.to_string())?;
    let result = session_value(&tx, &session_id)?;
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn refinement_get_for_subject(db_path: &Path, input: &Value) -> Result<Value, String> {
    let (kind, subject_id) = subject(input)?;
    let connection = database::open(db_path)?;
    let column = if kind == "capture" {
        "capture_id"
    } else {
        "task_id"
    };
    let session_id = connection
        .query_row(
            &format!("SELECT id FROM refinement_sessions WHERE {column}=?"),
            [subject_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Refinement session not found")?;
    session_value(&connection, &session_id)
}

fn session_value(connection: &rusqlite::Connection, session_id: &str) -> Result<Value, String> {
    let mut value = connection
        .query_row(
            "SELECT id,state,current_draft_revision,active_tab,scroll_anchor,input_draft,last_user_activity_at,updated_at,capture_id,task_id FROM refinement_sessions WHERE id=?",
            [session_id],
            |row| Ok(json!({
                "id": row.get::<_,String>(0)?, "state": row.get::<_,String>(1)?,
                "draftRevision": row.get::<_,i64>(2)?, "activeTab": row.get::<_,String>(3)?,
                "scrollAnchor": row.get::<_,String>(4)?, "inputDraft": row.get::<_,String>(5)?,
                "lastUserActivityAt": row.get::<_,String>(6)?, "updatedAt": row.get::<_,String>(7)?,
                "captureId": row.get::<_,Option<String>>(8)?, "taskId": row.get::<_,Option<String>>(9)?
            })),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Refinement session not found")?;
    let mut statement = connection
        .prepare("SELECT id,role,content,created_at FROM refinement_messages WHERE session_id=? ORDER BY created_at,id")
        .map_err(|error| error.to_string())?;
    let messages = statement
        .query_map([session_id], |row| {
            Ok(json!({"id":row.get::<_,String>(0)?,"role":row.get::<_,String>(1)?,"body":row.get::<_,String>(2)?,"createdAt":row.get::<_,String>(3)?}))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    value["messages"] = json!(messages);
    value["responseStatus"] = connection
        .query_row(
            "SELECT status FROM task_assistance_jobs WHERE kind='refinement_response' AND subject_id=? ORDER BY created_at DESC LIMIT 1",
            [session_id],
            |row| row.get::<_,String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(Value::String)
        .unwrap_or(Value::Null);
    Ok(value)
}

fn refinement_prompt(
    connection: &rusqlite::Connection,
    session_id: &str,
) -> Result<String, String> {
    let mut session = session_value(connection, session_id)?;
    if let Some(capture_id) = session.get("captureId").and_then(Value::as_str) {
        let original: String = connection
            .query_row(
                "SELECT text FROM captures WHERE id=?",
                [capture_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        session["originalCapture"] = json!({"id":capture_id,"text":original});
    }
    Ok(format!(
        "Return JSON only as {{\"message\":string,\"proposals\":[{{\"id\":string,\"type\":\"task_patch|new_task|problem_snapshot|task_problem_link\",\"payload\":object}}]}}. Propose durable changes but do not apply them. Use only the supplied local session. A Capture may already contain a proposed solution: preserve it as the starting Task draft and ask only for details that are actually missing; do not restart broad problem or solution discovery.\n\n{}",
        session
    ))
}

fn refinement_workspace(db_path: &Path, input: &Value) -> Result<Value, String> {
    let session_id = required(input, "sessionId")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let current: i64 = tx
        .query_row(
            "SELECT current_draft_revision FROM refinement_sessions WHERE id=?",
            [session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Refinement session not found")?;
    let base = input
        .get("baseDraftRevision")
        .and_then(Value::as_i64)
        .ok_or("invalid_input: baseDraftRevision is required")?;
    if current != base {
        return Err(format!("draft_conflict: currentDraftRevision={current}"));
    }
    let timestamp = now();
    tx.execute(
        "UPDATE refinement_sessions SET input_draft=?,active_tab=?,scroll_anchor=?,last_user_activity_at=?,updated_at=? WHERE id=?",
        params![
            input.get("inputDraft").and_then(Value::as_str).unwrap_or(""),
            input.get("activeTab").and_then(Value::as_str).unwrap_or("refinement"),
            input.get("scrollAnchor").and_then(Value::as_str).unwrap_or(""),
            timestamp,
            timestamp,
            session_id
        ],
    )
    .map_err(|error| error.to_string())?;
    let result = session_value(&tx, session_id)?;
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

async fn refinement_message(
    db_path: &Path,
    settings_path: &Path,
    vault_root: &Path,
    semantic: SemanticEngine,
    input: &Value,
) -> Result<Value, String> {
    let session_id = required(input, "sessionId")?;
    let message = input
        .get("message")
        .or_else(|| input.get("body"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("invalid_input: message is required")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    tx.query_row(
        "SELECT id FROM refinement_sessions WHERE id=?",
        [session_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|error| error.to_string())?
    .ok_or("Refinement session not found")?;
    let message_id = id();
    let job_id = id();
    let timestamp = now();
    tx.execute(
        "INSERT INTO refinement_messages(id,session_id,role,content,created_at) VALUES(?,?,'user',?,?)",
        params![message_id, session_id, message, timestamp],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO task_assistance_jobs(id,kind,subject_id,status,input_json,created_at) VALUES(?,'refinement_response',?,'queued',?,?)",
        params![job_id, session_id, input.to_string(), timestamp],
    )
    .map_err(|error| error.to_string())?;
    tx.execute(
        "UPDATE refinement_sessions SET input_draft='',last_user_activity_at=?,updated_at=? WHERE id=?",
        params![timestamp, timestamp, session_id],
    )
    .map_err(|error| error.to_string())?;
    let result = json!({"id":message_id,"sessionId":session_id,"jobId":job_id,"status":"queued"});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;

    let db = db_path.to_owned();
    let settings = settings_path.to_owned();
    let vault = vault_root.to_owned();
    let auto_review = input
        .get("autoReview")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let session = session_id.to_owned();
    tokio::spawn(async move {
        if let Err(error) = run_refinement_response(
            &db,
            &settings,
            &vault,
            semantic,
            &session,
            &job_id,
            auto_review,
        )
        .await
        {
            if let Ok(connection) = database::open(&db) {
                let _ = connection.execute(
                    "UPDATE task_assistance_jobs SET status='failed',error=?,finished_at=? WHERE id=? AND status IN ('queued','running')",
                    params![safe_async_error(&error), now(), job_id],
                );
            }
        }
    });
    Ok(result)
}

async fn run_refinement_response(
    db_path: &Path,
    settings_path: &Path,
    vault_root: &Path,
    semantic: SemanticEngine,
    session_id: &str,
    job_id: &str,
    auto_review: bool,
) -> Result<(), String> {
    let connection = database::open(db_path)?;
    connection
        .execute(
            "UPDATE task_assistance_jobs SET status='running',started_at=? WHERE id=? AND status='queued'",
            params![now(), job_id],
        )
        .map_err(|error| error.to_string())?;
    let session = session_value(&connection, session_id)?;
    let task_kind = if session.get("taskId").is_some_and(|value| !value.is_null()) {
        "solution_assistance"
    } else {
        "capture_assistance"
    };
    let prompt = refinement_prompt(&connection, session_id)?;
    match provider_json(settings_path, task_kind, prompt, None).await {
        Ok(response) => {
            let assistant = required(&response, "message")?.to_owned();
            let proposals = response
                .get("proposals")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            validate_proposals(&proposals)?;
            let payload = json!({"message":assistant,"proposals":proposals});
            // Keep SQLite values inside this scope. rusqlite transactions are not Send and
            // must be released before the optional asynchronous review is started.
            let revision = {
                let mut connection = database::open(db_path)?;
                let tx = connection
                    .transaction()
                    .map_err(|error| error.to_string())?;
                let revision: i64 = tx
                    .query_row(
                        "SELECT current_draft_revision+1 FROM refinement_sessions WHERE id=?",
                        [session_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())?;
                let timestamp = now();
                tx.execute(
                    "INSERT INTO refinement_messages(id,session_id,role,content,created_at) VALUES(?,?,'assistant',?,?)",
                    params![id(),session_id,assistant,timestamp],
                )
                .map_err(|error| error.to_string())?;
                tx.execute(
                    "INSERT INTO refinement_drafts(session_id,revision,material_hash,payload_json,created_at) VALUES(?,?,?,?,?)",
                    params![session_id,revision,digest(&payload.to_string()),payload.to_string(),timestamp],
                )
                .map_err(|error| error.to_string())?;
                tx.execute(
                    "UPDATE refinement_sessions SET current_draft_revision=?,updated_at=? WHERE id=?",
                    params![revision, timestamp, session_id],
                )
                .map_err(|error| error.to_string())?;
                tx.execute(
                    "UPDATE task_assistance_jobs SET status='completed',result_json=?,finished_at=? WHERE id=?",
                    params![payload.to_string(),timestamp,job_id],
                )
                .map_err(|error| error.to_string())?;
                tx.commit().map_err(|error| error.to_string())?;
                revision
            };
            if auto_review && !proposals.is_empty() {
                let session = database::open(db_path)?
                    .query_row(
                        "SELECT capture_id,task_id FROM refinement_sessions WHERE id=?",
                        [session_id],
                        |row| {
                            Ok((
                                row.get::<_, Option<String>>(0)?,
                                row.get::<_, Option<String>>(1)?,
                            ))
                        },
                    )
                    .map_err(|error| error.to_string())?;
                let subject = if let Some(capture_id) = session.0 {
                    json!({"kind":"capture_draft","captureId":capture_id,"sessionId":session_id,"draftRevision":revision})
                } else {
                    let task_id = session.1.ok_or("Refinement subject disappeared")?;
                    let task_revision: i64 = database::open(db_path)?
                        .query_row(
                            "SELECT current_revision FROM tasks WHERE id=?",
                            [&task_id],
                            |row| row.get(0),
                        )
                        .map_err(|error| error.to_string())?;
                    json!({"kind":"task_revision","taskId":task_id,"taskRevision":task_revision})
                };
                let input = json!({"operationId":id(),"subject":subject,"triggerKind":"automatic"});
                let _ = review_create(db_path, settings_path, vault_root, semantic, &input).await;
            }
            Ok(())
        }
        Err(error) => {
            database::open(db_path)?
                .execute(
                    "UPDATE task_assistance_jobs SET status='failed',error=?,finished_at=? WHERE id=?",
                    params![error, now(), job_id],
                )
                .map_err(|write_error| write_error.to_string())?;
            Ok(())
        }
    }
}

fn validate_proposals(proposals: &[Value]) -> Result<(), String> {
    for proposal in proposals {
        required(proposal, "id")?;
        let kind = required(proposal, "type")?;
        if !matches!(
            kind,
            "task_patch" | "new_task" | "problem_snapshot" | "task_problem_link"
        ) {
            return Err(format!("Unsupported refinement proposal type: {kind}"));
        }
        if !proposal.get("payload").is_some_and(Value::is_object) {
            return Err("Refinement proposal payload must be an object".into());
        }
    }
    Ok(())
}

fn refinement_proposals(db_path: &Path, session_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let row = connection
        .query_row(
            "SELECT revision,payload_json FROM refinement_drafts WHERE session_id=? ORDER BY revision DESC LIMIT 1",
            [session_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((revision, payload)) = row else {
        return Ok(json!([]));
    };
    let mut proposals = serde_json::from_str::<Value>(&payload)
        .map_err(|error| error.to_string())?
        .get("proposals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut statement = connection.prepare("SELECT proposal_id FROM refinement_proposal_decisions WHERE session_id=? AND draft_revision=?").map_err(|error|error.to_string())?;
    let decided = statement
        .query_map(params![session_id, revision], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    proposals.retain(|proposal| {
        !decided
            .iter()
            .any(|id| proposal.get("id").and_then(Value::as_str) == Some(id))
    });
    for proposal in &mut proposals {
        proposal["draftRevision"] = json!(revision);
    }
    Ok(json!(proposals))
}

fn refinement_decision(db_path: &Path, input: &Value) -> Result<Value, String> {
    let session_id = required(input, "sessionId")?;
    let proposal_id = required(input, "proposalId")?;
    let draft_revision = input
        .get("draftRevision")
        .and_then(Value::as_i64)
        .ok_or("invalid_input: draftRevision is required")?;
    let decision = required(input, "decision")?;
    if !matches!(decision, "accept" | "reject" | "apply" | "edit") {
        return Err("invalid_input: decision must be accept, reject, apply, or edit".into());
    }
    let repo = SqliteTaskRepository::new(db_path);
    repo.transaction(|tx| {
        if let Some(result) = operation_replay(tx, input)? {
            return Ok(result);
        }
        let current: i64 = tx
            .query_row(
                "SELECT current_draft_revision FROM refinement_sessions WHERE id=?",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or("Refinement session not found")?;
        if current != draft_revision {
            return Err(format!("draft_conflict: currentDraftRevision={current}"));
        }
        let (draft_payload, capture_id, session_task_id): (String, Option<String>, Option<String>) = tx
            .query_row(
                "SELECT d.payload_json,s.capture_id,s.task_id FROM refinement_drafts d JOIN refinement_sessions s ON s.id=d.session_id WHERE d.session_id=? AND d.revision=?",
                params![session_id,draft_revision],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
            )
            .map_err(|error| error.to_string())?;
        let draft: Value = serde_json::from_str(&draft_payload).map_err(|error| error.to_string())?;
        let proposal = draft
            .get("proposals")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|item| item.get("id").and_then(Value::as_str) == Some(proposal_id))
            .cloned()
            .ok_or("Refinement proposal not found")?;
        let timestamp = now();
        let result = if decision == "reject" {
            json!({"proposalId":proposal_id,"decision":"reject"})
        } else {
            let payload = input.get("editedPayload").cloned().unwrap_or_else(|| proposal["payload"].clone());
            apply_proposal_tx(tx, proposal["type"].as_str().unwrap_or(""), &payload, capture_id.as_deref(), session_task_id.as_deref(), &timestamp)?
        };
        tx.execute(
            "INSERT INTO refinement_proposal_decisions(id,session_id,draft_revision,proposal_id,decision,result_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?,?)",
            params![id(),session_id,draft_revision,proposal_id,decision,result.to_string(),required(input,"operationId")?,timestamp],
        ).map_err(|error|error.to_string())?;
        record_operation(tx,input,&result)?;
        Ok(result)
    })
}

fn apply_proposal_tx(
    tx: &Transaction<'_>,
    kind: &str,
    payload: &Value,
    capture_id: Option<&str>,
    session_task_id: Option<&str>,
    timestamp: &str,
) -> Result<Value, String> {
    match kind {
        "new_task" => create_task_tx(tx, payload, capture_id, None, timestamp),
        "problem_snapshot" => create_problem_revision_tx(
            tx,
            payload,
            payload.get("problemId").and_then(Value::as_str),
            timestamp,
        ),
        "task_problem_link" => {
            let task_id = payload
                .get("taskId")
                .and_then(Value::as_str)
                .or(session_task_id)
                .ok_or("task_problem_link requires taskId")?;
            let problem_id = required(payload, "problemId")?;
            let problem_revision = payload
                .get("problemRevision")
                .and_then(Value::as_i64)
                .ok_or("task_problem_link requires problemRevision")?;
            let link_id = id();
            tx.execute(
                "INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES(?,?,?,?,?,?,?)",
                params![link_id,task_id,problem_id,problem_revision,payload.get("relationship").and_then(Value::as_str).unwrap_or("context"),payload.get("note").and_then(Value::as_str).unwrap_or(""),timestamp],
            ).map_err(|error|error.to_string())?;
            Ok(
                json!({"id":link_id,"taskId":task_id,"problemId":problem_id,"problemRevision":problem_revision}),
            )
        }
        "task_patch" => revise_task_tx(
            tx,
            session_task_id.ok_or("task_patch requires a Task session")?,
            payload,
            timestamp,
        ),
        _ => Err(format!("Unsupported refinement proposal type: {kind}")),
    }
}

fn revise_task_tx(
    tx: &Transaction<'_>,
    task_id: &str,
    payload: &Value,
    timestamp: &str,
) -> Result<Value, String> {
    let current: i64 = tx
        .query_row(
            "SELECT current_revision FROM tasks WHERE id=?",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if let Some(expected) = payload.get("expectedTaskRevision").and_then(Value::as_i64) {
        if expected != current {
            return Err(format!("head_conflict: currentRevision={current}"));
        }
    }
    let previous:(String,String,String,String,String,String)=tx.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",params![task_id,current],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).map_err(|error|error.to_string())?;
    let patch = payload.get("patch").unwrap_or(payload);
    let field = |key: &str, old: String| {
        patch
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or(&old)
            .trim()
            .to_owned()
    };
    let fields = [
        field("title", previous.0),
        field("detail", previous.1),
        field("outcome", previous.2),
        field("scope", previous.3),
        field("nonGoals", previous.4),
        field("validationCriteria", previous.5),
    ];
    if fields[0].is_empty() {
        return Err("invalid_input: title is required".into());
    }
    let revision = current + 1;
    tx.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,author,source_reference,created_at) VALUES(?,?,?,?,?,?,?,?,?,'ai_proposal','refinement',?)",params![task_id,revision,fields[0],fields[1],fields[2],fields[3],fields[4],fields[5],crate::domain::task::content_hash(&fields.iter().map(String::as_str).collect::<Vec<_>>()),timestamp]).map_err(|error|error.to_string())?;
    tx.execute(
        "UPDATE tasks SET current_revision=? WHERE id=?",
        params![revision, task_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(
        json!({"id":task_id,"taskRevision":revision,"title":fields[0],"detail":fields[1],"outcome":fields[2],"scope":fields[3],"nonGoals":fields[4],"validationCriteria":fields[5]}),
    )
}

#[derive(Clone)]
struct ReviewIdentity {
    kind: String,
    subject_id: String,
    revision: i64,
    material_hash: String,
    material: Value,
}

fn review_identity(
    connection: &rusqlite::Connection,
    subject: &Value,
) -> Result<ReviewIdentity, String> {
    match required(subject, "kind")? {
        "task_revision" => {
            let task_id = required(subject, "taskId")?;
            let revision = subject
                .get("taskRevision")
                .and_then(Value::as_i64)
                .ok_or("taskRevision is required")?;
            let current: i64 = connection
                .query_row(
                    "SELECT current_revision FROM tasks WHERE id=?",
                    [task_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or("Task not found")?;
            if current != revision {
                return Err(format!("head_conflict: currentRevision={current}"));
            }
            let mut material=connection.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria,content_hash FROM task_revisions WHERE task_id=? AND revision=?",params![task_id,revision],|row|Ok(json!({"title":row.get::<_,String>(0)?,"detail":row.get::<_,String>(1)?,"outcome":row.get::<_,String>(2)?,"scope":row.get::<_,String>(3)?,"nonGoals":row.get::<_,String>(4)?,"validationCriteria":row.get::<_,String>(5)?,"taskContentHash":row.get::<_,String>(6)?}))).optional().map_err(|error|error.to_string())?.ok_or("Task revision not found")?;
            let mut statement=connection.prepare("SELECT l.problem_id,l.problem_revision,r.statement,r.detail,r.content_hash FROM task_problem_links l JOIN problem_revisions r ON r.problem_id=l.problem_id AND r.revision=l.problem_revision WHERE l.task_id=? AND l.unlinked_at IS NULL ORDER BY l.problem_id,l.problem_revision").map_err(|error|error.to_string())?;
            let problems=statement.query_map([task_id],|row|Ok(json!({"problemId":row.get::<_,String>(0)?,"problemRevision":row.get::<_,i64>(1)?,"statement":row.get::<_,String>(2)?,"detail":row.get::<_,String>(3)?,"contentHash":row.get::<_,String>(4)?}))).map_err(|error|error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error|error.to_string())?;
            material["problems"] = json!(problems);
            let material_hash = digest(&material.to_string());
            if let Some(provided) = subject
                .get("materialHash")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                if provided != material_hash {
                    return Err("subject_material_conflict".into());
                }
            }
            Ok(ReviewIdentity {
                kind: "task_revision".into(),
                subject_id: task_id.into(),
                revision,
                material_hash,
                material,
            })
        }
        "capture_draft" => {
            let capture_id = required(subject, "captureId")?;
            let session_id = required(subject, "sessionId")?;
            let revision = subject
                .get("draftRevision")
                .and_then(Value::as_i64)
                .ok_or("draftRevision is required")?;
            let (session_capture,current_revision,payload):(String,i64,String)=connection.query_row("SELECT s.capture_id,s.current_draft_revision,d.payload_json FROM refinement_sessions s JOIN refinement_drafts d ON d.session_id=s.id WHERE s.id=? AND d.revision=?",params![session_id,revision],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|error|error.to_string())?.ok_or("Capture refinement draft not found")?;
            if session_capture != capture_id {
                return Err("subject_material_conflict".into());
            }
            if current_revision != revision {
                return Err(format!(
                    "draft_conflict: currentDraftRevision={current_revision}"
                ));
            }
            let material: Value =
                serde_json::from_str(&payload).map_err(|error| error.to_string())?;
            let material_hash = digest(&material.to_string());
            if let Some(provided) = subject
                .get("materialHash")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                if provided != material_hash {
                    return Err("subject_material_conflict".into());
                }
            }
            Ok(ReviewIdentity {
                kind: "capture_draft".into(),
                subject_id: capture_id.into(),
                revision,
                material_hash,
                material,
            })
        }
        _ => Err("invalid_input: review subject kind".into()),
    }
}

fn vault_revision(vault_root: &Path) -> Result<String, String> {
    let mut documents = WalkDir::new(vault_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    documents.sort();
    let mut hasher = Sha256::new();
    for path in documents {
        let relative = path
            .strip_prefix(vault_root)
            .map_err(|error| error.to_string())?;
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(fs::read(&path).map_err(|error| error.to_string())?);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Native task reviews search the user's local Vault directly.  Their authority is therefore the
/// active native desktop connection and its server-stored capabilities, never a client-supplied
/// label.  MCP evidence grants keep their own validation path; this local scope does not consume
/// those short-lived grants.
fn effective_review_scope(connection: &rusqlite::Connection) -> Result<String, String> {
    let (state, scopes, topics, policy): (String, String, String, String) = connection
        .query_row(
            "SELECT state,scopes_json,allowed_topics_json,checkpoint_policy FROM mcp_connections WHERE id='native-in-app-chat'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("review_scope_unavailable")?;
    if state != "active" {
        return Err("review_scope_unavailable".into());
    }
    let scopes: Vec<String> =
        serde_json::from_str(&scopes).map_err(|_| "review_scope_unavailable")?;
    if !scopes.iter().any(|scope| scope == "vault:search:lexical") {
        return Err("review_scope_unavailable".into());
    }
    Ok(digest(
        &json!({
            "kind": "native_desktop_local_vault",
            "connectionId": "native-in-app-chat",
            "scopes": scopes,
            "allowedTopics": serde_json::from_str::<Value>(&topics).unwrap_or(json!([])),
            "checkpointPolicy": policy,
        })
        .to_string(),
    ))
}

async fn review_create(
    db_path: &Path,
    settings_path: &Path,
    vault_root: &Path,
    semantic: SemanticEngine,
    input: &Value,
) -> Result<Value, String> {
    let subject = input
        .get("subject")
        .ok_or("invalid_input: subject is required")?;
    let mut connection = database::open(db_path)?;
    let identity = review_identity(&connection, subject)?;
    let vault_revision = vault_revision(vault_root)?;
    let scope_revision = effective_review_scope(&connection)?;
    let run_id = id();
    let trigger = input
        .get("triggerKind")
        .and_then(Value::as_str)
        .unwrap_or("explicit");
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let old_ids = {
        let mut old=tx.prepare("SELECT id FROM task_conflict_review_runs WHERE subject_kind=? AND subject_id=? AND status IN ('queued','running')").map_err(|error|error.to_string())?;
        let ids = old
            .query_map(params![identity.kind, identity.subject_id], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        ids
    };
    tx.execute("UPDATE task_conflict_review_runs SET status=CASE WHEN status='queued' THEN 'cancelled' ELSE 'stale' END,cancel_requested_at=?,superseded_by_run_id=?,finished_at=CASE WHEN status='queued' THEN ? ELSE finished_at END WHERE subject_kind=? AND subject_id=? AND status IN ('queued','running')",params![now(),run_id,now(),identity.kind,identity.subject_id]).map_err(|error|error.to_string())?;
    tx.execute("INSERT INTO task_conflict_review_runs(id,subject_kind,subject_id,subject_revision,material_hash,vault_revision,scope_revision,trigger_kind,status,subject_json,created_at) VALUES(?,?,?,?,?,?,?,?,'queued',?,?)",params![run_id,identity.kind,identity.subject_id,identity.revision,identity.material_hash,vault_revision,scope_revision,trigger,subject.to_string(),now()]).map_err(|error|error.to_string())?;
    let result = json!({"id":run_id,"status":"queued","current":true});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;

    let tokens = REVIEW_TOKENS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut token_guard = tokens
        .lock()
        .map_err(|_| "Review cancellation registry is unavailable")?;
    for old_id in &old_ids {
        if let Some(token) = token_guard.get(old_id) {
            token.cancel();
        }
    }
    let token = CancellationToken::new();
    token_guard.insert(run_id.clone(), token.clone());
    drop(token_guard);
    let db = db_path.to_owned();
    let settings = settings_path.to_owned();
    let vault = vault_root.to_owned();
    let run = run_id.clone();
    let trigger = trigger.to_owned();
    tokio::spawn(async move {
        if let Err(error) = run_review(
            &db, &settings, &vault, semantic, &run, &trigger, identity, token,
        )
        .await
        {
            if let Ok(connection) = database::open(&db) {
                let _ = connection.execute(
                    "UPDATE task_conflict_review_runs SET status='failed',error=?,finished_at=? WHERE id=? AND status IN ('queued','running')",
                    params![safe_async_error(&error), now(), run],
                );
            }
        }
        if let Ok(mut tokens) = REVIEW_TOKENS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
        {
            tokens.remove(&run);
        }
    });
    Ok(result)
}

fn review_is_current(
    db_path: &Path,
    vault_root: &Path,
    run_id: &str,
    expected_material_hash: &str,
) -> Result<bool, String> {
    let connection = database::open(db_path)?;
    let (subject_json, expected_vault_revision, expected_scope_revision): (String, String, String) = connection
        .query_row(
            "SELECT subject_json,vault_revision,scope_revision FROM task_conflict_review_runs WHERE id=?",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| error.to_string())?;
    let subject: Value = serde_json::from_str(&subject_json).map_err(|error| error.to_string())?;
    let current = match review_identity(&connection, &subject) {
        Ok(current) => current.material_hash == expected_material_hash,
        Err(error)
            if error.starts_with("head_conflict")
                || error.starts_with("draft_conflict")
                || error == "subject_material_conflict" =>
        {
            false
        }
        Err(error) => return Err(error),
    };
    Ok(current
        && vault_revision(vault_root)? == expected_vault_revision
        && effective_review_scope(&connection).is_ok_and(|scope| scope == expected_scope_revision))
}

#[allow(clippy::too_many_arguments)]
async fn run_review(
    db_path: &Path,
    settings_path: &Path,
    vault_root: &Path,
    semantic: SemanticEngine,
    run_id: &str,
    trigger: &str,
    identity: ReviewIdentity,
    token: CancellationToken,
) -> Result<(), String> {
    if trigger == "automatic" {
        tokio::select! {_=token.cancelled()=>return mark_review_cancelled(db_path,run_id),_=tokio::time::sleep(Duration::from_millis(800))=>{}}
    }
    let semaphore = REVIEW_LIMIT
        .get_or_init(|| Arc::new(Semaphore::new(2)))
        .clone();
    let _permit = tokio::select! {_=token.cancelled()=>return mark_review_cancelled(db_path,run_id),permit=semaphore.acquire_owned()=>permit.map_err(|_|"Review queue is unavailable")?};
    database::open(db_path)?.execute("UPDATE task_conflict_review_runs SET status='running',started_at=? WHERE id=? AND status='queued'",params![now(),run_id]).map_err(|error|error.to_string())?;
    if token.is_cancelled() {
        return mark_review_cancelled(db_path, run_id);
    }
    if !review_is_current(db_path, vault_root, run_id, &identity.material_hash)? {
        return mark_review_stale(db_path, run_id);
    }
    vault::index(db_path, vault_root, &semantic, semantic.available())?;
    let query = review_query(&identity.material);
    let evidence = review_evidence(db_path, &semantic, &query)?;
    if evidence.is_empty() {
        return finish_review(
            db_path,
            run_id,
            "insufficient_evidence",
            json!([]),
            json!([]),
            "",
        );
    }
    database::open(db_path)?
        .execute(
            "UPDATE task_conflict_review_runs SET first_evidence_at=? WHERE id=?",
            params![now(), run_id],
        )
        .map_err(|error| error.to_string())?;
    let prompt=format!("Return JSON only as {{\"status\":\"clear|findings|insufficient_evidence\",\"citations\":[{{\"path\":string}}],\"findings\":[{{\"id\":string,\"path\":string,\"summary\":string}}]}}. Compare only the supplied subject and Vault evidence. Clear requires at least one cited supplied path. Never invent a path or excerpt.\n\n{}",json!({"subject":identity.material,"evidence":evidence}));
    let response = match provider_json(settings_path, "conflict_review", prompt, Some(&token)).await
    {
        Ok(value) => value,
        Err(error) if error == "cancelled" => return mark_review_cancelled(db_path, run_id),
        Err(error) => {
            return finish_review(
                db_path,
                run_id,
                "failed",
                json!([]),
                json!(evidence),
                safe_async_error(&error),
            )
        }
    };
    if token.is_cancelled() {
        return mark_review_cancelled(db_path, run_id);
    }
    if !review_is_current(db_path, vault_root, run_id, &identity.material_hash)? {
        return mark_review_stale(db_path, run_id);
    }
    let (status, findings) = match validate_review_result(&response, &evidence) {
        Ok(result) => result,
        Err(error) => {
            return finish_review(
                db_path,
                run_id,
                "failed",
                json!([]),
                json!(evidence),
                safe_async_error(&error),
            )
        }
    };
    finish_review(
        db_path,
        run_id,
        status,
        json!(findings),
        json!(evidence),
        "",
    )
}

fn review_query(material: &Value) -> String {
    let mut parts = ["title", "outcome", "scope", "validationCriteria", "message"]
        .into_iter()
        .filter_map(|key| material.get(key).and_then(Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if let Some(proposals) = material.get("proposals") {
        parts.push(proposals.to_string());
    }
    parts.join(" ")
}

fn review_evidence(
    db_path: &Path,
    semantic: &SemanticEngine,
    query: &str,
) -> Result<Vec<Value>, String> {
    let mut evidence = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for term in query
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.chars().count() >= 4)
        .take(10)
    {
        let result = vault::search(db_path, semantic, term, 4, 0, semantic.available())?;
        for item in result
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let path = item.get("path").and_then(Value::as_str).unwrap_or("");
            if path.is_empty() || !seen.insert(path.to_owned()) {
                continue;
            }
            evidence.push(json!({"sourceId":item.get("source_hash"),"path":path,"title":item.get("title"),"excerpt":item.get("snippet"),"sourceHash":item.get("source_hash")}));
            if evidence.len() == 8 {
                return Ok(evidence);
            }
        }
    }
    Ok(evidence)
}

fn validate_review_result(
    response: &Value,
    evidence: &[Value],
) -> Result<(&'static str, Vec<Value>), String> {
    let status = required(response, "status")?;
    if status == "insufficient_evidence" {
        return Ok(("insufficient_evidence", Vec::new()));
    }
    let paths = evidence
        .iter()
        .filter_map(|item| {
            item.get("path")
                .and_then(Value::as_str)
                .map(|path| (path, item))
        })
        .collect::<HashMap<_, _>>();
    let citations = response
        .get("citations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if status == "clear" {
        if citations.is_empty()
            || citations.iter().any(|citation| {
                !citation
                    .get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|path| paths.contains_key(path))
            })
        {
            return Err("Clear review did not cite supplied evidence".into());
        }
        return Ok(("clear", Vec::new()));
    }
    if status != "findings" {
        return Err("Review status was invalid".into());
    }
    let mut findings = Vec::new();
    for finding in response
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = required(finding, "path")?;
        let evidence = paths
            .get(path)
            .ok_or("Review finding cited unknown evidence")?;
        findings.push(json!({"id":finding.get("id").and_then(Value::as_str).filter(|value|!value.is_empty()).map(str::to_owned).unwrap_or_else(id),"sourceId":evidence.get("sourceId"),"path":path,"excerpt":evidence.get("excerpt"),"summary":finding.get("summary").and_then(Value::as_str).unwrap_or("")}));
    }
    if findings.is_empty() {
        return Err("Findings review did not include a cited finding".into());
    }
    Ok(("findings", findings))
}

fn finish_review(
    db_path: &Path,
    run_id: &str,
    status: &str,
    findings: Value,
    evidence: Value,
    error: &str,
) -> Result<(), String> {
    database::open(db_path)?.execute("UPDATE task_conflict_review_runs SET status=?,findings_json=?,evidence_json=?,error=?,finished_at=? WHERE id=? AND status IN ('queued','running')",params![status,findings.to_string(),evidence.to_string(),error,now(),run_id]).map_err(|write_error|write_error.to_string())?;
    Ok(())
}
fn mark_review_cancelled(db_path: &Path, run_id: &str) -> Result<(), String> {
    finish_review(db_path, run_id, "cancelled", json!([]), json!([]), "")
}
fn mark_review_stale(db_path: &Path, run_id: &str) -> Result<(), String> {
    finish_review(db_path, run_id, "stale", json!([]), json!([]), "")
}

fn review_get(db_path: &Path, vault_root: &Path, input: &Value) -> Result<Value, String> {
    let run_id = required(input, "runId")?;
    let connection = database::open(db_path)?;
    let mut value=connection.query_row("SELECT id,subject_kind,subject_id,subject_revision,material_hash,vault_revision,scope_revision,trigger_kind,status,subject_json,findings_json,evidence_json,error,created_at,started_at,first_evidence_at,finished_at FROM task_conflict_review_runs WHERE id=?",[run_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"subjectKind":row.get::<_,String>(1)?,"subjectId":row.get::<_,String>(2)?,"subjectRevision":row.get::<_,i64>(3)?,"materialHash":row.get::<_,String>(4)?,"vaultRevision":row.get::<_,String>(5)?,"scopeRevision":row.get::<_,String>(6)?,"triggerKind":row.get::<_,String>(7)?,"status":row.get::<_,String>(8)?,"subject":serde_json::from_str::<Value>(&row.get::<_,String>(9)?).unwrap_or(Value::Null),"findings":serde_json::from_str::<Value>(&row.get::<_,String>(10)?).unwrap_or(json!([])),"evidence":serde_json::from_str::<Value>(&row.get::<_,String>(11)?).unwrap_or(json!([])),"safeError":row.get::<_,String>(12)?,"createdAt":row.get::<_,String>(13)?,"startedAt":row.get::<_,Option<String>>(14)?,"firstEvidenceAt":row.get::<_,Option<String>>(15)?,"finishedAt":row.get::<_,Option<String>>(16)?}))).optional().map_err(|error|error.to_string())?.ok_or("Conflict review not found")?;
    let identity = review_identity(&connection, &value["subject"]);
    let current = identity.is_ok_and(|current| {
        current.material_hash == value["materialHash"].as_str().unwrap_or("")
            && vault_revision(vault_root)
                .is_ok_and(|revision| revision == value["vaultRevision"].as_str().unwrap_or(""))
            && effective_review_scope(&connection)
                .is_ok_and(|revision| revision == value["scopeRevision"].as_str().unwrap_or(""))
    });
    value["current"] = json!(current);
    if !current
        && matches!(
            value["status"].as_str(),
            Some("queued" | "running" | "clear" | "findings" | "insufficient_evidence")
        )
    {
        value["status"] = json!("stale")
    }
    Ok(value)
}

fn review_value(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(
        json!({"id":row.get::<_,String>(0)?,"subjectKind":row.get::<_,String>(1)?,"subjectId":row.get::<_,String>(2)?,"subjectRevision":row.get::<_,i64>(3)?,"materialHash":row.get::<_,String>(4)?,"vaultRevision":row.get::<_,String>(5)?,"scopeRevision":row.get::<_,String>(6)?,"triggerKind":row.get::<_,String>(7)?,"status":row.get::<_,String>(8)?,"subject":serde_json::from_str::<Value>(&row.get::<_,String>(9)?).unwrap_or(Value::Null),"findings":serde_json::from_str::<Value>(&row.get::<_,String>(10)?).unwrap_or(json!([])),"evidence":serde_json::from_str::<Value>(&row.get::<_,String>(11)?).unwrap_or(json!([])),"safeError":row.get::<_,String>(12)?,"createdAt":row.get::<_,String>(13)?,"startedAt":row.get::<_,Option<String>>(14)?,"firstEvidenceAt":row.get::<_,Option<String>>(15)?,"finishedAt":row.get::<_,Option<String>>(16)?}),
    )
}

fn review_current(connection: &rusqlite::Connection, vault_root: &Path, value: &Value) -> bool {
    review_identity(connection, &value["subject"]).is_ok_and(|identity| {
        identity.material_hash == value["materialHash"].as_str().unwrap_or("")
            && vault_revision(vault_root)
                .is_ok_and(|revision| revision == value["vaultRevision"].as_str().unwrap_or(""))
            && effective_review_scope(connection)
                .is_ok_and(|revision| revision == value["scopeRevision"].as_str().unwrap_or(""))
    })
}

fn review_history(db_path: &Path, vault_root: &Path, input: &Value) -> Result<Value, String> {
    let subject = input
        .get("subject")
        .ok_or("invalid_input: subject is required")?;
    let connection = database::open(db_path)?;
    let identity = review_identity(&connection, subject)?;
    let mut statement = connection.prepare("SELECT id,subject_kind,subject_id,subject_revision,material_hash,vault_revision,scope_revision,trigger_kind,status,subject_json,findings_json,evidence_json,error,created_at,started_at,first_evidence_at,finished_at FROM task_conflict_review_runs WHERE subject_kind=? AND subject_id=? ORDER BY created_at DESC").map_err(|error| error.to_string())?;
    let attempts = statement
        .query_map(params![identity.kind, identity.subject_id], review_value)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|mut attempt| {
            let current = review_current(&connection, vault_root, &attempt);
            attempt["current"] = json!(current);
            if !current
                && matches!(
                    attempt["status"].as_str(),
                    Some("clear" | "findings" | "insufficient_evidence")
                )
            {
                attempt["status"] = json!("stale");
            }
            attempt
        })
        .collect::<Vec<_>>();
    let current_result = attempts
        .iter()
        .find(|attempt| {
            attempt["current"] == true
                && matches!(
                    attempt["status"].as_str(),
                    Some("clear" | "findings" | "insufficient_evidence")
                )
        })
        .cloned();
    Ok(json!({"attempts":attempts,"currentResult":current_result}))
}

fn review_cancel(db_path: &Path, run_id: &str) -> Result<Value, String> {
    if let Ok(guard) = REVIEW_TOKENS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        if let Some(token) = guard.get(run_id) {
            token.cancel();
        }
    }
    let changed=database::open(db_path)?.execute("UPDATE task_conflict_review_runs SET status='cancelled',cancel_requested_at=?,finished_at=? WHERE id=? AND status IN ('queued','running')",params![now(),now(),run_id]).map_err(|error|error.to_string())?;
    if changed == 0 {
        return Err("Conflict review is not cancellable".into());
    }
    Ok(json!({"id":run_id,"status":"cancelled","current":false}))
}

fn review_decision(db_path: &Path, input: &Value) -> Result<Value, String> {
    let finding_id = required(input, "findingId")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let run_id=input.get("runId").and_then(Value::as_str).map(str::to_owned).or_else(||tx.query_row("SELECT id FROM task_conflict_review_runs WHERE EXISTS(SELECT 1 FROM json_each(findings_json) WHERE json_extract(value,'$.id')=?) ORDER BY created_at DESC LIMIT 1",[finding_id],|row|row.get(0)).optional().ok().flatten()).ok_or("Conflict finding not found")?;
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM task_conflict_review_runs, json_each(findings_json) WHERE task_conflict_review_runs.id=? AND json_extract(value,'$.id')=?)",
            params![run_id, finding_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err("Conflict finding not found".into());
    }
    let disposition = required(input, "disposition")?;
    let decision_id = id();
    tx.execute("INSERT INTO task_conflict_decisions(id,run_id,finding_id,disposition,rationale,created_at) VALUES(?,?,?,?,?,?)",params![decision_id,run_id,finding_id,disposition,input.get("rationale").and_then(Value::as_str).unwrap_or(""),now()]).map_err(|error|error.to_string())?;
    let result =
        json!({"id":decision_id,"runId":run_id,"findingId":finding_id,"disposition":disposition});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn task_lineage(db_path: &Path, task_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (revision, state, origin): (i64, String, Option<String>) = connection
        .query_row(
            "SELECT current_revision,state,origin_capture_id FROM tasks WHERE id=?",
            [task_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Task not found")?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    if let Some(capture) = origin {
        nodes.push(json!({"id":format!("capture:{capture}"),"kind":"capture","recordId":capture}));
        edges.push(json!({"from":format!("capture:{capture}"),"to":format!("task:{task_id}:{revision}"),"kind":"derived_from"}));
    }
    let task_node = format!("task:{task_id}:{revision}");
    nodes.push(json!({"id":task_node,"kind":"task_revision","recordId":task_id,"revision":revision,"state":state}));
    collect_nodes(&connection,"SELECT l.problem_id,l.problem_revision,r.statement FROM task_problem_links l JOIN problem_revisions r ON r.problem_id=l.problem_id AND r.revision=l.problem_revision WHERE l.task_id=? AND l.unlinked_at IS NULL",task_id,|row|Ok((json!({"id":format!("problem:{}:{}",row.get::<_,String>(0)?,row.get::<_,i64>(1)?),"kind":"problem_revision","recordId":row.get::<_,String>(0)?,"revision":row.get::<_,i64>(1)?,"title":row.get::<_,String>(2)?}),"linked_problem".into())),&task_node,&mut nodes,&mut edges)?;
    collect_nodes(
        &connection,
        "SELECT id,body FROM task_work_log_entries WHERE task_id=? ORDER BY created_at",
        task_id,
        |row| {
            Ok((
                json!({"id":format!("work_log:{}",row.get::<_,String>(0)?),"kind":"work_log","recordId":row.get::<_,String>(0)?,"summary":row.get::<_,String>(1)?}),
                "evidences".into(),
            ))
        },
        &task_node,
        &mut nodes,
        &mut edges,
    )?;
    collect_nodes(
        &connection,
        "SELECT id,kind FROM task_decisions WHERE task_id=? ORDER BY created_at",
        task_id,
        |row| {
            Ok((
                json!({"id":format!("decision:{}",row.get::<_,String>(0)?),"kind":"decision","recordId":row.get::<_,String>(0)?,"decisionKind":row.get::<_,String>(1)?}),
                "evidences".into(),
            ))
        },
        &task_node,
        &mut nodes,
        &mut edges,
    )?;
    collect_nodes(
        &connection,
        "SELECT id,evidence FROM task_completions WHERE task_id=? ORDER BY created_at",
        task_id,
        |row| {
            Ok((
                json!({"id":format!("completion:{}",row.get::<_,String>(0)?),"kind":"completion","recordId":row.get::<_,String>(0)?,"summary":row.get::<_,String>(1)?}),
                "completed_by".into(),
            ))
        },
        &task_node,
        &mut nodes,
        &mut edges,
    )?;
    let mut statement=connection.prepare("SELECT revision,state,path,content_hash FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision").map_err(|error|error.to_string())?;
    let knowledge = statement
        .query_map([task_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for (draft_revision, state, path, hash) in knowledge {
        let node = format!("knowledge:{task_id}:{draft_revision}");
        nodes.push(json!({"id":node,"kind":"knowledge_revision","recordId":task_id,"revision":draft_revision,"state":state,"path":path,"contentHash":hash}));
        edges.push(json!({"from":task_node,"to":node,"kind":"published_as"}));
    }
    let source_hash = digest(&json!({"nodes":nodes,"edges":edges}).to_string());
    Ok(
        json!({"taskId":task_id,"taskRevision":revision,"sourceHash":source_hash,"nodes":nodes,"edges":edges}),
    )
}

fn collect_nodes<F>(
    connection: &rusqlite::Connection,
    sql: &str,
    task_id: &str,
    mut map: F,
    task_node: &str,
    nodes: &mut Vec<Value>,
    edges: &mut Vec<Value>,
) -> Result<(), String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<(Value, String)>,
{
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let values = statement
        .query_map([task_id], |row| map(row))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    for (node, kind) in values {
        let node_id = node["id"].as_str().unwrap_or("").to_owned();
        nodes.push(node);
        edges.push(json!({"from":node_id,"to":task_node,"kind":kind}));
    }
    Ok(())
}

async fn knowledge_draft(
    db_path: &Path,
    settings_path: &Path,
    input: &Value,
) -> Result<Value, String> {
    let task_id = required(input, "taskId")?;
    let expected = input
        .get("expectedTaskRevision")
        .and_then(Value::as_i64)
        .ok_or("expectedTaskRevision is required")?;
    let connection = database::open(db_path)?;
    let current: i64 = connection
        .query_row(
            "SELECT current_revision FROM tasks WHERE id=?",
            [task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Task not found")?;
    if current != expected {
        return Err(format!("head_conflict: currentRevision={current}"));
    }
    let completion_id:String=connection.query_row("SELECT id FROM task_completions WHERE task_id=? AND task_revision=? ORDER BY created_at DESC LIMIT 1",params![task_id,current],|row|row.get(0)).optional().map_err(|error|error.to_string())?.ok_or("Completed Task evidence not found")?;
    let lineage = task_lineage(db_path, task_id)?;
    let deterministic =
        deterministic_knowledge(&connection, task_id, current, &completion_id, &lineage)?;
    let (body, model_status, model_error) =
        match enhance_knowledge(settings_path, &deterministic, task_id).await {
            Ok(Some(enhanced)) => (
                format!(
                    "{}\n\n## AI-assisted synthesis\n\n{}\n",
                    deterministic.trim_end(),
                    enhanced.trim()
                ),
                "enhanced",
                String::new(),
            ),
            Ok(None) => (deterministic, "deterministic", String::new()),
            Err(error) => (
                deterministic,
                "deterministic",
                safe_async_error(&error).to_owned(),
            ),
        };
    save_knowledge_draft(
        db_path,
        input,
        task_id,
        current,
        &completion_id,
        body,
        lineage,
        model_status,
        &model_error,
    )
}

async fn knowledge_regenerate(
    db_path: &Path,
    settings_path: &Path,
    input: &Value,
) -> Result<Value, String> {
    let mut request = input.clone();
    if request.get("expectedTaskRevision").is_none() {
        let task_id = required(input, "taskId")?;
        let current: i64 = database::open(db_path)?
            .query_row(
                "SELECT current_revision FROM tasks WHERE id=?",
                [task_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or("Task not found")?;
        request["expectedTaskRevision"] = json!(current);
    }
    knowledge_draft(db_path, settings_path, &request).await
}

fn deterministic_knowledge(
    connection: &rusqlite::Connection,
    task_id: &str,
    revision: i64,
    completion_id: &str,
    lineage: &Value,
) -> Result<String, String> {
    let (title,detail,outcome,scope,non_goals,criteria):(String,String,String,String,String,String)=connection.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",params![task_id,revision],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).map_err(|error|error.to_string())?;
    let (evidence, report): (String, String) = connection
        .query_row(
            "SELECT evidence,report FROM task_completions WHERE id=?",
            [completion_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let mut work=connection.prepare("SELECT body,id FROM task_work_log_entries WHERE task_id=? AND trim(body)<>'' ORDER BY created_at").map_err(|error|error.to_string())?;
    let entries = work
        .query_map([task_id], |row| {
            Ok(format!(
                "{} — source `{}`",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut checklist = connection
        .prepare(
            "SELECT body,checked,id FROM task_checklist_items WHERE task_id=? ORDER BY created_at",
        )
        .map_err(|error| error.to_string())?;
    let checklist_lines = checklist
        .query_map([task_id], |row| {
            Ok(format!(
                "[{}] {} — source `{}`",
                if row.get::<_, i64>(1)? != 0 { "x" } else { " " },
                row.get::<_, String>(0)?,
                row.get::<_, String>(2)?
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut decisions = connection
        .prepare(
            "SELECT kind,payload_json,id FROM task_decisions WHERE task_id=? ORDER BY created_at",
        )
        .map_err(|error| error.to_string())?;
    let decision_lines = decisions
        .query_map([task_id], |row| {
            let kind = row.get::<_, String>(0)?;
            let payload =
                serde_json::from_str::<Value>(&row.get::<_, String>(1)?).unwrap_or(Value::Null);
            let body = payload.get("body").and_then(Value::as_str).unwrap_or("");
            Ok(format!(
                "{}{} — source `{}`",
                kind,
                if body.is_empty() {
                    String::new()
                } else {
                    format!(": {body}")
                },
                row.get::<_, String>(2)?
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut problems=connection.prepare("SELECT r.statement,l.problem_id,l.problem_revision FROM task_problem_links l JOIN problem_revisions r ON r.problem_id=l.problem_id AND r.revision=l.problem_revision WHERE l.task_id=? AND l.unlinked_at IS NULL ORDER BY l.created_at").map_err(|error|error.to_string())?;
    let problem_lines = problems
        .query_map([task_id], |row| {
            Ok(format!(
                "{} — Problem `{}` revision {}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(format!("# {title}\n\n## Outcome\n\n{}\n\n## Context\n\n{}\n\n## Scope\n\n{}\n\n## Non-goals\n\n{}\n\n## Validation\n\n{}\n\n## Work evidence\n\n{}\n\n## Checklist\n\n{}\n\n## Decisions\n\n{}\n\n## Completion evidence\n\n{}\n\n{}\n\n## Linked Problem revisions\n\n{}\n\n## Provenance\n\n- Task: `{task_id}` revision {revision}\n- Completion: `{completion_id}`\n- Lineage source: `{}`\n",empty(&outcome),empty(&detail),empty(&scope),empty(&non_goals),empty(&criteria),bullets(&entries),bullets(&checklist_lines),bullets(&decision_lines),evidence,report,bullets(&problem_lines),lineage["sourceHash"].as_str().unwrap_or("")))
}
fn empty(value: &str) -> &str {
    if value.trim().is_empty() {
        "Not recorded."
    } else {
        value
    }
}
fn bullets(values: &[String]) -> String {
    if values.is_empty() {
        "- None recorded.".into()
    } else {
        values
            .iter()
            .map(|value| format!("- {}", value.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

async fn enhance_knowledge(
    settings_path: &Path,
    deterministic: &str,
    task_id: &str,
) -> Result<Option<String>, String> {
    let prompt=format!("Return JSON only as {{\"markdown\":string}}. Improve readability using only this evidence-bound Task document. Preserve the exact Task id `{task_id}` and do not add facts.\n\n{deterministic}");
    let response = match provider_json(settings_path, "completion_report", prompt, None).await {
        Ok(value) => value,
        Err(error) if error.contains("Configure") => return Ok(None),
        Err(error) => return Err(error),
    };
    let markdown = required(&response, "markdown")?;
    if !markdown.contains(task_id) {
        return Err("Model enhancement omitted the exact Task identity".into());
    }
    Ok(Some(markdown.to_owned()))
}

#[allow(clippy::too_many_arguments)]
fn save_knowledge_draft(
    db_path: &Path,
    input: &Value,
    task_id: &str,
    task_revision: i64,
    completion_id: &str,
    body: String,
    lineage: Value,
    model_status: &str,
    model_error: &str,
) -> Result<Value, String> {
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let revision: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM task_knowledge_drafts WHERE task_id=?",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let content_hash = digest(&body);
    let timestamp = now();
    let prior_publication: Option<(String, String)> = tx
        .query_row(
            "SELECT path,published_hash FROM task_knowledge_drafts WHERE task_id=? AND state='published' AND path IS NOT NULL AND published_hash IS NOT NULL ORDER BY revision DESC LIMIT 1",
            [task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    tx.execute("INSERT INTO task_knowledge_drafts(task_id,revision,task_revision,completion_id,body_markdown,content_hash,lineage_json,state,path,published_hash,model_status,model_error,created_at,updated_at) VALUES(?,?,?,?,?,?,?,'draft',?,?,?,?,?,?)",params![task_id,revision,task_revision,completion_id,body,content_hash,lineage.to_string(),prior_publication.as_ref().map(|value|value.0.as_str()),prior_publication.as_ref().map(|value|value.1.as_str()),model_status,model_error,timestamp,timestamp]).map_err(|error|error.to_string())?;
    let result = json!({"taskId":task_id,"draftRevision":revision,"taskRevision":task_revision,"completionId":completion_id,"bodyMarkdown":body,"contentHash":content_hash,"state":"draft","modelStatus":model_status,"modelError":model_error,"lineage":lineage});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

/// A person may correct the private, reviewable draft before publication.  The
/// exact revision and content hash are both required so an older preview can
/// never overwrite a newer correction.
fn knowledge_correction(db_path: &Path, input: &Value) -> Result<Value, String> {
    let task_id = required(input, "taskId")?;
    let revision = input
        .get("draftRevision")
        .and_then(Value::as_i64)
        .ok_or("draftRevision is required")?;
    let expected_hash = required(input, "expectedContentHash")?;
    let body = required(input, "bodyMarkdown")?;
    let hash = digest(body);
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let (stored_hash, state): (String, String) = tx
        .query_row(
            "SELECT content_hash,state FROM task_knowledge_drafts WHERE task_id=? AND revision=?",
            params![task_id, revision],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Knowledge draft not found")?;
    if state != "draft" {
        return Err("Only an unpublished Knowledge draft can be corrected".into());
    }
    if stored_hash != expected_hash {
        return Err("draft_conflict: content hash changed".into());
    }
    tx.execute(
        "UPDATE task_knowledge_drafts SET body_markdown=?,content_hash=?,updated_at=? WHERE task_id=? AND revision=?",
        params![body, hash, now(), task_id, revision],
    )
    .map_err(|error| error.to_string())?;
    let result = json!({"taskId":task_id,"draftRevision":revision,"bodyMarkdown":body,"contentHash":hash,"state":"draft"});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn knowledge_publish(db_path: &Path, vault_root: &Path, input: &Value) -> Result<Value, String> {
    let task_id = required(input, "taskId")?;
    let revision = input
        .get("draftRevision")
        .and_then(Value::as_i64)
        .ok_or("draftRevision is required")?;
    let expected = required(input, "expectedContentHash")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let (body,hash,state,path,published_hash,title):(String,String,String,Option<String>,Option<String>,String)=tx.query_row("SELECT k.body_markdown,k.content_hash,k.state,k.path,k.published_hash,r.title FROM task_knowledge_drafts k JOIN task_revisions r ON r.task_id=k.task_id AND r.revision=k.task_revision WHERE k.task_id=? AND k.revision=?",params![task_id,revision],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional().map_err(|error|error.to_string())?.ok_or("Knowledge draft not found")?;
    if hash != expected {
        return Err("draft_conflict: content hash changed".into());
    }
    if state == "withdrawn" {
        return Err("Withdrawn Knowledge requires a new draft".into());
    }
    let relative =
        path.unwrap_or_else(|| format!("Knowledge/Tasks/{}-{}.md", task_id, slug(&title)));
    let target = vault::resolve_markdown(vault_root, &relative, false)?;
    if target.exists() {
        let current = fs::read_to_string(&target).map_err(|error| error.to_string())?;
        if published_hash.as_deref() != Some(&digest(&current)) {
            return Err("source_changed: Knowledge file changed outside LLM Wiki".into());
        }
    }
    let document=format!("---\nllm_wiki_task_id: \"{task_id}\"\nllm_wiki_task_revision: {}\nllm_wiki_draft_revision: {revision}\nsource_hash: \"{hash}\"\n---\n\n{}",tx.query_row("SELECT task_revision FROM task_knowledge_drafts WHERE task_id=? AND revision=?",params![task_id,revision],|row|row.get::<_,i64>(0)).map_err(|error|error.to_string())?,body);
    vault::atomic_write(vault_root, &relative, &document)?;
    let published = digest(&document);
    tx.execute("UPDATE task_knowledge_drafts SET state='published',path=?,published_hash=?,updated_at=? WHERE task_id=? AND revision=?",params![relative,published,now(),task_id,revision]).map_err(|error|error.to_string())?;
    let result = json!({"taskId":task_id,"draftRevision":revision,"state":"published","path":relative,"publishedHash":published});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn knowledge_withdraw(db_path: &Path, vault_root: &Path, input: &Value) -> Result<Value, String> {
    let task_id = required(input, "taskId")?;
    let revision = input
        .get("draftRevision")
        .and_then(Value::as_i64)
        .ok_or("draftRevision is required")?;
    let mut connection = database::open(db_path)?;
    let tx = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    if let Some(result) = operation_replay(&tx, input)? {
        return Ok(result);
    }
    let (path,published_hash,state):(String,String,String)=tx.query_row("SELECT path,published_hash,state FROM task_knowledge_drafts WHERE task_id=? AND revision=?",params![task_id,revision],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|error|error.to_string())?.ok_or("Published Knowledge draft not found")?;
    if state != "published" {
        return Err("Only published Knowledge can be withdrawn".into());
    }
    let target = vault::resolve_markdown(vault_root, &path, true)?;
    let current = fs::read_to_string(&target).map_err(|error| error.to_string())?;
    if digest(&current) != published_hash {
        return Err("source_changed: Knowledge file changed outside LLM Wiki".into());
    }
    let recovery_relative = format!(".llm-wiki-withdrawn/{}-r{}-{}.md", task_id, revision, id());
    let recovery = vault_root.join(&recovery_relative);
    if let Some(parent) = recovery.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?
    }
    fs::rename(&target, &recovery).map_err(|error| error.to_string())?;
    tx.execute("UPDATE task_knowledge_drafts SET state='withdrawn',updated_at=? WHERE task_id=? AND revision=?",params![now(),task_id,revision]).map_err(|error|error.to_string())?;
    let result = json!({"taskId":task_id,"draftRevision":revision,"state":"withdrawn","recoveryPath":recovery_relative});
    record_operation(&tx, input, &result)?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(result)
}

fn slug(title: &str) -> String {
    let value = title
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let compact = value
        .split('-')
        .filter(|part| !part.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        "task".into()
    } else {
        compact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("app.sqlite3");
        let vault = root.path().join("vault");
        let settings = root.path().join("settings.json");
        fs::create_dir_all(&vault).unwrap();
        database::initialize(&db).unwrap();
        database::open(&db).unwrap().execute("INSERT INTO mcp_connections(id,name,scopes_json,allowed_topics_json,checkpoint_policy,state,created_at,updated_at) VALUES('native-in-app-chat','LLM Wiki Chat','[\"vault:search:lexical\"]','[]','allowed_for_started_sessions','active',?,?)",params![now(),now()]).unwrap();
        (root, db, vault, settings)
    }

    fn capture(db: &Path) -> String {
        let capture = id();
        database::open(db).unwrap().execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,'draft capture',?,'capture',?)",params![capture,now(),now()]).unwrap();
        capture
    }
    fn completed_task(db: &Path) -> String {
        let repo = SqliteTaskRepository::new(db);
        repo.transaction(|tx|{let at=now();let value=create_task_tx(tx,&json!({"title":"Evidence Task","outcome":"A verified outcome","scope":"Local files","validationCriteria":"Evidence exists"}),None,None,&at)?;let task=value["id"].as_str().unwrap().to_owned();tx.execute("UPDATE tasks SET state='completed',completed_at=? WHERE id=?",params![at,task]).map_err(|error|error.to_string())?;tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES(?,?,?,?)",params![id(),task,"Observed exact behavior",at]).map_err(|error|error.to_string())?;tx.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES(?,?,1,'Passing contract test','No open issue',?,?)",params![id(),task,id(),at]).map_err(|error|error.to_string())?;Ok(task)}).unwrap()
    }

    #[tokio::test]
    async fn refinement_persists_one_subject_workspace_and_rejects_stale_draft() {
        let (_root, db, vault, settings) = fixture();
        let capture = capture(&db);
        let opened = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-refinement.open",
            &json!({"operationId":"open-1","captureId":capture}),
        )
        .await
        .unwrap();
        assert_eq!(opened["captureId"], capture);
        assert!(opened["taskId"].is_null());
        assert_eq!(opened["inputDraft"], "draft capture");
        let session = opened["id"].as_str().unwrap();
        let saved=execute(&db,&settings,&vault,SemanticEngine::new(None),"task-refinement.workspace",&json!({"operationId":"save-1","sessionId":session,"inputDraft":"unfinished","activeTab":"proposals","scrollAnchor":"item-3","baseDraftRevision":0})).await.unwrap();
        assert_eq!(saved["inputDraft"], "unfinished");
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE refinement_sessions SET current_draft_revision=1 WHERE id=?",
                [session],
            )
            .unwrap();
        let error=execute(&db,&settings,&vault,SemanticEngine::new(None),"task-refinement.workspace",&json!({"operationId":"save-2","sessionId":session,"inputDraft":"stale","baseDraftRevision":0})).await.unwrap_err();
        assert!(error.contains("draft_conflict"));
    }

    #[test]
    fn refinement_prompt_preserves_original_capture_solution_across_every_turn() {
        let (_root, db, _vault, _settings) = fixture();
        let capture = capture(&db);
        let original = "Existing Solution preserved exactly";
        let connection = database::open(&db).unwrap();
        connection
            .execute(
                "UPDATE captures SET text=? WHERE id=?",
                params![original, capture],
            )
            .unwrap();
        let opened = refinement_open(
            &db,
            &json!({"operationId":"prompt-open","captureId":capture}),
        )
        .unwrap();
        let session = opened["id"].as_str().unwrap();
        connection
            .execute(
                "UPDATE refinement_sessions SET input_draft='' WHERE id=?",
                [session],
            )
            .unwrap();

        let mut expected = Vec::new();
        for turn in 1..=3 {
            let user = format!("user turn {turn}");
            let user_second = turn * 2 - 1;
            connection
                .execute(
                    "INSERT INTO refinement_messages(id,session_id,role,content,created_at) VALUES(?,?, 'user', ?, ?)",
                    params![format!("u{turn}"), session, user, format!("2026-09-09T00:00:{user_second:02}Z")],
                )
                .unwrap();
            expected.push(("user", user));

            let prompt = refinement_prompt(&connection, session).unwrap();
            let context: Value =
                serde_json::from_str(prompt.split_once("\n\n").unwrap().1).unwrap();
            assert_eq!(context["inputDraft"], "");
            assert_eq!(
                context["originalCapture"],
                json!({"id":capture,"text":original})
            );
            let messages = context["messages"].as_array().unwrap();
            assert_eq!(messages.len(), expected.len());
            for (message, (role, body)) in messages.iter().zip(&expected) {
                assert_eq!(message["role"], *role);
                assert_eq!(message["body"], *body);
            }

            let assistant = format!("assistant turn {turn}");
            let assistant_second = turn * 2;
            connection
                .execute(
                    "INSERT INTO refinement_messages(id,session_id,role,content,created_at) VALUES(?,?, 'assistant', ?, ?)",
                    params![format!("a{turn}"), session, assistant, format!("2026-09-09T00:00:{assistant_second:02}Z")],
                )
                .unwrap();
            expected.push(("assistant", assistant));
        }
    }

    #[tokio::test]
    async fn proposal_decision_atomically_creates_only_accepted_task() {
        let (_root, db, vault, settings) = fixture();
        let capture = capture(&db);
        let opened = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-refinement.open",
            &json!({"operationId":"open-2","captureId":capture}),
        )
        .await
        .unwrap();
        let session = opened["id"].as_str().unwrap();
        let payload = json!({"proposals":[{"id":"p1","type":"new_task","payload":{"title":"Accepted Task"}},{"id":"p2","type":"new_task","payload":{"title":"Rejected Task"}}]});
        database::open(&db).unwrap().execute("INSERT INTO refinement_drafts(session_id,revision,material_hash,payload_json) VALUES(?,1,?,?)",params![session,digest(&payload.to_string()),payload.to_string()]).unwrap();
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE refinement_sessions SET current_draft_revision=1 WHERE id=?",
                [session],
            )
            .unwrap();
        let accepted=execute(&db,&settings,&vault,SemanticEngine::new(None),"task-refinement.decision",&json!({"operationId":"decide-1","sessionId":session,"proposalId":"p1","draftRevision":1,"decision":"accept"})).await.unwrap();
        assert_eq!(accepted["title"], "Accepted Task");
        execute(&db,&settings,&vault,SemanticEngine::new(None),"task-refinement.decision",&json!({"operationId":"decide-2","sessionId":session,"proposalId":"p2","draftRevision":1,"decision":"reject"})).await.unwrap();
        let count: i64 = database::open(&db)
            .unwrap()
            .query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn lineage_preserves_many_to_many_problem_revision_provenance() {
        let (_root, db, _vault, _settings) = fixture();
        let repo = SqliteTaskRepository::new(&db);
        let (first_task, second_task, first_problem, second_problem) = repo
            .transaction(|tx| {
                let at = now();
                let first_task = create_task_tx(tx, &json!({"title":"First"}), None, None, &at)?["id"]
                    .as_str().unwrap().to_owned();
                let second_task = create_task_tx(tx, &json!({"title":"Second"}), None, None, &at)?["id"]
                    .as_str().unwrap().to_owned();
                let first_problem = create_problem_revision_tx(tx, &json!({"statement":"Shared context"}), None, &at)?["id"]
                    .as_str().unwrap().to_owned();
                let second_problem = create_problem_revision_tx(tx, &json!({"statement":"Additional context"}), None, &at)?["id"]
                    .as_str().unwrap().to_owned();
                for (task, problem) in [
                    (&first_task, &first_problem),
                    (&first_task, &second_problem),
                    (&second_task, &first_problem),
                ] {
                    tx.execute("INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,created_at) VALUES(?,?,?,1,'context',?)",params![id(),task,problem,at]).map_err(|error|error.to_string())?;
                }
                Ok((first_task, second_task, first_problem, second_problem))
            })
            .unwrap();
        let lineage = task_lineage(&db, &first_task).unwrap();
        let problem_nodes = lineage["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["kind"] == "problem_revision")
            .count();
        assert_eq!(problem_nodes, 2);
        let shared_links: i64 = database::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM task_problem_links WHERE problem_id=?",
                [&first_problem],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(shared_links, 2);
        assert_ne!(first_task, second_task);
        assert_ne!(first_problem, second_problem);
    }

    #[tokio::test]
    async fn review_without_vault_evidence_is_insufficient_and_never_clear() {
        let (_root, db, vault, settings) = fixture();
        let task = completed_task(&db);
        let request = json!({"operationId":"review-1","subject":{"kind":"task_revision","taskId":task,"taskRevision":1},"triggerKind":"explicit"});
        let created = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-review.create",
            &request,
        )
        .await
        .unwrap();
        let replayed = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-review.create",
            &request,
        )
        .await
        .unwrap();
        assert_eq!(created["id"], replayed["id"]);
        let run_count: i64 = database::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM task_conflict_review_runs",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(run_count, 1);
        let run = created["id"].as_str().unwrap();
        for _ in 0..40 {
            let status = execute(
                &db,
                &settings,
                &vault,
                SemanticEngine::new(None),
                "task-review.get",
                &json!({"runId":run}),
            )
            .await
            .unwrap();
            if status["status"] != "queued" && status["status"] != "running" {
                assert_eq!(status["status"], "insufficient_evidence");
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await
        }
        panic!("review did not finish");
    }

    #[tokio::test]
    async fn review_scope_is_server_derived_and_revocation_makes_a_result_stale() {
        let (_root, db, vault, settings) = fixture();
        let task = completed_task(&db);
        let created = execute(&db, &settings, &vault, SemanticEngine::new(None), "task-review.create", &json!({"operationId":"scope-review","subject":{"kind":"task_revision","taskId":task,"taskRevision":1,"scopeRevision":"forged"}})).await.unwrap();
        let stored: String = database::open(&db)
            .unwrap()
            .query_row(
                "SELECT scope_revision FROM task_conflict_review_runs WHERE id=?",
                [created["id"].as_str().unwrap()],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(stored, "forged");
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE mcp_connections SET state='revoked' WHERE id='native-in-app-chat'",
                [],
            )
            .unwrap();
        let status = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-review.get",
            &json!({"runId":created["id"]}),
        )
        .await
        .unwrap();
        assert_eq!(status["status"], "stale");
        assert_eq!(status["current"], false);
        let blocked = execute(&db, &settings, &vault, SemanticEngine::new(None), "task-review.create", &json!({"operationId":"revoked-review","subject":{"kind":"task_revision","taskId":task,"taskRevision":1}})).await.unwrap_err();
        assert_eq!(blocked, "review_scope_unavailable");
    }

    #[tokio::test]
    async fn delayed_automatic_review_stays_cancelled_after_its_debounce() {
        let (_root, db, vault, settings) = fixture();
        let task = completed_task(&db);
        let created = execute(&db, &settings, &vault, SemanticEngine::new(None), "task-review.create", &json!({"operationId":"automatic-review","triggerKind":"automatic","subject":{"kind":"task_revision","taskId":task,"taskRevision":1}})).await.unwrap();
        review_cancel(&db, created["id"].as_str().unwrap()).unwrap();
        tokio::time::sleep(Duration::from_millis(850)).await;
        let status = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-review.get",
            &json!({"runId":created["id"]}),
        )
        .await
        .unwrap();
        assert_eq!(status["status"], "cancelled");
    }

    #[test]
    fn review_history_keeps_the_last_current_result_when_a_new_attempt_fails() {
        let (_root, db, vault, _settings) = fixture();
        let task = completed_task(&db);
        let subject = json!({"kind":"task_revision","taskId":task,"taskRevision":1});
        let connection = database::open(&db).unwrap();
        let identity = review_identity(&connection, &subject).unwrap();
        let vault = vault_revision(&vault).unwrap();
        let scope = effective_review_scope(&connection).unwrap();
        for (run, status, created) in [
            ("current-clear", "clear", "2026-01-01T00:00:00Z"),
            ("failed-retry", "failed", "2026-01-02T00:00:00Z"),
        ] {
            connection.execute("INSERT INTO task_conflict_review_runs(id,subject_kind,subject_id,subject_revision,material_hash,vault_revision,scope_revision,trigger_kind,status,subject_json,created_at,finished_at,error) VALUES(?,?,?,?,?,?,?,'explicit',?,?,?, ?,?)",params![run,identity.kind,identity.subject_id,identity.revision,identity.material_hash,vault,scope,status,subject.to_string(),created,created,if status == "failed" { "provider_unavailable" } else { "" }]).unwrap();
        }
        drop(connection);
        let history = review_history(
            &db,
            &_root.path().join("vault"),
            &json!({"subject":subject}),
        )
        .unwrap();
        assert_eq!(history["attempts"].as_array().unwrap().len(), 2);
        assert_eq!(history["attempts"][0]["status"], "failed");
        assert_eq!(history["currentResult"]["id"], "current-clear");
    }

    #[test]
    fn malformed_clear_cannot_be_accepted() {
        assert!(validate_review_result(
            &json!({"status":"clear","citations":[]}),
            &[json!({"path":"Knowledge/a.md"})]
        )
        .is_err());
    }

    #[tokio::test]
    async fn knowledge_publish_and_withdraw_preserve_exact_evidence_and_recovery_copy() {
        let (_root, db, vault, settings) = fixture();
        let task = completed_task(&db);
        let draft = execute(
            &db,
            &settings,
            &vault,
            SemanticEngine::new(None),
            "task-knowledge.draft",
            &json!({"operationId":"draft-1","taskId":task,"expectedTaskRevision":1}),
        )
        .await
        .unwrap();
        assert!(draft["bodyMarkdown"]
            .as_str()
            .unwrap()
            .contains("Passing contract test"));
        let published=execute(&db,&settings,&vault,SemanticEngine::new(None),"task-knowledge.publish",&json!({"operationId":"publish-1","taskId":task,"draftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]})).await.unwrap();
        assert!(vault.join(published["path"].as_str().unwrap()).is_file());
        let withdrawn=execute(&db,&settings,&vault,SemanticEngine::new(None),"task-knowledge.withdraw",&json!({"operationId":"withdraw-1","taskId":task,"draftRevision":draft["draftRevision"]})).await.unwrap();
        assert!(vault
            .join(withdrawn["recoveryPath"].as_str().unwrap())
            .is_file());
        assert!(!vault.join(published["path"].as_str().unwrap()).exists());
    }
}
