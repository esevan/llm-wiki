use crate::domain::{
    knowledge_publication::PublicationState,
    work_tracking_state::{AppError, EventKind, EventOrderKey, ProjectionStatus},
};
use crate::native::database;
use crate::ports::{
    event_log::EventLog, work_projection::WorkProjection, workflow_repository::WorkflowRepository,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteWorkTrackingStore {
    path: PathBuf,
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

const OVERVIEW_TEXT_LIMIT: usize = 240;
const OVERVIEW_ATTENTION_LIMIT: usize = 50;

fn hash_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn bounded_overview_text(value: &str, limit: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    #[cfg(debug_assertions)]
    eprintln!("work-tracking storage error: {error}");
    let mut result = AppError::new(
        "storage_unavailable",
        "LLM Wiki storage is temporarily unavailable",
    );
    result.retryable = true;
    let _ = error;
    result
}

fn require_text<'a>(value: &'a Value, field: &str, max: usize) -> Result<&'a str, AppError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return Err(AppError::new(
            "invalid_input",
            format!("{field} is required"),
        ));
    }
    if text.len() > max {
        return Err(AppError::new(
            "content_too_large",
            format!("{field} is too long"),
        ));
    }
    Ok(text)
}

fn action_result_type(action: &str) -> Option<&'static str> {
    match action {
        "create_task" | "task.create" | "revise_task" | "task.revision" | "transition_task"
        | "task.transition" | "task.reopen" | "link_task" => Some("tasks"),
        "complete_task" | "task.completion.create" => Some("task_completions"),
        "resolve_problem" | "problem.resolution.create" => Some("problems"),
        "problem.create" | "problem.revision" => Some("problems"),
        "task.problem-link.create" | "task.problem-link.delete" => Some("task_problem_links"),
        "task.relationship.create" | "task.relationship.delete" => Some("task_relationships"),
        "task.work-log.create"
        | "work-log.comment.create"
        | "task.checklist.create"
        | "task.checklist.update"
        | "task.decision.create"
        | "task.readiness.decision"
        | "review_conflict" => Some("tasks"),
        "adopt_problem" | "approve_problem" => Some("problems"),
        "adopt_solution" | "resolve_conflict" | "approve_solution" => Some("features"),
        "accept_completion_proposal" => Some("completion_reviews"),
        "verify_and_complete" => Some("completions"),
        _ => None,
    }
}

impl SqliteWorkTrackingStore {
    pub(crate) fn database_path(&self) -> &std::path::Path {
        &self.path
    }
    fn require_scope_on(connection: &Connection, owner: &str, scope: &str) -> Result<(), AppError> {
        let raw: Option<String> = connection
            .query_row(
                "SELECT scopes_json FROM mcp_connections WHERE id=? AND state='active'",
                [owner],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        let scopes = raw
            .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
            .unwrap_or_default();
        if scopes.iter().any(|candidate| candidate == scope) {
            Ok(())
        } else {
            Err(AppError::new(
                "not_found_or_not_visible",
                "Connection capability is unavailable",
            ))
        }
    }
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn ensure_native_connection(&self) -> Result<(), AppError> {
        const NATIVE_CONNECTION: &str = "native-in-app-chat";
        let connection = database::open(&self.path).map_err(storage_error)?;
        let timestamp = now();
        connection.execute("INSERT OR IGNORE INTO mcp_connections(id,name,scopes_json,checkpoint_policy,state,created_at,updated_at) VALUES (?,'LLM Wiki Chat',?,'allowed_for_started_sessions','active',?,?)",
            params![NATIVE_CONNECTION,serde_json::to_string(&["session:read","session:write","workbench:current:read","workbench:overview:read","vault:search:lexical","vault:search:semantic","vault:evidence:read","knowledge:draft:write","knowledge:publish"]).unwrap(),timestamp,timestamp]).map_err(storage_error)?;
        Ok(())
    }

    pub fn create_connection(
        &self,
        name: &str,
        scopes: &[String],
        topic_ids: &[String],
        checkpoint_policy: &str,
    ) -> Result<Value, AppError> {
        if name.trim().is_empty() || name.len() > 80 {
            return Err(AppError::new(
                "invalid_input",
                "Connection name must be 1–80 characters",
            ));
        }
        let allowed = [
            "session:read",
            "session:write",
            "topic:read",
            "workbench:current:read",
            "workbench:overview:read",
            "vault:search:lexical",
            "vault:search:semantic",
            "vault:evidence:read",
            "knowledge:draft:write",
            "knowledge:publish",
        ];
        if scopes
            .iter()
            .any(|scope| !allowed.contains(&scope.as_str()))
        {
            return Err(AppError::new(
                "invalid_input",
                "Connection contains an unsupported scope",
            ));
        }
        if topic_ids.len() > 20
            || topic_ids
                .iter()
                .any(|topic| topic.trim().is_empty() || topic.len() > 120)
        {
            return Err(AppError::new(
                "invalid_input",
                "A connection may allow up to 20 valid topic IDs",
            ));
        }
        if !["confirm_each", "allowed_for_started_sessions"].contains(&checkpoint_policy) {
            return Err(AppError::new(
                "invalid_input",
                "Unsupported checkpoint policy",
            ));
        }
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        database::open(&self.path)
            .and_then(|connection| {
                connection.execute(
                    "INSERT INTO mcp_connections(id,name,scopes_json,allowed_topics_json,checkpoint_policy,state,created_at,updated_at) VALUES (?,?,?,?,?, 'active',?,?)",
                    params![id, name.trim(), serde_json::to_string(scopes).unwrap_or_else(|_| "[]".into()),serde_json::to_string(topic_ids).unwrap_or_else(|_|"[]".into()), checkpoint_policy, timestamp, timestamp],
                ).map_err(|error| error.to_string())?;
                Ok(())
            })
            .map_err(storage_error)?;
        Ok(
            json!({"id":id,"name":name.trim(),"state":"active","scopes":scopes,"topicIds":topic_ids,"checkpointPolicy":checkpoint_policy}),
        )
    }

    pub fn list_connections(&self) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement = connection.prepare(
            "SELECT id,name,scopes_json,allowed_topics_json,checkpoint_policy,state,created_at,last_used_at,revoked_at FROM mcp_connections WHERE id!='native-in-app-chat' ORDER BY created_at DESC"
        ).map_err(storage_error)?;
        let rows = statement.query_map([], |row| {
            let scopes: String = row.get(2)?;
            let topics:String=row.get(3)?;
            Ok(json!({
                "id":row.get::<_,String>(0)?, "name":row.get::<_,String>(1)?,
                "scopes":serde_json::from_str::<Value>(&scopes).unwrap_or_else(|_| json!([])),
                "topicIds":serde_json::from_str::<Value>(&topics).unwrap_or_else(|_|json!([])),
                "checkpointPolicy":row.get::<_,String>(4)?, "state":row.get::<_,String>(5)?,
                "createdAt":row.get::<_,String>(6)?, "lastUsedAt":row.get::<_,Option<String>>(7)?,
                "revokedAt":row.get::<_,Option<String>>(8)?
            }))
        }).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        Ok(json!({"connections":rows}))
    }

    pub fn revoke_connection(&self, connection_id: &str) -> Result<(), AppError> {
        let timestamp = now();
        let connection = database::open(&self.path).map_err(storage_error)?;
        connection.execute(
            "UPDATE mcp_connections SET state='revoked',revoked_at=COALESCE(revoked_at,?),updated_at=? WHERE id=?",
            params![timestamp, timestamp, connection_id],
        ).map_err(storage_error)?;
        Ok(())
    }

    // Keep the storage boundary explicit: these arguments map to one atomic record.
    #[allow(clippy::too_many_arguments)]
    pub fn record_activity(
        &self,
        source: &str,
        connection_id: Option<&str>,
        session_id: Option<&str>,
        operation: &str,
        outcome: &str,
        error: Option<&str>,
        duration_ms: u128,
    ) {
        if let Ok(connection) = database::open(&self.path) {
            Self::record_activity_on(
                &connection, source, connection_id, session_id, operation, outcome, error, duration_ms,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_activity_on(
        connection: &Connection,
        source: &str,
        connection_id: Option<&str>,
        session_id: Option<&str>,
        operation: &str,
        outcome: &str,
        error: Option<&str>,
        duration_ms: u128,
    ) {
        let _=connection.execute("INSERT INTO work_tracking_activity_events(id,source_interface,connection_id,session_id,operation,outcome,safe_error_code,duration_ms,created_at) VALUES (?,?,?,?,?,?,?,?,?)",params![Uuid::new_v4().to_string(),source,connection_id,session_id,operation,outcome,error,duration_ms.min(i64::MAX as u128) as i64,now()]);
    }

    pub fn connection_scopes(&self, connection_id: &str) -> Result<Vec<String>, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let row = connection
            .query_row(
                "SELECT scopes_json,state FROM mcp_connections WHERE id=?",
                [connection_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((scopes, state)) = row else {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Connection is unavailable",
            ));
        };
        if state != "active" {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Connection is unavailable",
            ));
        }
        serde_json::from_str(&scopes)
            .map_err(|_| AppError::new("internal_error", "Connection scope data is invalid"))
    }

    pub fn require_topic(&self, connection_id: &str, topic_id: &str) -> Result<(), AppError> {
        if topic_id.trim().is_empty() {
            return Err(AppError::new("invalid_input", "A topic target is required"));
        }
        if connection_id == "native-in-app-chat" {
            return Ok(());
        }
        let connection = database::open(&self.path).map_err(storage_error)?;
        let encoded: String = connection
            .query_row(
                "SELECT allowed_topics_json FROM mcp_connections WHERE id=? AND state='active'",
                [connection_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new(
                    "not_found_or_not_visible",
                    "The requested capability is unavailable",
                )
            })?;
        let allowed = serde_json::from_str::<Vec<String>>(&encoded).unwrap_or_default();
        if allowed.iter().any(|topic| topic == topic_id) {
            Ok(())
        } else {
            Err(AppError::new(
                "not_found_or_not_visible",
                "The requested topic is unavailable",
            ))
        }
    }

