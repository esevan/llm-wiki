use crate::adapters::{sqlite::SqliteWorkTrackingStore, vault::MarkdownVaultAdapter};
use crate::application::task_assistance_service::{
    TaskAssistanceAction, TaskAssistanceApplicationService,
};
use crate::domain::work_tracking_state::{AppError, EventKind};
use crate::ports::{
    event_log::EventLog, vault_repository::VaultRepository, work_projection::WorkProjection,
    workflow_repository::WorkflowRepository,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[derive(Clone)]
pub struct WorkTrackingApplicationService {
    store: SqliteWorkTrackingStore,
    vault: MarkdownVaultAdapter,
    assistance: TaskAssistanceApplicationService,
}

impl WorkTrackingApplicationService {
    pub(crate) fn new(
        store: SqliteWorkTrackingStore,
        vault: MarkdownVaultAdapter,
        assistance: TaskAssistanceApplicationService,
    ) -> Self {
        Self {
            store,
            vault,
            assistance,
        }
    }

    fn require_scope(&self, connection_id: &str, scope: &str) -> Result<(), AppError> {
        let scopes = self.store.connection_scopes(connection_id)?;
        if scopes.iter().any(|candidate| candidate == scope) {
            Ok(())
        } else {
            Err(AppError::new(
                "not_found_or_not_visible",
                "The requested capability is unavailable",
            ))
        }
    }

    pub fn create_connection(
        &self,
        name: &str,
        scopes: &[String],
        topic_ids: &[String],
        checkpoint_policy: &str,
    ) -> Result<Value, AppError> {
        self.store
            .create_connection(name, scopes, topic_ids, checkpoint_policy)
    }

    pub fn list_connections(&self) -> Result<Value, AppError> {
        self.store.list_connections()
    }
    pub fn owned_sessions(&self, connection_id: &str) -> Result<Vec<(String, String)>, AppError> {
        self.require_scope(connection_id, "session:read")?;
        self.store.owned_sessions(connection_id)
    }
    pub fn scopes(&self, connection_id: &str) -> Result<Vec<String>, AppError> {
        self.store.connection_scopes(connection_id)
    }
    pub fn revoke_connection(&self, id: &str) -> Result<(), AppError> {
        self.store.revoke_connection(id)
    }

    /// Closed native-assistance dispatch for a future MCP adapter.  The adapter selects an enum
    /// variant from a typed tool; callers cannot provide a native operation string.
    pub fn task_context_read(&self, connection_id: &str, task_id: &str) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:read")?;
        self.store.task_context_read(connection_id, task_id)
    }

    pub(crate) async fn task_assistance(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<Value, AppError> {
        if matches!(
            action,
            TaskAssistanceAction::Lineage
                | TaskAssistanceAction::TaskLineage
                | TaskAssistanceAction::RefinementOpen
                | TaskAssistanceAction::RefinementGet
                | TaskAssistanceAction::ReviewHistory
                | TaskAssistanceAction::KnowledgeDraft
                | TaskAssistanceAction::KnowledgeCorrection
                | TaskAssistanceAction::KnowledgeRegenerate
                | TaskAssistanceAction::KnowledgePublish
                | TaskAssistanceAction::KnowledgeWithdraw
        ) {
            self.store
                .task_continuation_discoverable(connection_id, text(input, "taskId")?)?;
        }
        if matches!(
            action,
            TaskAssistanceAction::RefinementWorkspace
                | TaskAssistanceAction::RefinementProposals
                | TaskAssistanceAction::RefinementMessage
                | TaskAssistanceAction::RefinementDecision
        ) {
            self.store
                .refinement_session_discoverable(connection_id, text(input, "sessionId")?)?;
        }
        if matches!(
            action,
            TaskAssistanceAction::ReviewGet
                | TaskAssistanceAction::ReviewCancel
                | TaskAssistanceAction::ReviewDecision
        ) {
            self.store
                .current_chat_advisory_discoverable(connection_id, text(input, "runId")?)?;
        }
        let write = !matches!(
            action,
            TaskAssistanceAction::RefinementGet
                | TaskAssistanceAction::RefinementProposals
                | TaskAssistanceAction::ReviewGet
                | TaskAssistanceAction::ReviewHistory
                | TaskAssistanceAction::Lineage
                | TaskAssistanceAction::TaskLineage
        );
        if action != TaskAssistanceAction::Lineage {
            self.require_scope(
                connection_id,
                if write {
                    "session:write"
                } else {
                    "session:read"
                },
            )?;
        }
        match action {
            TaskAssistanceAction::KnowledgeDraft
            | TaskAssistanceAction::KnowledgeCorrection
            | TaskAssistanceAction::KnowledgeRegenerate => {
                self.require_scope(connection_id, "knowledge:draft:write")?;
            }
            TaskAssistanceAction::KnowledgePublish | TaskAssistanceAction::KnowledgeWithdraw => {
                self.require_scope(connection_id, "knowledge:publish")?;
            }
            _ => {}
        }
        self.assistance
            .execute(action, input)
            .await
            .map_err(assistance_error)
    }

    /// Starts a provider-free Current Chat advisory over the exact current Task material.
    /// The caller supplies bounded findings later; no native conflict job is queued.
    pub fn create_current_chat_advisory(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:write")?;
        self.store
            .task_continuation_discoverable(connection_id, text(input, "taskId")?)?;
        self.assistance
            .create_current_chat_advisory(input)
            .map_err(assistance_error)
    }

    pub fn complete_current_chat_advisory(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:write")?;
        self.store
            .current_chat_advisory_discoverable(connection_id, text(input, "runId")?)?;
        let findings = input
            .get("findings")
            .ok_or_else(|| AppError::new("invalid_input", "findings is required"))?;
        let evidence_refs = input
            .get("evidenceRefs")
            .ok_or_else(|| AppError::new("invalid_input", "evidenceRefs is required"))?;
        let findings = findings
            .as_array()
            .ok_or_else(|| AppError::new("invalid_input", "findings must be an array"))?;
        let evidence_refs = evidence_refs
            .as_array()
            .ok_or_else(|| AppError::new("invalid_input", "evidenceRefs must be an array"))?;
        if findings.len() > 50 || evidence_refs.len() > 100 {
            return Err(AppError::new(
                "invalid_input",
                "Too many advisory findings or evidence references",
            ));
        }
        if !evidence_refs.is_empty() {
            self.require_scope(connection_id, "vault:evidence:read")?;
        }
        for evidence in evidence_refs {
            self.vault.evidence_read(
                connection_id,
                text(evidence, "evidenceId")?,
                Some(text(evidence, "revision")?),
            )?;
        }
        self.assistance
            .complete_current_chat_advisory(
                input,
                &Value::Array(findings.clone()),
                &Value::Array(evidence_refs.clone()),
            )
            .map_err(assistance_error)
    }

    /// Persist an exact typed mutation for MCP elicitation. The generic review table is
    /// durable, binds the connection, operation, action and full input hash, and avoids
    /// manufacturing a Capture session for a desktop-created Task.
    pub(crate) async fn start_task_assistance_review(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<Value, AppError> {
        self.authorize_task_assistance_review(connection_id, action, input)?;
        let task_id = self.task_assistance_review_task(connection_id, action, input)?;
        let target = self
            .store
            .task_assistance_review_target(connection_id, &task_id)?;
        let prepared = match action {
            TaskAssistanceAction::KnowledgeRegenerate => self
                .assistance
                .prepare_knowledge_draft(input)
                .await
                .map_err(assistance_error)?,
            TaskAssistanceAction::KnowledgeDraft => {
                let body = text(input, "bodyMarkdown")?;
                let lineage = self
                    .task_assistance(connection_id, TaskAssistanceAction::Lineage, input)
                    .await?;
                json!({"bodyMarkdown":body,"taskId":input["taskId"],"taskRevision":input["expectedTaskRevision"],"completionId":input["completionId"],"sourceHash":lineage["sourceHash"],"lineage":lineage})
            }
            TaskAssistanceAction::KnowledgePublish => self
                .store
                .canonical_knowledge_draft_for_review(connection_id, input, false)?,
            TaskAssistanceAction::KnowledgeWithdraw => self
                .store
                .canonical_knowledge_draft_for_review(connection_id, input, true)?,
            _ => Value::Null,
        };
        // Provider preparation and lineage reads may yield. Bind the returned body
        // to the same full Task snapshot that will be reviewed, never a later mix.
        let fresh_target = self
            .store
            .task_assistance_review_target(connection_id, &task_id)?;
        if fresh_target != target {
            return Err(AppError::new(
                "head_conflict",
                "Task material changed while preparing the review; refresh and retry",
            ));
        }
        self.store.begin_task_assistance_review(
            connection_id,
            assistance_review_name(action),
            input,
            &target,
            &prepared,
        )
    }

    pub(crate) async fn finish_task_assistance_review(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
        review_state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.authorize_task_assistance_review(connection_id, action, input)?;
        self.task_assistance_review_task(connection_id, action, input)?;
        let result = self.store.consume_task_assistance_review_with(
            connection_id, assistance_review_name(action), input, review_state, decision,
            |tx, envelope| {
                if matches!(action, TaskAssistanceAction::KnowledgePublish | TaskAssistanceAction::KnowledgeWithdraw) {
                    return Ok(json!({"decision":"accept","publicationStatus":"queued","reviewState":review_state}));
                }
                let mut request = envelope["request"].clone();
                let persist_action = if action == TaskAssistanceAction::KnowledgeRegenerate {
                    let prepared = &envelope["prepared"];
                    let body = prepared["bodyMarkdown"].as_str().ok_or_else(|| AppError::new("draft_conflict", "Prepared Knowledge body is unavailable"))?;
                    request["bodyMarkdown"] = json!(body);
                    request["completionId"] = prepared["completionId"].clone();
                    request["expectedTaskRevision"] = prepared["taskRevision"].clone();
                    TaskAssistanceAction::KnowledgeDraft
                } else { action };
                self.assistance.execute_reviewed_tx(tx, persist_action, &request).map_err(assistance_error)
            },
        )?;
        Ok(result.unwrap_or_else(|| json!({"decision":decision})))
    }

    fn authorize_task_assistance_review(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<(), AppError> {
        match action {
            TaskAssistanceAction::RefinementDecision => {
                self.require_scope(connection_id, "session:write")?;
                self.store
                    .refinement_session_discoverable(connection_id, text(input, "sessionId")?)
            }
            TaskAssistanceAction::ReviewDecision => {
                self.require_scope(connection_id, "session:write")?;
                self.store
                    .current_chat_advisory_discoverable(connection_id, text(input, "runId")?)
            }
            TaskAssistanceAction::KnowledgeDraft
            | TaskAssistanceAction::KnowledgeCorrection
            | TaskAssistanceAction::KnowledgeRegenerate => {
                self.require_scope(connection_id, "knowledge:draft:write")?;
                self.store
                    .task_continuation_discoverable(connection_id, text(input, "taskId")?)
            }
            TaskAssistanceAction::KnowledgePublish | TaskAssistanceAction::KnowledgeWithdraw => {
                self.require_scope(connection_id, "knowledge:publish")?;
                self.store
                    .task_continuation_discoverable(connection_id, text(input, "taskId")?)
            }
            _ => Err(AppError::new(
                "invalid_input",
                "Unsupported Task assistance review action",
            )),
        }
    }

    fn task_assistance_review_task(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<String, AppError> {
        match action {
            TaskAssistanceAction::RefinementDecision => self.store.task_assistance_subject_task(
                connection_id,
                "refinement",
                text(input, "sessionId")?,
            ),
            TaskAssistanceAction::ReviewDecision => self.store.task_assistance_subject_task(
                connection_id,
                "advisory",
                text(input, "runId")?,
            ),
            TaskAssistanceAction::KnowledgeDraft
            | TaskAssistanceAction::KnowledgeCorrection
            | TaskAssistanceAction::KnowledgeRegenerate
            | TaskAssistanceAction::KnowledgePublish
            | TaskAssistanceAction::KnowledgeWithdraw => Ok(text(input, "taskId")?.to_owned()),
            _ => Err(AppError::new(
                "invalid_input",
                "Unsupported Task assistance review action",
            )),
        }
    }

    pub fn open(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        self.open_reviewed(connection_id, input, None)
    }

    pub fn finish_open(
        &self,
        connection_id: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.open_reviewed(connection_id, input, Some((state, decision)))
    }

    fn open_reviewed(
        &self,
        connection_id: &str,
        input: &Value,
        review: Option<(&str, &str)>,
    ) -> Result<Value, AppError> {
        let started = Instant::now();
        self.require_scope(connection_id, "session:write")?;
        let operation_id = text(input, "operationId")?;
        let lineage_key = text(input, "lineageKey")?;
        let mode = input
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("create");
        if !matches!(mode, "create" | "resume" | "continue_task") {
            return Err(AppError::new(
                "invalid_input",
                "mode must be create, resume, or continue_task",
            ));
        }
        if mode == "continue_task" {
            // A session-write grant alone cannot turn a guessed desktop Task ID into a target.
            // Either whole-Workbench discovery grant, or its explicit topic membership, is
            // required and is checked again by the transaction that accepts the review.
            if input
                .get("capture")
                .is_some_and(|capture| !capture.is_null())
            {
                return Err(AppError::new(
                    "invalid_input",
                    "Task continuation does not accept a Capture",
                ));
            }
            self.store
                .task_continuation_discoverable(connection_id, text(input, "taskId")?)?;
        }
        let result = self.store.open_session(
            connection_id,
            operation_id,
            lineage_key,
            mode,
            input.get("capture").filter(|capture| !capture.is_null()),
            input.get("parentSessionId").and_then(Value::as_str),
            input.get("reviewContext"),
            review,
            input.get("taskId").and_then(Value::as_str),
        );
        let session = result
            .as_ref()
            .ok()
            .and_then(|value| value.get("sessionId"))
            .and_then(Value::as_str);
        self.store.record_activity(
            "chat",
            Some(connection_id),
            session,
            "inbound_work_open",
            if result.is_ok() {
                "accepted"
            } else {
                "rejected"
            },
            result.as_ref().err().map(|error| error.code.as_str()),
            started.elapsed().as_millis(),
        );
        result
    }

    pub fn append(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        let started = Instant::now();
        self.require_scope(connection_id, "session:write")?;
        let event = input
            .get("event")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::new("invalid_input", "event is required"))?;
        let kind = event
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::new("invalid_input", "event.kind is required"))?;
        let kind = parse_kind(kind)?;
        reject_authority_fields(input)?;
        reject_authority_fields(input.get("event").unwrap_or(&Value::Null))?;
        let result = self.store.append_event(
            connection_id,
            text(input, "operationId")?,
            text(input, "sessionId")?,
            input
                .get("expectedHeadRevision")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::new("invalid_input", "expectedHeadRevision is required")
                })?,
            kind,
            input.get("event").unwrap_or(&Value::Null),
            input.get("observedAt").and_then(Value::as_str),
            input.get("supersedesEventId").and_then(Value::as_str),
        );
        self.store.record_activity(
            "chat",
            Some(connection_id),
            input.get("sessionId").and_then(Value::as_str),
            "inbound_work_append",
            if result.is_ok() {
                "accepted"
            } else {
                "rejected"
            },
            result.as_ref().err().map(|error| error.code.as_str()),
            started.elapsed().as_millis(),
        );
        result
    }

    pub fn session(&self, connection_id: &str, session_id: &str) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:read")?;
        self.store.session(connection_id, session_id)
    }

    pub fn select_work(&self, input: &Value) -> Result<Value, AppError> {
        self.store.select_work(input)
    }

    pub fn current_workbench(&self, connection_id: &str, limit: usize) -> Result<Value, AppError> {
        self.require_scope(connection_id, "workbench:current:read")?;
        self.store
            .current_workbench(connection_id, limit.clamp(1, 20))
    }

    pub fn overview(
        &self,
        connection_id: &str,
        limit: usize,
        cursor: Option<&str>,
        expected_snapshot: Option<i64>,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "workbench:overview:read")?;
        let mut offset = 0usize;
        let mut attention_offset = 0usize;
        let mut cursor_snapshot = expected_snapshot;
        if let Some(cursor) = cursor {
            let decoded = URL_SAFE_NO_PAD
                .decode(cursor)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .ok_or_else(|| {
                    AppError::new(
                        "snapshot_stale",
                        "The overview cursor is invalid or expired",
                    )
                })?;
            let owner = format!("{:x}", Sha256::digest(connection_id.as_bytes()));
            let expires = decoded
                .get("expires")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            if decoded.get("owner").and_then(Value::as_str) != Some(owner.as_str())
                || expires < chrono::Utc::now().timestamp()
            {
                return Err(AppError::new(
                    "snapshot_stale",
                    "The overview cursor is invalid or expired",
                ));
            }
            offset = decoded
                .get("offset")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            attention_offset = decoded
                .get("attentionOffset")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            let embedded = decoded
                .get("snapshot")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::new(
                        "snapshot_stale",
                        "The overview cursor is invalid or expired",
                    )
                })?;
            if expected_snapshot.is_some_and(|value| value != embedded) {
                return Err(AppError::new(
                    "snapshot_stale",
                    "The overview snapshot changed; refresh from the first page",
                ));
            }
            cursor_snapshot = Some(embedded);
        }
        let mut result =
            self.store
                .overview(connection_id, limit.clamp(1, 50), offset, attention_offset)?;
        let current = result
            .get("snapshotRevision")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        if cursor_snapshot.is_some_and(|snapshot| snapshot != current) {
            return Err(AppError::new(
                "snapshot_stale",
                "The overview snapshot changed; refresh from the first page",
            ));
        }
        let next_offset = result.get("nextOffset").and_then(Value::as_u64);
        let next_attention_offset = result.get("nextAttentionOffset").and_then(Value::as_u64);
        result
            .as_object_mut()
            .expect("overview object")
            .remove("nextOffset");
        result
            .as_object_mut()
            .expect("overview object")
            .remove("nextAttentionOffset");
        result["nextCursor"] = if next_offset.is_some() || next_attention_offset.is_some() {
            Value::String(
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&json!({
                        "offset":next_offset.unwrap_or(offset as u64),
                        "attentionOffset":next_attention_offset.unwrap_or(attention_offset as u64),
                        "snapshot":current,
                        "owner":format!("{:x}",Sha256::digest(connection_id.as_bytes())),
                        "expires":chrono::Utc::now().timestamp()+300
                    }))
                    .expect("cursor serialization"),
                ),
            )
        } else {
            Value::Null
        };
        Ok(result)
    }

    pub fn topic(
        &self,
        connection_id: &str,
        topic_id: &str,
        limit: usize,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "topic:read")?;
        self.store.require_topic(connection_id, topic_id)?;
        self.store.topic(topic_id, limit.clamp(1, 50))
    }

    fn authorize_search_scope(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
    ) -> Result<(), AppError> {
        match scope_kind {
            "workbench" => self.require_scope(connection_id, "workbench:overview:read"),
            "topic" => {
                self.require_scope(connection_id, "topic:read")?;
                self.store.require_topic(connection_id, scope_target)
            }
            "session" => {
                self.require_scope(connection_id, "session:read")?;
                self.store.session(connection_id, scope_target).map(|_| ())
            }
            _ => Err(AppError::new(
                "invalid_input",
                "scope must be session, topic, or workbench",
            )),
        }
    }

    pub fn lexical_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "vault:search:lexical")?;
        self.authorize_search_scope(connection_id, scope_kind, scope_target)?;
        self.vault
            .lexical_search(connection_id, scope_kind, scope_target, query, limit)
    }

    pub fn semantic_search(
        &self,
        connection_id: &str,
        scope_kind: &str,
        scope_target: &str,
        query: &str,
        limit: usize,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "vault:search:semantic")?;
        self.authorize_search_scope(connection_id, scope_kind, scope_target)?;
        self.vault
            .semantic_search(connection_id, scope_kind, scope_target, query, limit)
    }

    pub fn evidence_read(&self, connection_id: &str, items: &[Value]) -> Result<Value, AppError> {
        self.require_scope(connection_id, "vault:evidence:read")?;
        if items.is_empty() || items.len() > 8 {
            return Err(AppError::new("invalid_input", "Provide 1–8 evidence items"));
        }
        let mut results = Vec::with_capacity(items.len());
        let mut characters = 0usize;
        for item in items {
            let value = self.vault.evidence_read(
                connection_id,
                text(item, "evidenceId")?,
                item.get("expectedRevision").and_then(Value::as_str),
            )?;
            characters += value
                .get("content")
                .and_then(Value::as_str)
                .map(str::len)
                .unwrap_or(0);
            if characters > 24_000 {
                break;
            }
            results.push(value);
        }
        Ok(json!({"items":results,"truncated":results.len()<items.len()}))
    }

    pub fn decide(
        &self,
        connection_id: &str,
        input: &Value,
        channel: &str,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:write")?;
        self.store.record_decision(
            connection_id,
            text(input, "sessionId")?,
            text(input, "sourceEventId")?,
            text(input, "decision")?,
            channel,
        )
    }

    pub fn begin_advance(&self, connection_id: &str, input: &Value) -> Result<String, AppError> {
        self.require_scope(connection_id, "session:write")?;
        if input["action"] == "link_current_work" {
            self.require_scope(connection_id, "workbench:current:read")?;
        }
        self.store.create_challenge(connection_id, input)
    }

    /// Replaying a terminal result is safe only for the same connection-owned operation
    /// and exact request hash. Pending reviews deliberately return None so MCP still elicits.
    pub fn advance_result_replay(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Option<Value>, AppError> {
        self.require_scope(connection_id, "session:write")?;
        if input["action"] == "link_current_work" {
            self.require_scope(connection_id, "workbench:current:read")?;
        }
        self.store.advance_result_replay(connection_id, input)
    }

    pub fn review_target(&self, owner: &str, state: &str) -> Result<Value, AppError> {
        self.require_scope(owner, "session:write")?;
        self.store.review_target(owner, state)
    }
    pub fn set_topic_membership(&self, input: &Value) -> Result<Value, AppError> {
        self.store.set_topic_membership(input)
    }

    pub fn finish_advance(
        &self,
        connection_id: &str,
        state: &str,
        input: &Value,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:write")?;
        if decision == "accept"
            && matches!(
                input["action"].as_str(),
                Some("resolve_conflict" | "review_conflict")
            )
        {
            let evidence = input["proposedPayload"]["evidenceRefs"]
                .as_array()
                .or_else(|| input["proposedPayload"]["evidence"].as_array());
            if let Some(evidence) = evidence {
                if !evidence.is_empty() {
                    self.require_scope(connection_id, "vault:evidence:read")?;
                }
                for item in evidence {
                    self.vault.evidence_read(
                        connection_id,
                        text(item, "evidenceId")?,
                        Some(text(item, "revision")?),
                    )?;
                }
            }
        }
        self.store
            .consume_challenge(connection_id, state, input, decision)
    }

    pub fn save_knowledge_draft(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        let input = self.canonical_gui_knowledge_input(connection_id, input)?;
        self.start_gui_task_knowledge_review(
            connection_id,
            TaskAssistanceAction::KnowledgeDraft,
            &input,
        )
    }

    pub fn finish_knowledge_draft(
        &self,
        connection_id: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        let input = self.canonical_gui_knowledge_input(connection_id, input)?;
        self.authorize_task_assistance_review(
            connection_id,
            TaskAssistanceAction::KnowledgeDraft,
            &input,
        )?;
        let result = self.store.consume_task_assistance_review_with(
            connection_id,
            assistance_review_name(TaskAssistanceAction::KnowledgeDraft),
            &input,
            state,
            decision,
            |tx, envelope| {
                self.assistance
                    .execute_reviewed_tx(
                        tx,
                        TaskAssistanceAction::KnowledgeDraft,
                        &envelope["request"],
                    )
                    .map_err(assistance_error)
            },
        )?;
        Ok(result.unwrap_or_else(|| json!({"decision":decision})))
    }

    pub fn publish_knowledge(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        self.begin_publish(connection_id, input)
    }

    pub fn begin_publish(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        self.start_gui_task_knowledge_review(
            connection_id,
            TaskAssistanceAction::KnowledgePublish,
            input,
        )
    }

    pub fn finish_publish(
        &self,
        connection_id: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        let Some(_) = self.consume_gui_task_knowledge_review(
            connection_id,
            TaskAssistanceAction::KnowledgePublish,
            input,
            state,
            decision,
        )?
        else {
            return Ok(json!({"decision":decision}));
        };
        Ok(json!({"decision":"accept","publicationStatus":"queued","reviewState":state}))
    }

    pub fn defer_publication(
        &self,
        connection_id: &str,
        session_id: &str,
        completion_revision: i64,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "session:write")?;
        self.store
            .defer_publication(connection_id, session_id, completion_revision)
    }

    pub fn begin_withdrawal(&self, owner: &str, input: &Value) -> Result<Value, AppError> {
        self.start_gui_task_knowledge_review(owner, TaskAssistanceAction::KnowledgeWithdraw, input)
    }

    pub fn finish_withdrawal(
        &self,
        owner: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        let Some(_) = self.consume_gui_task_knowledge_review(
            owner,
            TaskAssistanceAction::KnowledgeWithdraw,
            input,
            state,
            decision,
        )?
        else {
            return Ok(json!({"decision":decision}));
        };
        Ok(json!({"decision":"accept","publicationStatus":"queued","reviewState":state}))
    }

    fn canonical_gui_knowledge_input(
        &self,
        connection_id: &str,
        input: &Value,
    ) -> Result<Value, AppError> {
        if input.get("taskId").and_then(Value::as_str).is_some() {
            return Ok(input.clone());
        }
        let session_id = text(input, "sessionId")?;
        let mut canonical = input.clone();
        let resolved = self
            .store
            .canonical_task_completion_for_session(connection_id, session_id)?;
        canonical["taskId"] = resolved["taskId"].clone();
        canonical["expectedTaskRevision"] = resolved["expectedTaskRevision"].clone();
        canonical["completionId"] = resolved["completionId"].clone();
        Ok(canonical)
    }

    fn start_gui_task_knowledge_review(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
    ) -> Result<Value, AppError> {
        let input = self.canonical_gui_knowledge_input(connection_id, input)?;
        self.authorize_task_assistance_review(connection_id, action, &input)?;
        let target = self
            .store
            .task_assistance_review_target(connection_id, text(&input, "taskId")?)?;
        let prepared = match action {
            TaskAssistanceAction::KnowledgePublish => self
                .store
                .canonical_knowledge_draft_for_review(connection_id, &input, false)?,
            TaskAssistanceAction::KnowledgeWithdraw => self
                .store
                .canonical_knowledge_draft_for_review(connection_id, &input, true)?,
            TaskAssistanceAction::KnowledgeDraft => {
                json!({"bodyMarkdown":input["bodyMarkdown"],"taskId":input["taskId"],"taskRevision":input["expectedTaskRevision"],"completionId":input["completionId"]})
            }
            _ => Value::Null,
        };
        self.store.begin_task_assistance_review(
            connection_id,
            assistance_review_name(action),
            &input,
            &target,
            &prepared,
        )
    }

    fn consume_gui_task_knowledge_review(
        &self,
        connection_id: &str,
        action: TaskAssistanceAction,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Option<Value>, AppError> {
        let input = self.canonical_gui_knowledge_input(connection_id, input)?;
        self.authorize_task_assistance_review(connection_id, action, &input)?;
        self.store
            .consume_task_assistance_review(
                connection_id,
                assistance_review_name(action),
                &input,
                state,
                decision,
            )
            .map(|envelope| envelope.map(|value| value["request"].clone()))
    }

    fn finish_task_knowledge_publication(
        &self,
        owner: &str,
        input: &Value,
        withdraw: bool,
    ) -> Result<Value, AppError> {
        let action = if withdraw {
            TaskAssistanceAction::KnowledgeWithdraw
        } else {
            TaskAssistanceAction::KnowledgePublish
        };
        self.authorize_task_assistance_review(owner, action, input)?;
        let result = if withdraw {
            crate::native::task_assistance::withdraw_reviewed_knowledge(
                self.store.database_path(),
                self.vault.root_path(),
                input,
            )
        } else {
            crate::native::task_assistance::publish_reviewed_knowledge(
                self.store.database_path(),
                self.vault.root_path(),
                input,
            )
        };
        result.map_err(assistance_error)
    }

    /// Legacy `knowledge_drafts` are immutable audit history. The only remaining
    /// write is recovery of a publication job that was already explicitly accepted
    /// before migration; new GUI and MCP paths cannot enter this handler.
    fn recover_approved_legacy_publication(
        &self,
        owner: &str,
        input: &Value,
        review_id: &str,
        withdraw: bool,
    ) -> Result<Value, AppError> {
        let draft_id = text(input, "draftId")?;
        let revision = input
            .get("expectedDraftRevision")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::new("invalid_input", "expectedDraftRevision is required"))?;
        let content_hash = text(input, "expectedContentHash")?;
        let draft = self
            .store
            .knowledge_draft(owner, draft_id, revision, content_hash)?;
        if withdraw {
            if draft["state"] != "published" {
                return Err(AppError::new(
                    "workflow_precondition",
                    "Only an already published legacy draft can be recovered for withdrawal",
                ));
            }
            let recovery =
                self.vault
                    .withdraw(text(&draft, "publishedPath")?, content_hash, review_id)?;
            self.store
                .finalize_withdrawal(review_id, draft_id, revision, &recovery)
        } else {
            if draft["state"] == "withdrawn" {
                return Err(AppError::new(
                    "publish_conflict",
                    "Withdrawn legacy draft cannot be republished",
                ));
            }
            let path = draft["publishedPath"].as_str().ok_or_else(|| {
                AppError::new(
                    "publish_conflict",
                    "Approved legacy publication path is unavailable",
                )
            })?;
            self.vault
                .publish(path, None, text(&draft, "bodyMarkdown")?)?;
            self.store
                .finalize_publication(draft_id, revision, content_hash, path)
        }
    }

    pub fn drain(&self, limit: usize) -> Result<usize, AppError> {
        let processed = self.store.drain(limit)?;
        for (state, owner, input) in self.store.publication_recovery()? {
            let action = self.store.review_action(&state)?;
            let result = match action.as_str() {
                "task_knowledge_publish" => {
                    self.finish_task_knowledge_publication(&owner, &input, false)
                }
                "task_knowledge_withdraw" => {
                    self.finish_task_knowledge_publication(&owner, &input, true)
                }
                "withdraw" => {
                    self.recover_approved_legacy_publication(&owner, &input, &state, true)
                }
                "publish" => {
                    self.recover_approved_legacy_publication(&owner, &input, &state, false)
                }
                _ => Err(AppError::new(
                    "invalid_input",
                    "Unsupported publication recovery action",
                )),
            };
            match result {
                Ok(_)
                    if matches!(
                        action.as_str(),
                        "task_knowledge_publish" | "task_knowledge_withdraw"
                    ) =>
                {
                    self.store.publication_job_result(&state, None)?;
                }
                Ok(_) => {
                    // A recovered legacy job is terminal; otherwise every restart republishes it.
                    self.store.publication_job_result(&state, None)?;
                }
                Err(error) => {
                    self.store
                        .publication_job_result(&state, Some(&error.code))?;
                }
            }
        }
        Ok(processed)
    }
}

