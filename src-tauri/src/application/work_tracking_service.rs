use crate::adapters::{sqlite::SqliteWorkTrackingStore, vault::MarkdownVaultAdapter};
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
}

impl WorkTrackingApplicationService {
    pub fn new(store: SqliteWorkTrackingStore, vault: MarkdownVaultAdapter) -> Self {
        Self { store, vault }
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
        if !matches!(mode, "create" | "resume") {
            return Err(AppError::new(
                "invalid_input",
                "mode must be create or resume",
            ));
        }
        let result = self.store.open_session(
            connection_id,
            operation_id,
            lineage_key,
            mode,
            input.get("capture"),
            input.get("parentSessionId").and_then(Value::as_str),
            input.get("reviewContext"),
            review,
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
        self.store.current_workbench(limit.clamp(1, 20))
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
        let mut result = self.store.overview(limit.clamp(1, 50), offset)?;
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
        result
            .as_object_mut()
            .expect("overview object")
            .remove("nextOffset");
        result["nextCursor"]=next_offset.map(|next|URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({"offset":next,"snapshot":current,"owner":format!("{:x}",Sha256::digest(connection_id.as_bytes())),"expires":chrono::Utc::now().timestamp()+300})).expect("cursor serialization"))).map(Value::String).unwrap_or(Value::Null);
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
        if decision == "accept" && input["action"] == "resolve_conflict" {
            if let Some(evidence) = input["proposedPayload"]["evidence"].as_array() {
                for item in evidence {
                    self.vault.evidence_read(
                        connection_id,
                        text(item, "evidenceId")?,
                        item["revision"].as_str(),
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
        self.require_scope(connection_id, "knowledge:draft:write")?;
        self.store.begin_review(connection_id, "draft", input)
    }

    pub fn finish_knowledge_draft(
        &self,
        connection_id: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "knowledge:draft:write")?;
        if !self
            .store
            .decide_review(connection_id, "draft", input, state, decision)?
        {
            return Ok(json!({"decision":decision}));
        }
        if let Some(items) = input.get("evidenceRefs").and_then(Value::as_array) {
            for item in items {
                let evidence = self.vault.evidence_read(
                    connection_id,
                    text(item, "evidenceId")?,
                    item.get("revision").and_then(Value::as_str),
                )?;
                if let Some(hash) = item.get("contentHash").and_then(Value::as_str) {
                    if evidence["contentHash"] != hash {
                        return Err(AppError::new(
                            "evidence_revision_changed",
                            "Evidence changed; refresh the draft",
                        ));
                    }
                }
            }
        }
        self.store
            .save_knowledge_draft(connection_id, text(input, "operationId")?, input)
    }

    pub fn publish_knowledge(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        let _ = input;
        self.require_scope(connection_id, "knowledge:publish")?;
        Err(AppError::new(
            "approval_required",
            "Review the exact Knowledge draft before publication",
        ))
    }

    pub fn begin_publish(&self, connection_id: &str, input: &Value) -> Result<Value, AppError> {
        self.require_scope(connection_id, "knowledge:publish")?;
        let draft = self.store.knowledge_draft(
            connection_id,
            text(input, "draftId")?,
            input["expectedDraftRevision"]
                .as_i64()
                .ok_or_else(|| AppError::new("invalid_input", "Draft revision is required"))?,
            text(input, "expectedContentHash")?,
        )?;
        if draft["state"] == "withdrawn" {
            return Err(AppError::new(
                "publish_conflict",
                "Withdrawn publication requires a newly reviewed draft revision",
            ));
        }
        if draft["state"] == "published" {
            return self.store.finalize_publication(
                text(input, "draftId")?,
                input["expectedDraftRevision"].as_i64().unwrap_or_default(),
                text(input, "expectedContentHash")?,
                draft["publishedPath"].as_str().unwrap_or_default(),
            );
        }
        let mut review = self.store.begin_review(connection_id, "publish", input)?;
        review["preview"] = draft;
        Ok(review)
    }

    pub fn finish_publish(
        &self,
        connection_id: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.require_scope(connection_id, "knowledge:publish")?;
        if !self
            .store
            .decide_review(connection_id, "publish", input, state, decision)?
        {
            return Ok(json!({"decision":decision}));
        }
        let draft_id = text(input, "draftId")?;
        let revision = input
            .get("expectedDraftRevision")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::new("invalid_input", "expectedDraftRevision is required"))?;
        let hash = text(input, "expectedContentHash")?;
        let draft = self
            .store
            .knowledge_draft(connection_id, draft_id, revision, hash)?;
        if draft["state"] == "withdrawn" {
            return Err(AppError::new(
                "publish_conflict",
                "Withdrawn publication requires a newly reviewed draft revision",
            ));
        }
        if draft["state"] == "published" {
            let result = self.store.finalize_publication(
                draft_id,
                revision,
                hash,
                draft["publishedPath"].as_str().unwrap_or_default(),
            )?;
            self.store.publication_job_result(state, None)?;
            return Ok(result);
        }
        let slug = draft["title"]
            .as_str()
            .unwrap_or("knowledge")
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_lowercase();
        let path = format!(
            "Knowledge/{}-{}.md",
            if slug.is_empty() { "knowledge" } else { &slug },
            &draft_id[..draft_id.len().min(8)]
        );
        let body = draft["bodyMarkdown"].as_str().unwrap_or_default();
        self.vault.publish(&path, None, body)?;
        let result = self
            .store
            .finalize_publication(draft_id, revision, hash, &path)?;
        self.store.publication_job_result(state, None)?;
        Ok(result)
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
        self.require_scope(owner, "knowledge:publish")?;
        let draft = self.store.knowledge_draft(
            owner,
            text(input, "draftId")?,
            input["expectedDraftRevision"]
                .as_i64()
                .ok_or_else(|| AppError::new("invalid_input", "Draft revision is required"))?,
            text(input, "expectedContentHash")?,
        )?;
        if draft["state"] != "published" {
            return Err(AppError::new(
                "workflow_precondition",
                "Only published Knowledge can be withdrawn",
            ));
        }
        let mut review = self.store.begin_review(owner, "withdraw", input)?;
        review["preview"] = draft;
        Ok(review)
    }

    pub fn finish_withdrawal(
        &self,
        owner: &str,
        input: &Value,
        state: &str,
        decision: &str,
    ) -> Result<Value, AppError> {
        self.require_scope(owner, "knowledge:publish")?;
        if !self
            .store
            .decide_review(owner, "withdraw", input, state, decision)?
        {
            return Ok(json!({"decision":decision}));
        }
        let id = text(input, "draftId")?;
        let revision = input["expectedDraftRevision"]
            .as_i64()
            .ok_or_else(|| AppError::new("invalid_input", "Draft revision is required"))?;
        let hash = text(input, "expectedContentHash")?;
        let draft = self.store.knowledge_draft(owner, id, revision, hash)?;
        let recovery = self
            .vault
            .withdraw(text(&draft, "publishedPath")?, hash, state)?;
        self.store
            .finalize_withdrawal(state, id, revision, &recovery)
    }

    pub fn drain(&self, limit: usize) -> Result<usize, AppError> {
        let processed = self.store.drain(limit)?;
        for (state, owner, input) in self.store.publication_recovery()? {
            let result = if self.store.review_action(&state)? == "withdraw" {
                self.finish_withdrawal(&owner, &input, &state, "accept")
            } else {
                self.finish_publish(&owner, &input, &state, "accept")
            };
            if let Err(error) = result {
                self.store
                    .publication_job_result(&state, Some(&error.code))?;
            }
        }
        Ok(processed)
    }
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