    pub fn task_continuation_discoverable(
        &self,
        connection_id: &str,
        task_id: &str,
    ) -> Result<(), AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        Self::task_continuation_access_on(&connection, connection_id, task_id)
    }

    pub fn refinement_session_discoverable(
        &self,
        connection_id: &str,
        refinement_session_id: &str,
    ) -> Result<(), AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let task_id: Option<String> = connection
            .query_row(
                "SELECT task_id FROM refinement_sessions WHERE id=?",
                [refinement_session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .flatten();
        let task_id = task_id.ok_or_else(|| {
            AppError::new(
                "not_found_or_not_visible",
                "Refinement session is unavailable",
            )
        })?;
        Self::task_continuation_access_on(&connection, connection_id, &task_id)
    }

    /// A Current Chat advisory is a Task-scoped record. Resolve its Task in SQLite
    /// before exposing a guessed run ID to an MCP connection.
    pub fn current_chat_advisory_discoverable(
        &self,
        connection_id: &str,
        run_id: &str,
    ) -> Result<(), AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let task_id: String = connection
            .query_row(
                "SELECT subject_id FROM task_conflict_review_runs WHERE id=? AND subject_kind='current_chat'",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new(
                    "not_found_or_not_visible",
                    "Current Chat advisory is unavailable",
                )
            })?;
        Self::task_continuation_access_on(&connection, connection_id, &task_id)
    }

    pub fn task_assistance_subject_task(
        &self,
        connection_id: &str,
        subject_kind: &str,
        subject_id: &str,
    ) -> Result<String, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let task_id: String = match subject_kind {
            "refinement" => connection.query_row("SELECT task_id FROM refinement_sessions WHERE id=?",[subject_id],|row|row.get::<_,Option<String>>(0)).optional().map_err(storage_error)?.flatten(),
            "advisory" => connection.query_row("SELECT subject_id FROM task_conflict_review_runs WHERE id=? AND subject_kind='current_chat'",[subject_id],|row|row.get::<_,String>(0)).optional().map_err(storage_error)?,
            _ => None,
        }.ok_or_else(||AppError::new("not_found_or_not_visible","Task assistance subject is unavailable"))?;
        Self::task_continuation_access_on(&connection, connection_id, &task_id)?;
        Ok(task_id)
    }

    /// Translate an in-app session-shaped Knowledge request to the immutable
    /// canonical Task completion that it is allowed to address.
    pub fn canonical_task_completion_for_session(
        &self,
        connection_id: &str,
        session_id: &str,
    ) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let task_id: String = connection.query_row("SELECT l.entity_id FROM work_tracking_links l JOIN work_tracking_sessions s ON s.id=l.session_id WHERE l.session_id=? AND l.relationship='adopted_task' AND (s.connection_id=? OR ?='native-in-app-chat') ORDER BY l.created_at DESC LIMIT 1",params![session_id,connection_id,connection_id],|row|row.get(0)).optional().map_err(storage_error)?.ok_or_else(||AppError::new("workflow_precondition","A canonical Task must be linked before Knowledge can be drafted"))?;
        let (revision, completion_id): (i64, String) = connection.query_row("SELECT t.current_revision,c.id FROM tasks t JOIN task_completions c ON c.task_id=t.id AND c.task_revision=t.current_revision WHERE t.id=? ORDER BY c.created_at DESC LIMIT 1",[&task_id],|row|Ok((row.get(0)?,row.get(1)?))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("workflow_precondition","A completed current Task is required before Knowledge can be drafted"))?;
        Ok(json!({"taskId":task_id,"expectedTaskRevision":revision,"completionId":completion_id}))
    }

    pub fn canonical_knowledge_draft_for_review(
        &self,
        connection_id: &str,
        input: &Value,
        withdraw: bool,
    ) -> Result<Value, AppError> {
        let task_id = require_text(input, "taskId", 120)?;
        let revision = input
            .get("draftRevision")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::new("invalid_input", "draftRevision is required"))?;
        let content_hash = require_text(input, "expectedContentHash", 128)?;
        let source_hash = require_text(input, "expectedSourceHash", 128)?;
        let connection = database::open(&self.path).map_err(storage_error)?;
        Self::task_continuation_access_on(&connection, connection_id, task_id)?;
        let draft: Value = connection.query_row("SELECT k.task_id,k.revision,k.body_markdown,k.content_hash,k.lineage_json,k.state,r.title FROM task_knowledge_drafts k JOIN task_revisions r ON r.task_id=k.task_id AND r.revision=k.task_revision WHERE k.task_id=? AND k.revision=?",params![task_id,revision],|row| {
            let lineage: Value=serde_json::from_str(&row.get::<_,String>(4)?).unwrap_or(Value::Null);
            Ok(json!({"taskId":row.get::<_,String>(0)?,"draftRevision":row.get::<_,i64>(1)?,"bodyMarkdown":row.get::<_,String>(2)?,"contentHash":row.get::<_,String>(3)?,"sourceHash":lineage["sourceHash"],"lineage":lineage,"state":row.get::<_,String>(5)?,"title":row.get::<_,String>(6)?}))
        }).optional().map_err(storage_error)?.ok_or_else(||AppError::new("not_found_or_not_visible","Knowledge draft is unavailable"))?;
        if draft["contentHash"] != content_hash || draft["sourceHash"] != source_hash {
            return Err(AppError::new(
                "draft_conflict",
                "Knowledge draft body or source changed; refresh before review",
            ));
        }
        let expected_state = if withdraw { "published" } else { "draft" };
        if draft["state"] != expected_state {
            return Err(AppError::new(
                "workflow_precondition",
                format!("Knowledge draft must be {expected_state}"),
            ));
        }
        Ok(draft)
    }

    fn task_continuation_access_on(
        connection: &Connection,
        connection_id: &str,
        task_id: &str,
    ) -> Result<(), AppError> {
        let (scopes, allowed): (String, String) = connection.query_row(
            "SELECT scopes_json,allowed_topics_json FROM mcp_connections WHERE id=? AND state='active'",
            [connection_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(storage_error)?.ok_or_else(|| AppError::new("not_found_or_not_visible", "Connection is unavailable"))?;
        let scopes = serde_json::from_str::<Vec<String>>(&scopes).unwrap_or_default();
        if scopes
            .iter()
            .any(|scope| scope == "workbench:overview:read")
        {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?)",
                    [task_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            return exists
                .then_some(())
                .ok_or_else(|| AppError::new("not_found_or_not_visible", "Task is unavailable"));
        }
        if scopes.iter().any(|scope| scope == "workbench:current:read") {
            let visible: bool = connection.query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM work_tracking_links l JOIN work_tracking_sessions s ON s.id=l.session_id
                     WHERE l.entity_type='tasks' AND l.entity_id=?1 AND (s.connection_id=?2 OR ?2='native-in-app-chat')
                   UNION ALL
                   SELECT 1 FROM (
                     SELECT t.id FROM tasks t WHERE NOT EXISTS(
                       SELECT 1 FROM work_tracking_links l JOIN work_tracking_sessions s ON s.id=l.session_id
                        WHERE l.entity_type='tasks' AND l.entity_id=t.id AND s.state='active' AND (s.connection_id=?2 OR ?2='native-in-app-chat')
                     ) ORDER BY t.last_user_activity_at DESC,t.id LIMIT 20
                   ) WHERE id=?1
                   UNION ALL
                   SELECT 1 FROM work_tracking_workspace WHERE id=1
                     AND json_extract(selection_json,'$.entityType')='tasks'
                     AND json_extract(selection_json,'$.entityId')=?1
                 )",
                params![task_id, connection_id],
                |row| row.get(0),
            ).map_err(storage_error)?;
            if visible {
                return Ok(());
            }
        }
        if !scopes.iter().any(|scope| scope == "topic:read") {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Task discovery is unavailable",
            ));
        }
        let allowed = serde_json::from_str::<Vec<String>>(&allowed).unwrap_or_default();
        let visible: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_topic_memberships WHERE entity_type='tasks' AND entity_id=? AND topic_id IN (SELECT value FROM json_each(?)))",params![task_id,serde_json::to_string(&allowed).unwrap_or_else(|_|"[]".into())],|row|row.get(0)).map_err(storage_error)?;
        visible
            .then_some(())
            .ok_or_else(|| AppError::new("not_found_or_not_visible", "Task topic is unavailable"))
    }

    pub fn owned_sessions(&self, connection_id: &str) -> Result<Vec<(String, String)>, AppError> {
        self.connection_scopes(connection_id)?;
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT s.id,substr(c.text,1,120) FROM work_tracking_sessions s JOIN captures c ON c.id=s.capture_id WHERE s.connection_id=? ORDER BY s.updated_at DESC LIMIT 50").map_err(storage_error)?;
        let rows = statement
            .query_map([connection_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn check_idempotency(
        transaction: &Transaction<'_>,
        owner: &str,
        operation: &str,
        operation_id: &str,
        request_hash: &str,
    ) -> Result<Option<Value>, AppError> {
        let existing = transaction.query_row(
            "SELECT request_hash,response_json FROM work_tracking_idempotency_records WHERE source_interface IN ('external_mcp_chat','in_app_chat') AND source_owner_hash=? AND operation_name=? AND operation_id=?",
            params![owner, operation, operation_id],
            |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)),
        ).optional().map_err(storage_error)?;
        if let Some((stored_hash, response)) = existing {
            if stored_hash != request_hash {
                return Err(AppError::new(
                    "idempotency_conflict",
                    "Operation ID was already used with different input",
                ));
            }
            let mut value: Value = serde_json::from_str(&response).map_err(storage_error)?;
            value["deduplicated"] = json!(true);
            return Ok(Some(value));
        }
        Ok(None)
    }

    // Keep the storage boundary explicit: these arguments map to one atomic record.
    #[allow(clippy::too_many_arguments)]
    fn insert_idempotency(
        transaction: &Transaction<'_>,
        owner: &str,
        operation: &str,
        operation_id: &str,
        request_hash: &str,
        session_id: Option<&str>,
        event_id: Option<&str>,
        response: &Value,
        timestamp: &str,
    ) -> Result<(), AppError> {
        let source = if owner == hash_text("native-in-app-chat") {
            "in_app_chat"
        } else {
            "external_mcp_chat"
        };
        transaction.execute(
            "INSERT INTO work_tracking_idempotency_records(source_interface,source_owner_hash,operation_name,operation_id,request_hash,session_id,event_id,response_json,committed_at) VALUES (?,?,?,?,?,?,?,?,?)",
            params![source,owner,operation,operation_id,request_hash,session_id,event_id,response.to_string(),timestamp],
        ).map_err(storage_error)?;
        Ok(())
    }

    fn replace_idempotency_response(
        transaction: &Transaction<'_>,
        owner: &str,
        operation: &str,
        operation_id: &str,
        request_hash: &str,
        response: &Value,
        timestamp: &str,
    ) -> Result<(), AppError> {
        let source = if owner == hash_text("native-in-app-chat") {
            "in_app_chat"
        } else {
            "external_mcp_chat"
        };
        let updated = transaction
            .execute(
                "UPDATE work_tracking_idempotency_records SET response_json=?,committed_at=? WHERE source_interface=? AND source_owner_hash=? AND operation_name=? AND operation_id=? AND request_hash=?",
                params![response.to_string(), timestamp, source, owner, operation, operation_id, request_hash],
            )
            .map_err(storage_error)?;
        if updated != 1 {
            return Err(AppError::new(
                "storage_unavailable",
                "Reviewed operation replay record is unavailable",
            ));
        }
        Ok(())
    }

    // Keep the storage boundary explicit: these arguments map to one atomic record.
    #[allow(clippy::too_many_arguments)]
    pub fn open_session(
        &self,
        connection_id: &str,
        operation_id: &str,
        lineage_key: &str,
        mode: &str,
        capture: Option<&Value>,
        parent_session_id: Option<&str>,
        review_context: Option<&Value>,
        review: Option<(&str, &str)>,
        task_id: Option<&str>,
    ) -> Result<Value, AppError> {
        self.connection_scopes(connection_id)?;
        let owner = hash_text(connection_id);
        let lineage_hash = hash_text(&format!("{connection_id}:{lineage_key}"));
        let request = json!({"lineageKey":lineage_key,"mode":mode,"capture":capture,"parentSessionId":parent_session_id,"reviewContext":review_context,"taskId":task_id});
        let request_hash = hash_text(&request.to_string());
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let authorized: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mcp_connections WHERE id=? AND state='active')",
                [connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !authorized {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Connection is unavailable",
            ));
        }
        if let Some(response) = Self::check_idempotency(
            &transaction,
            &owner,
            "inbound_work_open",
            operation_id,
            &request_hash,
        )? {
            return Ok(response);
        }
        let existing = transaction.query_row(
            "SELECT id,capture_id,head_event_id,head_revision,state FROM work_tracking_sessions WHERE connection_id=? AND conversation_ref_hash=?",
            params![connection_id,lineage_hash],
            |row| Ok(json!({"sessionId":row.get::<_,String>(0)?,"captureId":row.get::<_,String>(1)?,"headEventId":row.get::<_,String>(2)?,"headRevision":row.get::<_,i64>(3)?,"state":row.get::<_,String>(4)?})),
        ).optional().map_err(storage_error)?;
        if let Some(mut response) = existing {
            response["created"] = json!(false);
            response["deduplicated"] = json!(false);
            response["resourceUri"] = json!(format!(
                "llm-wiki://work-session/{}",
                response["sessionId"].as_str().unwrap_or_default()
            ));
            Self::insert_idempotency(
                &transaction,
                &owner,
                "inbound_work_open",
                operation_id,
                &request_hash,
                response["sessionId"].as_str(),
                None,
                &response,
                &now(),
            )?;
            transaction.commit().map_err(storage_error)?;
            return Ok(response);
        }
        if mode == "continue_task" {
            let task_id =
                task_id.ok_or_else(|| AppError::new("invalid_input", "taskId is required"))?;
            Self::task_continuation_access_on(&transaction, connection_id, task_id)?;
            let snapshot = Self::task_continuation_snapshot(&transaction, connection_id, task_id)?;
            let stored = transaction.query_row("SELECT id,payload_hash,state,expires_at,payload_json FROM work_tracking_reviews WHERE connection_id=? AND operation_id=? AND action='task_continuation'", params![connection_id,operation_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?))).optional().map_err(storage_error)?;
            let (review_id, expiry, reviewed_snapshot) = if let Some((
                id,
                hash,
                state,
                expiry,
                payload,
            )) = stored
            {
                if hash != request_hash {
                    return Err(AppError::new(
                        "idempotency_conflict",
                        "Preview content changed; create a new review",
                    ));
                }
                if state != "pending" {
                    return Err(AppError::new(
                        "challenge_replayed",
                        "Task continuation review was already decided",
                    ));
                }
                if expiry < now() {
                    return Err(AppError::new(
                        "challenge_expired",
                        "Task continuation review expired",
                    ));
                }
                let target = serde_json::from_str::<Value>(&payload)
                    .ok()
                    .and_then(|value| value.get("target").cloned())
                    .ok_or_else(|| {
                        AppError::new("storage_unavailable", "Task continuation review is invalid")
                    })?;
                (id, expiry, target)
            } else {
                if review.is_some() {
                    return Err(AppError::new(
                        "invalid_input",
                        "Task continuation review is unavailable",
                    ));
                }
                let id = Uuid::new_v4().to_string();
                let expiry = (Utc::now() + chrono::Duration::minutes(10))
                    .to_rfc3339_opts(SecondsFormat::Nanos, true);
                let payload = json!({"request":request,"target":snapshot});
                transaction.execute("INSERT INTO work_tracking_reviews(id,connection_id,operation_id,action,payload_hash,payload_json,expires_at,created_at) VALUES (?,?,?,'task_continuation',?,?,?,?)",params![id,connection_id,operation_id,request_hash,payload.to_string(),expiry,now()]).map_err(storage_error)?;
                (id, expiry, snapshot)
            };
            let Some((supplied_id, decision)) = review else {
                transaction.commit().map_err(storage_error)?;
                return Ok(
                    json!({"reviewState":review_id,"stage":"task_continuation","preview":Self::task_continuation_preview(&reviewed_snapshot),"expiresAt":expiry,"created":false,"decisionRequired":true}),
                );
            };
            if supplied_id != review_id || !matches!(decision, "accept" | "reject" | "cancel") {
                return Err(AppError::new(
                    "invalid_input",
                    "Invalid Task continuation decision",
                ));
            }
            transaction
                .execute(
                    "UPDATE work_tracking_reviews SET state=? WHERE id=?",
                    params![decision, review_id],
                )
                .map_err(storage_error)?;
            if decision != "accept" {
                transaction.commit().map_err(storage_error)?;
                return Ok(json!({"decision":decision,"created":false}));
            }
            Self::task_continuation_access_on(&transaction, connection_id, task_id)?;
            let current_snapshot =
                Self::task_continuation_snapshot(&transaction, connection_id, task_id)?;
            if current_snapshot != reviewed_snapshot {
                return Err(AppError::new(
                    "head_conflict",
                    "Task changed; review the refreshed target",
                ));
            }
            let session_id = Uuid::new_v4().to_string();
            let event_id = Uuid::new_v4().to_string();
            let decision_id = Uuid::new_v4().to_string();
            let timestamp = now();
            let source_interface = if connection_id == "native-in-app-chat" {
                "in_app_chat"
            } else {
                "external_mcp_chat"
            };
            let payload = json!({"taskId":task_id,"target":reviewed_snapshot,"operation":"bind_existing_task"});
            let payload_hash = hash_text(&payload.to_string());
            transaction.execute("INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,state,publication_state,parent_session_id,created_at,updated_at) VALUES (?,?,?,?,NULL,?,1,'active','not_requested',?,?,?)",params![session_id,connection_id,source_interface,lineage_hash,event_id,parent_session_id,timestamp,timestamp]).map_err(storage_error)?;
            transaction.execute("INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) VALUES (?,?,1,?,1,'task_binding',?,?,?,?)",params![event_id,session_id,format!("{source_interface}:{connection_id}"),payload.to_string(),payload_hash,timestamp,timestamp]).map_err(storage_error)?;
            transaction.execute("INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,decision_channel,created_at) VALUES (?,?,?,'accepted',?,?,?)",params![decision_id,session_id,event_id,payload_hash,format!("{source_interface}:task_continuation_review:{review_id}"),timestamp]).map_err(storage_error)?;
            transaction.execute("INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'tasks',?,'adopted_task',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,task_id,decision_id,timestamp]).map_err(storage_error)?;
            let response = json!({"sessionId":session_id,"captureId":Value::Null,"headEventId":event_id,"headRevision":1,"state":"active","created":true,"deduplicated":false,"persistenceStatus":"durable","projectionStatus":"not_required","resourceUri":format!("llm-wiki://work-session/{session_id}")});
            Self::insert_idempotency(
                &transaction,
                &owner,
                "inbound_work_open",
                operation_id,
                &request_hash,
                Some(&session_id),
                Some(&event_id),
                &response,
                &timestamp,
            )?;
            transaction.commit().map_err(storage_error)?;
            return Ok(response);
        }
        if mode == "resume" {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Work session is unavailable",
            ));
        }
        let capture =
            capture.ok_or_else(|| AppError::new("invalid_input", "capture is required"))?;
        let title = require_text(capture, "title", 200)?;
        let summary = require_text(capture, "summary", 20_000)?;
        let stored = transaction.query_row("SELECT id,payload_hash,state,expires_at FROM work_tracking_reviews WHERE connection_id=? AND operation_id=? AND action='capture'", params![connection_id,operation_id], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?))).optional().map_err(storage_error)?;
        let (review_id, expiry) = if let Some((id, hash, state, expiry)) = stored {
            if hash != request_hash {
                return Err(AppError::new(
                    "idempotency_conflict",
                    "Preview content changed; create a new review",
                ));
            }
            if state != "pending" {
                return Err(AppError::new(
                    "challenge_replayed",
                    "Capture review was already decided",
                ));
            }
            if expiry < now() {
                return Err(AppError::new("challenge_expired", "Capture review expired"));
            }
            (id, expiry)
        } else {
            if review.is_some() {
                return Err(AppError::new(
                    "invalid_input",
                    "Capture review is unavailable",
                ));
            }
            let id = Uuid::new_v4().to_string();
            let expiry = (Utc::now() + chrono::Duration::minutes(10))
                .to_rfc3339_opts(SecondsFormat::Nanos, true);
            transaction.execute("INSERT INTO work_tracking_reviews(id,connection_id,operation_id,action,payload_hash,payload_json,expires_at,created_at) VALUES (?,?,?,'capture',?,?,?,?)",params![id,connection_id,operation_id,request_hash,request.to_string(),expiry,now()]).map_err(storage_error)?;
            (id, expiry)
        };
        let Some((supplied_id, decision)) = review else {
            transaction.commit().map_err(storage_error)?;
            return Ok(
                json!({"reviewState":review_id,"stage":"capture","preview":capture,"expiresAt":expiry,"created":false,"decisionRequired":true}),
            );
        };
        if supplied_id != review_id || !matches!(decision, "accept" | "reject" | "cancel") {
            return Err(AppError::new("invalid_input", "Invalid Capture decision"));
        }
        transaction
            .execute(
                "UPDATE work_tracking_reviews SET state=? WHERE id=?",
                params![decision, review_id],
            )
            .map_err(storage_error)?;
        if decision != "accept" {
            transaction.commit().map_err(storage_error)?;
            return Ok(json!({"decision":decision,"created":false}));
        }
        let session_id = Uuid::new_v4().to_string();
        let capture_id = Uuid::new_v4().to_string();
        let event_id = Uuid::new_v4().to_string();
        let timestamp = now();
        let payload = json!({"title":title,"summary":summary,"userIntent":capture.get("userIntent"),"sourceExcerpt":capture.get("sourceExcerpt")});
        let source_interface = if connection_id == "native-in-app-chat" {
            "in_app_chat"
        } else {
            "external_mcp_chat"
        };
        let payload_hash = hash_text(&payload.to_string());
        transaction
            .execute(
                "INSERT INTO captures(id,text,created_at) VALUES (?,?,?)",
                params![capture_id, summary, timestamp],
            )
            .map_err(storage_error)?;
        transaction.execute(
            "INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,state,publication_state,parent_session_id,created_at,updated_at) VALUES (?,?,?,?,?,?,1,'active','not_requested',?,?,?)",
            params![session_id,connection_id,source_interface,lineage_hash,capture_id,event_id,parent_session_id,timestamp,timestamp],
        ).map_err(storage_error)?;
        transaction.execute(
            "INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) VALUES (?,?,1,?,1,'capture',?,?,?,?)",
            params![event_id,session_id,format!("{source_interface}:{connection_id}"),payload.to_string(),payload_hash,timestamp,timestamp],
        ).map_err(storage_error)?;
        transaction.execute("INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,decision_channel,created_at) VALUES (?,?,?,'accepted',?,?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,payload_hash,format!("{source_interface}:capture_review:{review_id}"),timestamp]).map_err(storage_error)?;
        transaction.execute(
            "INSERT INTO work_tracking_projection_jobs(event_id,projection_name,state,created_at,updated_at) VALUES (?,'workflow','pending',?,?)",
            params![event_id,timestamp,timestamp],
        ).map_err(storage_error)?;
        let response = json!({"sessionId":session_id,"captureId":capture_id,"headEventId":event_id,"headRevision":1,"state":"active","created":true,"deduplicated":false,"persistenceStatus":"durable","projectionStatus":"queued","resourceUri":format!("llm-wiki://work-session/{session_id}")});
        Self::insert_idempotency(
            &transaction,
            &owner,
            "inbound_work_open",
            operation_id,
            &request_hash,
            Some(&session_id),
            Some(&event_id),
            &response,
            &timestamp,
        )?;
        transaction.commit().map_err(storage_error)?;
        Ok(response)
    }

    // Keep the storage boundary explicit: these arguments map to one atomic record.
    #[allow(clippy::too_many_arguments)]
    pub fn append_event(
        &self,
        connection_id: &str,
        operation_id: &str,
        session_id: &str,
        expected_revision: i64,
        kind: EventKind,
        payload: &Value,
        observed_at: Option<&str>,
        supersedes_event_id: Option<&str>,
    ) -> Result<Value, AppError> {
        let started = std::time::Instant::now();
        let canonical = payload.to_string();
        if canonical.len() > 16 * 1024 {
            return Err(AppError::new(
                "content_too_large",
                "Checkpoint exceeds 16 KiB",
            ));
        }
        let owner = hash_text(connection_id);
        let request_hash = hash_text(&json!({"sessionId":session_id,"expectedHeadRevision":expected_revision,"kind":kind.as_str(),"event":payload,"observedAt":observed_at,"supersedesEventId":supersedes_event_id}).to_string());
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        Self::require_scope_on(&transaction, connection_id, "session:write")?;
        if let Some(response) = Self::check_idempotency(
            &transaction,
            &owner,
            "inbound_work_append",
            operation_id,
            &request_hash,
        )? {
            Self::record_activity_on(
                &transaction, "chat", Some(connection_id), Some(session_id),
                "inbound_work_append", "accepted", None, started.elapsed().as_millis(),
            );
            transaction.commit().map_err(storage_error)?;
            return Ok(response);
        }
        let (head_event, head_revision, state) = transaction.query_row(
            "SELECT head_event_id,head_revision,state FROM work_tracking_sessions WHERE id=?1 AND (connection_id=?2 OR ?2='native-in-app-chat')",
            params![session_id,connection_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?)),
        ).optional().map_err(storage_error)?.ok_or_else(|| AppError::new("not_found_or_not_visible","Work session is unavailable"))?;
        if state == "completed" {
            return Err(AppError::new(
                "session_closed",
                "Completed work requires a follow-up session",
            ));
        }
        if head_revision != expected_revision {
            return Err(AppError::conflict(
                "Work session changed; refresh before saving",
                head_revision,
            ));
        }
        let stream_id = format!(
            "{}:{connection_id}",
            if connection_id == "native-in-app-chat" {
                "in_app_chat"
            } else {
                "external_mcp_chat"
            }
        );
        let source_sequence: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(source_sequence),0)+1 FROM work_tracking_events WHERE session_id=? AND stream_id=?",
            params![session_id,stream_id], |row| row.get(0),
        ).map_err(storage_error)?;
        let revision = head_revision + 1;
        let event_id = Uuid::new_v4().to_string();
        let timestamp = now();
        let payload_hash = hash_text(&canonical);
        transaction.execute(
            "INSERT INTO work_tracking_events(id,session_id,revision,previous_event_id,stream_id,source_sequence,kind,payload_json,payload_hash,supersedes_event_id,occurred_at,observed_at,ingested_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![event_id,session_id,revision,head_event,stream_id,source_sequence,kind.as_str(),canonical,payload_hash,supersedes_event_id,timestamp,observed_at,timestamp],
        ).map_err(storage_error)?;
        transaction.execute("UPDATE work_tracking_sessions SET head_event_id=?,head_revision=?,updated_at=?,state=CASE WHEN ?='completion_proposal' THEN 'completion_proposed' ELSE state END WHERE id=? AND head_revision=?",
            params![event_id,revision,timestamp,kind.as_str(),session_id,head_revision]).map_err(storage_error)?;
        let checkpoint_policy: String = transaction
            .query_row(
                "SELECT checkpoint_policy FROM mcp_connections WHERE id=?",
                [connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let auto_accept = kind == EventKind::WorkLogCheckpoint
            && checkpoint_policy == "allowed_for_started_sessions";
        let job_state = if auto_accept {
            ProjectionStatus::Queued.as_str()
        } else {
            ProjectionStatus::PendingReview.as_str()
        };
        transaction.execute("INSERT INTO work_tracking_projection_jobs(event_id,projection_name,state,created_at,updated_at) VALUES (?,'workflow',?,?,?)",params![event_id,job_state,timestamp,timestamp]).map_err(storage_error)?;
        if auto_accept {
            transaction.execute("INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,decision_channel,created_at) VALUES (?,?,?,'accepted',?,'checkpoint_policy',?)",params![Uuid::new_v4().to_string(),session_id,event_id,payload_hash,timestamp]).map_err(storage_error)?;
        }
        let response = json!({"sessionId":session_id,"eventId":event_id,"kind":kind.as_str(),"revision":revision,"headEventId":event_id,"headRevision":revision,"deduplicated":false,"persistenceStatus":"durable","projectionStatus":if auto_accept{"queued"}else{"pending_review"},"workflowState":"unchanged","reviewState":if auto_accept{"accepted_by_policy"}else{"pending"},"resourceUri":format!("llm-wiki://work-session/{session_id}")});
        Self::insert_idempotency(
            &transaction,
            &owner,
            "inbound_work_append",
            operation_id,
            &request_hash,
            Some(session_id),
            Some(&event_id),
            &response,
            &timestamp,
        )?;
        Self::record_activity_on(
            &transaction, "chat", Some(connection_id), Some(session_id),
            "inbound_work_append", "accepted", None, started.elapsed().as_millis(),
        );
        transaction.commit().map_err(storage_error)?;
        Ok(response)
    }

    pub fn record_decision(
        &self,
        connection_id: &str,
        session_id: &str,
        event_id: &str,
        decision: &str,
        channel: &str,
    ) -> Result<Value, AppError> {
        self.connection_scopes(connection_id)?;
        if !["accepted", "rejected", "accepted_with_edits"].contains(&decision) {
            return Err(AppError::new("invalid_input", "Unsupported decision"));
        }
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (kind,payload_hash,revision) = transaction.query_row(
            "SELECT e.kind,e.payload_hash,s.head_revision FROM work_tracking_events e JOIN work_tracking_sessions s ON s.id=e.session_id WHERE e.id=?1 AND e.session_id=?2 AND (s.connection_id=?3 OR ?3='native-in-app-chat')",
            params![event_id,session_id,connection_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?)),
        ).optional().map_err(storage_error)?.ok_or_else(||AppError::new("not_found_or_not_visible","Work event is unavailable"))?;
        let decision_id = Uuid::new_v4().to_string();
        let timestamp = now();
        transaction.execute("INSERT OR IGNORE INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,decision_channel,created_at) VALUES (?,?,?,?,?,?,?)",
            params![decision_id,session_id,event_id,decision,if decision.starts_with("accepted"){Some(payload_hash)}else{None},channel,timestamp]).map_err(storage_error)?;
        if decision.starts_with("accepted") {
            transaction.execute("UPDATE work_tracking_projection_jobs SET state='pending',next_attempt_at=NULL,updated_at=? WHERE event_id=?",params![timestamp,event_id]).map_err(storage_error)?;
        } else {
            transaction.execute("UPDATE work_tracking_projection_jobs SET state='conflict',safe_error_code='rejected',updated_at=? WHERE event_id=?",params![timestamp,event_id]).map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(
            json!({"decisionId":decision_id,"sessionId":session_id,"eventId":event_id,"decision":decision,"eventKind":kind,"headRevision":revision,"projectionStatus":if decision.starts_with("accepted"){"queued"}else{"conflict"}}),
        )
    }

    pub fn create_challenge(&self, connection_id: &str, input: &Value) -> Result<String, AppError> {
        self.connection_scopes(connection_id)?;
        let operation_id = require_text(input, "operationId", 160)?;
        let owner = hash_text(connection_id);
        let request_hash = hash_text(&input.to_string());
        let session_id = require_text(input, "sessionId", 80)?;
        let source_event_id = require_text(input, "sourceEventId", 80)?;
        let action = require_text(input, "action", 80)?;
        if matches!(
            action,
            "adopt_problem"
                | "approve_problem"
                | "adopt_solution"
                | "approve_solution"
                | "resolve_conflict"
                | "accept_completion_proposal"
                | "verify_and_complete"
        ) {
            return Err(AppError::new(
                "legacy_action_rejected",
                "This legacy workflow action is preserved only as history; use a canonical Task action",
            ));
        }
        if !matches!(
            action,
            "task.create"
                | "task.revision"
                | "task.transition"
                | "task.reopen"
                | "task.completion.create"
                | "problem.create"
                | "problem.revision"
                | "problem.resolution.create"
                | "task.problem-link.create"
                | "task.problem-link.delete"
                | "task.relationship.create"
                | "task.relationship.delete"
                | "task.readiness.decision"
                | "task.work-log.create"
                | "work-log.comment.create"
                | "task.checklist.create"
                | "task.checklist.update"
                | "task.decision.create"
                | "review_conflict"
                | "link_current_work"
                | "create_task"
                | "revise_task"
                | "transition_task"
                | "complete_task"
                | "resolve_problem"
                | "accept_checkpoint"
        ) {
            return Err(AppError::new(
                "invalid_input",
                "Unsupported governed Task action",
            ));
        }
        let expected = input
            .get("expectedHeadRevision")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::new("invalid_input", "expectedHeadRevision is required"))?;
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mcp_connections WHERE id=? AND state='active')",
                [connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !active {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Connection is unavailable",
            ));
        }
        if let Some(response) = Self::check_idempotency(
            &tx,
            &owner,
            "inbound_work_advance",
            operation_id,
            &request_hash,
        )? {
            let review_state = response["reviewState"]
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::new(
                        "storage_unavailable",
                        "Reviewed operation replay record is invalid",
                    )
                })?
                .to_owned();
            tx.commit().map_err(storage_error)?;
            return Ok(review_state);
        }
        let current: i64 = tx
            .query_row(
                "SELECT head_revision FROM work_tracking_sessions WHERE id=?1 AND (connection_id=?2 OR ?2='native-in-app-chat')",
                params![session_id, connection_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new("not_found_or_not_visible", "Work session is unavailable")
            })?;
        if current != expected {
            return Err(AppError::conflict(
                "Work session changed; refresh before reviewing",
                current,
            ));
        }
        let exists: i64 = tx
            .query_row(
                "SELECT count(*) FROM work_tracking_events WHERE id=? AND session_id=?",
                params![source_event_id, session_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if exists == 0 {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Source event is unavailable",
            ));
        }
        let target = Self::governed_target_snapshot(
            &tx,
            connection_id,
            session_id,
            action,
            &input["proposedPayload"],
        )?;
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        let expiry = (Utc::now() + chrono::Duration::minutes(10))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
        tx.execute("INSERT INTO mcp_elicitation_challenges(id,connection_id,session_id,action,source_event_id,proposed_payload_hash,expected_head_revision,nonce_hash,expires_at,created_at,target_snapshot_json) VALUES (?,?,?,?,?,?,?,?,?,?,?)",params![id,connection_id,session_id,action,source_event_id,&request_hash,expected,hash_text(&Uuid::new_v4().to_string()),expiry,timestamp,target.to_string()]).map_err(storage_error)?;
        Self::insert_idempotency(
            &tx,
            &owner,
            "inbound_work_advance",
            operation_id,
            &request_hash,
            Some(session_id),
            Some(source_event_id),
            &json!({"reviewState":id,"decisionRequired":true}),
            &timestamp,
        )?;
        tx.commit().map_err(storage_error)?;
        Ok(id)
    }

    pub fn advance_result_replay(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Option<Value>, AppError> {
        self.connection_scopes(connection_id)?;
        let operation_id = require_text(input, "operationId", 160)?;
        let owner = hash_text(connection_id);
        let request_hash = hash_text(&input.to_string());
        let connection = database::open(&self.path).map_err(storage_error)?;
        let stored = connection
            .query_row(
                "SELECT request_hash,response_json FROM work_tracking_idempotency_records WHERE source_interface IN ('external_mcp_chat','in_app_chat') AND source_owner_hash=? AND operation_name='inbound_work_advance' AND operation_id=?",
                params![owner, operation_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((stored_hash, response)) = stored else {
            return Ok(None);
        };
        if stored_hash != request_hash {
            return Err(AppError::new(
                "idempotency_conflict",
                "Operation ID was already used with different input",
            ));
        }
        let mut response: Value = serde_json::from_str(&response).map_err(storage_error)?;
        if response.get("decision").is_none() {
            return Ok(None);
        }
        response["deduplicated"] = json!(true);
        Ok(Some(response))
    }

    fn governed_problem_access_on(
        connection: &Connection,
        connection_id: &str,
        problem_id: &str,
    ) -> Result<(), AppError> {
        let (scopes, topics): (String, String) = connection
            .query_row(
                "SELECT scopes_json,allowed_topics_json FROM mcp_connections WHERE id=? AND state='active'",
                [connection_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| AppError::new("not_found_or_not_visible", "Connection is unavailable"))?;
        let scopes = serde_json::from_str::<Vec<String>>(&scopes).unwrap_or_default();
        let topics = serde_json::from_str::<Vec<String>>(&topics).unwrap_or_default();
        // The in-app connection has an explicit Workbench overview grant. It can attach an
        // existing Problem without first manufacturing a session link; unknown IDs remain
        // indistinguishable from hidden records.
        if scopes
            .iter()
            .any(|scope| scope == "workbench:overview:read")
        {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM problems WHERE id=?)",
                    [problem_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            if exists {
                return Ok(());
            }
        }
        let linked: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM work_tracking_links l JOIN work_tracking_sessions s ON s.id=l.session_id WHERE l.entity_type='problems' AND l.entity_id=? AND (s.connection_id=? OR ?='native-in-app-chat'))",
                params![problem_id, connection_id, connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if linked && scopes.iter().any(|scope| scope == "workbench:current:read") {
            return Ok(());
        }
        let linked_through_task: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM task_problem_links p JOIN work_tracking_links l ON l.entity_type='tasks' AND l.entity_id=p.task_id JOIN work_tracking_sessions s ON s.id=l.session_id WHERE p.problem_id=? AND p.unlinked_at IS NULL AND (s.connection_id=? OR ?='native-in-app-chat'))",
                params![problem_id, connection_id, connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if linked_through_task && scopes.iter().any(|scope| scope == "workbench:current:read") {
            return Ok(());
        }
        if scopes.iter().any(|scope| scope == "topic:read") {
            let visible: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM work_tracking_topic_memberships WHERE entity_type='problems' AND entity_id=? AND topic_id IN (SELECT value FROM json_each(?)))",
                    params![problem_id, serde_json::to_string(&topics).unwrap_or_else(|_| "[]".into())],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            if visible {
                return Ok(());
            }
        }
        Err(AppError::new(
            "not_found_or_not_visible",
            "Problem target is unavailable",
        ))
    }

    fn governed_target_snapshot(
        connection: &Connection,
        connection_id: &str,
        session_id: &str,
        action: &str,
        payload: &Value,
    ) -> Result<Value, AppError> {
        let mut tasks = BTreeSet::new();
        let mut problems = BTreeSet::new();
        let mut exact_problem_revisions = BTreeMap::new();
        let task_from_payload = |payload: &Value| {
            payload
                .get("taskId")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
        };
        let session_task = || -> Result<String, AppError> {
            connection
                .query_row(
                    "SELECT entity_id FROM work_tracking_links WHERE session_id=? AND entity_type='tasks' AND relationship='adopted_task' ORDER BY created_at DESC LIMIT 1",
                    [session_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage_error)?
                .ok_or_else(|| AppError::new("workflow_precondition", "Select an authorized Task first"))
        };
        let primary_task = || {
            task_from_payload(payload)
                .map(Ok)
                .unwrap_or_else(session_task)
        };
        match action {
            "task.create" | "create_task" | "problem.create" | "accept_checkpoint" => {}
            "problem.revision" | "problem.resolution.create" | "resolve_problem" => {
                let id = require_text(payload, "problemId", 120)?.to_owned();
                let revision = payload
                    .get("expectedProblemRevision")
                    .or_else(|| payload.get("problemRevision"))
                    .and_then(Value::as_i64)
                    .ok_or_else(|| {
                        AppError::new("invalid_input", "An exact Problem revision is required")
                    })?;
                problems.insert(id.clone());
                exact_problem_revisions.insert(id, revision);
            }
            "work-log.comment.create" => {
                let entry_id = require_text(payload, "entryId", 120)?;
                let task: String = connection
                    .query_row(
                        "SELECT task_id FROM task_work_log_entries WHERE id=?",
                        [entry_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or_else(|| {
                        AppError::new("not_found_or_not_visible", "Work Log target is unavailable")
                    })?;
                tasks.insert(task);
            }
            "task.relationship.create" => {
                tasks.insert(primary_task()?);
                tasks.insert(require_text(payload, "targetTaskId", 120)?.to_owned());
            }
            "task.relationship.delete" => {
                let relationship_id = require_text(payload, "relationshipId", 120)?;
                let endpoints: (String, String) = connection
                    .query_row(
                        "SELECT source_task_id,target_task_id FROM task_relationships WHERE id=? AND unlinked_at IS NULL",
                        [relationship_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .ok_or_else(|| AppError::new("not_found_or_not_visible", "Relationship target is unavailable"))?;
                tasks.insert(endpoints.0);
                tasks.insert(endpoints.1);
            }
            "task.problem-link.delete" => {
                let link_id = require_text(payload, "linkId", 120)?;
                let link: (String, String, i64) = connection.query_row(
                    "SELECT task_id,problem_id,problem_revision FROM task_problem_links WHERE id=? AND unlinked_at IS NULL",
                    [link_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                ).optional().map_err(storage_error)?.ok_or_else(|| AppError::new("not_found_or_not_visible", "Problem link target is unavailable"))?;
                tasks.insert(link.0);
                problems.insert(link.1.clone());
                exact_problem_revisions.insert(link.1, link.2);
            }
            "link_current_work" => match require_text(payload, "entityType", 40)? {
                "tasks" => {
                    tasks.insert(require_text(payload, "entityId", 120)?.to_owned());
                }
                "problems" => {
                    problems.insert(require_text(payload, "entityId", 120)?.to_owned());
                }
                _ => return Err(AppError::new("invalid_input", "Unsupported link target")),
            },
            _ => {
                tasks.insert(primary_task()?);
            }
        }
        if let Some(problem_id) = payload.get("problemId").and_then(Value::as_str) {
            problems.insert(problem_id.to_owned());
            if let Some(revision) = payload
                .get("problemRevision")
                .or_else(|| payload.get("expectedProblemRevision"))
                .and_then(Value::as_i64)
            {
                exact_problem_revisions.insert(problem_id.to_owned(), revision);
            }
        }
        if let Some(related_task) = payload.get("relatedTaskId").and_then(Value::as_str) {
            tasks.insert(related_task.to_owned());
        }
        if let Some(links) = payload.get("problemLinks").and_then(Value::as_array) {
            for link in links {
                let problem_id = require_text(link, "problemId", 120)?.to_owned();
                let revision = link
                    .get("problemRevision")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| {
                        AppError::new(
                            "invalid_input",
                            "Task Problem links require an exact Problem revision",
                        )
                    })?;
                problems.insert(problem_id.clone());
                exact_problem_revisions.insert(problem_id, revision);
            }
        }
        let task_snapshots = tasks
            .iter()
            .map(|task_id| {
                Self::task_continuation_access_on(connection, connection_id, task_id)?;
                Self::task_continuation_snapshot(connection, connection_id, task_id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let problem_snapshots = problems
            .iter()
            .map(|problem_id| {
                Self::governed_problem_access_on(connection, connection_id, problem_id)?;
                let selected_revision = exact_problem_revisions.get(problem_id).copied();
                connection
                    .query_row(
                        "SELECT json_object('problemId',p.id,'currentRevision',p.current_revision,'state',p.state,'selectedRevision',r.revision,'revision',json_object('statement',r.statement,'detail',r.detail,'contentHash',r.content_hash)) FROM problems p JOIN problem_revisions r ON r.problem_id=p.id AND r.revision=COALESCE(?2,p.current_revision) WHERE p.id=?1",
                        params![problem_id, selected_revision],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(storage_error)?
                    .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    .ok_or_else(|| AppError::new("not_found_or_not_visible", "Problem target is unavailable"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({"tasks":task_snapshots,"problems":problem_snapshots}))
    }

    fn link_snapshot(connection: &Connection, payload: &Value) -> Result<Value, AppError> {
        let kind = require_text(payload, "entityType", 40)?;
        let id = require_text(payload, "entityId", 80)?;
        if kind == "tasks" {
            return connection.query_row("SELECT r.title,t.current_revision,w.revision FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision CROSS JOIN work_tracking_workspace w WHERE t.id=? AND w.id=1",[id],|row|Ok(json!({"entityType":kind,"entityId":id,"title":row.get::<_,String>(0)?,"entityRevision":row.get::<_,i64>(1)?,"workspaceRevision":row.get::<_,i64>(2)?}))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("not_found_or_not_visible","Workbench target is unavailable"));
        }
        let field = match kind {
            "problems" => "statement",
            "features" => "title",
            _ => {
                return Err(AppError::new(
                    "invalid_input",
                    "Select an exact Problem or Solution",
                ))
            }
        };
        let (title,revision):(String,i64)=connection.query_row(&format!("SELECT e.{field},v.revision FROM {kind} e JOIN work_tracking_entity_versions v ON v.entity_id=e.id AND v.entity_type=? WHERE e.id=?"),params![kind,id],|row|Ok((row.get(0)?,row.get(1)?))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("not_found_or_not_visible","Workbench target is unavailable"))?;
        let workspace: i64 = connection
            .query_row(
                "SELECT revision FROM work_tracking_workspace WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        Ok(
            json!({"entityType":kind,"entityId":id,"title":title,"entityRevision":revision,"workspaceRevision":workspace}),
        )
    }

    /// Snapshot every Task dependency that this continuation exposes.  It is deliberately
    /// compared byte-for-byte at acceptance, which makes a child-only update stale even when
    /// `tasks.current_revision` did not change.
    fn task_continuation_snapshot(
        connection: &Connection,
        _connection_id: &str,
        task_id: &str,
    ) -> Result<Value, AppError> {
        let topics: String = connection
            .query_row(
                "SELECT COALESCE(json_group_array(topic_id),'[]') FROM (SELECT topic_id FROM work_tracking_topic_memberships WHERE entity_type='tasks' AND entity_id=? ORDER BY topic_id)",
                [task_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let topics_value: Value = serde_json::from_str(&topics).unwrap_or_else(|_| json!([]));
        let task: Value = connection.query_row(
            "SELECT json_object('taskId',t.id,'taskRevision',t.current_revision,'state',t.state,'title',r.title,'detail',r.detail,'outcome',r.outcome,'scope',r.scope,'nonGoals',r.non_goals,'validationCriteria',r.validation_criteria,'contentHash',r.content_hash,'lastUserActivityAt',t.last_user_activity_at,'workspaceRevision',w.revision) FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision CROSS JOIN work_tracking_workspace w WHERE t.id=? AND w.id=1",
            [task_id],
            |row| row.get::<_, String>(0),
        ).optional().map_err(storage_error)?.and_then(|row|serde_json::from_str(&row).ok()).ok_or_else(|| AppError::new("not_found_or_not_visible", "Task is unavailable"))?;
        let aggregate = |sql: &str| -> Result<Value, AppError> {
            connection
                .query_row(sql, [task_id], |row| row.get::<_, String>(0))
                .map_err(storage_error)
                .map(|raw| serde_json::from_str(&raw).unwrap_or_else(|_| json!([])))
        };
        let links = aggregate("SELECT COALESCE(json_group_array(json_object('id',id,'problemId',problem_id,'problemRevision',problem_revision,'relationship',relationship,'note',note,'unlinkedAt',unlinked_at)),'[]') FROM (SELECT * FROM task_problem_links WHERE task_id=? ORDER BY id)")?;
        // SQLite parameters are positional; the relationship query uses the Task twice.
        let relationships: Value = connection.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'sourceTaskId',source_task_id,'targetTaskId',target_task_id,'kind',kind,'note',note,'unlinkedAt',unlinked_at)),'[]') FROM (SELECT * FROM task_relationships WHERE source_task_id=? OR target_task_id=? ORDER BY id)",params![task_id,task_id],|row|row.get::<_,String>(0)).map_err(storage_error).map(|raw|serde_json::from_str(&raw).unwrap_or_else(|_|json!([])))?;
        // Binary evidence affects freshness but never becomes an elicitation payload.
        let mut work_statement = connection.prepare("SELECT id,body,image_data,image_media_type,image_summary,created_at FROM task_work_log_entries WHERE task_id=? ORDER BY id").map_err(storage_error)?;
        let work = work_statement.query_map([task_id], |row| {
            let image_data: String = row.get(2)?;
            Ok(json!({"id":row.get::<_,String>(0)?,"body":row.get::<_,String>(1)?,"imageHash":hash_text(&image_data),"imageBytes":image_data.len(),"imageMediaType":row.get::<_,Option<String>>(3)?,"imageSummary":row.get::<_,Option<String>>(4)?,"createdAt":row.get::<_,String>(5)?}))
        }).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        let comments = aggregate("SELECT COALESCE(json_group_array(json_object('id',c.id,'entryId',c.entry_id,'body',c.body,'createdAt',c.created_at)),'[]') FROM (SELECT c.* FROM task_work_log_comments c JOIN task_work_log_entries e ON e.id=c.entry_id WHERE e.task_id=? ORDER BY c.id) c")?;
        let mut attachment_statement = connection.prepare("SELECT id,entry_id,name,media_type,data,byte_hash,created_at FROM task_attachments WHERE task_id=? ORDER BY id").map_err(storage_error)?;
        let attachments = attachment_statement.query_map([task_id], |row| {
            let data: String = row.get(4)?;
            let stored_hash: String = row.get(5)?;
            Ok(json!({"id":row.get::<_,String>(0)?,"entryId":row.get::<_,String>(1)?,"name":row.get::<_,String>(2)?,"mediaType":row.get::<_,String>(3)?,"byteHash":if stored_hash.is_empty(){hash_text(&data)}else{stored_hash},"bytes":data.len(),"createdAt":row.get::<_,String>(6)?}))
        }).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        let readiness = aggregate("SELECT COALESCE(json_group_array(json_object('id',id,'taskRevision',task_revision,'fieldKey',field_key,'status',status,'reason',reason,'evidenceRefs',evidence_refs_json,'createdAt',created_at)),'[]') FROM (SELECT * FROM task_readiness_decisions WHERE task_id=? ORDER BY id)")?;
        let checklist = aggregate("SELECT COALESCE(json_group_array(json_object('id',id,'body',body,'checked',checked,'updatedAt',updated_at)),'[]') FROM (SELECT * FROM task_checklist_items WHERE task_id=? ORDER BY id)")?;
        let decisions = aggregate("SELECT COALESCE(json_group_array(json_object('id',id,'taskRevision',task_revision,'kind',kind,'payload',payload_json,'createdAt',created_at)),'[]') FROM (SELECT * FROM task_decisions WHERE task_id=? ORDER BY id)")?;
        let completions = aggregate("SELECT COALESCE(json_group_array(json_object('id',id,'taskRevision',task_revision,'evidence',evidence,'report',report,'createdAt',created_at)),'[]') FROM (SELECT * FROM task_completions WHERE task_id=? ORDER BY id)")?;
        let knowledge = aggregate("SELECT COALESCE(json_group_array(json_object('revision',revision,'taskRevision',task_revision,'contentHash',content_hash,'lineage',lineage_json,'state',state,'publishedHash',published_hash,'updatedAt',updated_at)),'[]') FROM (SELECT * FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision)")?;
        Ok(
            json!({"task":task,"topics":topics_value,"problemLinks":links,"relationships":relationships,"workLog":work,"comments":comments,"attachments":attachments,"readinessDecisions":readiness,"checklist":checklist,"decisions":decisions,"completions":completions,"knowledge":knowledge}),
        )
    }

    /// The persisted target snapshot is intentionally exhaustive for stale detection.  The
    /// review display is not: raw attachments, work bodies, and decision payloads remain local
    /// evidence and never become an MCP elicitation preview.
    fn task_continuation_preview(snapshot: &Value) -> Value {
        let task = &snapshot["task"];
        let ids = |field: &str| {
            snapshot[field]
                .as_array()
                .map(|items| items.len())
                .unwrap_or_default()
        };
        json!({
            "task": {
                "taskId": task["taskId"], "taskRevision": task["taskRevision"],
                "title": bounded_overview_text(task["title"].as_str().unwrap_or_default(), OVERVIEW_TEXT_LIMIT),
                "state": task["state"], "contentHash": task["contentHash"],
                "workspaceRevision": task["workspaceRevision"]
            },
            "problemLinks": snapshot["problemLinks"], "relationships": snapshot["relationships"],
            "activity": {"workLogCount":ids("workLog"),"checklistCount":ids("checklist"),"decisionCount":ids("decisions"),"completionCount":ids("completions"),"knowledgeCount":ids("knowledge")},
            "sourceHash": hash_text(&snapshot.to_string())
        })
    }

    pub fn review_target(&self, owner: &str, state: &str) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let target:Option<String>=connection.query_row("SELECT target_snapshot_json FROM mcp_elicitation_challenges WHERE id=? AND connection_id=?",params![state,owner],|row|row.get(0)).map_err(storage_error)?;
        let target = target
            .map(|v| serde_json::from_str(&v).map_err(storage_error))
            .transpose()
            .map(|v| v.unwrap_or(Value::Null))?;
        // Challenge storage retains complete snapshots for acceptance freshness.  Elicitation
        // receives only a bounded Task summary and selected Problem revision text.
        if let Some(tasks) = target.get("tasks").and_then(Value::as_array) {
            let problems = target
                .get("problems")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .map(|problem| {
                            let mut problem = problem.clone();
                            if let Some(revision) =
                                problem.get_mut("revision").and_then(Value::as_object_mut)
                            {
                                for key in ["statement", "detail"] {
                                    if let Some(text) = revision.get(key).and_then(Value::as_str) {
                                        revision.insert(
                                            key.to_owned(),
                                            Value::String(bounded_overview_text(text, 4000)),
                                        );
                                    }
                                }
                            }
                            problem
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return Ok(
                json!({"tasks":tasks.iter().map(Self::task_continuation_preview).collect::<Vec<_>>(),"problems":problems}),
            );
        }
        Ok(target)
    }

    pub fn set_topic_membership(&self, input: &Value) -> Result<Value, AppError> {
        let topic = require_text(input, "topicId", 120)?;
        let kind = require_text(input, "entityType", 40)?;
        let id = require_text(input, "entityId", 1000)?;
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let table = match kind {
            "vault" => "vault_documents",
            "captures" => "captures",
            "problems" => "problems",
            "features" => "features",
            "tasks" => "tasks",
            _ => return Err(AppError::new("invalid_input", "Unsupported topic member")),
        };
        let key = if kind == "vault" { "path" } else { "id" };
        let exists: bool = tx
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {key}=?)"),
                [id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !exists {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Topic member is unavailable",
            ));
        }
        match input["included"].as_bool() {
            Some(true) => {
                tx.execute("INSERT OR IGNORE INTO work_tracking_topic_memberships(topic_id,entity_type,entity_id) VALUES(?,?,?)",params![topic,kind,id]).map_err(storage_error)?;
            }
            Some(false) => {
                tx.execute("DELETE FROM work_tracking_topic_memberships WHERE topic_id=? AND entity_type=? AND entity_id=?",params![topic,kind,id]).map_err(storage_error)?;
            }
            None => return Err(AppError::new("invalid_input", "included must be a boolean")),
        }
        tx.commit().map_err(storage_error)?;
        Ok(json!({"topicId":topic,"entityType":kind,"entityId":id,"included":input["included"]}))
    }

    pub fn consume_challenge(
        &self,
        connection_id: &str,
        challenge_id: &str,
        input: &Value,
        decision: &str,
    ) -> Result<Value, AppError> {
        if !matches!(decision, "accept" | "reject" | "cancel") {
            return Err(AppError::new(
                "invalid_input",
                "Decision must be accept, reject or cancel",
            ));
        }
        self.connection_scopes(connection_id)?;
        let operation_id = require_text(input, "operationId", 160)?;
        let owner = hash_text(connection_id);
        let request_hash = hash_text(&input.to_string());
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let has_idempotency = tx
            .query_row(
                "SELECT request_hash,response_json FROM work_tracking_idempotency_records WHERE source_interface IN ('external_mcp_chat','in_app_chat') AND source_owner_hash=? AND operation_name='inbound_work_advance' AND operation_id=?",
                params![owner, operation_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        if let Some((stored_hash, response)) = &has_idempotency {
            if stored_hash != &request_hash {
                return Err(AppError::new(
                    "idempotency_conflict",
                    "Operation ID was already used with different input",
                ));
            }
            let mut response: Value = serde_json::from_str(response).map_err(storage_error)?;
            let stored_state = response["reviewState"].as_str();
            if stored_state != Some(challenge_id) {
                return Err(AppError::new("invalid_input", "Review state is invalid"));
            }
            if response.get("decision").is_some() {
                response["deduplicated"] = json!(true);
                tx.commit().map_err(storage_error)?;
                return Ok(response);
            }
        }
        let (session_id,event_id,action,input_hash,expected,expires,status)=tx.query_row("SELECT session_id,source_event_id,action,proposed_payload_hash,expected_head_revision,expires_at,status FROM mcp_elicitation_challenges WHERE id=? AND connection_id=?",params![challenge_id,connection_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,i64>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("invalid_input","Review state is invalid"))?;
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mcp_connections WHERE id=? AND state='active')",
                [connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if !active {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Connection is unavailable",
            ));
        }
        if status != "pending" {
            return Err(AppError::new(
                "challenge_replayed",
                "Review state was already used",
            ));
        }
        if expires < now() {
            return Err(AppError::new("challenge_expired", "Review state expired"));
        }
        // Challenges created before the Task workflow migration are retained as
        // audit history only. A delayed accept must never resurrect a Problem /
        // Solution approval chain; the caller must begin a fresh canonical review.
        if matches!(
            action.as_str(),
            "adopt_problem"
                | "approve_problem"
                | "adopt_solution"
                | "approve_solution"
                | "resolve_conflict"
                | "accept_completion_proposal"
                | "verify_and_complete"
        ) {
            return Err(AppError::new(
                "legacy_action_rejected",
                "This legacy workflow action is preserved only as history; use a canonical Task action",
            ));
        }
        if input_hash != request_hash {
            return Err(AppError::new("invalid_input", "Reviewed payload changed"));
        }
        if decision == "accept" {
            let captured: String = tx
                .query_row(
                    "SELECT target_snapshot_json FROM mcp_elicitation_challenges WHERE id=?",
                    [challenge_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            let fresh = Self::governed_target_snapshot(
                &tx,
                connection_id,
                &session_id,
                &action,
                &input["proposedPayload"],
            )?;
            if serde_json::from_str::<Value>(&captured).map_err(storage_error)? != fresh {
                return Err(AppError::new(
                    "head_conflict",
                    "Governed target changed; review the refreshed target",
                ));
            }
        }
        let current: i64 = tx
            .query_row(
                "SELECT head_revision FROM work_tracking_sessions WHERE id=?1 AND (connection_id=?2 OR ?2='native-in-app-chat')",
                params![session_id, connection_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if decision == "accept" && current != expected {
            return Err(AppError::conflict(
                "Work session changed; review again",
                current,
            ));
        }
        let decision_id = Uuid::new_v4().to_string();
        let timestamp = now();
        let accepted = decision == "accept";
        Self::require_scope_on(&tx, connection_id, "session:write")?;
        if action == "link_current_work" {
            Self::require_scope_on(&tx, connection_id, "workbench:current:read")?;
        }
        let closed: bool = tx
            .query_row(
                "SELECT state='completed' FROM work_tracking_sessions WHERE id=?",
                [&session_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        if accepted && closed && action != "task.reopen" {
            return Err(AppError::new(
                "session_closed",
                "Completed work requires a follow-up session",
            ));
        }
        let payload_hash: String = tx
            .query_row(
                "SELECT payload_hash FROM work_tracking_events WHERE id=?",
                [&event_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let payload_hash = if action != "accept_checkpoint" {
            input
                .get("proposedPayload")
                .map(|payload| hash_text(&payload.to_string()))
                .unwrap_or(payload_hash)
        } else {
            payload_hash
        };
        let channel = format!(
            "{}:{action}",
            if connection_id == "native-in-app-chat" {
                "in_app_chat_review"
            } else {
                "mcp_elicitation"
            }
        );
        tx.execute("INSERT INTO work_tracking_decisions(id,session_id,event_id,decision,accepted_payload_hash,challenge_id,decision_channel,client_capability,created_at) VALUES (?,?,?,?,?,?,?,'2026-07-28',?)",params![decision_id,session_id,event_id,if accepted{"accepted"}else{"rejected"},if accepted{Some(payload_hash)}else{None},challenge_id,channel,timestamp]).map_err(storage_error)?;
        let mut result_entity_id = None::<String>;
        if accepted {
            let payload_text: String = tx
                .query_row(
                    "SELECT payload_json FROM work_tracking_events WHERE id=?",
                    [&event_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            let event_payload: Value =
                serde_json::from_str(&payload_text).map_err(storage_error)?;
            let proposed = input.get("proposedPayload").unwrap_or(&event_payload);
            result_entity_id = self.apply_action(
                &tx,
                &session_id,
                &event_id,
                &decision_id,
                &action,
                proposed,
                &timestamp,
            )?;
            if action == "task.reopen" {
                tx.execute(
                    "UPDATE work_tracking_sessions SET state='active',updated_at=? WHERE id=?",
                    params![timestamp, session_id],
                )
                .map_err(storage_error)?;
            }
            tx.execute("UPDATE work_tracking_decisions SET result_entity_type=?,result_entity_id=? WHERE id=?",params![action_result_type(&action),result_entity_id,decision_id]).map_err(storage_error)?;
            // Canonical Task actions finish in this transaction, so their source event
            // must be settled here rather than replayed by the asynchronous projector.
            // A Work Log checkpoint is different: its accepted decision deliberately
            // leaves the event queued until the projector can attach it to a Task (or
            // retain it as `waiting_solution` when no Task has been adopted yet).
            if action != "accept_checkpoint" {
                let (stream_id, source_sequence, occurred_at): (String, i64, String) = tx
                    .query_row(
                        "SELECT stream_id,source_sequence,occurred_at FROM work_tracking_events WHERE id=?",
                        [&event_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(storage_error)?;
                tx.execute("INSERT INTO work_tracking_projection_results(event_id,projection_name,state,result_entity_id,updated_at) VALUES (?,'workflow','applied',?,?) ON CONFLICT(event_id,projection_name) DO UPDATE SET state='applied',result_entity_id=excluded.result_entity_id,updated_at=excluded.updated_at",params![event_id,result_entity_id,timestamp]).map_err(storage_error)?;
                tx.execute("INSERT INTO work_tracking_stream_watermarks(session_id,stream_id,projection_name,last_occurred_at,last_source_sequence,last_event_id,version,updated_at) VALUES (?,?,'workflow',?,?,?,1,?) ON CONFLICT(session_id,stream_id,projection_name) DO UPDATE SET last_occurred_at=excluded.last_occurred_at,last_source_sequence=excluded.last_source_sequence,last_event_id=excluded.last_event_id,version=work_tracking_stream_watermarks.version+1,updated_at=excluded.updated_at WHERE excluded.last_occurred_at>work_tracking_stream_watermarks.last_occurred_at OR (excluded.last_occurred_at=work_tracking_stream_watermarks.last_occurred_at AND (excluded.last_source_sequence>work_tracking_stream_watermarks.last_source_sequence OR (excluded.last_source_sequence=work_tracking_stream_watermarks.last_source_sequence AND excluded.last_event_id>work_tracking_stream_watermarks.last_event_id)))",params![session_id,stream_id,occurred_at,source_sequence,event_id,timestamp]).map_err(storage_error)?;
            }
        }
        tx.execute("UPDATE mcp_elicitation_challenges SET status=?,consumed_at=? WHERE id=? AND status='pending'",params![if accepted{"accepted"}else{"rejected"},timestamp,challenge_id]).map_err(storage_error)?;
        let projection_state = if accepted && action == "accept_checkpoint" {
            "pending"
        } else if accepted {
            "applied"
        } else {
            "conflict"
        };
        tx.execute("UPDATE work_tracking_projection_jobs SET state=?,lease_owner=NULL,lease_expires_at=NULL,safe_error_code=?,updated_at=? WHERE event_id=?",params![projection_state,if accepted{None::<String>}else{Some("rejected".to_owned())},timestamp,event_id]).map_err(storage_error)?;
        let final_head: i64 = tx
            .query_row(
                "SELECT head_revision FROM work_tracking_sessions WHERE id=?",
                [&session_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let response = json!({"decisionId":decision_id,"sessionId":session_id,"sourceEventId":event_id,"action":action,"decision":decision,"resultEntityId":result_entity_id,"headRevision":final_head,"projectionStatus":projection_state,"resourceUri":format!("llm-wiki://work-session/{session_id}"),"reviewState":challenge_id,"deduplicated":false});
        if has_idempotency.is_some() {
            Self::replace_idempotency_response(
                &tx,
                &owner,
                "inbound_work_advance",
                operation_id,
                &request_hash,
                &response,
                &timestamp,
            )?;
        }
        tx.commit().map_err(storage_error)?;
        Ok(response)
    }

    // Keep the storage boundary explicit: these arguments map to one atomic record.
    #[allow(clippy::too_many_arguments)]
    fn apply_action(
        &self,
        tx: &Transaction<'_>,
        session_id: &str,
        event_id: &str,
        decision_id: &str,
        action: &str,
        payload: &Value,
        timestamp: &str,
    ) -> Result<Option<String>, AppError> {
        let linked = |relationship: &str| -> Result<Option<String>, AppError> {
            tx.query_row("SELECT entity_id FROM work_tracking_links WHERE session_id=? AND relationship=? ORDER BY created_at DESC LIMIT 1",params![session_id,relationship],|row|row.get(0)).optional().map_err(storage_error)
        };
        match action {
            "create_task" | "task.create" => {
                let capture: Option<String> = tx
                    .query_row(
                        "SELECT capture_id FROM work_tracking_sessions WHERE id=?",
                        [session_id],
                        |r| r.get(0),
                    )
                    .map_err(storage_error)?;
                let service =
                    crate::application::task_service::TaskApplicationService::new(&self.path);
                let mut task_input = payload.clone();
                // The Task repository records canonical activity for creates too. This ID is
                // server-derived from the accepted decision, never a client suppression key.
                task_input["operationId"] = json!(format!("review:{decision_id}"));
                let task = service
                    .create_task_tx(tx, &task_input, capture.as_deref(), None, timestamp)
                    .map_err(storage_error)?;
                let id = task["id"]
                    .as_str()
                    .ok_or_else(|| storage_error("Task result missing identity"))?;
                if let Some(problem_id) = payload.get("problemId").and_then(Value::as_str) {
                    let revision = payload
                        .get("problemRevision")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| {
                            AppError::new(
                                "invalid_input",
                                "Task proposal requires an exact Problem revision",
                            )
                        })?;
                    let exists: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM problem_revisions WHERE problem_id=? AND revision=?)",
                        params![problem_id, revision],
                        |row| row.get(0),
                    ).map_err(storage_error)?;
                    if !exists {
                        return Err(AppError::new(
                            "not_found_or_not_visible",
                            "Problem revision is unavailable",
                        ));
                    }
                    tx.execute(
                        "INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES(?,?,?,?,?,'',?)",
                        params![Uuid::new_v4().to_string(), id, problem_id, revision, "context", timestamp],
                    ).map_err(storage_error)?;
                }
                tx.execute("INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'tasks',?,'adopted_task',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,id,decision_id,timestamp]).map_err(storage_error)?;
                Ok(Some(id.to_owned()))
            }
            "problem.create" | "problem.revision" => {
                let mut input = payload.clone();
                input["operationId"] = json!(format!("review:{decision_id}"));
                let result =
                    crate::application::task_service::TaskApplicationService::new(&self.path)
                        .execute_tx_for_tracking(tx, action, &input, Some(session_id))
                        .map_err(storage_error)?;
                Ok(result["id"].as_str().map(str::to_owned))
            }
            "revise_task"
            | "task.revision"
            | "transition_task"
            | "task.transition"
            | "task.reopen"
            | "complete_task"
            | "task.completion.create"
            | "resolve_problem"
            | "problem.resolution.create" => {
                let mut input = payload.clone();
                input["operationId"] = json!(format!("review:{decision_id}"));
                if !matches!(action, "resolve_problem" | "problem.resolution.create")
                    && input.get("taskId").is_none()
                {
                    input["taskId"] =
                        json!(linked("adopted_task")?.ok_or_else(|| AppError::new(
                            "workflow_precondition",
                            "Select a Task first"
                        ))?);
                }
                let name = match action {
                    "revise_task" | "task.revision" => "task.revision",
                    "transition_task" | "task.transition" => "task.transition",
                    "task.reopen" => "task.reopen",
                    "complete_task" | "task.completion.create" => "task.completion.create",
                    _ => "problem.resolution.create",
                };
                let result =
                    crate::application::task_service::TaskApplicationService::new(&self.path)
                        .execute_tx_for_tracking(tx, name, &input, Some(session_id))
                        .map_err(storage_error)?;
                if matches!(action, "complete_task" | "task.completion.create") {
                    tx.execute("UPDATE work_tracking_sessions SET state='completed',publication_state='offered',publication_offer_revision=head_revision,updated_at=? WHERE id=?",params![timestamp,session_id]).map_err(storage_error)?;
                }
                Ok(result["id"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| input["taskId"].as_str().map(str::to_owned)))
            }
            "task.problem-link.create"
            | "task.problem-link.delete"
            | "task.relationship.create"
            | "task.relationship.delete" => {
                let mut input = payload.clone();
                input["operationId"] = json!(format!("review:{decision_id}"));
                if input.get("taskId").is_none() {
                    input["taskId"] = json!(linked("adopted_task")?.ok_or_else(|| {
                        AppError::new("workflow_precondition", "Select a Task first")
                    })?);
                }
                let result =
                    crate::application::task_service::TaskApplicationService::new(&self.path)
                        .execute_tx_for_tracking(tx, action, &input, Some(session_id))
                        .map_err(storage_error)?;
                Ok(result["id"].as_str().map(str::to_owned))
            }
            "task.work-log.create"
            | "work-log.comment.create"
            | "task.checklist.create"
            | "task.checklist.update"
            | "task.decision.create"
            | "task.readiness.decision"
            | "review_conflict" => {
                let mut input = payload.clone();
                input["operationId"] = json!(format!("review:{decision_id}"));
                if input.get("taskId").is_none() {
                    input["taskId"] = json!(linked("adopted_task")?.ok_or_else(|| {
                        AppError::new("workflow_precondition", "Select a Task first")
                    })?);
                }
                let name = if action == "review_conflict" {
                    input["kind"] = json!("advisory_review");
                    input["payload"] = payload.clone();
                    "task.decision.create"
                } else {
                    action
                };
                let result =
                    crate::application::task_service::TaskApplicationService::new(&self.path)
                        .execute_tx_for_tracking(tx, name, &input, Some(session_id))
                        .map_err(storage_error)?;
                Ok(result["id"].as_str().map(str::to_owned))
            }
            "adopt_problem" => {
                let id = linked("adopted_problem")?.unwrap_or_else(|| Uuid::new_v4().to_string());
                let capture_id: String = tx
                    .query_row(
                        "SELECT capture_id FROM work_tracking_sessions WHERE id=?",
                        [session_id],
                        |row| row.get(0),
                    )
                    .map_err(storage_error)?;
                tx.execute("INSERT INTO problems(id,capture_id,statement,detail,state,created_at) VALUES (?,?,?,?,'draft',?) ON CONFLICT(id) DO UPDATE SET statement=excluded.statement,detail=excluded.detail",params![id,capture_id,require_text(payload,"statement",1000)?,payload.get("detail").and_then(Value::as_str).unwrap_or(""),timestamp]).map_err(storage_error)?;
                tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'problems',?,'adopted_problem',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,id,decision_id,timestamp]).map_err(storage_error)?;
                Ok(Some(id))
            }
            "approve_problem" => {
                let id = linked("adopted_problem")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Problem first")
                })?;
                tx.execute("UPDATE problems SET state='approved' WHERE id=?", [&id])
                    .map_err(storage_error)?;
                tx.execute("INSERT INTO approvals(id,entity_type,entity_id,action,created_at) VALUES (?,'problems',?,'approve',?)",params![Uuid::new_v4().to_string(),id,timestamp]).map_err(storage_error)?;
                Ok(Some(id))
            }
            "adopt_solution" => {
                let problem = linked("adopted_problem")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Problem first")
                })?;
                let state: String = tx
                    .query_row("SELECT state FROM problems WHERE id=?", [&problem], |row| {
                        row.get(0)
                    })
                    .map_err(storage_error)?;
                if state != "approved" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Approve the Problem first",
                    ));
                }
                let id = linked("adopted_solution")?.unwrap_or_else(|| Uuid::new_v4().to_string());
                tx.execute("INSERT INTO features(id,problem_id,title,outcome,non_goals,conflict_state,validation_criteria,state,created_at) VALUES (?,?,?,?,?,'unknown',?,'proposed',?) ON CONFLICT(id) DO UPDATE SET title=excluded.title,outcome=excluded.outcome,non_goals=excluded.non_goals,validation_criteria=excluded.validation_criteria",params![id,problem,require_text(payload,"title",1000)?,payload.get("outcome").and_then(Value::as_str).unwrap_or(""),payload.get("nonGoals").and_then(Value::as_str).unwrap_or(""),payload.get("validationCriteria").and_then(Value::as_str).unwrap_or(""),timestamp]).map_err(storage_error)?;
                tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'features',?,'adopted_solution',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,id,decision_id,timestamp]).map_err(storage_error)?;
                Ok(Some(id))
            }
            "resolve_conflict" => {
                let source_kind: String = tx
                    .query_row(
                        "SELECT kind FROM work_tracking_events WHERE id=? AND session_id=?",
                        params![event_id, session_id],
                        |row| row.get(0),
                    )
                    .map_err(storage_error)?;
                if source_kind != "conflict_proposal" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Review an explicit current-chat conflict proposal first",
                    ));
                }
                require_text(payload, "rationale", 4000)?;
                require_text(payload, "proposedResolution", 4000)?;
                let evidence = payload["evidence"].as_array().ok_or_else(|| {
                    AppError::new("invalid_input", "Conflict evidence must be an array")
                })?;
                if evidence.is_empty() && payload["coverage"] != "insufficient" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Acknowledge insufficient evidence before resolving without citations",
                    ));
                }
                let id = linked("adopted_solution")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Solution first")
                })?;
                tx.execute(
                    "UPDATE features SET conflict_state='clear' WHERE id=?",
                    [&id],
                )
                .map_err(storage_error)?;
                Ok(Some(id))
            }
            "approve_solution" => {
                let id = linked("adopted_solution")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Solution first")
                })?;
                let conflict: String = tx
                    .query_row(
                        "SELECT conflict_state FROM features WHERE id=?",
                        [&id],
                        |row| row.get(0),
                    )
                    .map_err(storage_error)?;
                if conflict != "clear" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Review Vault conflicts with the current-session AI first",
                    ));
                }
                tx.execute("UPDATE features SET state='approved' WHERE id=?", [&id])
                    .map_err(storage_error)?;
                tx.execute("INSERT INTO approvals(id,entity_type,entity_id,action,created_at) VALUES (?,'features',?,'approve',?)",params![Uuid::new_v4().to_string(),id,timestamp]).map_err(storage_error)?;
                Ok(Some(id))
            }
            "accept_checkpoint" => Ok(None),
            "accept_completion_proposal" => {
                let feature = linked("adopted_solution")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Solution first")
                })?;
                let id = Uuid::new_v4().to_string();
                tx.execute("INSERT INTO completion_reviews(id,feature_id,status,report_json,created_at) VALUES (?,?,'ready',?,?)",params![id,feature,payload.to_string(),timestamp]).map_err(storage_error)?;
                Ok(Some(id))
            }
            "verify_and_complete" => {
                let kind: String = tx
                    .query_row(
                        "SELECT kind FROM work_tracking_events WHERE id=? AND session_id=?",
                        params![event_id, session_id],
                        |row| row.get(0),
                    )
                    .map_err(storage_error)?;
                if kind != "completion_proposal" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "An exact completion proposal is required",
                    ));
                }
                let source: String = tx
                    .query_row(
                        "SELECT payload_json FROM work_tracking_events WHERE id=?",
                        [event_id],
                        |row| row.get(0),
                    )
                    .map_err(storage_error)?;
                let source: Value = serde_json::from_str(&source).map_err(storage_error)?;
                let selected = source["selectedEvidence"]
                    .as_array()
                    .filter(|items| !items.is_empty())
                    .ok_or_else(|| {
                        AppError::new(
                            "workflow_precondition",
                            "Select accepted Work Log evidence before completion",
                        )
                    })?;
                for selected in selected {
                    let selected = selected.as_str().ok_or_else(|| {
                        AppError::new("invalid_input", "Evidence event ID is required")
                    })?;
                    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_events e JOIN work_tracking_projection_results p ON p.event_id=e.id AND p.state='applied' WHERE e.id=? AND e.session_id=? AND e.kind='work_log_checkpoint' AND EXISTS(SELECT 1 FROM work_tracking_decisions d WHERE d.event_id=e.id AND d.decision LIKE 'accepted%') AND NOT EXISTS(SELECT 1 FROM work_tracking_events replacement WHERE replacement.supersedes_event_id=e.id))",params![selected,session_id],|row|row.get(0)).map_err(storage_error)?;
                    if !valid {
                        return Err(AppError::new("evidence_revision_changed","Selected Work Log evidence is unavailable, superseded or not yet applied"));
                    }
                }
                let verification = payload
                    .get("verification")
                    .and_then(Value::as_array)
                    .filter(|items| {
                        !items.is_empty()
                            && items
                                .iter()
                                .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                    })
                    .ok_or_else(|| {
                        AppError::new(
                            "workflow_precondition",
                            "Record completion verification evidence",
                        )
                    })?;
                let feature = linked("adopted_solution")?.ok_or_else(|| {
                    AppError::new("workflow_precondition", "Adopt a Solution first")
                })?;
                let (problem, state, conflict): (String, String, String) = tx
                    .query_row(
                        "SELECT problem_id,state,conflict_state FROM features WHERE id=?",
                        [&feature],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(storage_error)?;
                if state != "approved" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Approve the Solution first",
                    ));
                }
                if conflict != "clear" {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Resolve the Solution conflict first",
                    ));
                }
                let unfinished:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM features WHERE problem_id=? AND id!=? AND state NOT IN ('completed','archived','rejected'))",params![problem,feature],|row|row.get(0)).map_err(storage_error)?;
                let unchecked:i64=tx.query_row("SELECT count(*) FROM solution_checklist_items WHERE feature_id=? AND checked=0",[&feature],|row|row.get(0)).map_err(storage_error)?;
                if unchecked > 0 {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Complete the Solution checklist first",
                    ));
                }
                let captured:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM problems p JOIN captures c ON c.id=p.capture_id WHERE p.id=?)",[&problem],|row|row.get(0)).map_err(storage_error)?;
                if !captured {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Completion requires Capture lineage",
                    ));
                }
                let id = Uuid::new_v4().to_string();
                tx.execute("INSERT INTO completions(id,feature_id,evidence,report,knowledge_status,state,created_at) VALUES (?,?,?,?,'pending','verified',?)",params![id,feature,payload.get("verification").unwrap_or(payload).to_string(),payload.to_string(),timestamp]).map_err(storage_error)?;
                tx.execute(
                    "UPDATE features SET state='completed' WHERE id=?",
                    [&feature],
                )
                .map_err(storage_error)?;
                if !unfinished {
                    tx.execute(
                        "UPDATE problems SET state='completed' WHERE id=?",
                        [&problem],
                    )
                    .map_err(storage_error)?;
                    tx.execute("INSERT INTO problem_completion_decisions(id,problem_id,review_id,reason,created_at) VALUES (?,?,?,?,?)",params![Uuid::new_v4().to_string(),problem,event_id,serde_json::to_string(verification).map_err(storage_error)?,timestamp]).map_err(storage_error)?;
                }
                tx.execute("UPDATE work_tracking_sessions SET state='completed',publication_state=?,publication_offer_revision=head_revision,updated_at=? WHERE id=?",params![PublicationState::Offered.as_str(),timestamp,session_id]).map_err(storage_error)?;
                let lineage =
                    crate::native::lineage::create_on(tx, &feature, false).map_err(|_| {
                        AppError::new(
                            "storage_unavailable",
                            "Completion lineage could not be saved",
                        )
                    })?;
                if lineage["lineage"]["stages"]
                    .as_array()
                    .is_none_or(|stages| stages.len() != 4)
                {
                    return Err(AppError::new(
                        "workflow_precondition",
                        "Completion lineage is incomplete",
                    ));
                }
                Ok(Some(id))
            }
            "link_current_work" => {
                let entity_type = require_text(payload, "entityType", 40)?;
                let entity_id = require_text(payload, "entityId", 80)?;
                match entity_type {
                    "tasks" => {
                        Self::link_snapshot(tx, payload)?;
                        tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'tasks',?,'adopted_task',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,entity_id,decision_id,timestamp]).map_err(storage_error)?;
                        Ok(Some(entity_id.to_owned()))
                    }
                    "problems" => {
                        let exists: i64 = tx
                            .query_row(
                                "SELECT count(*) FROM problems WHERE id=?",
                                [entity_id],
                                |row| row.get(0),
                            )
                            .map_err(storage_error)?;
                        if exists == 0 {
                            return Err(AppError::new(
                                "not_found_or_not_visible",
                                "Workbench item is unavailable",
                            ));
                        }
                        tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'problems',?,'adopted_problem',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,entity_id,decision_id,timestamp]).map_err(storage_error)?;
                        Ok(Some(entity_id.to_owned()))
                    }
                    "features" => {
                        let problem: String = tx
                            .query_row(
                                "SELECT problem_id FROM features WHERE id=?",
                                [entity_id],
                                |row| row.get(0),
                            )
                            .optional()
                            .map_err(storage_error)?
                            .ok_or_else(|| {
                                AppError::new(
                                    "not_found_or_not_visible",
                                    "Workbench item is unavailable",
                                )
                            })?;
                        tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'problems',?,'adopted_problem',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,problem,decision_id,timestamp]).map_err(storage_error)?;
                        tx.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES (?,?,?,'features',?,'adopted_solution',?,?)",params![Uuid::new_v4().to_string(),session_id,event_id,entity_id,decision_id,timestamp]).map_err(storage_error)?;
                        Ok(Some(entity_id.to_owned()))
                    }
                    _ => Err(AppError::new(
                        "invalid_input",
                        "Only a Problem or Solution can be linked",
                    )),
                }
            }
            _ => Err(AppError::new(
                "invalid_input",
                "Unsupported governed action",
            )),
        }
    }

    pub fn session_value(&self, connection_id: &str, session_id: &str) -> Result<Value, AppError> {
        self.connection_scopes(connection_id)?;
        let connection = database::open(&self.path).map_err(storage_error)?;
        let snapshot = connection.unchecked_transaction().map_err(storage_error)?;
        Self::session_value_on(&snapshot, connection_id, session_id)
    }

    fn session_value_on(
        connection: &Connection,
        connection_id: &str,
        session_id: &str,
    ) -> Result<Value, AppError> {
        let mut value = connection.query_row(
            "SELECT s.id,s.capture_id,s.head_event_id,s.head_revision,s.state,s.publication_state,c.text,s.updated_at FROM work_tracking_sessions s LEFT JOIN captures c ON c.id=s.capture_id WHERE s.id=?1 AND (s.connection_id=?2 OR ?2='native-in-app-chat')",
            params![session_id,connection_id],|row| {
                let capture_id: Option<String> = row.get(1)?;
                let capture_summary: Option<String> = row.get(6)?;
                Ok(json!({"sessionId":row.get::<_,String>(0)?,"capture":capture_id.map(|id|json!({"id":id,"summary":capture_summary.unwrap_or_default()})).unwrap_or(Value::Null),"headEventId":row.get::<_,String>(2)?,"headRevision":row.get::<_,i64>(3)?,"state":row.get::<_,String>(4)?,"publicationState":row.get::<_,String>(5)?,"updatedAt":row.get::<_,String>(7)?}))
            },
        ).optional().map_err(storage_error)?.ok_or_else(||AppError::new("not_found_or_not_visible","Work session is unavailable"))?;
        let mut statement = connection.prepare(
            "SELECT e.id,e.revision,e.kind,e.payload_json,e.occurred_at,COALESCE(r.state,j.state,'queued') FROM work_tracking_events e LEFT JOIN work_tracking_projection_results r ON r.event_id=e.id AND r.projection_name='workflow' LEFT JOIN work_tracking_projection_jobs j ON j.event_id=e.id AND j.projection_name='workflow' WHERE e.session_id=? ORDER BY e.revision DESC LIMIT 8"
        ).map_err(storage_error)?;
        let events = statement.query_map([session_id],|row|{
            let payload:String=row.get(3)?;
            Ok(json!({"eventId":row.get::<_,String>(0)?,"revision":row.get::<_,i64>(1)?,"kind":row.get::<_,String>(2)?,"payload":serde_json::from_str::<Value>(&payload).unwrap_or(Value::Null),"occurredAt":row.get::<_,String>(4)?,"projectionStatus":row.get::<_,String>(5)?}))
        }).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        value["recentEvents"] = json!(events);
        // Evidence and undecided proposals must survive noisy workflow events and restart.
        // Keep IDs bounded separately from the eight-entry conversational preview.
        let mut evidence_statement=connection.prepare("SELECT e.id FROM work_tracking_events e JOIN work_tracking_projection_results r ON r.event_id=e.id AND r.projection_name='workflow' AND r.state='applied' WHERE e.session_id=? AND e.kind='work_log_checkpoint' AND EXISTS(SELECT 1 FROM work_tracking_decisions d WHERE d.event_id=e.id AND d.decision LIKE 'accepted%') AND NOT EXISTS(SELECT 1 FROM work_tracking_events replacement WHERE replacement.supersedes_event_id=e.id) ORDER BY e.revision DESC LIMIT 100").map_err(storage_error)?;
        value["acceptedEvidenceIds"] = json!(evidence_statement
            .query_map([session_id], |row| row.get::<_, String>(0))
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?);
        let mut pending_statement=connection.prepare("SELECT e.id,e.kind,e.payload_json FROM work_tracking_events e WHERE e.session_id=? AND e.kind IN ('task_created','task_revision_proposed','task_transition_proposed','problem_resolution_proposed','problem_draft','solution_draft','conflict_proposal','completion_proposal') AND NOT EXISTS(SELECT 1 FROM work_tracking_decisions d WHERE d.event_id=e.id) AND NOT EXISTS(SELECT 1 FROM work_tracking_events replacement WHERE replacement.supersedes_event_id=e.id) ORDER BY e.revision DESC LIMIT 8").map_err(storage_error)?;
        value["pendingProposals"]=json!(pending_statement.query_map([session_id],|row|Ok(json!({"eventId":row.get::<_,String>(0)?,"kind":row.get::<_,String>(1)?,"payload":serde_json::from_str::<Value>(&row.get::<_,String>(2)?).unwrap_or(Value::Null)}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?);
        value["acceptedCompletion"]=connection.query_row("SELECT e.id,e.payload_json FROM work_tracking_events e JOIN work_tracking_decisions d ON d.event_id=e.id WHERE e.session_id=? AND d.result_entity_type IN ('task_completions','completions') AND d.decision='accepted' ORDER BY e.revision DESC LIMIT 1",[session_id],|row|Ok(json!({"eventId":row.get::<_,String>(0)?,"payload":serde_json::from_str::<Value>(&row.get::<_,String>(1)?).unwrap_or(Value::Null)}))).optional().map_err(storage_error)?.unwrap_or(Value::Null);
        let problem=connection.query_row("SELECT p.id,p.statement,p.state,l.source_event_id FROM work_tracking_links l JOIN problems p ON p.id=l.entity_id WHERE l.session_id=? AND l.relationship='adopted_problem' ORDER BY l.created_at DESC LIMIT 1",[session_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"sourceEventId":row.get::<_,String>(3)?}))).optional().map_err(storage_error)?;
        let solution=connection.query_row("SELECT f.id,f.title,f.state,f.conflict_state,l.source_event_id FROM work_tracking_links l JOIN features f ON f.id=l.entity_id WHERE l.session_id=? AND l.relationship='adopted_solution' ORDER BY l.created_at DESC LIMIT 1",[session_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"conflictState":row.get::<_,String>(3)?,"sourceEventId":row.get::<_,String>(4)?}))).optional().map_err(storage_error)?;
        let task = connection.query_row("SELECT t.id,r.title,t.state,t.current_revision,l.source_event_id FROM work_tracking_links l JOIN tasks t ON t.id=l.entity_id JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE l.session_id=? AND l.relationship='adopted_task' ORDER BY l.created_at DESC LIMIT 1",[session_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"taskId":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"taskRevision":row.get::<_,i64>(3)?,"sourceEventId":row.get::<_,String>(4)?}))).optional().map_err(storage_error)?;
        value["linkedWorkflow"] = json!({"problem":problem,"solution":solution,"task":task});
        if let Some(task_id) = value["linkedWorkflow"]["task"]["id"].as_str() {
            let links: String = connection.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'problemId',problem_id,'problemRevision',problem_revision,'relationship',relationship,'note',note)),'[]') FROM (SELECT * FROM task_problem_links WHERE task_id=? AND unlinked_at IS NULL ORDER BY id)",[task_id],|row|row.get(0)).map_err(storage_error)?;
            let relationships: String = connection.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'sourceTaskId',source_task_id,'targetTaskId',target_task_id,'kind',kind,'note',note)),'[]') FROM (SELECT * FROM task_relationships WHERE (source_task_id=? OR target_task_id=?) AND unlinked_at IS NULL ORDER BY id)",params![task_id,task_id],|row|row.get(0)).map_err(storage_error)?;
            value["linkedWorkflow"]["task"]["problemLinks"] =
                serde_json::from_str(&links).unwrap_or_else(|_| json!([]));
            value["linkedWorkflow"]["task"]["relationships"] =
                serde_json::from_str(&relationships).unwrap_or_else(|_| json!([]));
        }
        let mut decisions_statement=connection.prepare("SELECT id,event_id,decision,decision_channel,created_at FROM work_tracking_decisions WHERE session_id=? ORDER BY created_at DESC LIMIT 8").map_err(storage_error)?;
        let decisions=decisions_statement.query_map([session_id],|row|Ok(json!({"decisionId":row.get::<_,String>(0)?,"eventId":row.get::<_,String>(1)?,"decision":row.get::<_,String>(2)?,"channel":row.get::<_,String>(3)?,"createdAt":row.get::<_,String>(4)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        value["recentDecisions"] = json!(decisions);
        value["publicationOfferRevision"] = connection
            .query_row(
                "SELECT publication_offer_revision FROM work_tracking_sessions WHERE id=?",
                [session_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .map(Value::from)
            .unwrap_or(Value::Null);
        // Canonical Task Knowledge wins whenever the session binds a Task. The old
        // session-only drafts table remains readable only for pre-migration history.
        value["latestDraft"] = if let Some(task_id) =
            task.as_ref().and_then(|item| item["id"].as_str())
        {
            connection.query_row("SELECT k.task_id,k.revision,k.content_hash,k.lineage_json,k.state,r.title FROM task_knowledge_drafts k JOIN task_revisions r ON r.task_id=k.task_id AND r.revision=k.task_revision WHERE k.task_id=? ORDER BY k.revision DESC LIMIT 1",[task_id],|row| {
                let lineage: Value = serde_json::from_str(&row.get::<_,String>(3)?).unwrap_or(Value::Null);
                Ok(json!({"taskId":row.get::<_,String>(0)?,"draftRevision":row.get::<_,i64>(1)?,"contentHash":row.get::<_,String>(2)?,"sourceHash":lineage["sourceHash"],"title":row.get::<_,String>(5)?,"state":row.get::<_,String>(4)?,"canonical":true}))
            }).optional().map_err(storage_error)?.unwrap_or(Value::Null)
        } else {
            connection.query_row("SELECT id,revision,content_hash,title,summary,state FROM knowledge_drafts WHERE session_id=? ORDER BY created_at DESC,revision DESC LIMIT 1",[session_id],|row|Ok(json!({"draftId":row.get::<_,String>(0)?,"draftRevision":row.get::<_,i64>(1)?,"contentHash":row.get::<_,String>(2)?,"title":row.get::<_,String>(3)?,"summary":row.get::<_,String>(4)?,"state":row.get::<_,String>(5)?,"legacy":true}))).optional().map_err(storage_error)?.unwrap_or(Value::Null)
        };
        // A bound canonical Task is the source of completion and publication state. Session
        // projection fields remain legacy compatibility state and must not hide Task drafts.
        let canonical_completion = task.as_ref().and_then(|item| item["id"].as_str()).map(|task_id| {
            connection.query_row("SELECT c.id,c.task_revision,c.evidence,c.report FROM task_completions c JOIN tasks t ON t.id=c.task_id WHERE c.task_id=? AND c.task_revision=t.current_revision AND t.state='completed' ORDER BY c.created_at DESC LIMIT 1", [task_id], |row| Ok(json!({"id":row.get::<_,String>(0)?,"taskRevision":row.get::<_,i64>(1)?,"evidence":row.get::<_,String>(2)?,"report":row.get::<_,String>(3)?}))).optional().map_err(storage_error)
        }).transpose()?.flatten();
        value["taskCompletion"] = canonical_completion.clone().unwrap_or(Value::Null);
        let legacy_publication = value["publicationState"]
            .as_str()
            .unwrap_or("not_requested")
            .to_owned();
        let canonical_completed = canonical_completion.is_some();
        let publication = if canonical_completed {
            match value["latestDraft"]["state"].as_str() {
                Some("draft") => "draft_saved",
                Some("published") => "published",
                Some("withdrawn") => "deferred",
                _ if legacy_publication == "deferred" => "deferred",
                _ => "offered",
            }
        } else if task.is_some() {
            // A reopened/noncompleted Task cannot inherit an old session offer. A historical
            // canonical draft may still be withdrawn, but a new draft needs a new completion.
            match value["latestDraft"]["state"].as_str() {
                Some("published") => "published",
                Some("withdrawn") => "deferred",
                Some("draft") => "draft_saved",
                _ if legacy_publication == "deferred" => "deferred",
                _ => "not_requested",
            }
        } else {
            legacy_publication.as_str()
        };
        value["publicationState"] = json!(publication);
        value["nextActions"] = if canonical_completed && publication == "offered" {
            json!(["offer_knowledge_publication", "reopen_task"])
        } else if canonical_completed && publication == "draft_saved" {
            json!(["review_knowledge_draft", "reopen_task"])
        } else if canonical_completed && publication == "published" {
            json!(["review_publication_withdrawal", "reopen_task"])
        } else if canonical_completed {
            json!(["reopen_task"])
        } else if task.as_ref().is_some_and(|t| t["state"] == "task") {
            json!(["transition_task", "accept_checkpoint"])
        } else if task.is_some() && publication == "published" {
            json!(["review_publication_withdrawal"])
        } else if task.is_some() {
            json!(["accept_checkpoint", "complete_task"])
        } else if value["state"] == "completed" {
            // Session-only history is read-only after migration.
            json!([])
        } else {
            json!(["create_task"])
        };
        Ok(value)
    }

    pub fn select_work(&self, input: &Value) -> Result<Value, AppError> {
        let kind = require_text(input, "entityType", 40)?;
        let id = require_text(input, "entityId", 80)?;
        if kind == "tasks" {
            let connection = database::open(&self.path).map_err(storage_error)?;
            let selected = Self::link_snapshot(&connection, input)?;
            connection.execute("UPDATE work_tracking_workspace SET selection_json=?,revision=revision+1 WHERE id=1",[selected.to_string()]).map_err(storage_error)?;
            return Ok(selected);
        }
        let field = match kind {
            "captures" => "text",
            "problems" => "statement",
            "features" => "title",
            _ => return Err(AppError::new("invalid_input", "Unsupported selection")),
        };
        let connection = database::open(&self.path).map_err(storage_error)?;
        let title: String = connection
            .query_row(
                &format!("SELECT {field} FROM {kind} WHERE id=?"),
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new("not_found_or_not_visible", "Workbench item is unavailable")
            })?;
        let selected = json!({"entityType":kind,"entityId":id,"title":title});
        connection.execute("UPDATE work_tracking_workspace SET selection_json=?,revision=revision+1 WHERE id=1",[selected.to_string()]).map_err(storage_error)?;
        Ok(selected)
    }

    pub fn task_assistance_review_target(
        &self,
        connection_id: &str,
        task_id: &str,
    ) -> Result<Value, AppError> {
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection.transaction().map_err(storage_error)?;
        Self::task_continuation_access_on(&tx, connection_id, task_id)?;
        let snapshot = Self::task_continuation_snapshot(&tx, connection_id, task_id)?;
        tx.commit().map_err(storage_error)?;
        Ok(snapshot)
    }

    /// One scoped read snapshot. Binary attachment contents remain local; text and child rows
    /// are bounded for Chat, while sourceHash covers the complete material before truncation.
    pub fn task_context_read(&self, connection_id: &str, task_id: &str) -> Result<Value, AppError> {
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection.transaction().map_err(storage_error)?;
        Self::task_continuation_access_on(&tx, connection_id, task_id)?;
        let mut snapshot = Self::task_continuation_snapshot(&tx, connection_id, task_id)?;
        let source_hash = hash_text(&snapshot.to_string());
        let readiness =
            crate::application::task_service::TaskApplicationService::readiness_on(&tx, task_id)
                .map_err(storage_error)?;
        let mut truncated = false;
        fn bound(value: &mut Value, truncated: &mut bool) {
            match value {
                Value::String(text) if text.chars().count() > 4000 => {
                    *text = text.chars().take(4000).collect();
                    *truncated = true;
                }
                Value::Array(rows) => {
                    if rows.len() > 100 {
                        rows.truncate(100);
                        *truncated = true;
                    }
                    for row in rows {
                        bound(row, truncated);
                    }
                }
                Value::Object(object) => {
                    object.remove("imageData");
                    object.remove("data");
                    for item in object.values_mut() {
                        bound(item, truncated);
                    }
                }
                _ => {}
            }
        }
        // Draft lineage is retained in the lineage tool rather than copied into Task context.
        if let Some(rows) = snapshot["knowledge"].as_array_mut() {
            for row in rows {
                if let Some(object) = row.as_object_mut() {
                    object.remove("lineage");
                }
            }
        }
        snapshot["readiness"] = readiness;
        bound(&mut snapshot, &mut truncated);
        snapshot["sourceHash"] = Value::String(source_hash);
        snapshot["truncated"] = Value::Bool(truncated);
        tx.commit().map_err(storage_error)?;
        Ok(snapshot)
    }

    /// Store the external request separately from internal, immutable material.  The model sees
    /// only a bounded preview, while acceptance compares the complete child-aware snapshot.
    pub fn begin_task_assistance_review(
        &self,
        owner: &str,
        action: &str,
        input: &Value,
        target: &Value,
        prepared: &Value,
    ) -> Result<Value, AppError> {
        self.connection_scopes(owner)?;
        let operation = require_text(input, "operationId", 120)?;
        let request_hash = hash_text(&input.to_string());
        let envelope = json!({"request":input,"target":target,"prepared":prepared});
        if envelope.to_string().len() > 256 * 1024 {
            return Err(AppError::new(
                "invalid_input",
                "Review content exceeds the bounded payload limit",
            ));
        }
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let existing = tx.query_row("SELECT id,payload_hash,state,expires_at FROM work_tracking_reviews WHERE connection_id=? AND operation_id=? AND action=?",params![owner,operation,action],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?))).optional().map_err(storage_error)?;
        let id = if let Some((id, hash, state, expiry)) = existing {
            if hash != request_hash {
                return Err(AppError::new(
                    "idempotency_conflict",
                    "Reviewed content changed",
                ));
            }
            if state != "pending" || expiry < now() {
                return Err(AppError::new(
                    "challenge_replayed",
                    "Create a fresh review operation",
                ));
            }
            id
        } else {
            let id = Uuid::new_v4().to_string();
            let expiry = (Utc::now() + chrono::Duration::minutes(10))
                .to_rfc3339_opts(SecondsFormat::Nanos, true);
            tx.execute("INSERT INTO work_tracking_reviews(id,connection_id,operation_id,action,payload_hash,payload_json,expires_at,created_at) VALUES (?,?,?,?,?,?,?,?)",params![id,owner,operation,action,request_hash,envelope.to_string(),expiry,now()]).map_err(storage_error)?;
            id
        };
        tx.commit().map_err(storage_error)?;
        Ok(
            json!({"decisionRequired":true,"reviewState":id,"preview":{"request":input,"target":Self::task_continuation_preview(target),"prepared":prepared}}),
        )
    }

    pub fn consume_task_assistance_review(
        &self,
        owner: &str,
        action: &str,
        input: &Value,
        id: &str,
        decision: &str,
    ) -> Result<Option<Value>, AppError> {
        self.consume_task_assistance_review_with(
            owner,
            action,
            input,
            id,
            decision,
            |_, envelope| Ok(envelope.clone()),
        )
    }

    pub(crate) fn consume_task_assistance_review_with(
        &self,
        owner: &str,
        action: &str,
        input: &Value,
        id: &str,
        decision: &str,
        persist: impl FnOnce(&rusqlite::Transaction<'_>, &Value) -> Result<Value, AppError>,
    ) -> Result<Option<Value>, AppError> {
        if !matches!(decision, "accept" | "reject" | "cancel") {
            return Err(AppError::new("invalid_input", "Invalid review decision"));
        }
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let (hash, state, expiry, stored): (String,String,String,String) = tx.query_row("SELECT payload_hash,state,expires_at,payload_json FROM work_tracking_reviews WHERE id=? AND connection_id=? AND action=?",params![id,owner,action],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("invalid_input", "Review is unavailable"))?;
        if hash != hash_text(&input.to_string()) {
            return Err(AppError::new("invalid_input", "Reviewed payload changed"));
        }
        if state != "pending" {
            return Err(AppError::new(
                "challenge_replayed",
                "Review was already decided",
            ));
        }
        if expiry < now() {
            return Err(AppError::new("challenge_expired", "Review expired"));
        }
        let envelope: Value = serde_json::from_str(&stored).map_err(storage_error)?;
        let required_scope = match action {
            "task_knowledge_publish" | "task_knowledge_withdraw" => "knowledge:publish",
            "task_knowledge_draft" | "task_knowledge_correction" | "task_knowledge_regenerate" => {
                "knowledge:draft:write"
            }
            _ => "session:write",
        };
        Self::require_scope_on(&tx, owner, required_scope)?;
        if decision == "accept" {
            let task_id = envelope["target"]["task"]["taskId"]
                .as_str()
                .ok_or_else(|| AppError::new("invalid_input", "Review Task is unavailable"))?;
            Self::task_continuation_access_on(&tx, owner, task_id)?;
            let target = Self::task_continuation_snapshot(&tx, owner, task_id)?;
            if envelope["target"] != target {
                return Err(AppError::new(
                    "head_conflict",
                    "The reviewed Task material changed; review again",
                ));
            }
        }
        let result = if decision == "accept" {
            Some(persist(&tx, &envelope)?)
        } else {
            None
        };
        tx.execute(
            "UPDATE work_tracking_reviews SET state=? WHERE id=?",
            params![decision, id],
        )
        .map_err(storage_error)?;
        if decision == "accept"
            && matches!(action, "task_knowledge_publish" | "task_knowledge_withdraw")
        {
            tx.execute(
                "INSERT OR IGNORE INTO work_tracking_publication_jobs(review_id) VALUES(?)",
                [id],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)?;
        Ok(result)
    }

    pub fn publication_recovery(&self) -> Result<Vec<(String, String, Value)>, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT r.id,r.connection_id,r.payload_json FROM work_tracking_publication_jobs j JOIN work_tracking_reviews r ON r.id=j.review_id JOIN mcp_connections c ON c.id=r.connection_id WHERE j.state='pending' AND r.state='accept' AND c.state='active' AND (j.next_attempt_at IS NULL OR j.next_attempt_at<=?) ORDER BY r.created_at LIMIT 5").map_err(storage_error)?;
        let rows = statement
            .query_map([now()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        rows.into_iter()
            .map(|(id, owner, payload)| {
                let payload: Value = serde_json::from_str(&payload).map_err(storage_error)?;
                // Canonical Task publication reviews persist an internal envelope.
                // Recovery must replay only the originally reviewed request, never
                // the envelope itself or caller-supplied replacement fields.
                let payload = payload.get("request").cloned().unwrap_or(payload);
                Ok((id, owner, payload))
            })
            .collect()
    }

    pub fn publication_job_result(&self, id: &str, error: Option<&str>) -> Result<(), AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let next = (Utc::now() + chrono::Duration::seconds(30))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
        connection.execute("UPDATE work_tracking_publication_jobs SET attempts=attempts+1,state=CASE WHEN ? IS NULL THEN 'complete' WHEN attempts>=4 THEN 'failed' ELSE 'pending' END,next_attempt_at=?,safe_error_code=? WHERE review_id=?",params![error,next,error,id]).map_err(storage_error)?;
        Ok(())
    }

    pub fn finalize_withdrawal(
        &self,
        review: &str,
        draft: &str,
        revision: i64,
        recovery: &str,
    ) -> Result<Value, AppError> {
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        tx.execute("INSERT OR IGNORE INTO knowledge_publication_undos(review_id,draft_id,draft_revision,recovery_path,created_at) VALUES(?,?,?,?,?)",params![review,draft,revision,recovery,now()]).map_err(storage_error)?;
        tx.execute(
            "UPDATE knowledge_drafts SET state='withdrawn' WHERE id=? AND revision=?",
            params![draft, revision],
        )
        .map_err(storage_error)?;
        tx.execute("UPDATE work_tracking_sessions SET publication_state='deferred' WHERE id=(SELECT session_id FROM knowledge_drafts WHERE id=? AND revision=?)",params![draft,revision]).map_err(storage_error)?;
        tx.execute(
            "UPDATE work_tracking_publication_jobs SET state='complete' WHERE review_id=?",
            [review],
        )
        .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(
            json!({"draftId":draft,"draftRevision":revision,"status":"withdrawn","recoveryReference":recovery}),
        )
    }

    pub fn review_action(&self, id: &str) -> Result<String, AppError> {
        database::open(&self.path)
            .map_err(storage_error)?
            .query_row(
                "SELECT action FROM work_tracking_reviews WHERE id=?",
                [id],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }

    pub fn knowledge_draft(
        &self,
        connection_id: &str,
        draft_id: &str,
        revision: i64,
        hash: &str,
    ) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        connection.query_row("SELECT d.session_id,d.title,d.summary,d.body_markdown,d.content_hash,d.state,d.published_path,d.completion_event_id,d.evidence_refs_json FROM knowledge_drafts d JOIN work_tracking_sessions s ON s.id=d.session_id WHERE d.id=?1 AND d.revision=?2 AND d.revision=(SELECT MAX(latest.revision) FROM knowledge_drafts latest WHERE latest.id=d.id) AND (s.connection_id=?3 OR ?3='native-in-app-chat')",params![draft_id,revision,connection_id],|row|Ok(json!({"sessionId":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"summary":row.get::<_,String>(2)?,"bodyMarkdown":row.get::<_,String>(3)?,"contentHash":row.get::<_,String>(4)?,"state":row.get::<_,String>(5)?,"publishedPath":row.get::<_,Option<String>>(6)?,"completionEventId":row.get::<_,String>(7)?,"evidenceRefs":serde_json::from_str::<Value>(&row.get::<_,String>(8)?).unwrap_or(json!([]))}))).optional().map_err(storage_error)?.filter(|value|value["contentHash"]==hash).ok_or_else(||AppError::new("publish_conflict","The reviewed draft revision or hash changed"))
    }

    pub fn defer_publication(
        &self,
        connection_id: &str,
        session_id: &str,
        completion_revision: i64,
    ) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let timestamp = now();
        let changed=connection.execute("UPDATE work_tracking_sessions SET publication_state='deferred',updated_at=?1 WHERE id=?2 AND (connection_id=?3 OR ?3='native-in-app-chat') AND state='completed' AND publication_offer_revision=?4 AND publication_state IN ('offered','deferred')",params![timestamp,session_id,connection_id,completion_revision]).map_err(storage_error)?;
        if changed == 0 {
            return Err(AppError::new(
                "head_conflict",
                "Completion or publication state changed; refresh first",
            ));
        }
        Ok(
            json!({"sessionId":session_id,"completionRevision":completion_revision,"publicationState":"deferred"}),
        )
    }

    pub fn finalize_publication(
        &self,
        draft_id: &str,
        revision: i64,
        hash: &str,
        path: &str,
    ) -> Result<Value, AppError> {
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let existing=tx.query_row("SELECT published_path FROM knowledge_publication_decisions WHERE draft_id=? AND draft_revision=?",params![draft_id,revision],|row|row.get::<_,String>(0)).optional().map_err(storage_error)?;
        if let Some(existing) = existing {
            tx.commit().map_err(storage_error)?;
            return Ok(
                json!({"draftId":draft_id,"draftRevision":revision,"contentHash":hash,"status":"published","path":existing,"deduplicated":true}),
            );
        }
        let session_id:String=tx.query_row("SELECT session_id FROM knowledge_drafts WHERE id=? AND revision=? AND content_hash=?",params![draft_id,revision,hash],|row|row.get(0)).map_err(storage_error)?;
        let timestamp = now();
        tx.execute("INSERT INTO knowledge_publication_decisions(id,draft_id,draft_revision,content_hash,decision_channel,published_path,created_at) VALUES (?,?,?,?,'explicit_publish',?,?)",params![Uuid::new_v4().to_string(),draft_id,revision,hash,path,timestamp]).map_err(storage_error)?;
        tx.execute("UPDATE knowledge_drafts SET state='published',published_path=? WHERE id=? AND revision=?",params![path,draft_id,revision]).map_err(storage_error)?;
        tx.execute("UPDATE work_tracking_sessions SET publication_state='published',updated_at=? WHERE id=?",params![timestamp,session_id]).map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(
            json!({"draftId":draft_id,"draftRevision":revision,"contentHash":hash,"status":"published","path":path,"deduplicated":false}),
        )
    }

    fn project_one(
        &self,
        connection: &mut Connection,
        wake_dependencies: bool,
    ) -> Result<bool, AppError> {
        let worker = Uuid::new_v4().to_string();
        let lease_until = (Utc::now() + chrono::Duration::seconds(30))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
        let timestamp = now();
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        transaction.execute("UPDATE work_tracking_projection_jobs SET state='failed',lease_owner=NULL,lease_expires_at=NULL,safe_error_code='retry_exhausted',updated_at=? WHERE state='claimed' AND lease_expires_at<? AND attempts>=5",params![timestamp,timestamp]).map_err(storage_error)?;
        // Waiting for a Solution is a workflow dependency, not a failed attempt.
        // Wake these events only after a link exists, without spinning/retrying.
        if wake_dependencies {
            transaction.execute("UPDATE work_tracking_projection_jobs SET state='pending',attempts=0,updated_at=? WHERE state='waiting_solution' AND EXISTS (SELECT 1 FROM work_tracking_events e JOIN work_tracking_links l ON l.session_id=e.session_id WHERE e.id=work_tracking_projection_jobs.event_id AND l.relationship='adopted_task')", [&timestamp]).map_err(storage_error)?;
        }
        let event_id = transaction.query_row(
            "SELECT event_id FROM work_tracking_projection_jobs WHERE state IN ('pending','claimed') AND (state='pending' OR lease_expires_at<?) AND attempts<5 AND (next_attempt_at IS NULL OR next_attempt_at<=?) ORDER BY created_at LIMIT 1",
            params![timestamp,timestamp], |row| row.get::<_,String>(0),
        ).optional().map_err(storage_error)?;
        let Some(event_id) = event_id else {
            transaction.commit().map_err(storage_error)?;
            return Ok(false);
        };
        let changed=transaction.execute("UPDATE work_tracking_projection_jobs SET state='claimed',lease_owner=?,lease_expires_at=?,attempts=attempts+1,updated_at=? WHERE event_id=? AND (state='pending' OR (state='claimed' AND lease_expires_at<?))",
            params![worker,lease_until,timestamp,event_id,timestamp]).map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        if changed == 0 {
            return Ok(true);
        }

        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let owned:bool=transaction.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_projection_jobs WHERE event_id=? AND state='claimed' AND lease_owner=?)",params![event_id,worker],|row|row.get(0)).map_err(storage_error)?;
        if !owned {
            return Ok(true);
        }
        let (session_id,stream_id,source_sequence,kind,payload,occurred_at) = transaction.query_row(
            "SELECT session_id,stream_id,source_sequence,kind,payload_json,occurred_at FROM work_tracking_events WHERE id=?",
            [&event_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,i64>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?)),
        ).map_err(storage_error)?;
        let watermark=transaction.query_row("SELECT last_occurred_at,last_source_sequence,last_event_id FROM work_tracking_stream_watermarks WHERE session_id=? AND stream_id=? AND projection_name='workflow'",
            params![session_id,stream_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?))).optional().map_err(storage_error)?;
        let incoming = EventOrderKey {
            occurred_at: occurred_at.clone(),
            source_sequence,
            event_id: event_id.clone(),
        };
        let already_ordered: bool = kind == "work_log_checkpoint" && transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_tracking_projection_results WHERE event_id=? AND projection_name='workflow' AND state='waiting_solution')",
            [&event_id], |row| row.get(0),
        ).map_err(storage_error)?;
        let late = !already_ordered
            && watermark.as_ref().is_some_and(|(time, sequence, id)| {
                incoming
                    <= EventOrderKey {
                        occurred_at: time.clone(),
                        source_sequence: *sequence,
                        event_id: id.clone(),
                    }
            });
        let accepted:i64=transaction.query_row("SELECT count(*) FROM work_tracking_decisions WHERE event_id=? AND decision LIKE 'accepted%'",[&event_id],|row|row.get(0)).map_err(storage_error)?;
        let mut state = if late {
            "ignored_late"
        } else if kind != "capture" && accepted == 0 {
            "conflict"
        } else {
            "applied"
        };
        let mut result_entity_id: Option<String> = None;
        if state == "applied" && kind == "work_log_checkpoint" {
            let feature_id=transaction.query_row("SELECT entity_id FROM work_tracking_links WHERE session_id=? AND relationship='adopted_task' ORDER BY created_at DESC LIMIT 1",[&session_id],|row|row.get::<_,String>(0)).optional().map_err(storage_error)?;
            if let Some(feature_id) = feature_id {
                let progress_id = Uuid::new_v4().to_string();
                let body = serde_json::from_str::<Value>(&payload)
                    .ok()
                    .and_then(|v| v.get("summary").and_then(Value::as_str).map(str::to_owned))
                    .unwrap_or_else(|| "Checkpoint".into());
                let inserted=transaction.execute("INSERT OR IGNORE INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,created_at) VALUES (?,?,?,'task_work_log_entries',?,'accepted_progress',?)",params![Uuid::new_v4().to_string(),session_id,event_id,progress_id,timestamp]).map_err(storage_error)?;
                if inserted == 1 {
                    transaction.execute("INSERT OR IGNORE INTO task_work_log_entries(id,task_id,body,created_at) VALUES (?,?,?,?)",params![progress_id,feature_id,body,timestamp]).map_err(storage_error)?;
                    result_entity_id = Some(progress_id);
                }
            } else {
                state = "waiting_solution";
            }
        }
        if !already_ordered && (state == "applied" || state == "waiting_solution") {
            transaction.execute("INSERT INTO work_tracking_stream_watermarks(session_id,stream_id,projection_name,last_occurred_at,last_source_sequence,last_event_id,version,updated_at) VALUES (?,?,'workflow',?,?,?,1,?) ON CONFLICT(session_id,stream_id,projection_name) DO UPDATE SET last_occurred_at=excluded.last_occurred_at,last_source_sequence=excluded.last_source_sequence,last_event_id=excluded.last_event_id,version=work_tracking_stream_watermarks.version+1,updated_at=excluded.updated_at",
                params![session_id,stream_id,occurred_at,source_sequence,event_id,timestamp]).map_err(storage_error)?;
        }
        transaction.execute("INSERT INTO work_tracking_projection_results(event_id,projection_name,state,result_entity_id,updated_at) VALUES (?,'workflow',?,?,?) ON CONFLICT(event_id,projection_name) DO UPDATE SET state=excluded.state,result_entity_id=excluded.result_entity_id,updated_at=excluded.updated_at",params![event_id,state,result_entity_id,timestamp]).map_err(storage_error)?;
        transaction.execute("UPDATE work_tracking_projection_jobs SET state=?,lease_owner=NULL,lease_expires_at=NULL,safe_error_code=CASE WHEN ?='conflict' THEN 'review_required' ELSE NULL END,updated_at=? WHERE event_id=?",params![state,state,timestamp,event_id]).map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(true)
    }
}

