use crate::adapters::sqlite::task_repository::{self, SqliteTaskRepository};
use crate::domain::task::{content_hash, validate_state_transition, READINESS_FIELDS};
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
            "task.create" => self.create(input),
            "task.get" => self.get(req(input, "taskId")?),
            "workbench.get" => self.workbench(),
            "task.revision" => self.revise(req(input, "taskId")?, input),
            "task.transition" => self.transition(req(input, "taskId")?, input),
            "task.problem-link.create" => self.problem_link(req(input, "taskId")?, input),
            "task.problem-link.delete" => {
                self.unlink("task_problem_links", req(input, "linkId")?, input)
            }
            "task.relationship.create" => self.relationship(req(input, "taskId")?, input),
            "task.relationship.delete" => {
                self.unlink("task_relationships", req(input, "relationshipId")?, input)
            }
            "task.readiness.get" => self.readiness(req(input, "taskId")?),
            "task.readiness.decision" => self.readiness_decision(req(input, "taskId")?, input),
            "task.work-log.get" => self.work_log(req(input, "taskId")?),
            "task.work-log.create" => self.work_log_create(req(input, "taskId")?, input),
            "work-log.comment.create" => self.comment(req(input, "entryId")?, input),
            "task.checklist.create" => self.checklist(req(input, "taskId")?, input),
            "task.checklist.update" => {
                self.checklist_update(req(input, "taskId")?, req(input, "itemId")?, input)
            }
            "task.decision.create" => self.decision(req(input, "taskId")?, input),
            "task.completion.create" => self.complete(req(input, "taskId")?, input),
            "problem.create" => self.problem(input, None),
            "problem.revision" => self.problem(input, Some(req(input, "problemId")?)),
            "problem.resolution.create" => self.resolve_problem(req(input, "problemId")?, input),
            _ => Err(format!("Native operation is not implemented: {name}")),
        }
    }
    /// Transaction-scoped entry point for MCP/work-tracking proposal adoption.  Callers own
    /// the outer transaction so creating a Task and appending its event cannot partially commit.
    pub(crate) fn execute_tx(
        &self,
        tx: &Transaction<'_>,
        name: &str,
        input: &Value,
    ) -> Result<Value, String> {
        let at = task_repository::now();
        match name {
            "task.create" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let task = self.create_task_tx(
                    tx,
                    input,
                    input.get("originCaptureId").and_then(Value::as_str),
                    None,
                    &at,
                )?;
                task_repository::record_activity_tx(
                    tx,
                    "task",
                    task["id"].as_str().unwrap_or_default(),
                    "created",
                    req(input, "operationId")?,
                    &at,
                )?;
                self.finish(tx, input, name, &task)?;
                Ok(task)
            }
            "problem.create" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let problem = self.create_problem_revision_tx(tx, input, None, &at)?;
                self.finish(tx, input, name, &problem)?;
                Ok(problem)
            }
            "problem.revision" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let problem = self.create_problem_revision_tx(
                    tx,
                    input,
                    Some(req(input, "problemId")?),
                    &at,
                )?;
                self.finish(tx, input, name, &problem)?;
                Ok(problem)
            }
            "task.transition" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let id = req(input, "taskId")?;
                let revision = self.expected(tx, id, input)?;
                let from: String = tx
                    .query_row("SELECT state FROM tasks WHERE id=?", [id], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                let state = validate_state_transition(&from, req(input, "to")?)?;
                tx.execute("UPDATE tasks SET state=?,started_at=CASE WHEN ?='in_progress' AND started_at IS NULL THEN ? ELSE started_at END,reopened_at=CASE WHEN ?='in_progress' AND ?='completed' THEN ? ELSE reopened_at END WHERE id=?",params![state,state,at,state,from,at,id]).map_err(|e|e.to_string())?;
                task_repository::record_activity_tx(
                    tx,
                    "task",
                    id,
                    "transition",
                    req(input, "operationId")?,
                    &at,
                )?;
                let result = json!({"id":id,"taskRevision":revision,"state":state});
                self.finish(tx, input, name, &result)?;
                Ok(result)
            }
            "task.revision" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let id = req(input, "taskId")?;
                let rev = self.expected(tx, id, input)?;
                let patch = input
                    .get("patch")
                    .and_then(Value::as_object)
                    .ok_or("invalid_input: patch is required")?;
                let prior:(String,String,String,String,String,String)=tx.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",params![id,rev],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|e|e.to_string())?;
                let field = |key: &str, old: String| {
                    patch
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or(&old)
                        .trim()
                        .to_owned()
                };
                let values = [
                    field("title", prior.0),
                    field("detail", prior.1),
                    field("outcome", prior.2),
                    field("scope", prior.3),
                    field("nonGoals", prior.4),
                    field("validationCriteria", prior.5),
                ];
                if values[0].is_empty() {
                    return Err("invalid_input: title is required".into());
                }
                tx.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)",params![id,rev+1,values[0],values[1],values[2],values[3],values[4],values[5],content_hash(&values.iter().map(String::as_str).collect::<Vec<_>>()),at]).map_err(|e|e.to_string())?;
                tx.execute(
                    "UPDATE tasks SET current_revision=? WHERE id=?",
                    params![rev + 1, id],
                )
                .map_err(|e| e.to_string())?;
                task_repository::record_activity_tx(
                    tx,
                    "task",
                    id,
                    "revised",
                    req(input, "operationId")?,
                    &at,
                )?;
                let result = json!({"id":id,"taskRevision":rev+1,"title":values[0],"detail":values[1],"outcome":values[2],"scope":values[3],"nonGoals":values[4],"validationCriteria":values[5]});
                self.finish(tx, input, name, &result)?;
                Ok(result)
            }
            "task.work-log.create" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let task = req(input, "taskId")?;
                self.expected(tx, task, input)?;
                let id = task_repository::new_id();
                tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,image_data,image_media_type,image_summary,created_at) VALUES(?,?,?,?,?,?,?)",params![id,task,input.get("body").and_then(Value::as_str).unwrap_or(""),input.get("imageData").and_then(Value::as_str).unwrap_or(""),input.get("imageMediaType").and_then(Value::as_str).unwrap_or(""),input.get("imageSummary").and_then(Value::as_str).unwrap_or(""),at]).map_err(|e|e.to_string())?;
                task_repository::record_activity_tx(
                    tx,
                    "task",
                    task,
                    "work_log",
                    req(input, "operationId")?,
                    &at,
                )?;
                let result = json!({"id":id,"taskId":task,"createdAt":at});
                self.finish(tx, input, name, &result)?;
                Ok(result)
            }
            "task.completion.create" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let task = req(input, "taskId")?;
                let revision = self.expected(tx, task, input)?;
                let state: String = tx
                    .query_row("SELECT state FROM tasks WHERE id=?", [task], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                if state != "in_progress" {
                    return Err(
                        "transition_invalid: task must be in_progress before completion".into(),
                    );
                }
                let id = task_repository::new_id();
                tx.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,task,revision,req(input,"evidence")?,input.get("report").and_then(Value::as_str).unwrap_or(""),req(input,"operationId")?,at]).map_err(|e|e.to_string())?;
                tx.execute(
                    "UPDATE tasks SET state='completed',completed_at=? WHERE id=?",
                    params![at, task],
                )
                .map_err(|e| e.to_string())?;
                let result =
                    json!({"id":id,"taskId":task,"taskRevision":revision,"state":"completed"});
                self.finish(tx, input, name, &result)?;
                Ok(result)
            }
            "problem.resolution.create" => {
                if let Some(result) = self.op(tx, input, name)? {
                    return Ok(result);
                }
                let problem = req(input, "problemId")?;
                let expected = input
                    .get("expectedProblemRevision")
                    .and_then(Value::as_i64)
                    .ok_or("invalid_input: expectedProblemRevision is required")?;
                let current: i64 = tx
                    .query_row(
                        "SELECT current_revision FROM problems WHERE id=?",
                        [problem],
                        |r| r.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                if expected != current {
                    return Err(format!("head_conflict: currentRevision={current}"));
                };
                let id = task_repository::new_id();
                tx.execute("INSERT INTO problem_resolution_decisions(id,problem_id,problem_revision,rationale,evidence_refs_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,problem,current,req(input,"rationale")?,input.get("evidenceRefs").cloned().unwrap_or(json!([])).to_string(),req(input,"operationId")?,at]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE problems SET state='resolved' WHERE id=?", [problem])
                    .map_err(|e| e.to_string())?;
                let result = json!({"id":id,"problemId":problem,"problemRevision":current,"state":"resolved"});
                self.finish(tx, input, name, &result)?;
                Ok(result)
            }
            _ => Err(format!("operation is not transaction-scoped: {name}")),
        }
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
        self.repo.transaction(|tx|{if let Some(x)=self.op(tx,input,"capture.create")?{return Ok(x)}let id=task_repository::new_id();let at=task_repository::now();let text=req(input,"text")?;tx.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,?,?,'capture',?)",params![id,text,at,at]).map_err(|e|e.to_string())?;let x=json!({"id":id,"text":text,"createdAt":at});self.finish(tx,input,"capture.create",&x)?;Ok(x)})
    }
    fn create(&self, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{if let Some(x)=self.op(tx,input,"task.create")?{return Ok(x)}let at=task_repository::now();let origin=input.get("originCaptureId").and_then(Value::as_str);let direct=input.get("inputText").and_then(Value::as_str);if origin.is_some()==direct.is_some(){return Err("invalid_input: supply exactly one originCaptureId or inputText".into())}let provenance=if let Some(text)=direct{let id=task_repository::new_id();tx.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES(?,?,?,'direct_task_provenance',?)",params![id,text,at,at]).map_err(|e|e.to_string())?;Some(id)}else{origin.map(str::to_owned)};let x=self.create_task_tx(tx,input,provenance.as_deref(),None,&at)?;task_repository::record_activity_tx(tx,"task",x["id"].as_str().unwrap(),"created",req(input,"operationId")?,&at)?;self.finish(tx,input,"task.create",&x)?;Ok(x)})
    }
    fn expected(&self, tx: &Transaction<'_>, id: &str, input: &Value) -> Result<i64, String> {
        let got: i64 = tx
            .query_row("SELECT current_revision FROM tasks WHERE id=?", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Task not found")?;
        let expected = input
            .get("expectedTaskRevision")
            .and_then(Value::as_i64)
            .ok_or("invalid_input: expectedTaskRevision is required")?;
        if got != expected {
            return Err(format!("head_conflict: currentRevision={got}"));
        }
        Ok(got)
    }
    fn revise(&self, id: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{if let Some(x)=self.op(tx,input,"task.revision")?{return Ok(x)}let rev=self.expected(tx,id,input)?;let patch=input.get("patch").and_then(Value::as_object).ok_or("invalid_input: patch is required")?;let prior:(String,String,String,String,String,String)=tx.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",params![id,rev],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|e|e.to_string())?;let f=|camel:&str,old:String|patch.get(camel).and_then(Value::as_str).unwrap_or(&old).trim().to_owned();let fields=[f("title",prior.0),f("detail",prior.1),f("outcome",prior.2),f("scope",prior.3),f("nonGoals",prior.4),f("validationCriteria",prior.5)];if fields[0].is_empty(){return Err("invalid_input: title is required".into())}let at=task_repository::now();tx.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)",params![id,rev+1,fields[0],fields[1],fields[2],fields[3],fields[4],fields[5],content_hash(&fields.iter().map(String::as_str).collect::<Vec<_>>()),at]).map_err(|e|e.to_string())?;tx.execute("UPDATE tasks SET current_revision=? WHERE id=?",params![rev+1,id]).map_err(|e|e.to_string())?;task_repository::record_activity_tx(tx,"task",id,"revised",req(input,"operationId")?,&at)?;let x=json!({"id":id,"taskRevision":rev+1,"title":fields[0],"detail":fields[1],"outcome":fields[2],"scope":fields[3],"nonGoals":fields[4],"validationCriteria":fields[5]});self.finish(tx,input,"task.revision",&x)?;Ok(x)})
    }
    fn transition(&self, id: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{let rev=self.expected(tx,id,input)?;let state:String=tx.query_row("SELECT state FROM tasks WHERE id=?",[id],|r|r.get(0)).map_err(|e|e.to_string())?;let to=validate_state_transition(&state,req(input,"to")?)?;let at=task_repository::now();tx.execute("UPDATE tasks SET state=?,started_at=CASE WHEN ?='in_progress' AND started_at IS NULL THEN ? ELSE started_at END,reopened_at=CASE WHEN ?='in_progress' AND ?='completed' THEN ? ELSE reopened_at END WHERE id=?",params![to,to,at,to,state,at,id]).map_err(|e|e.to_string())?;task_repository::record_activity_tx(tx,"task",id,"transition",req(input,"operationId")?,&at)?;let x=json!({"id":id,"taskRevision":rev,"state":to});self.finish(tx,input,"task.transition",&x)?;Ok(x)})
    }
    fn problem(&self, input: &Value, id: Option<&str>) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            let x = self.create_problem_revision_tx(tx, input, id, &task_repository::now())?;
            self.finish(
                tx,
                input,
                if id.is_some() {
                    "problem.revision"
                } else {
                    "problem.create"
                },
                &x,
            )?;
            Ok(x)
        })
    }
    fn problem_link(&self, task: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{self.expected(tx,task,input)?;let p=req(input,"problemId")?;let rev=input.get("problemRevision").and_then(Value::as_i64).ok_or("invalid_input: problemRevision is required")?;let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM problem_revisions WHERE problem_id=? AND revision=?)",params![p,rev],|r|r.get(0)).map_err(|e|e.to_string())?;if !exists{return Err("Problem revision not found".into())}let at=task_repository::now();let id=task_repository::new_id();tx.execute("INSERT INTO task_problem_links(id,task_id,problem_id,problem_revision,relationship,note,created_at) VALUES(?,?,?,?,?,?,?)",params![id,task,p,rev,input.get("relationship").and_then(Value::as_str).unwrap_or("context"),input.get("note").and_then(Value::as_str).unwrap_or(""),at]).map_err(|e|e.to_string())?;let x=json!({"id":id,"taskId":task,"problemId":p,"problemRevision":rev});self.finish(tx,input,"task.problem-link.create",&x)?;Ok(x)})
    }
    fn unlink(&self, table: &str, id: &str, input: &Value) -> Result<Value, String> {
        if !["task_problem_links", "task_relationships"].contains(&table) {
            return Err("invalid_input".into());
        }
        self.repo.transaction(|tx| {
            let n = tx
                .execute(
                    &format!("UPDATE {table} SET unlinked_at=? WHERE id=? AND unlinked_at IS NULL"),
                    params![task_repository::now(), id],
                )
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Link not found".into());
            }
            let x = json!({"id":id,"unlinked":true});
            self.finish(
                tx,
                input,
                if table == "task_problem_links" {
                    "task.problem-link.delete"
                } else {
                    "task.relationship.delete"
                },
                &x,
            )?;
            Ok(x)
        })
    }
    fn relationship(&self, task: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            self.expected(tx, task, input)?;
            let x = task_repository::add_relationship_tx(tx, task, input, &task_repository::now())?;
            self.finish(tx, input, "task.relationship.create", &x)?;
            Ok(x)
        })
    }
    fn readiness(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let rev: i64 = c
            .query_row("SELECT current_revision FROM tasks WHERE id=?", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Task not found")?;
        let fields:(String,String,String)=c.query_row("SELECT outcome,scope,validation_criteria FROM task_revisions WHERE task_id=? AND revision=?",params![id,rev],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
        let vals = [fields.0, fields.1, fields.2];
        let mut entries = Vec::new();
        for (i, key) in READINESS_FIELDS.iter().enumerate() {
            let override_row:Option<(String,String,String)>=c.query_row("SELECT status,reason,evidence_refs_json FROM task_readiness_decisions WHERE task_id=? AND task_revision=? AND field_key=? ORDER BY created_at DESC LIMIT 1",params![id,rev,key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
            let (status, reason, evidence): (String, String, String) =
                if let Some((s, r, e)) = override_row {
                    (
                        if s == "not_applicable" {
                            "not_applicable"
                        } else {
                            "missing"
                        }
                        .into(),
                        r,
                        e,
                    )
                } else if *key == "prerequisites" {
                    (
                        "resolved".into(),
                        "No explicit prerequisite".into(),
                        "[]".into(),
                    )
                } else if !vals[i].is_empty() {
                    ("resolved".into(), "".into(), "[]".into())
                } else {
                    ("missing".into(), "".into(), "[]".into())
                };
            entries.push(json!({"key":key,"status":status,"reason":reason,"evidenceRefs":serde_json::from_str::<Value>(&evidence).unwrap_or(json!([])),"sourceRevision":rev,"provenance":"calculated"}));
        }
        Ok(json!({"taskId":id,"taskRevision":rev,"entries":entries}))
    }
    fn readiness_decision(&self, id: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{let rev=self.expected(tx,id,input)?;let status=req(input,"status")?;if !["not_applicable","calculated"].contains(&status){return Err("invalid_input: status".into())}let at=task_repository::now();let did=task_repository::new_id();tx.execute("INSERT INTO task_readiness_decisions(id,task_id,task_revision,field_key,status,reason,evidence_refs_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?,?,?)",params![did,id,rev,req(input,"key")?,status,input.get("reason").and_then(Value::as_str).unwrap_or(""),input.get("evidenceRefs").cloned().unwrap_or(json!([])).to_string(),req(input,"operationId")?,at]).map_err(|e|e.to_string())?;Ok(json!({"id":did,"taskId":id,"taskRevision":rev}))})
    }
    fn work_log_create(&self, id: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{self.expected(tx,id,input)?;let at=task_repository::now();let eid=task_repository::new_id();tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,image_data,image_media_type,image_summary,created_at) VALUES(?,?,?,?,?,?,?)",params![eid,id,input.get("body").and_then(Value::as_str).unwrap_or(""),input.get("imageData").and_then(Value::as_str).unwrap_or(""),input.get("imageMediaType").and_then(Value::as_str).unwrap_or(""),input.get("imageSummary").and_then(Value::as_str).unwrap_or(""),at]).map_err(|e|e.to_string())?;if let Some(a)=input.get("attachment"){tx.execute("INSERT INTO task_attachments(id,task_id,entry_id,name,media_type,data,byte_hash,created_at) VALUES(?,?,?,?,?,?,?,?)",params![task_repository::new_id(),id,eid,a.get("name").and_then(Value::as_str).unwrap_or(""),a.get("mediaType").and_then(Value::as_str).unwrap_or(""),a.get("data").and_then(Value::as_str).unwrap_or(""),content_hash(&[a.get("data").and_then(Value::as_str).unwrap_or("")]),at]).map_err(|e|e.to_string())?;}task_repository::record_activity_tx(tx,"task",id,"work_log",req(input,"operationId")?,&at)?;let x=json!({"id":eid,"taskId":id,"body":input.get("body").cloned().unwrap_or(json!("")),"createdAt":at});self.finish(tx,input,"task.work-log.create",&x)?;Ok(x)})
    }
    fn work_log(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let mut s=c.prepare("SELECT id,body,image_data,image_media_type,image_summary,created_at FROM task_work_log_entries WHERE task_id=? ORDER BY created_at").map_err(|e|e.to_string())?;
        let rows=s.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"body":r.get::<_,String>(1)?,"imageData":r.get::<_,String>(2)?,"imageMediaType":r.get::<_,String>(3)?,"imageSummary":r.get::<_,String>(4)?,"createdAt":r.get::<_,String>(5)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        Ok(json!({"entries":rows}))
    }
    fn comment(&self, e: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx| {
            let id = task_repository::new_id();
            let at = task_repository::now();
            tx.execute(
                "INSERT INTO task_work_log_comments(id,entry_id,body,created_at) VALUES(?,?,?,?)",
                params![id, e, req(input, "body")?, at],
            )
            .map_err(|x| x.to_string())?;
            Ok(json!({"id":id,"entryId":e,"createdAt":at}))
        })
    }
    fn checklist(&self, t: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{self.expected(tx,t,input)?;let id=task_repository::new_id();let at=task_repository::now();tx.execute("INSERT INTO task_checklist_items(id,task_id,body,checked,created_at,updated_at) VALUES(?,?,?,0,?,?)",params![id,t,req(input,"body")?,at,at]).map_err(|x|x.to_string())?;Ok(json!({"id":id,"taskId":t,"body":input["body"],"checked":false,"createdAt":at}))})
    }
    fn checklist_update(&self, t: &str, item: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{self.expected(tx,t,input)?;let at=task_repository::now();let n=tx.execute("UPDATE task_checklist_items SET checked=COALESCE(?,checked),updated_at=? WHERE id=? AND task_id=?",params![input.get("checked").and_then(Value::as_bool).map(i64::from),at,item,t]).map_err(|x|x.to_string())?;if n==0{return Err("Checklist item not found".into())}Ok(json!({"id":item,"taskId":t,"updatedAt":at}))})
    }
    fn decision(&self, t: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{let rev=self.expected(tx,t,input)?;let id=task_repository::new_id();let at=task_repository::now();tx.execute("INSERT INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,t,rev,req(input,"kind")?,input.get("payload").cloned().unwrap_or(json!({})).to_string(),req(input,"operationId")?,at]).map_err(|x|x.to_string())?;Ok(json!({"id":id,"taskId":t,"taskRevision":rev,"createdAt":at}))})
    }
    fn complete(&self, t: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{let rev=self.expected(tx,t,input)?;let state:String=tx.query_row("SELECT state FROM tasks WHERE id=?",[t],|r|r.get(0)).map_err(|x|x.to_string())?;if state!="in_progress"{return Err("transition_invalid: task must be in_progress before completion".into())}let id=task_repository::new_id();let at=task_repository::now();tx.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,t,rev,req(input,"evidence")?,input.get("report").and_then(Value::as_str).unwrap_or(""),req(input,"operationId")?,at]).map_err(|x|x.to_string())?;tx.execute("UPDATE tasks SET state='completed',completed_at=? WHERE id=?",params![at,t]).map_err(|x|x.to_string())?;Ok(json!({"id":id,"taskId":t,"taskRevision":rev,"state":"completed","createdAt":at}))})
    }
    fn resolve_problem(&self, p: &str, input: &Value) -> Result<Value, String> {
        self.repo.transaction(|tx|{let r=input.get("expectedProblemRevision").and_then(Value::as_i64).ok_or("invalid_input: expectedProblemRevision is required")?;let cur:i64=tx.query_row("SELECT current_revision FROM problems WHERE id=?",[p],|row|row.get(0)).optional().map_err(|x|x.to_string())?.ok_or("Problem not found")?;if r!=cur{return Err(format!("head_conflict: currentRevision={cur}"))}let id=task_repository::new_id();let at=task_repository::now();tx.execute("INSERT INTO problem_resolution_decisions(id,problem_id,problem_revision,rationale,evidence_refs_json,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,p,r,req(input,"rationale")?,input.get("evidenceRefs").cloned().unwrap_or(json!([])).to_string(),req(input,"operationId")?,at]).map_err(|x|x.to_string())?;tx.execute("UPDATE problems SET state='resolved' WHERE id=?",[p]).map_err(|x|x.to_string())?;Ok(json!({"id":id,"problemId":p,"problemRevision":r,"state":"resolved","createdAt":at}))})
    }
    fn get(&self, id: &str) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let mut x=c.query_row("SELECT t.current_revision,t.state,r.title,r.detail,r.outcome,r.scope,r.non_goals,r.validation_criteria,t.category,t.created_at,t.last_user_activity_at FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=?",[id],|r|Ok(json!({"id":id,"taskRevision":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"detail":r.get::<_,String>(3)?,"outcome":r.get::<_,String>(4)?,"scope":r.get::<_,String>(5)?,"nonGoals":r.get::<_,String>(6)?,"validationCriteria":r.get::<_,String>(7)?,"category":r.get::<_,String>(8)?,"createdAt":r.get::<_,String>(9)?,"lastUserActivityAt":r.get::<_,String>(10)?}))).optional().map_err(|x|x.to_string())?.ok_or("Task not found")?;
        let mut work_log = self.work_log(id)?["entries"].clone();
        if let Some(entries) = work_log.as_array_mut() {
            for entry in entries {
                let entry_id = entry["id"].as_str().unwrap_or("");
                let attachment=c.query_row("SELECT name,media_type,data FROM task_attachments WHERE entry_id=? LIMIT 1",[entry_id],|r|Ok(json!({"name":r.get::<_,String>(0)?,"mediaType":r.get::<_,String>(1)?,"data":r.get::<_,String>(2)?}))).optional().map_err(|e|e.to_string())?;
                let comments=c.prepare("SELECT id,body,created_at FROM task_work_log_comments WHERE entry_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([entry_id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"body":r.get::<_,String>(1)?,"createdAt":r.get::<_,String>(2)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
                if let Some(o) = entry.as_object_mut() {
                    if let Some(a) = attachment {
                        o.insert("attachment".into(), a);
                    }
                    o.insert("comments".into(), json!(comments));
                }
            }
        }
        let checklist = c.prepare("SELECT id,body,checked FROM task_checklist_items WHERE task_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"body":r.get::<_,String>(1)?,"checked":r.get::<_,i64>(2)? != 0}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let decisions = c.prepare("SELECT id,kind,payload_json,created_at FROM task_decisions WHERE task_id=? ORDER BY created_at").map_err(|e|e.to_string())?.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"body":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).ok().and_then(|v|v.get("body").cloned()).unwrap_or(Value::Null),"createdAt":r.get::<_,String>(3)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let completion = c.query_row("SELECT id,evidence,report,created_at FROM task_completions WHERE task_id=? ORDER BY created_at DESC LIMIT 1",[id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"evidence":r.get::<_,String>(1)?,"report":r.get::<_,String>(2)?,"createdAt":r.get::<_,String>(3)?}))).optional().map_err(|e|e.to_string())?;
        let publication = c.query_row("SELECT revision,state,content_hash FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision DESC LIMIT 1",[id],|r|Ok(json!({"draftRevision":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,"contentHash":r.get::<_,String>(2)?}))).optional().map_err(|e|e.to_string())?;
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
        if let Some(completion) = completion {
            object.insert("completion".into(), completion);
        }
        if let Some(publication) = publication {
            object.insert("publication".into(), publication);
        }
        Ok(x)
    }
    fn workbench(&self) -> Result<Value, String> {
        let c = crate::native::database::open(self.repo.path())?;
        let mut s=c.prepare("SELECT t.id,t.current_revision,t.state,r.title,t.category,t.last_user_activity_at FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.archived_at IS NULL ORDER BY t.last_user_activity_at DESC").map_err(|x|x.to_string())?;
        let mut rows=s.query_map([],|r|Ok(json!({"kind":"task","id":r.get::<_,String>(0)?,"taskRevision":r.get::<_,i64>(1)?,"state":r.get::<_,String>(2)?,"title":r.get::<_,String>(3)?,"category":r.get::<_,String>(4)?,"lastUserActivityAt":r.get::<_,String>(5)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?;
        let mut captures=c.prepare("SELECT c.id,c.text,c.created_at,COALESCE(o.category,'General') FROM captures c LEFT JOIN workbench_category_overrides o ON o.entity_type='captures' AND o.entity_id=c.id WHERE c.source_mode='capture' ORDER BY c.last_user_activity_at DESC").map_err(|x|x.to_string())?;
        rows.extend(captures.query_map([],|r|Ok(json!({"kind":"capture","id":r.get::<_,String>(0)?,"text":r.get::<_,String>(1)?,"lastUserActivityAt":r.get::<_,String>(2)?,"category":r.get::<_,String>(3)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?);
        let mut legacy_refinements=c.prepare("SELECT i.id,i.problem_id,i.problem_revision,r.statement,i.source_kind,i.created_at,COALESCE(o.category,'General') FROM refinement_items i JOIN problem_revisions r ON r.problem_id=i.problem_id AND r.revision=i.problem_revision LEFT JOIN workbench_category_overrides o ON o.entity_type='problems' AND o.entity_id=i.problem_id ORDER BY i.created_at DESC").map_err(|x|x.to_string())?;
        rows.extend(legacy_refinements.query_map([],|r|Ok(json!({"kind":"refinement","id":r.get::<_,String>(0)?,"problemId":r.get::<_,String>(1)?,"problemRevision":r.get::<_,i64>(2)?,"title":r.get::<_,String>(3)?,"sourceKind":r.get::<_,String>(4)?,"lastUserActivityAt":r.get::<_,String>(5)?,"category":r.get::<_,String>(6)?}))).map_err(|x|x.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|x|x.to_string())?);
        rows.sort_by(|a, b| {
            b["lastUserActivityAt"]
                .as_str()
                .cmp(&a["lastUserActivityAt"].as_str())
        });
        let active = rows
            .iter()
            .filter(|x| x["state"] == "in_progress")
            .take(3)
            .map(|x| json!({"kind":"task","id":x["id"],"taskRevision":x["taskRevision"]}))
            .collect::<Vec<_>>();
        let active_ids = active
            .iter()
            .filter_map(|x| x["id"].as_str())
            .collect::<Vec<_>>();
        let mut refining = Vec::new();
        if let Ok(mut sessions) = c.prepare("SELECT capture_id,task_id,current_draft_revision FROM refinement_sessions WHERE (capture_id IS NOT NULL OR task_id IS NOT NULL) AND state!='completed' ORDER BY last_user_activity_at DESC LIMIT 6") {
            let subjects = sessions.query_map([], |row| Ok((row.get::<_,Option<String>>(0)?, row.get::<_,Option<String>>(1)?, row.get::<_,i64>(2)?))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
            refining.extend(subjects.into_iter().filter_map(|(capture_id,task_id,revision)| {
                let (kind,id) = if let Some(id) = task_id { if active_ids.contains(&id.as_str()) { return None; } ("task",id) } else { ("capture",capture_id?) };
                Some(json!({"kind":kind,"id":id,"draftRevision":revision}))
            }).take(3));
        }
        let mut groups = std::collections::BTreeMap::<String, Vec<Value>>::new();
        for item in rows {
            let category = item["category"].as_str().unwrap_or("General").to_owned();
            groups.entry(category).or_default().push(item);
        }
        let categories = groups
            .into_iter()
            .map(|(id, items)| json!({"id":id,"label":id,"items":items}))
            .collect::<Vec<_>>();
        Ok(
            json!({"revision":0,"activeShortcuts":active,"refiningShortcuts":refining,"categories":categories}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    }
}
