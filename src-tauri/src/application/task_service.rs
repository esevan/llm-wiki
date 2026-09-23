use crate::adapters::sqlite::task_repository::{self, SqliteTaskRepository};
use crate::domain::task::{content_hash, validate_state_transition, READINESS_FIELDS};
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct TaskApplicationService {
    repo: SqliteTaskRepository,
}

fn req<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v.get(k)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .ok_or_else(|| format!("invalid_input: {k} is required"))
}

impl TaskApplicationService {
    pub(crate) fn new(path: impl AsRef<Path>) -> Self {
        Self {
            repo: SqliteTaskRepository::new(path),
        }
    }
    pub(crate) fn execute(&self, name: &str, input: &Value) -> Result<Value, String> {
        match name {
            "capture.create" => self.capture(input),
            "task.get" => self.get(req(input, "taskId")?),
            "task.delete" => self.delete(req(input, "taskId")?),
            "workbench.get" => self.workbench(),
            "task.readiness.get" => self.readiness(req(input, "taskId")?),
            "task.work-log.get" => self.work_log(req(input, "taskId")?),
            "task.work-session.list" => self.work_sessions(req(input, "taskId")?),
            "task.work-session.get" => {
                self.work_session(req(input, "taskId")?, req(input, "sessionId")?)
            }
            "task.work-session.create" => self.work_session_create(input),
            "task.work-session.update" => self.work_session_update(input),
            "task.work-session.entry.create" => self.work_session_entry(input),
            "task.create"
            | "task.revision"
            | "task.transition"
            | "task.reopen"
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
            | "task.completion.create"
            | "problem.create"
            | "problem.revision"
            | "problem.resolution.create" => self
                .repo
                .transaction(|tx| self.execute_tx_for_tracking(tx, name, input, None)),
            _ => Err(format!("Native operation is not implemented: {name}")),
        }
    }
    /// Transaction-scoped Task mutation with a trusted tracking origin. Only the SQLite
    /// tracking adapter may supply `origin_session_id`; external payloads cannot suppress their
    /// own linked-session event.
    pub(crate) fn execute_tx_for_tracking(
        &self,
        tx: &Transaction<'_>,
        name: &str,
        input: &Value,
        origin_session_id: Option<&str>,
    ) -> Result<Value, String> {
        if let Some(result) = self.op(tx, input, name)? {
            return Ok(result);
        }
        let at = task_repository::now();
        let result = self.mutation_tx(tx, name, input, &at)?;
        self.finish(tx, input, name, &result)?;
        task_repository::sync_linked_sessions_tx(
            tx,
            req(input, "operationId")?,
            name,
            origin_session_id,
            &at,
        )?;
        Ok(result)
    }

    fn mutation_tx(
        &self,
        tx: &Transaction<'_>,
        name: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        match name {
            "task.create" => self.task_create_mutation_tx(tx, input, at),
            "task.revision" => self.task_revision_tx(tx, req(input, "taskId")?, input, at),
            "task.transition" => {
                self.task_transition_tx(tx, req(input, "taskId")?, input, None, at)
            }
            "task.reopen" => {
                self.task_transition_tx(tx, req(input, "taskId")?, input, Some("reopen"), at)
            }
            "problem.create" => self.problem_tx(tx, input, None, at),
            "problem.revision" => self.problem_tx(tx, input, Some(req(input, "problemId")?), at),
            "problem.resolution.create" => {
                self.resolve_problem_tx(tx, req(input, "problemId")?, input, at)
            }
            "task.problem-link.create" => {
                self.problem_link_tx(tx, req(input, "taskId")?, input, at)
            }
            "task.problem-link.delete" => self.problem_link_delete_tx(
                tx,
                req(input, "taskId")?,
                req(input, "linkId")?,
                input,
                at,
            ),
            "task.relationship.create" => {
                self.relationship_tx(tx, req(input, "taskId")?, input, at)
            }
            "task.relationship.delete" => self.relationship_delete_tx(
                tx,
                req(input, "taskId")?,
                req(input, "relationshipId")?,
                input,
                at,
            ),
            "task.readiness.decision" => {
                self.readiness_decision_tx(tx, req(input, "taskId")?, input, at)
            }
            "task.work-log.create" => self.work_log_create_tx(tx, req(input, "taskId")?, input, at),
            "work-log.comment.create" => self.comment_tx(tx, req(input, "entryId")?, input, at),
            "task.checklist.create" => self.checklist_tx(tx, req(input, "taskId")?, input, at),
            "task.checklist.update" => self.checklist_update_tx(
                tx,
                req(input, "taskId")?,
                req(input, "itemId")?,
                input,
                at,
            ),
            "task.decision.create" => self.decision_tx(tx, req(input, "taskId")?, input, at),
            "task.completion.create" => self.complete_tx(tx, req(input, "taskId")?, input, at),
            _ => Err(format!("operation is not transaction-scoped: {name}")),
        }
    }

