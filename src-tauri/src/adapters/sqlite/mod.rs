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
        "create_task" | "revise_task" | "transition_task" | "link_task" => Some("tasks"),
        "complete_task" => Some("task_completions"),
        "resolve_problem" => Some("problems"),
        "adopt_problem" | "approve_problem" => Some("problems"),
        "adopt_solution" | "resolve_conflict" | "approve_solution" => Some("features"),
        "accept_completion_proposal" => Some("completion_reviews"),
        "verify_and_complete" => Some("completions"),
        _ => None,
    }
}

impl SqliteWorkTrackingStore {
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
            let _=connection.execute("INSERT INTO work_tracking_activity_events(id,source_interface,connection_id,session_id,operation,outcome,safe_error_code,duration_ms,created_at) VALUES (?,?,?,?,?,?,?,?,?)",params![Uuid::new_v4().to_string(),source,connection_id,session_id,operation,outcome,error,duration_ms.min(i64::MAX as u128) as i64,now()]);
        }
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
    ) -> Result<Value, AppError> {
        self.connection_scopes(connection_id)?;
        let owner = hash_text(connection_id);
        let lineage_hash = hash_text(&format!("{connection_id}:{lineage_key}"));
        let request = json!({"lineageKey":lineage_key,"mode":mode,"capture":capture,"parentSessionId":parent_session_id,"reviewContext":review_context});
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
        self.connection_scopes(connection_id)?;
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
        transaction.commit().map_err(storage_error)?;
        Ok(response)
    }

    pub fn append_native_event(
        &self,
        operation_id: &str,
        conversation_key: &str,
        session_id: Option<&str>,
        expected_revision: Option<i64>,
        kind: EventKind,
        payload: &Value,
    ) -> Result<Value, AppError> {
        const NATIVE_CONNECTION: &str = "native-in-app-chat";
        self.ensure_native_connection()?;
        if let Some(session_id) = session_id {
            self.append_event(
                NATIVE_CONNECTION,
                operation_id,
                session_id,
                expected_revision.unwrap_or(0),
                kind,
                payload,
                None,
                None,
            )
        } else {
            self.open_session(
                NATIVE_CONNECTION,
                operation_id,
                conversation_key,
                "create",
                Some(payload),
                None,
                None,
                None,
            )
        }
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
        let session_id = require_text(input, "sessionId", 80)?;
        let source_event_id = require_text(input, "sourceEventId", 80)?;
        let action = require_text(input, "action", 80)?;
        let expected = input
            .get("expectedHeadRevision")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::new("invalid_input", "expectedHeadRevision is required"))?;
        let connection = database::open(&self.path).map_err(storage_error)?;
        let current: i64 = connection
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
        let exists: i64 = connection
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
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        let expiry = (Utc::now() + chrono::Duration::minutes(10))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
        let target = if action == "link_current_work" {
            Some(Self::link_snapshot(&connection, &input["proposedPayload"])?)
        } else {
            None
        };
        connection.execute("INSERT INTO mcp_elicitation_challenges(id,connection_id,session_id,action,source_event_id,proposed_payload_hash,expected_head_revision,nonce_hash,expires_at,created_at,target_snapshot_json) VALUES (?,?,?,?,?,?,?,?,?,?,?)",params![id,connection_id,session_id,action,source_event_id,hash_text(&input.to_string()),expected,hash_text(&Uuid::new_v4().to_string()),expiry,timestamp,target.map(|v|v.to_string())]).map_err(storage_error)?;
        Ok(id)
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

    pub fn review_target(&self, owner: &str, state: &str) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let target:Option<String>=connection.query_row("SELECT target_snapshot_json FROM mcp_elicitation_challenges WHERE id=? AND connection_id=?",params![state,owner],|row|row.get(0)).map_err(storage_error)?;
        target
            .map(|v| serde_json::from_str(&v).map_err(storage_error))
            .transpose()
            .map(|v| v.unwrap_or(Value::Null))
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
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
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
        if input_hash != hash_text(&input.to_string()) {
            return Err(AppError::new("invalid_input", "Reviewed payload changed"));
        }
        if decision == "accept" && action == "link_current_work" {
            let captured: String = tx
                .query_row(
                    "SELECT target_snapshot_json FROM mcp_elicitation_challenges WHERE id=?",
                    [challenge_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            if serde_json::from_str::<Value>(&captured).map_err(storage_error)?
                != Self::link_snapshot(&tx, &input["proposedPayload"])?
            {
                return Err(AppError::new(
                    "head_conflict",
                    "Workbench target changed; review the refreshed target",
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
        if accepted && closed {
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
            tx.execute("UPDATE work_tracking_decisions SET result_entity_type=?,result_entity_id=? WHERE id=?",params![action_result_type(&action),result_entity_id,decision_id]).map_err(storage_error)?;
        }
        tx.execute("UPDATE mcp_elicitation_challenges SET status=?,consumed_at=? WHERE id=? AND status='pending'",params![if accepted{"accepted"}else{"rejected"},timestamp,challenge_id]).map_err(storage_error)?;
        tx.execute("UPDATE work_tracking_projection_jobs SET state=?,safe_error_code=?,updated_at=? WHERE event_id=?",params![if accepted{"pending"}else{"conflict"},if accepted{None::<String>}else{Some("rejected".to_owned())},timestamp,event_id]).map_err(storage_error)?;
        let final_head: i64 = tx
            .query_row(
                "SELECT head_revision FROM work_tracking_sessions WHERE id=?",
                [&session_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(
            json!({"decisionId":decision_id,"sessionId":session_id,"sourceEventId":event_id,"action":action,"decision":decision,"resultEntityId":result_entity_id,"headRevision":final_head,"projectionStatus":if accepted{"queued"}else{"conflict"},"resourceUri":format!("llm-wiki://work-session/{session_id}")}),
        )
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
            "create_task" => {
                let capture: String = tx
                    .query_row(
                        "SELECT capture_id FROM work_tracking_sessions WHERE id=?",
                        [session_id],
                        |r| r.get(0),
                    )
                    .map_err(storage_error)?;
                let service =
                    crate::application::task_service::TaskApplicationService::new(&self.path);
                let task = service
                    .create_task_tx(tx, payload, Some(&capture), None, timestamp)
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
            "revise_task" | "transition_task" | "complete_task" | "resolve_problem" => {
                let mut input = payload.clone();
                input["operationId"] = json!(format!("review:{decision_id}"));
                if action != "resolve_problem" && input.get("taskId").is_none() {
                    input["taskId"] =
                        json!(linked("adopted_task")?.ok_or_else(|| AppError::new(
                            "workflow_precondition",
                            "Select a Task first"
                        ))?);
                }
                let name = match action {
                    "revise_task" => "task.revision",
                    "transition_task" => "task.transition",
                    "complete_task" => "task.completion.create",
                    _ => "problem.resolution.create",
                };
                let result =
                    crate::application::task_service::TaskApplicationService::new(&self.path)
                        .execute_tx(tx, name, &input)
                        .map_err(storage_error)?;
                if action == "complete_task" {
                    tx.execute("UPDATE work_tracking_sessions SET state='completed',publication_state='offered',publication_offer_revision=head_revision,updated_at=? WHERE id=?",params![timestamp,session_id]).map_err(storage_error)?;
                }
                Ok(result["id"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| input["taskId"].as_str().map(str::to_owned)))
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
            "SELECT s.id,s.capture_id,s.head_event_id,s.head_revision,s.state,s.publication_state,c.text,s.updated_at FROM work_tracking_sessions s JOIN captures c ON c.id=s.capture_id WHERE s.id=?1 AND (s.connection_id=?2 OR ?2='native-in-app-chat')",
            params![session_id,connection_id],|row|Ok(json!({"sessionId":row.get::<_,String>(0)?,"capture":{"id":row.get::<_,String>(1)?,"summary":row.get::<_,String>(6)?},"headEventId":row.get::<_,String>(2)?,"headRevision":row.get::<_,i64>(3)?,"state":row.get::<_,String>(4)?,"publicationState":row.get::<_,String>(5)?,"updatedAt":row.get::<_,String>(7)?})),
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
        let task = connection.query_row("SELECT t.id,r.title,t.state,t.current_revision,l.source_event_id FROM work_tracking_links l JOIN tasks t ON t.id=l.entity_id JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE l.session_id=? AND l.relationship='adopted_task' ORDER BY l.created_at DESC LIMIT 1",[session_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"taskRevision":row.get::<_,i64>(3)?,"sourceEventId":row.get::<_,String>(4)?}))).optional().map_err(storage_error)?;
        value["linkedWorkflow"] = json!({"problem":problem,"solution":solution,"task":task});
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
        value["latestDraft"]=connection.query_row("SELECT id,revision,content_hash,title,summary,state FROM knowledge_drafts WHERE session_id=? ORDER BY created_at DESC,revision DESC LIMIT 1",[session_id],|row|Ok(json!({"draftId":row.get::<_,String>(0)?,"draftRevision":row.get::<_,i64>(1)?,"contentHash":row.get::<_,String>(2)?,"title":row.get::<_,String>(3)?,"summary":row.get::<_,String>(4)?,"state":row.get::<_,String>(5)?}))).optional().map_err(storage_error)?.unwrap_or(Value::Null);
        let state = value
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("active");
        let publication = value
            .get("publicationState")
            .and_then(Value::as_str)
            .unwrap_or("not_requested");
        value["nextActions"] = if state == "completed" && publication == "offered" {
            json!(["offer_knowledge_publication"])
        } else if state == "completed" && publication == "draft_saved" {
            json!(["review_knowledge_draft"])
        } else if state == "completed" && publication == "published" {
            json!(["review_publication_withdrawal"])
        } else if state == "completed" {
            json!([])
        } else if task.as_ref().is_some_and(|t| t["state"] == "task") {
            json!(["transition_task", "accept_checkpoint"])
        } else if task.is_some() {
            json!(["accept_checkpoint", "complete_task"])
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

    pub fn begin_review(
        &self,
        owner: &str,
        action: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        self.connection_scopes(owner)?;
        if input.to_string().len() > 64 * 1024 {
            return Err(AppError::new(
                "invalid_input",
                "Review content exceeds the bounded payload limit",
            ));
        }
        let operation = require_text(input, "operationId", 120)?;
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let hash = hash_text(&input.to_string());
        let existing=tx.query_row("SELECT id,payload_hash,state,expires_at FROM work_tracking_reviews WHERE connection_id=? AND operation_id=? AND action=?",params![owner,operation,action],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?))).optional().map_err(storage_error)?;
        let id = if let Some((id, stored, state, expiry)) = existing {
            if stored != hash {
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
            tx.execute("INSERT INTO work_tracking_reviews(id,connection_id,operation_id,action,payload_hash,payload_json,expires_at,created_at) VALUES (?,?,?,?,?,?,?,?)",params![id,owner,operation,action,hash,input.to_string(),expiry,now()]).map_err(storage_error)?;
            id
        };
        tx.commit().map_err(storage_error)?;
        Ok(json!({"decisionRequired":true,"reviewState":id,"preview":input}))
    }

    pub fn decide_review(
        &self,
        owner: &str,
        action: &str,
        input: &Value,
        id: &str,
        decision: &str,
    ) -> Result<bool, AppError> {
        if !matches!(decision, "accept" | "reject" | "cancel") {
            return Err(AppError::new("invalid_input", "Invalid review decision"));
        }
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let scopes: String = tx
            .query_row(
                "SELECT scopes_json FROM mcp_connections WHERE id=? AND state='active'",
                [owner],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new("not_found_or_not_visible", "Connection is unavailable")
            })?;
        let required = if matches!(action, "publish" | "withdraw") {
            "knowledge:publish"
        } else {
            "knowledge:draft:write"
        };
        if !serde_json::from_str::<Vec<String>>(&scopes)
            .unwrap_or_default()
            .iter()
            .any(|scope| scope == required)
        {
            return Err(AppError::new(
                "not_found_or_not_visible",
                "Capability is unavailable",
            ));
        }
        let (hash,state,expiry)=tx.query_row("SELECT payload_hash,state,expires_at FROM work_tracking_reviews WHERE id=? AND connection_id=? AND action=?",params![id,owner,action],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).optional().map_err(storage_error)?.ok_or_else(||AppError::new("invalid_input","Review is unavailable"))?;
        if hash != hash_text(&input.to_string()) {
            return Err(AppError::new("invalid_input", "Reviewed payload changed"));
        }
        if state == "accept" && decision == "accept" {
            return Ok(true);
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
        tx.execute(
            "UPDATE work_tracking_reviews SET state=? WHERE id=?",
            params![decision, id],
        )
        .map_err(storage_error)?;
        if matches!(action, "publish" | "withdraw") && decision == "accept" {
            tx.execute(
                "INSERT OR IGNORE INTO work_tracking_publication_jobs(review_id) VALUES (?)",
                [id],
            )
            .map_err(storage_error)?;
        }
        tx.commit().map_err(storage_error)?;
        Ok(decision == "accept")
    }

    pub fn publication_recovery(&self) -> Result<Vec<(String, String, Value)>, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT r.id,r.connection_id,r.payload_json FROM work_tracking_publication_jobs j JOIN work_tracking_reviews r ON r.id=j.review_id JOIN mcp_connections c ON c.id=r.connection_id WHERE j.state='pending' AND c.state='active' AND (j.next_attempt_at IS NULL OR j.next_attempt_at<=?) ORDER BY r.created_at LIMIT 5").map_err(storage_error)?;
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
                Ok((
                    id,
                    owner,
                    serde_json::from_str(&payload).map_err(storage_error)?,
                ))
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

    pub fn save_knowledge_draft(
        &self,
        connection_id: &str,
        operation_id: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        let owner = hash_text(connection_id);
        let request_hash = hash_text(&input.to_string());
        let session_id = require_text(input, "sessionId", 80)?;
        let mut connection = database::open(&self.path).map_err(storage_error)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        Self::require_scope_on(&tx, connection_id, "knowledge:draft:write")?;
        if let Some(response) = Self::check_idempotency(
            &tx,
            &owner,
            "knowledge_draft_save",
            operation_id,
            &request_hash,
        )? {
            return Ok(response);
        }
        let state: String = tx
            .query_row(
                "SELECT state FROM work_tracking_sessions WHERE id=?1 AND (connection_id=?2 OR ?2='native-in-app-chat')",
                params![session_id, connection_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or_else(|| {
                AppError::new("not_found_or_not_visible", "Completed work is unavailable")
            })?;
        if state != "completed" {
            return Err(AppError::new(
                "workflow_precondition",
                "Complete the work before creating a Knowledge draft",
            ));
        }
        let draft_id = input
            .get("draftId")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let current: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(revision),0) FROM knowledge_drafts WHERE id=?",
                [&draft_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_publication_jobs j JOIN work_tracking_reviews r ON r.id=j.review_id WHERE j.state='pending' AND json_extract(r.payload_json,'$.draftId')=?)",[&draft_id],|row|row.get(0)).map_err(storage_error)?;
        if pending {
            return Err(AppError::new("publish_conflict","A reviewed publication change is pending recovery; refresh before editing its draft"));
        }
        let expected = input
            .get("expectedDraftRevision")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if current > 0 {
            let original_session:String=tx.query_row("SELECT session_id FROM knowledge_drafts WHERE id=? ORDER BY revision DESC LIMIT 1",[&draft_id],|row|row.get(0)).map_err(storage_error)?;
            if original_session != session_id {
                return Err(AppError::new(
                    "not_found_or_not_visible",
                    "Draft does not belong to this work session",
                ));
            }
        }
        if current != expected {
            return Err(AppError::conflict(
                "Knowledge draft changed; refresh before saving",
                current,
            ));
        }
        let completion_event_id = require_text(input, "completionEventId", 80)?;
        let completed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM work_tracking_decisions WHERE session_id=? AND event_id=? AND result_entity_type IN ('task_completions','completions') AND decision='accepted')",params![session_id,completion_event_id],|row|row.get(0)).map_err(storage_error)?;
        if !completed {
            return Err(AppError::new(
                "workflow_precondition",
                "Use the accepted completion event for this work session",
            ));
        }
        let title = require_text(input, "title", 200)?;
        let summary = require_text(input, "summary", 4000)?;
        let body = require_text(input, "bodyMarkdown", 50_000)?;
        let revision = current + 1;
        let content_hash = hash_text(body);
        let timestamp = now();
        let evidence = input
            .get("evidenceRefs")
            .cloned()
            .unwrap_or_else(|| json!([]));
        tx.execute("INSERT INTO knowledge_drafts(id,revision,session_id,completion_event_id,title,summary,body_markdown,evidence_refs_json,content_hash,state,created_at) VALUES (?,?,?,?,?,?,?,?,?,'draft',?)",params![draft_id,revision,session_id,completion_event_id,title,summary,body,evidence.to_string(),content_hash,timestamp]).map_err(storage_error)?;
        tx.execute("UPDATE work_tracking_sessions SET publication_state='draft_saved',updated_at=? WHERE id=?",params![timestamp,session_id]).map_err(storage_error)?;
        let response = json!({"draftId":draft_id,"draftRevision":revision,"contentHash":content_hash,"status":"draft","sessionId":session_id});
        Self::insert_idempotency(
            &tx,
            &owner,
            "knowledge_draft_save",
            operation_id,
            &request_hash,
            Some(session_id),
            Some(completion_event_id),
            &response,
            &timestamp,
        )?;
        tx.commit().map_err(storage_error)?;
        Ok(response)
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
    fn current_workbench(&self, limit: usize) -> Result<Value, AppError> {
        let connection = database::open(&self.path).map_err(storage_error)?;
        let connection = connection.unchecked_transaction().map_err(storage_error)?;
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM work_tracking_workspace WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let mut statement=connection.prepare("SELECT s.id,c.text,s.state,s.publication_state,s.head_revision,s.source_interface,s.updated_at FROM work_tracking_sessions s JOIN captures c ON c.id=s.capture_id ORDER BY s.updated_at DESC LIMIT ?").map_err(storage_error)?;
        let mut rows=statement.query_map([limit as i64],|row|Ok(json!({"trackedSessionId":row.get::<_,String>(0)?,"capture":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?,"publicationState":row.get::<_,String>(3)?,"headRevision":row.get::<_,i64>(4)?,"sourceInterface":row.get::<_,String>(5)?,"updatedAt":row.get::<_,String>(6)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
        for row in &mut rows {
            let session = Self::session_value_on(
                &connection,
                "native-in-app-chat",
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
                let (table, field) = match selected["entityType"].as_str()? {
                    "captures" => ("captures", "text"),
                    "problems" => ("problems", "statement"),
                    "features" => ("features", "title"),
                    _ => return None,
                };
                let id = selected["entityId"].as_str()?;
                let title = connection
                    .query_row(
                        &format!("SELECT {field} FROM {table} WHERE id=?"),
                        [id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .ok()??;
                selected["title"] = json!(title);
                Some(selected)
            });
        Ok(
            json!({"workspaceRevision":revision,"activeSelection":selection,"activeWork":rows,"ttlMs":0,"cacheScope":"private"}),
        )
    }

    fn overview(
        &self,
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
        let connection = database::open(&self.path).map_err(storage_error)?;
        let mut statement = connection.prepare("SELECT e.id,e.session_id,e.kind,e.payload_json,e.ingested_at FROM work_tracking_events e WHERE e.kind IN ('task_created','task_revision_proposed','task_transition_proposed','problem_resolution_proposed','completion_proposal') AND NOT EXISTS(SELECT 1 FROM work_tracking_decisions d WHERE d.event_id=e.id) AND NOT EXISTS(SELECT 1 FROM work_tracking_events x WHERE x.supersedes_event_id=e.id) ORDER BY e.ingested_at DESC,e.id").map_err(storage_error)?;
        let attention=statement.query_map([],|row|Ok(json!({"eventId":row.get::<_,String>(0)?,"sessionId":row.get::<_,String>(1)?,"kind":row.get::<_,String>(2)?,"summary":bounded_overview_text(&row.get::<_,String>(3)?,OVERVIEW_TEXT_LIMIT),"createdAt":row.get::<_,String>(4)?}))).map_err(storage_error)?.collect::<Result<Vec<_>,_>>().map_err(storage_error)?;
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