fn assistance_review_name(action: TaskAssistanceAction) -> &'static str {
    match action {
        TaskAssistanceAction::RefinementDecision => "task_refinement_decision",
        TaskAssistanceAction::ReviewDecision => "task_advisory_decision",
        TaskAssistanceAction::KnowledgeDraft => "task_knowledge_draft",
        TaskAssistanceAction::KnowledgeCorrection => "task_knowledge_correction",
        TaskAssistanceAction::KnowledgeRegenerate => "task_knowledge_regenerate",
        TaskAssistanceAction::KnowledgePublish => "task_knowledge_publish",
        TaskAssistanceAction::KnowledgeWithdraw => "task_knowledge_withdraw",
        _ => "unsupported_task_assistance_review",
    }
}

/// The native layer owns provider diagnostics.  MCP may return only stable workflow
/// outcomes that a caller can act on; all other failures are deliberately opaque.
fn assistance_error(error: String) -> AppError {
    let code = error
        .split_once(':')
        .map(|(code, _)| code)
        .unwrap_or(error.as_str());
    let message = match code {
        "draft_conflict" => "The Knowledge draft changed; read fresh lineage before retrying",
        "head_conflict" => "The Task changed; read fresh state before retrying",
        "operation_conflict" => "That operation ID was already used with different input",
        "completion_conflict" => "The Task completion changed; read fresh state before retrying",
        "subject_material_conflict" => {
            "The reviewed material changed; read fresh state before retrying"
        }
        "source_changed" => "The Knowledge source changed outside LLM Wiki",
        "review_scope_unavailable" => "The requested review is unavailable in this scope",
        "invalid_input" => "The requested Task assistance input is invalid",
        "cancelled" => "The requested Task assistance was cancelled",
        _ => return AppError::new("assistance_unavailable", "Task assistance is unavailable"),
    };
    AppError::new(code, message)
}

fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, AppError> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| AppError::new("invalid_input", format!("{name} is required")))
}

fn parse_kind(value: &str) -> Result<EventKind, AppError> {
    match value {
        "problem_draft" => Ok(EventKind::ProblemDraft),
        "task_created" => Ok(EventKind::TaskCreated),
        "task_revision_proposed" => Ok(EventKind::TaskRevisionProposed),
        "task_transition_proposed" => Ok(EventKind::TaskTransitionProposed),
        "problem_resolution_proposed" => Ok(EventKind::ProblemResolutionProposed),
        "solution_draft" => Ok(EventKind::SolutionDraft),
        "work_log_checkpoint" => Ok(EventKind::WorkLogCheckpoint),
        "conflict_proposal" => Ok(EventKind::ConflictProposal),
        "completion_proposal" => Ok(EventKind::CompletionProposal),
        _ => Err(AppError::new("invalid_input", "Unsupported event kind")),
    }
}

fn reject_authority_fields(value: &Value) -> Result<(), AppError> {
    const FORBIDDEN: [&str; 8] = [
        "confirmed",
        "approved",
        "decision",
        "workflowState",
        "completed",
        "published",
        "occurredAt",
        "author",
    ];
    if let Some(object) = value.as_object() {
        if object.keys().any(|key| FORBIDDEN.contains(&key.as_str())) {
            return Err(AppError::new(
                "invalid_input",
                "Caller authority fields are not accepted",
            ));
        }
    }
    Ok(())
}