impl EventLog for SqliteWorkTrackingStore {
    fn session(&self, connection_id: &str, session_id: &str) -> Result<Value, AppError> {
        self.session_value(connection_id, session_id)
    }

    fn drain_projection_jobs(&self, limit: usize) -> Result<usize, AppError> {
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let mut processed = 0;
        while processed < limit && self.project_one(&mut connection, processed == 0)? {
            processed += 1;
        }
        Ok(processed)
    }
}

impl WorkProjection for SqliteWorkTrackingStore {
    fn drain(&self, limit: usize) -> Result<usize, AppError> {
        EventLog::drain_projection_jobs(self, limit)
    }
}

impl WorkflowRepository for SqliteWorkTrackingStore {
    fn current_workbench(&self, connection_id: &str, limit: usize) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let connection = connection.unchecked_transaction().map_err(storage_error)?;
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM work_tracking_workspace WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT s.id,c.text,s.state,s.publication_state,s.head_revision,s.source_interface,s.updated_at FROM work_tracking_sessions s LEFT JOIN captures c ON c.id=s.capture_id WHERE s.connection_id=? OR ?='native-in-app-chat' ORDER BY s.updated_at DESC LIMIT ?").map_err(storage_error)?;
        let mut rows=statement.query_map(params![connection_id,connection_id,limit as i64],|row|Ok(json!({"trackedSessionId":row.get::<_,String>(0)?,"capture":row.get::<_,Option<String>>(1)?,"state":row.get::<_,String>(2)?,"publicationState":row.get::<_,String>(3)?,"headRevision":row.get::<_,i64>(4)?,"sourceInterface":row.get::<_,String>(5)?,"updatedAt":row.get::<_,String>(6)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        for row in &mut rows {
            let session = Self::session_value_on(
                &connection,
                connection_id,
                row["trackedSessionId"].as_str().unwrap_or_default(),
            )?;
            row["problem"] = session["linkedWorkflow"]["problem"].clone();
            row["solution"] = session["linkedWorkflow"]["solution"].clone();
            row["task"] = session["linkedWorkflow"]["task"].clone();
            row["recentProgress"] = json!(session["recentEvents"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|event| event["kind"] == "work_log_checkpoint")
                .take(2)
                .map(|event|json!({"eventId":event["eventId"],"occurredAt":event["occurredAt"],"projectionStatus":event["projectionStatus"],"payload":{"summary":event["payload"]["summary"].as_str().unwrap_or_default().chars().take(500).collect::<String>()}}))
                .collect::<Vec<_>>());
            row["recentDecisions"] = session["recentDecisions"].clone();
            row["nextDecision"] = session["nextActions"].clone();
        }
        // Desktop-created Tasks are canonical work even before any chat session binds them.
        // Keep them sessionless here; discovery must not manufacture a Capture or tracking event.
        if rows.len() < limit {
            let remaining = (limit - rows.len()) as i64;
            let mut tasks = connection.prepare("SELECT t.id,r.title,t.state,t.current_revision,t.last_user_activity_at FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE NOT EXISTS(SELECT 1 FROM work_tracking_links l JOIN work_tracking_sessions s ON s.id=l.session_id WHERE l.entity_type='tasks' AND l.entity_id=t.id AND s.state='active' AND (s.connection_id=? OR ?='native-in-app-chat')) ORDER BY t.last_user_activity_at DESC,t.id LIMIT ?").map_err(storage_error)?;
            let direct = tasks.query_map(params![connection_id,connection_id,remaining],|row|Ok(json!({"trackedSessionId":Value::Null,"capture":Value::Null,"state":row.get::<_,String>(2)?,"publicationState":"not_requested","headRevision":Value::Null,"sourceInterface":"desktop","updatedAt":row.get::<_,String>(4)?,"task":{"id":row.get::<_,String>(0)?,"taskId":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"taskRevision":row.get::<_,i64>(3)?},"problem":Value::Null,"solution":Value::Null,"recentProgress":[],"recentDecisions":[],"nextDecision":["continue_task"]}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
            let mut direct = direct;
            for row in &mut direct {
                let task_id = row["task"]["id"].as_str().unwrap_or_default();
                let links: String = connection.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'problemId',problem_id,'problemRevision',problem_revision,'relationship',relationship,'note',note)),'[]') FROM (SELECT * FROM task_problem_links WHERE task_id=? AND unlinked_at IS NULL ORDER BY id)",[task_id],|record|record.get(0)).map_err(storage_error)?;
                let relationships: String = connection.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'sourceTaskId',source_task_id,'targetTaskId',target_task_id,'kind',kind,'note',note)),'[]') FROM (SELECT * FROM task_relationships WHERE (source_task_id=? OR target_task_id=?) AND unlinked_at IS NULL ORDER BY id)",params![task_id,task_id],|record|record.get(0)).map_err(storage_error)?;
                row["task"]["problemLinks"] =
                    serde_json::from_str(&links).unwrap_or_else(|_| json!([]));
                row["task"]["relationships"] =
                    serde_json::from_str(&relationships).unwrap_or_else(|_| json!([]));
            }
            rows.extend(direct);
        }
        let selection: Option<String> = connection
            .query_row(
                "SELECT selection_json FROM work_tracking_workspace WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let selection = selection
            .and_then(|value| serde_json::from_str::<Value>(&value).ok())
            .and_then(|mut selected| {
                let id = selected["entityId"].as_str()?;
                let title = match selected["entityType"].as_str()? {
                    "tasks" => connection
                        .query_row(
                            "SELECT r.title FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=?",
                            [id],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()
                        .ok()??,
                    "captures" => connection
                        .query_row("SELECT text FROM captures WHERE id=?", [id], |row| row.get::<_, String>(0))
                        .optional()
                        .ok()??,
                    "problems" => connection
                        .query_row("SELECT statement FROM problems WHERE id=?", [id], |row| row.get::<_, String>(0))
                        .optional()
                        .ok()??,
                    "features" => connection
                        .query_row("SELECT title FROM features WHERE id=?", [id], |row| row.get::<_, String>(0))
                        .optional()
                        .ok()??,
                    _ => return None,
                };
                selected["title"] = json!(title);
                Some(selected)
            });
        Ok(
            json!({"workspaceRevision":revision,"activeSelection":selection,"activeWork":rows,"ttlMs":0,"cacheScope":"private"}),
        )
    }

    fn overview(
        &self,
        connection_id: &str,
        limit: usize,
        offset: usize,
        attention_offset: usize,
    ) -> Result<Value, AppError> {
        let board = crate::application::task_service::TaskApplicationService::new(&self.path)
            .execute("workbench.get", &json!({}))
            .map_err(storage_error)?;
        let mut items: Vec<Value> = board["categories"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|category| {
                category["items"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(move |item| {
                        let title = item["title"]
                            .as_str()
                            .or_else(|| item["text"].as_str())
                            .unwrap_or_default();
                        json!({"entityRef":item["id"],"kind":item["kind"],
                    "title":bounded_overview_text(title,OVERVIEW_TEXT_LIMIT),
                    "state":item["state"],"taskRevision":item["taskRevision"],
                    "category":category["label"],"readiness":item["readiness"],
                    "updatedAt":item["lastUserActivityAt"]})
                    })
            })
            .collect();
        // Include the canonical Task dependencies in the public snapshot.  Task content
        // revision alone is not enough for a caller to decide whether a link/review is stale.
        let connection = database::open(&self.path).map_err(storage_error)?;
        let snapshot = connection.unchecked_transaction().map_err(storage_error)?;
        for item in &mut items {
            if item["kind"] != "task" {
                continue;
            }
            let Some(task_id) = item["entityRef"].as_str() else {
                continue;
            };
            let links: String = snapshot.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'problemId',problem_id,'problemRevision',problem_revision,'relationship',relationship,'note',note)),'[]') FROM (SELECT * FROM task_problem_links WHERE task_id=? AND unlinked_at IS NULL ORDER BY id)",[task_id],|row|row.get(0)).map_err(storage_error)?;
            let relationships: String = snapshot.query_row("SELECT COALESCE(json_group_array(json_object('id',id,'sourceTaskId',source_task_id,'targetTaskId',target_task_id,'kind',kind,'note',note)),'[]') FROM (SELECT * FROM task_relationships WHERE (source_task_id=? OR target_task_id=?) AND unlinked_at IS NULL ORDER BY id)",params![task_id,task_id],|row|row.get(0)).map_err(storage_error)?;
            let work_log: i64 = snapshot
                .query_row(
                    "SELECT count(*) FROM task_work_log_entries WHERE task_id=?",
                    [task_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            let checklist: i64 = snapshot
                .query_row(
                    "SELECT count(*) FROM task_checklist_items WHERE task_id=?",
                    [task_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            let decisions: i64 = snapshot
                .query_row(
                    "SELECT count(*) FROM task_decisions WHERE task_id=?",
                    [task_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            let knowledge: Value = snapshot.query_row("SELECT revision,state,content_hash,lineage_json FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision DESC LIMIT 1",[task_id],|row|Ok(json!({"revision":row.get::<_,i64>(0)?,"state":row.get::<_,String>(1)?,"contentHash":row.get::<_,String>(2)?,"lineageHash":hash_text(&row.get::<_,String>(3)?)}))).optional().map_err(storage_error)?.unwrap_or(Value::Null);
            item["problemLinks"] = serde_json::from_str(&links).unwrap_or_else(|_| json!([]));
            item["relationships"] =
                serde_json::from_str(&relationships).unwrap_or_else(|_| json!([]));
            item["activity"] = json!({"workLogCount":work_log,"checklistCount":checklist,"decisionCount":decisions});
            item["knowledge"] = knowledge;
        }
        items.sort_by(|a, b| {
            b["updatedAt"]
                .as_str()
                .cmp(&a["updatedAt"].as_str())
                .then_with(|| a["entityRef"].as_str().cmp(&b["entityRef"].as_str()))
        });
        let total = items.len();
        let captures = items.iter().filter(|x| x["kind"] == "capture").count();
        let tasks = items.iter().filter(|x| x["kind"] == "task").count();
        let in_progress = items.iter().filter(|x| x["state"] == "in_progress").count();
        let completed = items.iter().filter(|x| x["state"] == "completed").count();
        let recently_completed: Vec<_> = items
            .iter()
            .filter(|x| x["state"] == "completed")
            .take(10)
            .cloned()
            .collect();
        let mut statement = snapshot.prepare("SELECT e.id,e.session_id,e.kind,e.payload_json,e.ingested_at FROM work_tracking_events e JOIN work_tracking_sessions s ON s.id=e.session_id WHERE (s.connection_id=? OR ?='native-in-app-chat') AND e.kind IN ('task_created','task_revision_proposed','task_transition_proposed','problem_resolution_proposed','completion_proposal') AND NOT EXISTS(SELECT 1 FROM work_tracking_decisions d WHERE d.event_id=e.id) AND NOT EXISTS(SELECT 1 FROM work_tracking_events x WHERE x.supersedes_event_id=e.id) ORDER BY e.ingested_at DESC,e.id").map_err(storage_error)?;
        let attention=statement.query_map(params![connection_id,connection_id],|row|Ok(json!({"eventId":row.get::<_,String>(0)?,"sessionId":row.get::<_,String>(1)?,"kind":row.get::<_,String>(2)?,"summary":bounded_overview_text(&row.get::<_,String>(3)?,OVERVIEW_TEXT_LIMIT),"createdAt":row.get::<_,String>(4)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        let attention_total = attention.len();
        let attention_page: Vec<_> = attention
            .iter()
            .skip(attention_offset)
            .take(OVERVIEW_ATTENTION_LIMIT)
            .cloned()
            .collect();
        let next_attention = if attention_offset + attention_page.len() < attention_total {
            Some(attention_offset + attention_page.len())
        } else {
            None
        };
        let digest = Sha256::digest(
            json!({"board":board,"attention":attention})
                .to_string()
                .as_bytes(),
        );
        let revision =
            i64::from_be_bytes(digest[..8].try_into().expect("hash length")) & ((1_i64 << 53) - 1);
        let page: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
        let next = if offset + page.len() < total {
            Some(offset + page.len())
        } else {
            None
        };
        Ok(
            json!({"snapshotRevision":revision,"summary":{"captures":captures,"tasks":tasks,"inProgressTasks":in_progress,"completedTasks":completed,"total":total},
            "activeShortcuts":board["activeShortcuts"],"refiningShortcuts":board["refiningShortcuts"],
            "items":page,"attention":attention_page,"attentionTotal":attention_total,"attentionTruncated":next_attention.is_some(),"recentlyCompleted":recently_completed,
            "nextOffset":next,"nextAttentionOffset":next_attention,"ttlMs":15000,"cacheScope":"private"}),
        )
    }

    fn topic(&self, topic_id: &str, limit: usize) -> Result<Value, AppError> {
        if topic_id.trim().is_empty() || topic_id.len() > 120 {
            return Err(AppError::new(
                "invalid_input",
                "topicId must be 1–120 characters",
            ));
        }
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT o.entity_type,o.entity_id,CASE o.entity_type WHEN 'problems' THEN (SELECT statement FROM problems WHERE id=o.entity_id) WHEN 'tasks' THEN (SELECT r.title FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=o.entity_id) WHEN 'features' THEN (SELECT title FROM features WHERE id=o.entity_id) WHEN 'vault' THEN (SELECT title FROM vault_documents WHERE path=o.entity_id) ELSE (SELECT text FROM captures WHERE id=o.entity_id) END FROM work_tracking_topic_memberships o WHERE o.topic_id=? ORDER BY o.entity_type,o.entity_id LIMIT ?").map_err(storage_error)?;
        let mut items=statement.query_map(params![topic_id,limit as i64+1],|row|Ok(json!({"kind":row.get::<_,String>(0)?,"entityRef":row.get::<_,String>(1)?,"title":row.get::<_,Option<String>>(2)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        let truncated = items.len() > limit;
        items.truncate(limit);
        Ok(
            json!({"topicId":topic_id,"items":items,"truncated":truncated,"ttlMs":15000,"cacheScope":"private"}),
        )
    }
}
pub mod task_repository;

#[cfg(test)]
mod challenge_idempotency_tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture() -> (tempfile::TempDir, SqliteWorkTrackingStore, String, String) {
        let root = tempdir().unwrap();
        let path = root.path().join("state.sqlite3");
        database::initialize(&path).unwrap();
        let connection_id = "connection-1".to_owned();
        let session_id = "session-1".to_owned();
        let event_id = "event-1".to_owned();
        let capture_id = "capture-1";
        let timestamp = now();
        let connection = database::open(&path).unwrap();
        connection
            .execute(
                "INSERT INTO mcp_connections(id,name,scopes_json,allowed_topics_json,checkpoint_policy,state,created_at,updated_at) VALUES(?,?,?,?,'allowed_for_started_sessions','active',?,?)",
                params![connection_id, "Challenge test", r#"["session:write","workbench:current:read"]"#, "[]", timestamp, timestamp],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,'challenge capture',?,'capture',?)",
                params![capture_id, timestamp, timestamp],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,state,publication_state,created_at,updated_at) VALUES(?,?,?,'conversation',?,?,1,'active','not_requested',?,?)",
                params![session_id, connection_id, "external_mcp_chat", capture_id, event_id, timestamp, timestamp],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) VALUES(?,?,1,'mcp:connection-1',1,'task_created','{}',?, ?,?)",
                params![event_id, session_id, hash_text("{}"), timestamp, timestamp],
            )
            .unwrap();
        (
            root,
            SqliteWorkTrackingStore::new(path),
            connection_id,
            session_id,
        )
    }

    fn create_task_input(operation_id: &str, session_id: &str) -> Value {
        json!({
            "operationId":operation_id,
            "sessionId":session_id,
            "expectedHeadRevision":1,
            "sourceEventId":"event-1",
            "action":"task.create",
            "proposedPayload":{
                "title":"Idempotent Task",
                "outcome":"One durable Task",
                "scope":"Focused fixture",
                "validationCriteria":"Stored once"
            }
        })
    }

    #[test]
    fn advance_challenge_replays_same_operation_and_replaces_it_with_the_final_acceptance() {
        let (_root, store, connection_id, session_id) = fixture();
        let input = create_task_input("advance-1", &session_id);

        let review_state = store.create_challenge(&connection_id, &input).unwrap();
        assert_eq!(
            store.create_challenge(&connection_id, &input).unwrap(),
            review_state
        );
        let changed = json!({
            "operationId":"advance-1",
            "sessionId":session_id,
            "expectedHeadRevision":1,
            "sourceEventId":"event-1",
            "action":"task.create",
            "proposedPayload":{"title":"Different payload","outcome":"One durable Task","scope":"Focused fixture","validationCriteria":"Stored once"}
        });
        assert_eq!(
            store
                .create_challenge(&connection_id, &changed)
                .unwrap_err()
                .code,
            "idempotency_conflict"
        );

        let accepted = store
            .consume_challenge(&connection_id, &review_state, &input, "accept")
            .unwrap();
        assert_eq!(accepted["decision"], "accept");
        assert_eq!(accepted["reviewState"], review_state);
        assert_eq!(accepted["deduplicated"], false);
        let replayed = store
            .consume_challenge(&connection_id, &review_state, &input, "accept")
            .unwrap();
        assert_eq!(replayed["deduplicated"], true);
        let task_count: i64 = database::open(&store.path)
            .unwrap()
            .query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(task_count, 1);
    }

    #[test]
    fn stale_acceptance_keeps_the_operation_pending_for_a_fresh_review() {
        let (_root, store, connection_id, session_id) = fixture();
        let input = create_task_input("advance-stale", &session_id);
        let review_state = store.create_challenge(&connection_id, &input).unwrap();
        database::open(&store.path)
            .unwrap()
            .execute(
                "UPDATE work_tracking_sessions SET head_revision=2 WHERE id=?",
                [session_id],
            )
            .unwrap();

        assert_eq!(
            store
                .consume_challenge(&connection_id, &review_state, &input, "accept")
                .unwrap_err()
                .code,
            "head_conflict"
        );
        let status: String = database::open(&store.path)
            .unwrap()
            .query_row(
                "SELECT status FROM mcp_elicitation_challenges WHERE id=?",
                [review_state],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "pending");
    }

    #[test]
    fn challenge_rejects_a_guessed_task_outside_the_connection_scope() {
        let (_root, store, connection_id, session_id) = fixture();
        let timestamp = now();
        let hidden_task = "hidden-task";
        let connection = database::open(&store.path).unwrap();
        connection
            .execute(
                "UPDATE mcp_connections SET scopes_json='[\"session:write\"]' WHERE id=?",
                [&connection_id],
            )
            .unwrap();
        connection.execute("INSERT INTO tasks(id,current_revision,state,category,created_at,last_user_activity_at) VALUES(?,1,'task','General',?,?)", params![hidden_task, timestamp, timestamp]).unwrap();
        connection.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,1,'Hidden','','','','','','hash',?)", params![hidden_task, timestamp]).unwrap();
        let input = json!({
            "operationId":"hidden-task-attempt", "sessionId":session_id,
            "expectedHeadRevision":1, "sourceEventId":"event-1", "action":"task.revision",
            "proposedPayload":{"taskId":hidden_task,"expectedTaskRevision":1,"title":"Guess","operationId":"ignored"}
        });
        assert_eq!(
            store
                .create_challenge(&connection_id, &input)
                .unwrap_err()
                .code,
            "not_found_or_not_visible"
        );
        let challenge_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM mcp_elicitation_challenges",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(challenge_count, 0);
    }

    #[test]
    fn accepted_task_reopen_reactivates_the_completed_session() {
        let (_root, store, connection_id, session_id) = fixture();
        let timestamp = now();
        let task_id = "completed-task";
        let connection = database::open(&store.path).unwrap();
        connection.execute("INSERT INTO tasks(id,current_revision,state,category,created_at,last_user_activity_at) VALUES(?,1,'completed','General',?,?)", params![task_id, timestamp, timestamp]).unwrap();
        connection.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,1,'Done','','','','','','hash',?)", params![task_id, timestamp]).unwrap();
        connection.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion-1',?,1,'evidence','report','fixture-complete',?)", params![task_id, timestamp]).unwrap();
        connection.execute("INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES('task-link',?,'event-1','tasks',?,'adopted_task',NULL,?)", params![session_id, task_id, timestamp]).unwrap();
        connection
            .execute(
                "UPDATE work_tracking_sessions SET state='completed' WHERE id=?",
                [&session_id],
            )
            .unwrap();
        let input = json!({"operationId":"reopen-1","sessionId":session_id,"expectedHeadRevision":1,"sourceEventId":"event-1","action":"task.reopen","proposedPayload":{"taskId":task_id,"expectedTaskRevision":1}});
        let review = store.create_challenge(&connection_id, &input).unwrap();
        store
            .consume_challenge(&connection_id, &review, &input, "accept")
            .unwrap();
        let state: String = connection
            .query_row("SELECT state FROM tasks WHERE id=?", [task_id], |row| {
                row.get(0)
            })
            .unwrap();
        let session_state: String = connection
            .query_row(
                "SELECT state FROM work_tracking_sessions WHERE id=?",
                [session_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "in_progress");
        assert_eq!(session_state, "active");
    }

    #[test]
    fn current_scope_allows_a_problem_linked_from_the_owned_task_but_not_a_hidden_problem() {
        let (_root, store, connection_id, session_id) = fixture();
        let timestamp = now();
        let connection = database::open(&store.path).unwrap();
        let task_id = "continued-task";
        let visible_problem = "linked-problem";
        let hidden_problem = "hidden-problem";
        connection.execute("INSERT INTO tasks(id,current_revision,state,category,created_at,last_user_activity_at) VALUES(?,1,'in_progress','General',?,?)", params![task_id, timestamp, timestamp]).unwrap();
        connection.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,1,'Continued','','','','','','hash',?)", params![task_id, timestamp]).unwrap();
        connection.execute("INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,decision_id,created_at) VALUES('continued-task-link',?,'event-1','tasks',?,'adopted_task',NULL,?)", params![session_id, task_id, timestamp]).unwrap();
        for problem in [visible_problem, hidden_problem] {
            connection.execute("INSERT INTO problems(id,capture_id,statement,detail,state,created_at,current_revision) VALUES(?,NULL,?,'','open',?,1)", params![problem, problem, timestamp]).unwrap();
            connection.execute("INSERT INTO problem_revisions(problem_id,revision,statement,detail,content_hash,created_at) VALUES(?,1,?,'','hash',?)", params![problem, problem, timestamp]).unwrap();
        }
        connection.execute("INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES('visible-link',?,?,1,'context','',?)", params![task_id, visible_problem, timestamp]).unwrap();
        let visible = json!({"operationId":"resolve-visible","sessionId":&session_id,"expectedHeadRevision":1,"sourceEventId":"event-1","action":"problem.resolution.create","proposedPayload":{"problemId":visible_problem,"expectedProblemRevision":1,"rationale":"Address the linked problem","evidenceRefs":[]}});
        let review = store.create_challenge(&connection_id, &visible).unwrap();
        store
            .consume_challenge(&connection_id, &review, &visible, "accept")
            .unwrap();
        let state: String = connection
            .query_row(
                "SELECT state FROM problems WHERE id=?",
                [visible_problem],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "resolved");
        let hidden = json!({"operationId":"resolve-hidden","sessionId":&session_id,"expectedHeadRevision":1,"sourceEventId":"event-1","action":"problem.resolution.create","proposedPayload":{"problemId":hidden_problem,"expectedProblemRevision":1,"rationale":"Guess","evidenceRefs":[]}});
        assert_eq!(
            store
                .create_challenge(&connection_id, &hidden)
                .unwrap_err()
                .code,
            "not_found_or_not_visible"
        );
    }
}

pub(crate) mod input_images;