    fn task_create_mutation_tx(
        &self,
        tx: &Transaction<'_>,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let origin = input.get("originCaptureId").and_then(Value::as_str);
        let direct = input.get("inputText").and_then(Value::as_str);
        if origin.is_some() == direct.is_some() {
            return Err("invalid_input: supply exactly one originCaptureId or inputText".into());
        }
        let provenance = if let Some(text) = direct {
            let capture_id = task_repository::new_id();
            tx.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,?,?,'direct_task_provenance',?)", params![capture_id, text, at, at]).map_err(|e| e.to_string())?;
            Some(capture_id)
        } else {
            let origin_id = origin.expect("origin is present when direct input is absent");
            let visible: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM captures c WHERE c.id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='captures' AND d.entity_id=c.id))", [origin_id], |row| row.get(0)).map_err(|error| error.to_string())?;
            if !visible {
                return Err("Capture not found".into());
            }
            origin.map(str::to_owned)
        };
        let result = self.create_task_tx(tx, input, provenance.as_deref(), None, at)?;
        self.record_task_activity(
            tx,
            result["id"].as_str().unwrap_or_default(),
            "created",
            input,
            at,
        )?;
        Ok(result)
    }

    fn problem_tx(
        &self,
        tx: &Transaction<'_>,
        input: &Value,
        id: Option<&str>,
        at: &str,
    ) -> Result<Value, String> {
        let result = self.create_problem_revision_tx(tx, input, id, at)?;
        self.record_problem_activity(
            tx,
            result["id"].as_str().unwrap_or_default(),
            "revised",
            input,
            at,
        )?;
        Ok(result)
    }

    fn task_revision_tx(
        &self,
        tx: &Transaction<'_>,
        id: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let revision = self.expected(tx, id, input)?;
        let patch = input
            .get("patch")
            .and_then(Value::as_object)
            .ok_or("invalid_input: patch is required")?;
        let prior: (String, String, String, String, String, String) = tx.query_row(
            "SELECT title,detail,outcome,scope,non_goals,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",
            params![id, revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).map_err(|e| e.to_string())?;
        let field = |key: &str, old: String| {
            patch
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or(&old)
                .trim()
                .to_owned()
        };
        let fields = [
            field("title", prior.0),
            field("detail", prior.1),
            field("outcome", prior.2),
            field("scope", prior.3),
            field("nonGoals", prior.4),
            field("validationCriteria", prior.5),
        ];
        if fields[0].is_empty() {
            return Err("invalid_input: title is required".into());
        }
        tx.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)", params![id, revision + 1, fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], content_hash(&fields.iter().map(String::as_str).collect::<Vec<_>>()), at]).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE tasks SET current_revision=? WHERE id=?",
            params![revision + 1, id],
        )
        .map_err(|e| e.to_string())?;
        self.record_task_activity(tx, id, "revised", input, at)?;
        Ok(
            json!({"id":id,"taskRevision":revision + 1,"title":fields[0],"detail":fields[1],"outcome":fields[2],"scope":fields[3],"nonGoals":fields[4],"validationCriteria":fields[5]}),
        )
    }

    fn task_transition_tx(
        &self,
        tx: &Transaction<'_>,
        id: &str,
        input: &Value,
        forced_to: Option<&str>,
        at: &str,
    ) -> Result<Value, String> {
        let revision = self.expected(tx, id, input)?;
        let from: String = tx
            .query_row("SELECT state FROM tasks WHERE id=?", [id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        let requested_state = match forced_to {
            Some(state) => state,
            None => req(input, "to")?,
        };
        let state = validate_state_transition(&from, requested_state)?;
        if state == "completed" {
            return self.complete_tx(tx, id, input, at);
        }
        if from == "completed" && state == "in_progress" {
            crate::native::task_hierarchy::reopen_ancestors(
                tx,
                id,
                req(input, "operationId")?,
                at,
            )?;
        }
        tx.execute("UPDATE tasks SET state=?,completed_at=CASE WHEN ?='in_progress' THEN NULL ELSE completed_at END,started_at=CASE WHEN ?='in_progress' AND started_at IS NULL THEN ? ELSE started_at END,reopened_at=CASE WHEN ?='in_progress' AND ?='completed' THEN ? ELSE reopened_at END WHERE id=?", params![state, state, state, at, state, from, at, id]).map_err(|e| e.to_string())?;
        self.record_task_activity(tx, id, "transition", input, at)?;
        Ok(json!({"id":id,"taskRevision":revision,"state":state}))
    }

    fn problem_link_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        self.expected(tx, task, input)?;
        let problem = req(input, "problemId")?;
        let revision = input
            .get("problemRevision")
            .and_then(Value::as_i64)
            .ok_or("invalid_input: problemRevision is required")?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM problem_revisions WHERE problem_id=? AND revision=?)",
                params![problem, revision],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err("Problem revision not found".into());
        }
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES(?,?,?,?,?,?,?)", params![id, task, problem, revision, input.get("relationship").and_then(Value::as_str).unwrap_or("context"), input.get("note").and_then(Value::as_str).unwrap_or(""), at]).map_err(|e| e.to_string())?;
        self.record_task_activity(tx, task, "problem_link", input, at)?;
        Ok(
            json!({"id":id,"taskId":task,"problemId":problem,"problemRevision":revision,"createdAt":at}),
        )
    }

    fn problem_link_delete_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        id: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        if tx.execute("UPDATE task_problem_links SET unlinked_at=? WHERE id=? AND task_id=? AND unlinked_at IS NULL", params![at, id, task]).map_err(|e| e.to_string())? == 0 { return Err("Link not found".into()); }
        self.record_task_activity(tx, task, "problem_link_deleted", input, at)?;
        Ok(json!({"id":id,"taskId":task,"unlinkedAt":at}))
    }

    fn relationship_tx(
        &self,
        tx: &Transaction<'_>,
        source: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        self.expected(tx, source, input)?;
        let result = task_repository::add_relationship_tx(tx, source, input, at)?;
        let related_source = result["sourceTaskId"].as_str().unwrap_or(source);
        self.record_task_activity(tx, related_source, "relationship", input, at)?;
        let target = result["targetTaskId"].as_str().unwrap_or_default();
        if target != related_source {
            self.record_task_activity(tx, target, "relationship", input, at)?;
        }
        Ok(result)
    }

    fn relationship_delete_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        id: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let endpoints: Option<(String, String)> = tx.query_row("SELECT source_task_id,target_task_id FROM task_relationships WHERE id=? AND (source_task_id=? OR target_task_id=?) AND unlinked_at IS NULL", params![id, task, task], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|e| e.to_string())?;
        let (source, target) = endpoints.ok_or("Link not found")?;
        tx.execute(
            "UPDATE task_relationships SET unlinked_at=? WHERE id=? AND unlinked_at IS NULL",
            params![at, id],
        )
        .map_err(|e| e.to_string())?;
        self.record_task_activity(tx, &source, "relationship_deleted", input, at)?;
        if target != source {
            self.record_task_activity(tx, &target, "relationship_deleted", input, at)?;
        }
        Ok(json!({"id":id,"taskId":task,"unlinkedAt":at}))
    }

    fn readiness_decision_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let revision = self.expected(tx, task, input)?;
        let status = req(input, "status")?;
        if !["not_applicable", "calculated"].contains(&status) {
            return Err("invalid_input: status".into());
        }
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_readiness_decisions(id,task_id,task_revision,field_key,status,reason,evidence_refs_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?,?,?)", params![id, task, revision, req(input, "key")?, status, input.get("reason").and_then(Value::as_str).unwrap_or(""), input.get("evidenceRefs").cloned().unwrap_or(json!([])).to_string(), req(input, "operationId")?, at]).map_err(|e| e.to_string())?;
        self.record_task_activity(tx, task, "readiness_decision", input, at)?;
        Ok(json!({"id":id,"taskId":task,"taskRevision":revision,"createdAt":at}))
    }

    fn work_log_create_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        self.expected(tx, task, input)?;
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,image_data,image_media_type,image_summary,created_at) VALUES(?,?,?,?,?,?,?)", params![id, task, input.get("body").and_then(Value::as_str).unwrap_or(""), input.get("imageData").and_then(Value::as_str).unwrap_or(""), input.get("imageMediaType").and_then(Value::as_str).unwrap_or(""), input.get("imageSummary").and_then(Value::as_str).unwrap_or(""), at]).map_err(|e| e.to_string())?;
        if let Some(attachment) = input.get("attachment") {
            tx.execute("INSERT INTO task_attachments(id,task_id,entry_id,name,media_type,data,byte_hash,created_at) VALUES(?,?,?,?,?,?,?,?)", params![task_repository::new_id(), task, id, attachment.get("name").and_then(Value::as_str).unwrap_or(""), attachment.get("mediaType").and_then(Value::as_str).unwrap_or(""), attachment.get("data").and_then(Value::as_str).unwrap_or(""), content_hash(&[attachment.get("data").and_then(Value::as_str).unwrap_or("")]), at]).map_err(|e| e.to_string())?;
        }
        self.record_task_activity(tx, task, "work_log", input, at)?;
        Ok(
            json!({"id":id,"taskId":task,"body":input.get("body").cloned().unwrap_or(json!("")),"createdAt":at}),
        )
    }

    fn comment_tx(
        &self,
        tx: &Transaction<'_>,
        entry: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let task: String = tx
            .query_row(
                "SELECT task_id FROM task_work_log_entries WHERE id=?",
                [entry],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Work log entry not found")?;
        let id = task_repository::new_id();
        tx.execute(
            "INSERT INTO task_work_log_comments(id,entry_id,body,created_at) VALUES(?,?,?,?)",
            params![id, entry, req(input, "body")?, at],
        )
        .map_err(|e| e.to_string())?;
        self.record_task_activity(tx, &task, "work_log_comment", input, at)?;
        Ok(json!({"id":id,"entryId":entry,"taskId":task,"createdAt":at}))
    }

    fn checklist_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        self.expected(tx, task, input)?;
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_checklist_items(id,task_id,body,checked,created_at,updated_at) VALUES(?,?,?,0,?,?)", params![id, task, req(input, "body")?, at, at]).map_err(|e| e.to_string())?;
        self.record_task_activity(tx, task, "checklist", input, at)?;
        Ok(json!({"id":id,"taskId":task,"body":input["body"],"checked":false,"createdAt":at}))
    }

    fn checklist_update_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        item: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        self.expected(tx, task, input)?;
        if tx.execute("UPDATE task_checklist_items SET checked=COALESCE(?,checked),updated_at=? WHERE id=? AND task_id=?", params![input.get("checked").and_then(Value::as_bool).map(i64::from), at, item, task]).map_err(|e| e.to_string())? == 0 { return Err("Checklist item not found".into()); }
        self.record_task_activity(tx, task, "checklist", input, at)?;
        Ok(json!({"id":item,"taskId":task,"updatedAt":at}))
    }

    fn decision_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let revision = self.expected(tx, task, input)?;
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?)", params![id, task, revision, req(input, "kind")?, input.get("payload").cloned().unwrap_or(json!({})).to_string(), req(input, "operationId")?, at]).map_err(|e| e.to_string())?;
        self.record_task_activity(tx, task, "decision", input, at)?;
        Ok(json!({"id":id,"taskId":task,"taskRevision":revision,"createdAt":at}))
    }

    fn complete_tx(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let revision = self.expected(tx, task, input)?;
        let state: String = tx
            .query_row("SELECT state FROM tasks WHERE id=?", [task], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        if state != "in_progress" {
            return Err("transition_invalid: task must be in_progress before completion".into());
        }
        crate::native::task_hierarchy::ensure_children_completed(tx, task)?;
        let id = task_repository::new_id();
        tx.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES(?,?,?,?,?,?,?)", params![id, task, revision, req(input, "evidence")?, input.get("report").and_then(Value::as_str).unwrap_or(""), req(input, "operationId")?, at]).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE tasks SET state='completed',completed_at=? WHERE id=?",
            params![at, task],
        )
        .map_err(|e| e.to_string())?;
        self.record_task_activity(tx, task, "completed", input, at)?;
        crate::native::task_hierarchy::child_completed(
            tx,
            task,
            &id,
            req(input, "operationId")?,
            at,
        )?;
        Ok(
            json!({"id":id,"taskId":task,"taskRevision":revision,"state":"completed","createdAt":at}),
        )
    }

    fn resolve_problem_tx(
        &self,
        tx: &Transaction<'_>,
        problem: &str,
        input: &Value,
        at: &str,
    ) -> Result<Value, String> {
        let expected = input
            .get("expectedProblemRevision")
            .and_then(Value::as_i64)
            .ok_or("invalid_input: expectedProblemRevision is required")?;
        let current: i64 = tx
            .query_row(
                "SELECT current_revision FROM problems WHERE id=?",
                [problem],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Problem not found")?;
        if expected != current {
            return Err(format!("head_conflict: currentRevision={current}"));
        }
        let id = task_repository::new_id();
        tx.execute("INSERT INTO problem_resolution_decisions(id,problem_id,problem_revision,rationale,evidence_refs_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?)", params![id, problem, current, req(input, "rationale")?, input.get("evidenceRefs").cloned().unwrap_or(json!([])).to_string(), req(input, "operationId")?, at]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE problems SET state='resolved' WHERE id=?", [problem])
            .map_err(|e| e.to_string())?;
        self.record_problem_activity(tx, problem, "resolved", input, at)?;
        Ok(
            json!({"id":id,"problemId":problem,"problemRevision":current,"state":"resolved","createdAt":at}),
        )
    }

    fn record_task_activity(
        &self,
        tx: &Transaction<'_>,
        task: &str,
        operation: &str,
        input: &Value,
        at: &str,
    ) -> Result<(), String> {
        task_repository::record_activity_tx(
            tx,
            "task",
            task,
            operation,
            req(input, "operationId")?,
            at,
        )
    }

    fn record_problem_activity(
        &self,
        tx: &Transaction<'_>,
        problem: &str,
        operation: &str,
        input: &Value,
        at: &str,
    ) -> Result<(), String> {
        task_repository::record_activity_tx(
            tx,
            "problem",
            problem,
            operation,
            req(input, "operationId")?,
            at,
        )
    }

    pub(crate) fn create_task_tx(
        &self,
        tx: &Transaction<'_>,
        input: &Value,
        origin: Option<&str>,
        id: Option<&str>,
        at: &str,
    ) -> Result<Value, String> {
        task_repository::create_task_tx(tx, input, origin, id, at)
    }
    pub(crate) fn create_problem_revision_tx(
        &self,
        tx: &Transaction<'_>,
        input: &Value,
        id: Option<&str>,
        at: &str,
    ) -> Result<Value, String> {
        task_repository::create_problem_revision_tx(tx, input, id, at)
    }
    fn op(&self, tx: &Transaction<'_>, input: &Value, name: &str) -> Result<Option<Value>, String> {
        let id = req(input, "operationId")?;
        let hash = content_hash(&[name, &input.to_string()]);
        if let Some((old, body)) = tx
            .query_row(
                "SELECT payload_hash,result_json FROM task_operation_results WHERE operation_id=?",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            if old != hash {
                return Err("operation_conflict".into());
            }
            return Ok(Some(serde_json::from_str(&body).unwrap_or(Value::Null)));
        };
        let _ = name;
        Ok(None)
    }
    fn finish(
        &self,
        tx: &Transaction<'_>,
        input: &Value,
        name: &str,
        result: &Value,
    ) -> Result<(), String> {
        tx.execute("INSERT INTO task_operation_results(operation_id,operation,payload_hash,result_json,created_at) VALUES(?,?,?,?,?)",params![req(input,"operationId")?,name,content_hash(&[name,&input.to_string()]),result.to_string(),task_repository::now()]).map_err(|e|e.to_string())?;
        Ok(())
    }
    fn capture(&self, input: &Value) -> Result<Value, String> {
        let images = crate::adapters::sqlite::input_images::validate_all(input)?;
        let text = input["text"].as_str().unwrap_or("").trim();
        if text.is_empty() && images.is_empty() {
            return Err("invalid_input: text or image is required".into());
        }
        self.repo.transaction(|tx| {
            if let Some(result) = self.op(tx, input, "capture.create")? { return Ok(result); }
            let id = task_repository::new_id();
            let at = task_repository::now();
            tx.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,?,?,'capture',?)", params![id,text,at,at]).map_err(|e|e.to_string())?;
            for image in &images { crate::adapters::sqlite::input_images::save(tx, true, &id, image)?; }
            let result = json!({"id":id,"text":text,"createdAt":at});
            self.finish(tx, input, "capture.create", &result)?;
            Ok(result)
        })
    }
    fn expected(&self, tx: &Transaction<'_>, id: &str, input: &Value) -> Result<i64, String> {
        let current: i64 = tx
            .query_row(
                "SELECT current_revision FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=tasks.id)",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Task not found")?;
        let expected = input
            .get("expectedTaskRevision")
            .and_then(Value::as_i64)
            .ok_or("invalid_input: expectedTaskRevision is required")?;
        if current != expected {
            return Err(format!("head_conflict: currentRevision={current}"));
        }
        Ok(current)
    }
    fn readiness(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        Self::readiness_on(&c, id)
    }
    pub(crate) fn readiness_on(c: &rusqlite::Connection, id: &str) -> Result<Value, String> {
        let revision: i64 = c
            .query_row(
                "SELECT current_revision FROM tasks WHERE id=?",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Task not found")?;
        let fields: (String, String, String) = c.query_row("SELECT outcome,scope,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?", params![id, revision], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).map_err(|e| e.to_string())?;
        let values = [fields.0, fields.1, fields.2];
        let mut entries = Vec::new();
        for (index, key) in READINESS_FIELDS.iter().enumerate() {
            let override_row: Option<(String, String, String)> = c.query_row("SELECT status,reason,evidence_refs_json FROM task_readiness_decisions WHERE task_id=? AND task_revision=? AND field_key=? ORDER BY created_at DESC LIMIT 1", params![id, revision, key], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(|e| e.to_string())?;
            let (status, reason, evidence) = if let Some((status, reason, evidence)) = override_row
            {
                (
                    if status == "not_applicable" {
                        "not_applicable"
                    } else {
                        "missing"
                    }
                    .to_owned(),
                    reason,
                    evidence,
                )
            } else if *key == "prerequisites" {
                (
                    "resolved".into(),
                    "No explicit prerequisite".into(),
                    "[]".into(),
                )
            } else if !values[index].is_empty() {
                ("resolved".into(), "".into(), "[]".into())
            } else {
                ("missing".into(), "".into(), "[]".into())
            };
            entries.push(json!({"key":key,"status":status,"reason":reason,"evidenceRefs":serde_json::from_str::<Value>(&evidence).unwrap_or(json!([])),"sourceRevision":revision,"provenance":"calculated"}));
        }
        Ok(json!({"taskId":id,"taskRevision":revision,"entries":entries}))
    }
    fn work_log(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let mut statement = c.prepare("SELECT id,body,image_data,image_media_type,image_summary,created_at FROM task_work_log_entries WHERE task_id=? ORDER BY created_at").map_err(|e| e.to_string())?;
        let mut entries = statement.query_map([id], |row| Ok(json!({"id":row.get::<_, String>(0)?,"body":row.get::<_, String>(1)?,"imageData":row.get::<_, String>(2)?,"imageMediaType":row.get::<_, String>(3)?,"imageSummary":row.get::<_, String>(4)?,"createdAt":row.get::<_, String>(5)?}))).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        drop(statement);
        for entry in &mut entries {
            let entry_id = entry["id"].as_str().unwrap_or_default();
            let execution=c.query_row("SELECT id,session_id,status,provider,model,final_report,work_log_sync_state,error_message FROM task_work_session_runs WHERE work_log_entry_id=? AND task_id=?",params![entry_id,id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,Option<String>>(5)?,row.get::<_,String>(6)?,row.get::<_,Option<String>>(7)?))).optional().map_err(|e|e.to_string())?;
            if let Some(run) = execution {
                let mut evidence_statement=c.prepare("SELECT provider_item_id,kind,status,content_json,created_at FROM task_work_session_run_items WHERE run_id=? ORDER BY provider_order,provider_item_id LIMIT 20").map_err(|e|e.to_string())?;
                let evidence=evidence_statement.query_map([&run.0],|row|{let raw:String=row.get(3)?;let item:Value=serde_json::from_str(&raw).unwrap_or(json!({}));let summary=item.get("text").or_else(||item.get("aggregatedOutput")).or_else(||item.get("command")).and_then(Value::as_str).unwrap_or(item.get("type").and_then(Value::as_str).unwrap_or("Completed provider item"));Ok(json!({"id":row.get::<_,String>(0)?,"kind":row.get::<_,String>(1)?,"status":row.get::<_,String>(2)?,"label":item.get("type").and_then(Value::as_str).unwrap_or("item"),"summary":summary.chars().take(1000).collect::<String>(),"command":item.get("command"),"paths":item.get("changes"),"exitCode":item.get("exitCode"),"createdAt":row.get::<_,String>(4)?}))}).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
                let artifacts = evidence
                    .iter()
                    .flat_map(|item| {
                        item.get("paths")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                    })
                    .filter_map(|path| {
                        path.get("path")
                            .or_else(|| path.as_str().map(|_| path))
                            .and_then(Value::as_str)
                    })
                    .take(20)
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                let report = run
                    .5
                    .map(|text| text.chars().take(4000).collect::<String>());
                let mut limitations = Vec::new();
                if report.is_none() {
                    limitations.push(
                        "A model-authored final report is unavailable for this Run.".to_owned(),
                    );
                }
                if let Some(error) = run.7 {
                    limitations.push(error.chars().take(1000).collect());
                }
                entry["execution"] = json!({"runId":run.0,"sessionId":run.1,"status":run.2,"provider":run.3,"model":run.4,"reportExcerpt":report,"evidence":evidence,"artifacts":artifacts,"limitations":limitations,"syncState":run.6});
            }
        }
        Ok(json!({"entries":entries}))
    }
    fn task_exists(tx: &Transaction<'_>, task_id: &str) -> Result<(), String> {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?))",
            params![task_id, task_id], |row| row.get(0),
        ).map_err(|error| error.to_string())?;
        if exists {
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    fn work_sessions(&self, task_id: &str) -> Result<Value, String> {
        let connection = crate::native::database::open(self.repo.path())?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        Self::task_exists(&tx, task_id)?;
        let mut statement = tx.prepare("SELECT id,title,provider,model,approval_mode,workspace_path,created_at,updated_at,approvals_reviewer FROM task_work_sessions WHERE task_id=? ORDER BY updated_at DESC,id")
            .map_err(|error| error.to_string())?;
        let sessions = statement
            .query_map([task_id], |row| {
                Ok(json!({
                    "id":row.get::<_,String>(0)?, "taskId":task_id, "title":row.get::<_,String>(1)?,
                    "provider":row.get::<_,String>(2)?, "model":row.get::<_,String>(3)?,
                    "approvalMode":row.get::<_,String>(4)?, "workspacePath":row.get::<_,String>(5)?,
                    "createdAt":row.get::<_,String>(6)?, "updatedAt":row.get::<_,String>(7)?,
                    "approvalsReviewer":row.get::<_,String>(8)?
                }))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        drop(statement);
        tx.commit().map_err(|error| error.to_string())?;
        Ok(json!({"sessions":sessions}))
    }

    fn work_session(&self, task_id: &str, session_id: &str) -> Result<Value, String> {
        let connection = crate::native::database::open(self.repo.path())?;
        let task_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?))",
            params![task_id,task_id], |row| row.get(0),
        ).map_err(|error| error.to_string())?;
        if !task_exists {
            return Err("Task not found".into());
        }
        let session = connection.query_row(
            "SELECT title,provider,model,approval_mode,workspace_path,created_at,updated_at,approvals_reviewer FROM task_work_sessions WHERE id=? AND task_id=?",
            params![session_id,task_id], |row| Ok(json!({
                "id":session_id,"taskId":task_id,"title":row.get::<_,String>(0)?,
                "provider":row.get::<_,String>(1)?,"model":row.get::<_,String>(2)?,
                "approvalMode":row.get::<_,String>(3)?,"workspacePath":row.get::<_,String>(4)?,
                "createdAt":row.get::<_,String>(5)?,"updatedAt":row.get::<_,String>(6)?,
                "approvalsReviewer":row.get::<_,String>(7)?
            })),
        ).optional().map_err(|error| error.to_string())?.ok_or("Work session not found")?;
        let mut statement = connection.prepare("SELECT id,author,kind,body,attachment_json,created_at FROM task_work_session_entries WHERE session_id=? ORDER BY created_at,id")
            .map_err(|error| error.to_string())?;
        let entries = statement.query_map([session_id], |row| {
            let attachment: Option<String> = row.get(4)?;
            Ok(json!({"id":row.get::<_,String>(0)?,"author":row.get::<_,String>(1)?,"kind":row.get::<_,String>(2)?,"body":row.get::<_,String>(3)?,"attachment":attachment.and_then(|value| serde_json::from_str::<Value>(&value).ok()),"createdAt":row.get::<_,String>(5)?}))
        }).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
        Ok(json!({"session":session,"entries":entries}))
    }

    fn work_session_create(&self, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            if let Some(result) = self.op(tx,input,"task.work-session.create")? { return Ok(result); }
            let task_id=req(input,"taskId")?; Self::task_exists(tx,task_id)?;
            let id=task_repository::new_id(); let at=task_repository::now();
            let title=req(input,"title")?;
            if title.trim().is_empty() || title.len()>200 { return Err("invalid_input: work session title".into()); }
            tx.execute("INSERT INTO task_work_sessions(id,task_id,title,created_at,updated_at) VALUES(?,?,?,?,?)",params![id,task_id,title,at,at]).map_err(|error|error.to_string())?;
            let result=json!({"id":id,"taskId":task_id,"title":title,"provider":"codex","model":"gpt-5.6-sol","approvalMode":"ask","approvalsReviewer":"user","workspacePath":"","createdAt":at,"updatedAt":at});
            self.finish(tx,input,"task.work-session.create",&result)?; Ok(result)
        })
    }

    fn work_session_update(&self, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            if let Some(result)=self.op(tx,input,"task.work-session.update")? { return Ok(result); }
            let task_id=req(input,"taskId")?; let session_id=req(input,"sessionId")?; Self::task_exists(tx,task_id)?;
            let approval=input.get("approvalMode").and_then(Value::as_str).unwrap_or("ask");
            if !["ask","auto"].contains(&approval) { return Err("invalid_input: approvalMode".into()); }
            let title=req(input,"title")?; let provider=input.get("provider").and_then(Value::as_str).unwrap_or("").trim();
            let model=input.get("model").and_then(Value::as_str).unwrap_or("").trim();
            let workspace=input.get("workspacePath").and_then(Value::as_str).unwrap_or("").trim();
            let reviewer=input.get("approvalsReviewer").and_then(Value::as_str).unwrap_or("user");
            if !["user","auto_review"].contains(&reviewer) { return Err("invalid_input: approvalsReviewer".into()); }
            if title.len()>200 || provider.len()>200 || model.len()>200 || workspace.len()>4096 { return Err("invalid_input: work session setting is too long".into()); }
            if provider != "codex" || !["gpt-6-astra","gpt-5.6-sol","gpt-5.6-terra","gpt-5.6-luna","gpt-5.5"].contains(&model) { return Err("invalid_input: unsupported provider or model".into()); }
            let at=task_repository::now();
            let changed=tx.execute("UPDATE task_work_sessions SET title=?,provider=?,model=?,approval_mode=?,approvals_reviewer=?,workspace_path=?,effective_settings_revision=NULL,updated_at=? WHERE id=? AND task_id=?",params![title,provider,model,approval,reviewer,workspace,at,session_id,task_id]).map_err(|error|error.to_string())?;
            if changed!=1 { return Err("Work session not found".into()); }
            let result=json!({"id":session_id,"taskId":task_id,"title":title,"provider":provider,"model":model,"approvalMode":approval,"approvalsReviewer":reviewer,"workspacePath":workspace,"updatedAt":at});
            self.finish(tx,input,"task.work-session.update",&result)?; Ok(result)
        })
    }

    fn work_session_entry(&self, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            if let Some(result)=self.op(tx,input,"task.work-session.entry.create")? { return Ok(result); }
            let task_id=req(input,"taskId")?; let session_id=req(input,"sessionId")?; Self::task_exists(tx,task_id)?;
            let owned:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_work_sessions WHERE id=? AND task_id=?)",params![session_id,task_id],|row|row.get(0)).map_err(|error|error.to_string())?;
            if !owned { return Err("Work session not found".into()); }
            let author=input.get("author").and_then(Value::as_str).unwrap_or("user");
            let kind=input.get("kind").and_then(Value::as_str).unwrap_or("note");
            if !["user","assistant","system"].contains(&author) || !["note","ai_output","execution_result"].contains(&kind) { return Err("invalid_input: entry type".into()); }
            let body=input.get("body").and_then(Value::as_str).unwrap_or("").trim();
            let attachment=input.get("attachment").filter(|value|!value.is_null());
            if body.is_empty() && attachment.is_none() { return Err("invalid_input: body or attachment is required".into()); }
            let attachment_json=if let Some(value)=attachment {
                let name=req(value,"name")?; let media=req(value,"mediaType")?; let data=req(value,"data")?;
                let decoded=STANDARD.decode(data).map_err(|_|"invalid_input: attachment data is not valid base64")?;
                if name.len()>255 || media.len()>200 || decoded.len()>10*1024*1024 { return Err("invalid_input: attachment is too large".into()); }
                Some(json!({"name":name,"mediaType":media,"data":data}).to_string())
            } else { None };
            let id=task_repository::new_id(); let at=task_repository::now();
            tx.execute("INSERT INTO task_work_session_entries(id,session_id,author,kind,body,attachment_json,created_at) VALUES(?,?,?,?,?,?,?)",params![id,session_id,author,kind,body,attachment_json,at]).map_err(|error|error.to_string())?;
            tx.execute("UPDATE task_work_sessions SET updated_at=? WHERE id=? AND task_id=?",params![at,session_id,task_id]).map_err(|error|error.to_string())?;
            let result=json!({"id":id,"sessionId":session_id,"author":author,"kind":kind,"body":body,"attachment":attachment,"createdAt":at});
            self.finish(tx,input,"task.work-session.entry.create",&result)?; Ok(result)
        })
    }
    fn get(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let deleted: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?)", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
        if deleted {
            return Err("Task not found".into());
        }
        let mut x=c.query_row("SELECT t.current_revision,t.state,r.title,r.detail,r.outcome,r.scope,r.non_goals,r.validation_criteria,t.category,t.created_at,t.last_user_activity_at FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=?",[id],|r|Ok(json!({"id":id,"taskRevision":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"detail":r.get::<_,String>(3)?,"outcome":r.get::<_,String>(4)?,"scope":r.get::<_,String>(5)?,"nonGoals":r.get::<_,String>(6)?,"validationCriteria":r.get::<_,String>(7)?,"category":r.get::<_,String>(8)?,"createdAt":r.get::<_,String>(9)?,"lastUserActivityAt":r.get::<_,String>(10)?}))).optional().map_err(|x|x.to_string())?.ok_or("Task not found")?;
        x["contentVersions"] = crate::native::localization::task_versions(&c, id)?;
        x["originCapture"] = c.query_row(
            "SELECT c.id,c.text,c.created_at FROM captures c JOIN tasks t ON t.origin_capture_id=c.id WHERE t.id=?",
            [id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"text":r.get::<_,String>(1)?,"createdAt":r.get::<_,String>(2)?})),
        ).optional().map_err(|e| e.to_string())?.unwrap_or(Value::Null);
        for (key, value) in crate::native::task_hierarchy::metadata(&c, id)?
            .as_object()
            .ok_or("invalid metadata")?
        {
            x[key] = value.clone();
        }
        x["hierarchy"] = crate::native::task_hierarchy::context(&c, id)?;
        let mut work_log = self.work_log(id)?["entries"].clone();
        if let Some(entries) = work_log.as_array_mut() {
            for entry in entries {
                let entry_id = entry["id"].as_str().unwrap_or("");
                let attachment=c.query_row("SELECT name,media_type,data FROM task_attachments WHERE entry_id=? LIMIT 1",[entry_id],|r|Ok(json!({"name":r.get::<_,String>(0)?,"mediaType":r.get::<_,String>(1)?,"data":r.get::<_,String>(2)?}))).optional().map_err(|e|e.to_string())?;
                let localized = crate::native::localization::overlay(
                    &c,
                    "task_work_log_entries",
                    json!({"id":entry_id}),
                    "en",
                )?;
                let summary_job = c.query_row("SELECT id,status,error_message FROM ai_jobs_v2 WHERE task_kind='image_summary' AND entity_type='task_work_log_entries' AND entity_id=? ORDER BY rowid DESC LIMIT 1", [entry_id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"error":r.get::<_,String>(2)?}))).optional().map_err(|e|e.to_string())?;
                let translation_job = c.query_row("SELECT id,status,error_message FROM ai_jobs_v2 WHERE task_kind='derived_translation' AND entity_type='task_work_log_entries' AND entity_id=? ORDER BY rowid DESC LIMIT 1", [entry_id], |r| Ok(json!({"id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"error":r.get::<_,String>(2)?}))).optional().map_err(|e|e.to_string())?;
                let comments=c.prepare("SELECT id,body,created_at FROM task_work_log_comments WHERE entry_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([entry_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"body":r.get::<_,String>(1)?,"createdAt":r.get::<_,String>(2)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
                if let Some(o) = entry.as_object_mut() {
                    if let Some(a) = attachment {
                        o.insert("attachment".into(), a);
                    }
                    o.insert(
                        "bodyVersions".into(),
                        localized["localized_versions"].clone(),
                    );
                    if let Some(job) = translation_job {
                        o.insert("translationJob".into(), job);
                    }
                    o.insert(
                        "imageSummaryVersions".into(),
                        localized["localized_versions"].clone(),
                    );
                    if let Some(job) = summary_job {
                        o.insert("imageSummaryJob".into(), job);
                    }
                    o.insert("comments".into(), json!(comments));
                }
            }
        }
        let checklist = c.prepare("SELECT id,body,checked FROM task_checklist_items WHERE task_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"body":r.get::<_,String>(1)?,"checked":r.get::<_,i64>(2)? != 0}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let decisions = c.prepare("SELECT id,kind,payload_json,created_at FROM task_decisions WHERE task_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"body":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).ok().and_then(|v|v.get("body").cloned()).unwrap_or(Value::Null),"createdAt":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let completion = c.query_row("SELECT id,evidence,report,created_at FROM task_completions WHERE task_id=? ORDER BY created_at DESC LIMIT 1",[id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"evidence":r.get::<_,String>(1)?,"report":r.get::<_,String>(2)?,"createdAt":r.get::<_,String>(3)?}))).optional().map_err(|e|e.to_string())?;
        let publication = c.query_row("SELECT revision,state,content_hash,lineage_json,body_markdown FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision DESC LIMIT 1",[id],|r| {
            let lineage: Value = serde_json::from_str(&r.get::<_,String>(3)?).unwrap_or(Value::Null);
            Ok(json!({"draftRevision":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,"contentHash":r.get::<_,String>(2)?,"sourceHash":lineage["sourceHash"],"lineage":lineage,"bodyMarkdown":r.get::<_,String>(4)?}))
        }).optional().map_err(|e|e.to_string())?;
        // A newer private draft must not hide the last published document. A
        // withdrawal suppresses earlier revisions of the same published file.
        let published_knowledge = c.query_row("SELECT revision,state,body_markdown FROM task_knowledge_drafts WHERE task_id=? AND state IN ('published','withdrawn') ORDER BY revision DESC LIMIT 1", [id], |r| {
            Ok((r.get::<_,String>(1)?, json!({"draftRevision":r.get::<_,i64>(0)?,"bodyMarkdown":r.get::<_,String>(2)?})))
        }).optional().map_err(|e|e.to_string())?;
        let problem_links=c.prepare("SELECT id,problem_id,problem_revision,relationship,note FROM task_problem_links WHERE task_id=? AND unlinked_at IS NULL").map_err(|e|e.to_string())?.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"problemId":r.get::<_,String>(1)?,"problemRevision":r.get::<_,i64>(2)?,"relationship":r.get::<_,String>(3)?,"note":r.get::<_,String>(4)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let relationships=c.prepare("SELECT id,CASE WHEN source_task_id=? THEN target_task_id ELSE source_task_id END,kind,note FROM task_relationships WHERE (source_task_id=? OR (kind='related' AND target_task_id=?)) AND unlinked_at IS NULL").map_err(|e|e.to_string())?.query_map(params![id,id,id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"targetTaskId":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?,"note":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let readiness = self.readiness(id)?;
        let object = x.as_object_mut().ok_or("invalid task")?;
        object.insert("workLog".into(), work_log);
        object.insert("checklist".into(), json!(checklist));
        object.insert("decisions".into(), json!(decisions));
        object.insert("problemLinks".into(), json!(problem_links));
        object.insert("relationships".into(), json!(relationships));
        object.insert("readinessEntries".into(), readiness["entries"].clone());
        if object.get("state").and_then(Value::as_str) == Some("completed") {
            if let Some(completion) = completion {
                object.insert("completion".into(), completion);
            }
        }
        let auto_error: Option<String> = c.query_row("SELECT error FROM task_auto_publications WHERE task_id=? AND state='pending' AND error!='' ORDER BY revision LIMIT 1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        object.insert("autoPublicationError".into(), json!(auto_error));
        if let Some((state, published)) = published_knowledge {
            if state == "published" {
                object.insert("publishedKnowledge".into(), published);
            }
        }
        if let Some(publication) = publication {
            object.insert("publication".into(), publication);
        }
        Ok(x)
    }
    fn delete(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let exists: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=tasks.id))", [id], |r| r.get(0)).map_err(|e| e.to_string())?;
        if !exists {
            return Err("Task not found".into());
        }
        let owned: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM task_subtasks WHERE child_task_id=? OR parent_task_id=?)",params![id,id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if owned {
            return Err("Task hierarchy must be retained; complete or reopen its Subtasks".into());
        }
        c.execute(
            "INSERT OR REPLACE INTO deleted_entities(entity_type,entity_id) VALUES ('tasks',?)",
            [id],
        )
        .map_err(|e| e.to_string())?;
        Ok(Value::Null)
    }

    fn workbench(&self) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let mut s=c.prepare("SELECT t.id,t.current_revision,t.state,r.title,t.category,t.last_user_activity_at,f.revision,h.parent_task_id,(SELECT c.text FROM captures c WHERE c.id=t.origin_capture_id),t.completed_at FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision LEFT JOIN task_refinements f ON f.task_id=t.id LEFT JOIN task_subtasks h ON h.child_task_id=t.id WHERE t.archived_at IS NULL AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=t.id) ORDER BY t.last_user_activity_at DESC").map_err(|x|x.to_string())?;
        let mut rows=s.query_map([],|r|Ok(json!({"kind":"task","id":r.get::<_,String>(0)?,"taskRevision":r.get::<_,i64>(1)?,"state":r.get::<_,String>(2)?,"title":r.get::<_,String>(3)?,"category":r.get::<_,String>(4)?,"lastUserActivityAt":r.get::<_,String>(5)?,"refinedRevision":r.get::<_,Option<i64>>(6)?,"parentTaskId":r.get::<_,Option<String>>(7)?,"originCaptureText":r.get::<_,Option<String>>(8)?,"completedAt":r.get::<_,Option<String>>(9)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?;
        let mut translations = crate::native::localization::current_task_versions(&c)?;
        for item in &mut rows {
            item["contentVersions"] = translations
                .remove(item["id"].as_str().unwrap_or_default())
                .unwrap_or_else(|| json!({}));
        }
        let mut captures=c.prepare("SELECT c.id,c.text,c.created_at,COALESCE(o.category,'General'),EXISTS(SELECT 1 FROM input_images i WHERE i.capture_id=c.id) FROM captures c LEFT JOIN workbench_category_overrides o ON o.entity_type='captures' AND o.entity_id=c.id WHERE c.source_mode='capture' AND c.id NOT IN (SELECT origin_capture_id FROM tasks WHERE origin_capture_id IS NOT NULL) AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='captures' AND d.entity_id=c.id) ORDER BY c.last_user_activity_at DESC").map_err(|x|x.to_string())?;
        rows.extend(captures.query_map([],|r|Ok(json!({"kind":"capture","id":r.get::<_,String>(0)?,"text":r.get::<_,String>(1)?,"lastUserActivityAt":r.get::<_,String>(2)?,"category":r.get::<_,String>(3)?,"hasImage":r.get::<_,bool>(4)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?);
        let mut legacy_refinements=c.prepare("SELECT i.id,i.problem_id,i.problem_revision,r.statement,i.source_kind,i.created_at,COALESCE(o.category,'General') FROM refinement_items i JOIN problem_revisions r ON r.problem_id=i.problem_id AND r.revision=i.problem_revision LEFT JOIN workbench_category_overrides o ON o.entity_type='problems' AND o.entity_id=i.problem_id WHERE NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='problems' AND d.entity_id=i.problem_id) ORDER BY i.created_at DESC").map_err(|x|x.to_string())?;
        rows.extend(legacy_refinements.query_map([],|r|Ok(json!({"kind":"refinement","id":r.get::<_,String>(0)?,"problemId":r.get::<_,String>(1)?,"problemRevision":r.get::<_,i64>(2)?,"title":r.get::<_,String>(3)?,"sourceKind":r.get::<_,String>(4)?,"lastUserActivityAt":r.get::<_,String>(5)?,"category":r.get::<_,String>(6)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?);
        rows.sort_by(|a, b| {
            b["lastUserActivityAt"]
                .as_str()
                .cmp(&a["lastUserActivityAt"].as_str())
        });
        let active = rows
            .iter()
            .filter(|x| x["state"] == "in_progress")
            .map(|x| json!({"kind":"task","id":x["id"],"taskRevision":x["taskRevision"]}))
            .collect::<Vec<_>>();
        let active_ids = active
            .iter()
            .filter_map(|x| x["id"].as_str())
            .collect::<Vec<_>>();
        let mut refining = Vec::new();
        if let Ok(mut sessions) = c.prepare("SELECT s.capture_id,COALESCE(s.task_id,(SELECT json_extract(d.result_json,'$.id') FROM refinement_proposal_decisions d WHERE d.session_id=s.id AND d.decision!='reject' AND json_extract(d.result_json,'$.taskRevision') IS NOT NULL AND NOT EXISTS(SELECT 1 FROM task_subtasks h WHERE h.child_task_id=json_extract(d.result_json,'$.id')) ORDER BY d.created_at DESC,d.rowid DESC LIMIT 1)),s.current_draft_revision FROM refinement_sessions s WHERE (s.capture_id IS NOT NULL OR s.task_id IS NOT NULL) AND s.state!='completed' AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='captures' AND d.entity_id=s.capture_id) AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=s.task_id) ORDER BY s.last_user_activity_at DESC") {
            let subjects = sessions.query_map([], |row| Ok((row.get::<_,Option<String>>(0)?, row.get::<_,Option<String>>(1)?, row.get::<_,i64>(2)?))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
            refining.extend(subjects.into_iter().filter_map(|(capture_id,task_id,revision)| {
                let (kind,id) = if let Some(id) = task_id { if active_ids.contains(&id.as_str()) { return None; } ("task",id) } else { ("capture",capture_id?) };
                Some(json!({"kind":kind,"id":id,"draftRevision":revision}))
            }));
        }
        let mut groups = std::collections::BTreeMap::<String, Vec<Value>>::new();
        for item in rows {
            let category = item["category"].as_str().unwrap_or("General").to_owned();
            groups.entry(category).or_default().push(item);
        }
        let categories = groups
            .into_iter()
            .map(|(id, items)| {
                let mut category = json!({"id":id,"label":id});
                category["items"] = Value::Array(items);
                category
            })
            .collect::<Vec<_>>();
        let mut result = json!({"revision":0});
        result["activeShortcuts"] = Value::Array(active);
        result["refiningShortcuts"] = Value::Array(refining);
        result["categories"] = Value::Array(categories);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::tempdir;

    #[test]
    fn direct_task_keeps_its_initial_text_in_detail_and_workbench() {
        let root = tempdir().unwrap();
        let db = root.path().join("task.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let initial = "Keep this exact first thought";
        let created = service
            .execute(
                "task.create",
                &json!({"operationId":"direct-task","inputText":initial,"title":"A Task"}),
            )
            .unwrap();
        let id = created["id"].as_str().unwrap();

        assert_eq!(service.get(id).unwrap()["originCapture"]["text"], initial);
        let snapshot = service.workbench().unwrap();
        let has_initial = snapshot["categories"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|category| category["items"].as_array().unwrap())
            .any(|item| item["id"] == id && item["originCaptureText"] == initial);
        assert!(has_initial);
    }

    #[test]
    fn capture_image_is_atomic_persistent_and_replayable() {
        let root = tempdir().unwrap();
        let db = root.path().join("images.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let image = json!({"name":"capture.png","mediaType":"image/png","data":"iVBORw0KGgo="});
        let input = json!({"operationId":"image-capture","text":"","image":image});
        let saved = service.execute("capture.create", &input).unwrap();
        assert_eq!(service.execute("capture.create", &input).unwrap(), saved);
        let connection = crate::native::database::open(&db).unwrap();
        assert_eq!(
            crate::adapters::sqlite::input_images::get(
                &connection,
                true,
                saved["id"].as_str().unwrap()
            )
            .unwrap(),
            Some(image)
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM input_images", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let mut invalid = input.clone();
        invalid["operationId"] = json!("invalid-image");
        invalid["image"]["mediaType"] = json!("image/svg+xml");
        assert!(service.execute("capture.create", &invalid).is_err());
        invalid["image"]["mediaType"] = json!("image/png");
        invalid["image"]["data"] = json!("not base64");
        assert!(service.execute("capture.create", &invalid).is_err());
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM captures", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(service
            .execute("capture.create", &json!({"operationId":"empty","text":""}))
            .is_err());
    }

    #[test]
    fn multiple_capture_images_validate_atomically_and_replay_without_duplicates() {
        let root = tempdir().unwrap();
        let db = root.path().join("images.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let images = json!([
            {"name":"one.png","mediaType":"image/png","data":"iVBORw0KGgo="},
            {"name":"two.png","mediaType":"image/png","data":"iVBORw0KGgo="}
        ]);
        let input = json!({"operationId":"multi-capture","text":"","images":images});
        let saved = service.execute("capture.create", &input).unwrap();
        assert_eq!(service.execute("capture.create", &input).unwrap(), saved);
        let connection = crate::native::database::open(&db).unwrap();
        assert_eq!(
            json!(crate::adapters::sqlite::input_images::get_all(
                &connection,
                true,
                saved["id"].as_str().unwrap()
            )
            .unwrap()),
            images
        );
        let mut invalid = input.clone();
        invalid["operationId"] = json!("invalid-batch");
        invalid["images"][1]["data"] = json!("not base64");
        assert!(service.execute("capture.create", &invalid).is_err());
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM captures", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM input_images", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        for images in [json!([]), json!([null]), json!({})] {
            assert!(service
                .execute(
                    "capture.create",
                    &json!({"operationId":"bad","text":"","images":images})
                )
                .is_err());
        }
    }

    fn link_owned_session(
        connection: &rusqlite::Connection,
        connection_id: &str,
        session_id: &str,
        capture_id: &str,
        event_id: &str,
        task_id: &str,
    ) {
        connection
            .execute(
                "INSERT INTO mcp_connections(id,name,scopes_json,allowed_topics_json,checkpoint_policy,state,created_at,updated_at)
                 VALUES(?,?, '[]','[]','confirm_each','active','2026-01-01','2026-01-01')",
                params![connection_id, connection_id],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO captures(id,text,source_mode,last_user_activity_at,created_at)
                 VALUES(?,?,'capture','2026-01-01','2026-01-01')",
                params![capture_id, capture_id],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_tracking_sessions(id,connection_id,source_interface,conversation_ref_hash,capture_id,head_event_id,head_revision,created_at,updated_at)
                 VALUES(?,?, 'mcp', ?,?,?,1,'2026-01-01','2026-01-01')",
                params![session_id, connection_id, format!("lineage-{session_id}"), capture_id, event_id],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at)
                 VALUES(?,?,1,'mcp',1,'task_binding','{}','binding','2026-01-01','2026-01-01')",
                params![event_id, session_id],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO work_tracking_links(id,session_id,source_event_id,entity_type,entity_id,relationship,created_at)
                 VALUES(?,?,?,'tasks',?,'adopted_task','2026-01-01')",
                params![format!("link-{session_id}"), session_id, event_id, task_id],
            )
            .unwrap();
    }

    #[test]
    fn task_lifecycle_preserves_problem_independence_and_rejects_stale_revision() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task = service
            .execute(
                "task.create",
                &json!({"operationId":"create","inputText":"input","title":"Ship"}),
            )
            .unwrap();
        let id = task["id"].as_str().unwrap();
        let problem = service
            .execute(
                "problem.create",
                &json!({"operationId":"problem","statement":"Context"}),
            )
            .unwrap();
        let pid = problem["id"].as_str().unwrap();
        service.execute("task.transition",&json!({"operationId":"start","taskId":id,"expectedTaskRevision":1,"to":"in_progress"})).unwrap();
        service.execute("task.completion.create",&json!({"operationId":"complete","taskId":id,"expectedTaskRevision":1,"evidence":"done"})).unwrap();
        let c = crate::native::database::open(&db).unwrap();
        let state: String = c
            .query_row("SELECT state FROM problems WHERE id=?", [pid], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "open");
        service.execute("task.revision",&json!({"operationId":"revise","taskId":id,"expectedTaskRevision":1,"patch":{"title":"Shipped"}})).unwrap();
        assert!(service.execute("task.revision",&json!({"operationId":"stale","taskId":id,"expectedTaskRevision":1,"patch":{"title":"Again"}})).unwrap_err().contains("head_conflict"));
    }

    #[test]
    fn task_get_includes_the_latest_private_knowledge_draft_body() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task = service
            .execute(
                "task.create",
                &json!({"operationId":"create","inputText":"input","title":"Review saved draft"}),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap();
        service.execute("task.transition", &json!({"operationId":"start","taskId":task_id,"expectedTaskRevision":1,"to":"in_progress"})).unwrap();
        service.execute("task.completion.create", &json!({"operationId":"complete","taskId":task_id,"expectedTaskRevision":1,"evidence":"done"})).unwrap();
        let repo = SqliteTaskRepository::new(&db);
        let saved = repo
            .transaction(|tx| {
                crate::native::task_assistance::save_supplied_knowledge_draft_tx(
                    tx,
                    &json!({"operationId":"saved-draft","taskId":task_id,"expectedTaskRevision":1}),
                    "# Saved Knowledge\n\nReview this private draft.",
                    "supplied",
                )
            })
            .unwrap();

        let loaded = service
            .execute("task.get", &json!({"taskId":task_id}))
            .unwrap();
        assert_eq!(loaded["publication"]["state"], "draft");
        assert_eq!(
            loaded["publication"]["bodyMarkdown"],
            "# Saved Knowledge\n\nReview this private draft."
        );
        assert!(loaded["publication"]["lineage"].is_object());
        assert_eq!(loaded["publication"]["lineage"], saved["lineage"]);
    }

    #[test]
    fn workbench_includes_all_saved_refinements_for_workflow_lanes() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let connection = crate::native::database::open(&db).unwrap();
        for index in 0..8 {
            let capture = service.execute("capture.create", &json!({"operationId":format!("capture-{index}"),"text":format!("Thought {index}")})).unwrap();
            connection.execute(
                "INSERT INTO refinement_sessions(id,capture_id,state,current_draft_revision,last_user_activity_at,updated_at) VALUES(?,?,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
                params![format!("session-{index}"), capture["id"].as_str().unwrap()],
            ).unwrap();
        }
        let board = service.execute("workbench.get", &json!({})).unwrap();
        assert_eq!(board["refiningShortcuts"].as_array().unwrap().len(), 8);
        connection
            .execute(
                "UPDATE refinement_sessions SET state='completed' WHERE id='session-0'",
                [],
            )
            .unwrap();
        let board = service.execute("workbench.get", &json!({})).unwrap();
        assert_eq!(board["refiningShortcuts"].as_array().unwrap().len(), 7);
    }

    #[test]
    fn workbench_projects_legacy_problem_only_refinement_without_inventing_task() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let problem = service
            .execute(
                "problem.create",
                &json!({"operationId":"legacy-problem","statement":"Preserve this migrated Problem"}),
            )
            .unwrap();
        let problem_id = problem["id"].as_str().unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        connection
            .execute(
                "INSERT INTO refinement_items(id,problem_id,problem_revision,source_kind,created_at) VALUES('legacy-only',?,1,'legacy_problem',CURRENT_TIMESTAMP)",
                [problem_id],
            )
            .unwrap();

        let workbench = service.execute("workbench.get", &json!({})).unwrap();
        let items = workbench["categories"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|category| category["items"].as_array().unwrap())
            .collect::<Vec<_>>();
        let refinement = items
            .iter()
            .find(|item| item["kind"] == "refinement")
            .unwrap();
        assert_eq!(refinement["problemId"], problem_id);
        assert_eq!(refinement["problemRevision"], 1);
        assert_eq!(refinement["title"], "Preserve this migrated Problem");
        assert!(!items.iter().any(|item| item["kind"] == "task"));

        connection
            .execute(
                "INSERT INTO deleted_entities(entity_type,entity_id) VALUES('problems',?)",
                [problem_id],
            )
            .unwrap();
        let hidden = service.execute("workbench.get", &json!({})).unwrap();
        assert!(hidden["categories"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|category| category["items"].as_array().unwrap())
            .all(|item| item["kind"] != "refinement"));
    }

    #[test]
    fn delete_tombstones_task_across_reopen_and_hides_every_workbench_shortcut() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task = service
            .execute(
                "task.create",
                &json!({"operationId":"delete-create","inputText":"keep source","title":"Delete me"}),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap();
        service
            .execute(
                "task.transition",
                &json!({"operationId":"delete-start","taskId":task_id,"expectedTaskRevision":1,"to":"in_progress"}),
            )
            .unwrap();
        service
            .execute(
                "task.work-log.create",
                &json!({"operationId":"delete-log","taskId":task_id,"expectedTaskRevision":1,"body":"retain this"}),
            )
            .unwrap();
        crate::native::database::open(&db)
            .unwrap()
            .execute(
                "INSERT INTO refinement_sessions(id,task_id,state,current_draft_revision,last_user_activity_at,updated_at) VALUES('deleted-task-session',?,'active',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
                [task_id],
            )
            .unwrap();

        service
            .execute("task.delete", &json!({"taskId":task_id}))
            .unwrap();
        assert!(service
            .execute("task.delete", &json!({"taskId":"missing"}))
            .unwrap_err()
            .contains("Task not found"));
        assert!(service
            .execute("task.get", &json!({"taskId":task_id}))
            .unwrap_err()
            .contains("Task not found"));

        // A new service and connection emulate the next desktop launch. The tombstone hides the
        // Task while its work log remains durable for audit and recovery tooling.
        let reloaded = TaskApplicationService::new(&db);
        let workbench = reloaded.execute("workbench.get", &json!({})).unwrap();
        assert!(workbench["activeShortcuts"].as_array().unwrap().is_empty());
        assert!(workbench["refiningShortcuts"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(workbench["categories"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|category| category["items"].as_array().unwrap())
            .all(|item| item["id"] != task_id));
        let connection = crate::native::database::open(&db).unwrap();
        assert_eq!(connection.query_row("SELECT count(*) FROM deleted_entities WHERE entity_type='tasks' AND entity_id=?", [task_id], |row| row.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM task_work_log_entries WHERE task_id=?",
                    [task_id],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn desktop_task_mutation_appends_one_applied_event_per_linked_session_and_replays_cleanly() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task = service
            .execute(
                "task.create",
                &json!({"operationId":"desktop-create","inputText":"direct","title":"Sync this Task"}),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        link_owned_session(
            &connection,
            "mcp-a",
            "session-a",
            "capture-a",
            "event-a",
            task_id,
        );
        link_owned_session(
            &connection,
            "mcp-b",
            "session-b",
            "capture-b",
            "event-b",
            task_id,
        );
        drop(connection);

        let input = json!({
            "operationId":"desktop-revision",
            "taskId":task_id,
            "expectedTaskRevision":1,
            "patch":{"title":"Synced Task"}
        });
        service.execute("task.revision", &input).unwrap();
        let replay = service.execute("task.revision", &input).unwrap();
        assert_eq!(replay["taskRevision"], 2);

        let connection = crate::native::database::open(&db).unwrap();
        for session_id in ["session-a", "session-b"] {
            assert_eq!(
                connection
                    .query_row(
                        "SELECT head_revision FROM work_tracking_sessions WHERE id=?",
                        [session_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                2
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT count(*) FROM work_tracking_events WHERE session_id=? AND kind='desktop_task_change' AND json_extract(payload_json,'$.operationId')='desktop-revision'",
                        [session_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                1,
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT state FROM work_tracking_projection_results WHERE event_id=(SELECT head_event_id FROM work_tracking_sessions WHERE id=?) AND projection_name='workflow'",
                        [session_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap(),
                "applied"
            );
        }
    }

    #[test]
    fn tracking_origin_session_is_suppressed_but_other_linked_sessions_receive_the_change() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task = service
            .execute(
                "task.create",
                &json!({"operationId":"origin-create","inputText":"direct","title":"Origin Task"}),
            )
            .unwrap();
        let task_id = task["id"].as_str().unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        link_owned_session(
            &connection,
            "mcp-source",
            "source-session",
            "source-capture",
            "source-event",
            task_id,
        );
        link_owned_session(
            &connection,
            "mcp-other",
            "other-session",
            "other-capture",
            "other-event",
            task_id,
        );
        drop(connection);
        let input = json!({
            "operationId":"source-advance",
            "taskId":task_id,
            "expectedTaskRevision":1,
            "patch":{"title":"Applied by source"}
        });
        service
            .repo
            .transaction(|tx| {
                service.execute_tx_for_tracking(tx, "task.revision", &input, Some("source-session"))
            })
            .unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT head_revision FROM work_tracking_sessions WHERE id='source-session'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            1,
            "the accepted source event owns its session head"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT head_revision FROM work_tracking_sessions WHERE id='other-session'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn relationship_change_fans_out_once_to_each_endpoint_session() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let first = service
            .execute(
                "task.create",
                &json!({"operationId":"relationship-first","inputText":"first","title":"First"}),
            )
            .unwrap();
        let second = service
            .execute(
                "task.create",
                &json!({"operationId":"relationship-second","inputText":"second","title":"Second"}),
            )
            .unwrap();
        let first_id = first["id"].as_str().unwrap();
        let second_id = second["id"].as_str().unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        link_owned_session(
            &connection,
            "mcp-first",
            "first-session",
            "first-capture",
            "first-event",
            first_id,
        );
        link_owned_session(
            &connection,
            "mcp-second",
            "second-session",
            "second-capture",
            "second-event",
            second_id,
        );
        drop(connection);
        service
            .execute(
                "task.relationship.create",
                &json!({
                    "operationId":"relationship-both",
                    "taskId":first_id,
                    "targetTaskId":second_id,
                    "expectedTaskRevision":1,
                    "kind":"related"
                }),
            )
            .unwrap();
        let connection = crate::native::database::open(&db).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM user_activity_events WHERE operation_id LIKE 'relationship-both:%'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2,
        );
        for session_id in ["first-session", "second-session"] {
            assert_eq!(
                connection
                    .query_row(
                        "SELECT count(*) FROM work_tracking_events WHERE session_id=? AND kind='desktop_task_change' AND json_extract(payload_json,'$.operationId')='relationship-both'",
                        [session_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                1,
            );
        }
    }
    #[test]
    fn task_work_session_persists_isolates_retries_and_does_not_change_task_state() {
        let root = tempdir().unwrap();
        let db = root.path().join("state.db");
        crate::native::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let task_a = service
            .execute(
                "task.create",
                &json!({"operationId":"task-a","inputText":"a","title":"A"}),
            )
            .unwrap();
        let task_b = service
            .execute(
                "task.create",
                &json!({"operationId":"task-b","inputText":"b","title":"B"}),
            )
            .unwrap();
        let a = task_a["id"].as_str().unwrap();
        let b = task_b["id"].as_str().unwrap();
        assert_eq!(
            service
                .execute("task.work-session.list", &json!({"taskId":a}))
                .unwrap()["sessions"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert!(service
            .execute(
                "task.work-session.create",
                &json!({"operationId":"invalid-title","taskId":a,"title":"x".repeat(201)}),
            )
            .is_err());
        let session = service
            .execute(
                "task.work-session.create",
                &json!({"operationId":"session-a","taskId":a,"title":"Investigation"}),
            )
            .unwrap();
        let session_id = session["id"].as_str().unwrap();
        let append = json!({"operationId":"entry-a","taskId":a,"sessionId":session_id,"author":"user","kind":"note","body":"persistent note","attachment":{"name":"evidence.txt","mediaType":"text/plain","data":"aGVsbG8="}});
        let first = service
            .execute("task.work-session.entry.create", &append)
            .unwrap();
        let replay = service
            .execute("task.work-session.entry.create", &append)
            .unwrap();
        assert_eq!(first, replay);
        let second = service
            .execute(
                "task.work-session.create",
                &json!({"operationId":"session-a-two","taskId":a,"title":"Implementation"}),
            )
            .unwrap();
        let second_id = second["id"].as_str().unwrap();
        service.execute("task.work-session.entry.create",&json!({"operationId":"entry-a-two","taskId":a,"sessionId":second_id,"body":"second session only"})).unwrap();
        assert_eq!(
            service
                .execute("task.work-session.list", &json!({"taskId":a}))
                .unwrap()["sessions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let second_record = service
            .execute(
                "task.work-session.get",
                &json!({"taskId":a,"sessionId":second_id}),
            )
            .unwrap();
        assert_eq!(second_record["entries"].as_array().unwrap().len(), 1);
        assert_eq!(second_record["entries"][0]["body"], "second session only");
        assert!(service
            .execute(
                "task.work-session.get",
                &json!({"taskId":b,"sessionId":session_id})
            )
            .is_err());
        assert!(service
            .execute(
                "task.work-session.entry.create",
                &json!({"operationId":"cross","taskId":b,"sessionId":session_id,"body":"wrong"})
            )
            .is_err());
        drop(service);
        let reopened = TaskApplicationService::new(&db);
        let record = reopened
            .execute(
                "task.work-session.get",
                &json!({"taskId":a,"sessionId":session_id}),
            )
            .unwrap();
        assert_eq!(record["entries"].as_array().unwrap().len(), 1);
        assert_eq!(record["entries"][0]["body"], "persistent note");
        assert_eq!(record["entries"][0]["attachment"]["data"], "aGVsbG8=");
        assert_eq!(
            reopened.execute("task.get", &json!({"taskId":a})).unwrap()["state"],
            task_a["state"]
        );
        let activity: i64 = crate::native::database::open(&db)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM user_activity_events WHERE entity_id=?",
                [a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(activity, 1, "only Task creation records user activity");
        reopened
            .execute("task.delete", &json!({"taskId":a}))
            .unwrap();
        assert!(reopened
            .execute(
                "task.work-session.get",
                &json!({"taskId":a,"sessionId":session_id})
            )
            .unwrap_err()
            .contains("Task not found"));
    }
}
