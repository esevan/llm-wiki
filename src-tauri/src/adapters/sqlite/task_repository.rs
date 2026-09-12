use crate::domain::task::{content_hash, validate_relationship};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteTaskRepository {
    path: PathBuf,
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
pub(crate) fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("invalid_input: {key} is required"))
}

impl SqliteTaskRepository {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut c = crate::native::database::open(&self.path)?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let x = f(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(x)
    }
}

pub(crate) fn record_activity_tx(
    tx: &Transaction<'_>,
    kind: &str,
    id: &str,
    operation: &str,
    op: &str,
    at: &str,
) -> Result<(), String> {
    // One canonical operation can update two relationship endpoints.  Preserve each affected
    // aggregate under a stable per-entity operation key so the session synchronizer can fan out
    // once per linked session without treating a retry as another desktop change.
    let activity_operation = format!("{op}:{kind}:{id}");
    let inserted = tx.execute("INSERT OR IGNORE INTO user_activity_events(id,entity_type,entity_id,operation,operation_id,created_at) VALUES(?,?,?,?,?,?)",params![new_id(),kind,id,operation,activity_operation,at]).map_err(|e|e.to_string())?;
    if inserted == 0 {
        return Ok(());
    }
    if kind == "task" {
        tx.execute(
            "UPDATE tasks SET last_user_activity_at=? WHERE id=?",
            params![at, id],
        )
        .map_err(|e| e.to_string())?;
    }
    // v8 removed the legacy Workbench mutation triggers.  Canonical Capture, Problem, and
    // Task writes share this application helper so every aggregate mutation invalidates the
    // same workspace freshness counter once in its owning transaction.
    tx.execute(
        "UPDATE work_tracking_workspace SET revision=revision+1 WHERE id=1",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Append an immutable, already-applied desktop change to every work session that owns an
/// affected Task (including Tasks linked through an affected Problem). The origin is supplied
/// only by the trusted tracking adapter after it resolves the challenge's session row.
pub(crate) fn sync_linked_sessions_tx(
    tx: &Transaction<'_>,
    operation_id: &str,
    operation: &str,
    origin_session_id: Option<&str>,
    at: &str,
) -> Result<(), String> {
    let prefix = format!("{operation_id}:");
    let mut activity = tx
        .prepare(
            "SELECT entity_type,entity_id FROM user_activity_events
             WHERE operation_id=? OR substr(operation_id,1,length(?))=?",
        )
        .map_err(|error| error.to_string())?;
    let changed = activity
        .query_map(params![operation_id, prefix, prefix], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(activity);
    if changed.is_empty() {
        return Ok(());
    }
    // Session events are readable by their owners. Do not copy a cross-connection list of
    // affected aggregate IDs into every linked session merely to explain a refresh.
    let payload = serde_json::json!({
        "operationId": operation_id,
        "operation": operation,
    });
    let payload_text = payload.to_string();
    let payload_hash = content_hash(&[&payload_text]);

    let mut sessions = tx
        .prepare(
            "WITH changed AS (
                SELECT entity_type,entity_id FROM user_activity_events
                WHERE operation_id=? OR substr(operation_id,1,length(?))=?
             ), affected_sessions AS (
                SELECT l.session_id FROM work_tracking_links l
                  JOIN changed c ON c.entity_type='task' AND l.entity_type='tasks' AND l.entity_id=c.entity_id
                UNION
                SELECT l.session_id FROM work_tracking_links l
                  JOIN task_problem_links p ON p.task_id=l.entity_id AND p.unlinked_at IS NULL
                  JOIN changed c ON c.entity_type='problem' AND p.problem_id=c.entity_id
                 WHERE l.entity_type='tasks'
                UNION
                SELECT s.id FROM work_tracking_sessions s
                  JOIN changed c ON c.entity_type='problem'
                  JOIN problems p ON p.id=c.entity_id AND p.capture_id=s.capture_id
                UNION
                SELECT s.id FROM work_tracking_sessions s
                  JOIN changed c ON c.entity_type='capture' AND s.capture_id=c.entity_id
             )
             SELECT DISTINCT session_id FROM affected_sessions",
        )
        .map_err(|error| error.to_string())?;
    let session_ids = sessions
        .query_map(params![operation_id, prefix, prefix], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(sessions);

    for session_id in session_ids {
        if origin_session_id == Some(session_id.as_str()) {
            continue;
        }
        let already_present: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM work_tracking_events WHERE session_id=? AND kind='desktop_task_change' AND json_extract(payload_json,'$.operationId')=?)",
                params![session_id, operation_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if already_present {
            continue;
        }
        let (head_revision, previous_event_id): (i64, String) = tx
            .query_row(
                "SELECT head_revision,head_event_id FROM work_tracking_sessions WHERE id=?",
                [&session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| error.to_string())?;
        let stream_id = "desktop_task_sync";
        let source_sequence: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(source_sequence),0)+1 FROM work_tracking_events WHERE session_id=? AND stream_id=?",
                params![session_id, stream_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let event_id = new_id();
        let revision = head_revision + 1;
        tx.execute(
            "INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
             VALUES(?,?,?,?,?,?, 'desktop_task_change', ?,?,?,?)",
            params![event_id, session_id, revision, previous_event_id, stream_id, source_sequence, payload_text, payload_hash, at, at],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "UPDATE work_tracking_sessions SET head_revision=?,head_event_id=?,updated_at=? WHERE id=?",
            params![revision, event_id, at, session_id],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO work_tracking_projection_results(event_id,projection_name,state,result_entity_id,updated_at)
             VALUES(?,'workflow','applied',NULL,?)
             ON CONFLICT(event_id,projection_name) DO NOTHING",
            params![event_id, at],
        )
        .map_err(|error| error.to_string())?;
        tx.execute(
            "INSERT INTO work_tracking_stream_watermarks(session_id,stream_id,projection_name,last_occurred_at,last_source_sequence,last_event_id,version,updated_at)
             VALUES(?,?,'workflow',?,?,?,1,?)
             ON CONFLICT(session_id,stream_id,projection_name) DO UPDATE SET
               last_occurred_at=excluded.last_occurred_at,last_source_sequence=excluded.last_source_sequence,last_event_id=excluded.last_event_id,
               version=work_tracking_stream_watermarks.version+1,updated_at=excluded.updated_at",
            params![session_id, stream_id, at, source_sequence, event_id, at],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}
pub(crate) fn create_task_tx(
    tx: &Transaction<'_>,
    input: &Value,
    origin: Option<&str>,
    id: Option<&str>,
    at: &str,
) -> Result<Value, String> {
    let task_id = id.unwrap_or("");
    let task_id = if task_id.is_empty() {
        new_id()
    } else {
        task_id.to_owned()
    };
    let title = required(input, "title")?;
    let get = |k| input.get(k).and_then(Value::as_str).unwrap_or("").trim();
    let detail = get("detail");
    let outcome = get("outcome");
    let scope = get("scope");
    let non_goals = get("nonGoals");
    let criteria = get("validationCriteria");
    let category = input
        .get("category")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("General");
    tx.execute("INSERT INTO tasks(id,origin_capture_id,current_revision,state,category,created_at,last_user_activity_at) VALUES(?,?,1,'task',?,?,?)",params![task_id,origin,category,at,at]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,1,?,?,?,?,?,?,?,?)",params![task_id,title,detail,outcome,scope,non_goals,criteria,content_hash(&[title,detail,outcome,scope,non_goals,criteria]),at]).map_err(|e|e.to_string())?;
    if let Some(operation_id) = input.get("operationId").and_then(Value::as_str) {
        record_activity_tx(tx, "task", &task_id, "created", operation_id, at)?;
    }
    Ok(
        json!({"id":task_id,"taskRevision":1,"state":"task","title":title,"detail":detail,"outcome":outcome,"scope":scope,"nonGoals":non_goals,"validationCriteria":criteria,"category":category,"createdAt":at}),
    )
}
pub(crate) fn create_problem_revision_tx(
    tx: &Transaction<'_>,
    input: &Value,
    problem_id: Option<&str>,
    at: &str,
) -> Result<Value, String> {
    let statement = required(input, "statement")?;
    let detail = input.get("detail").and_then(Value::as_str).unwrap_or("");
    let id = problem_id.map(str::to_owned).unwrap_or_else(new_id);
    let prior: i64 = tx
        .query_row(
            "SELECT COALESCE(current_revision,0) FROM problems WHERE id=?",
            [&id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    let rev = prior + 1;
    if prior == 0 {
        tx.execute("INSERT INTO problems(id,capture_id,statement,detail,state,created_at,current_revision) VALUES(?,NULL,?,?, 'open',?,1)",params![id,statement,detail,at]).map_err(|e|e.to_string())?;
    } else {
        tx.execute(
            "UPDATE problems SET statement=?,detail=?,current_revision=? WHERE id=?",
            params![statement, detail, rev, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute("INSERT INTO problem_revisions(problem_id,revision,statement,detail,content_hash,created_at) VALUES(?,?,?,?,?,?)",params![id,rev,statement,detail,content_hash(&[statement,detail]),at]).map_err(|e|e.to_string())?;
    Ok(json!({"id":id,"problemRevision":rev,"statement":statement,"detail":detail}))
}
pub(crate) fn relationship_cycle_tx(
    tx: &Transaction<'_>,
    source: &str,
    target: &str,
) -> Result<bool, String> {
    let exists:bool=tx.query_row("WITH RECURSIVE reach(id) AS (SELECT target_task_id FROM task_relationships WHERE source_task_id=? AND kind='prerequisite' AND unlinked_at IS NULL UNION SELECT r.target_task_id FROM task_relationships r JOIN reach ON r.source_task_id=reach.id WHERE r.kind='prerequisite' AND r.unlinked_at IS NULL) SELECT EXISTS(SELECT 1 FROM reach WHERE id=?)",params![target,source],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok(exists)
}
pub(crate) fn add_relationship_tx(
    tx: &Transaction<'_>,
    source: &str,
    input: &Value,
    at: &str,
) -> Result<Value, String> {
    let target = required(input, "targetTaskId")?;
    let kind = required(input, "kind")?;
    validate_relationship(source, target, kind)?;
    let (a, b) = if kind == "related" && source > target {
        (target, source)
    } else {
        (source, target)
    };
    if kind == "prerequisite" && relationship_cycle_tx(tx, a, b)? {
        return Err("relationship_invalid: prerequisite cycle".into());
    };
    let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_relationships WHERE source_task_id=? AND target_task_id=? AND kind=? AND unlinked_at IS NULL)",params![a,b,kind],|r|r.get(0)).map_err(|e|e.to_string())?;
    if duplicate {
        return Err("relationship_invalid: duplicate relationship".into());
    };
    let id = new_id();
    tx.execute("INSERT INTO task_relationships(id,source_task_id,target_task_id,kind,note,created_at) VALUES(?,?,?,?,?,?)",params![id,a,b,kind,input.get("note").and_then(Value::as_str).unwrap_or(""),at]).map_err(|e|e.to_string())?;
    Ok(json!({"id":id,"sourceTaskId":a,"targetTaskId":b,"kind":kind,"createdAt":at}))
}
