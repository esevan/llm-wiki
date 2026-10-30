//! Explicitly reviewed stop-and-complete, with a durable gate against new dispatches.
use super::{
    task_execution_service::TaskExecutionApplicationService, task_service::TaskApplicationService,
};
use crate::{domain::task::content_hash, native::database};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
fn req<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("invalid_input: {k} is required"))
}
const ACTIVE:&str="(status IN ('queued','running','awaiting_response') OR (status='needs_attention' AND dispatch_state IN ('uncertain','accepted','dispatch_recorded')))";
pub(crate) fn ensure_execution_allowed(c: &Connection, task: &str) -> Result<(), String> {
    let blocked:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM task_completion_stops WHERE task_id=? AND state='stopping')",[task],|r|r.get(0)).map_err(|e|e.to_string())?;
    if blocked {
        Err("completion_pending: Task completion is stopping executions; cancel completion before starting work".into())
    } else {
        Ok(())
    }
}
fn preview_on(c: &Connection, task: &str) -> Result<Value, String> {
    let (revision,state):(i64,String)=c.query_row("SELECT current_revision,state FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=tasks.id)",[task],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    let mut stmt=c.prepare(&format!("SELECT id,session_id,instruction,status,revision,dispatch_state,provider_thread_id,provider_turn_id FROM task_work_session_runs WHERE task_id=? AND {ACTIVE} ORDER BY created_at,id")).map_err(|e|e.to_string())?;
    let runs=stmt.query_map([task],|r|Ok(json!({"taskId":task,"id":r.get::<_,String>(0)?,"sessionId":r.get::<_,String>(1)?,"instruction":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"revision":r.get::<_,i64>(4)?,"dispatchState":r.get::<_,String>(5)?,"threadId":r.get::<_,Option<String>>(6)?,"turnId":r.get::<_,Option<String>>(7)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut stmt=c.prepare("SELECT id,session_id,instruction,revision FROM task_session_queue_items WHERE task_id=? AND state='pending' ORDER BY position,id").map_err(|e|e.to_string())?;
    let queued=stmt.query_map([task],|r|Ok(json!({"taskId":task,"id":r.get::<_,String>(0)?,"sessionId":r.get::<_,String>(1)?,"instruction":r.get::<_,String>(2)?,"revision":r.get::<_,i64>(3)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut stmt=c.prepare("SELECT f.id,f.run_id,COALESCE(json_extract(f.request_json,'$.title'),f.kind),f.kind,f.status,f.updated_at FROM task_work_session_formal_requests f JOIN task_work_session_runs r ON r.id=f.run_id WHERE r.task_id=? AND f.status IN ('pending','submitting','error') ORDER BY f.id").map_err(|e|e.to_string())?;
    let responses=stmt.query_map([task],|r|Ok(json!({"id":r.get::<_,String>(0)?,"runId":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"kind":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"updatedAt":r.get::<_,String>(5)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut result = json!({"taskId":task,"taskRevision":revision,"taskState":state,"runs":runs,"queued":queued,"responses":responses});
    // Approval names immutable Run identities, not every streaming checkpoint.
    // New/replaced Runs and edited queue items still require a fresh review.
    let mut authority = result.clone();
    for run in authority["runs"].as_array_mut().into_iter().flatten() {
        for key in ["revision", "status", "dispatchState", "threadId", "turnId"] {
            run.as_object_mut().unwrap().remove(key);
        }
    }
    result["reviewHash"] = json!(content_hash(&[&authority.to_string()]));
    Ok(result)
}
impl TaskExecutionApplicationService {
    pub(crate) fn completion_session_ids(&self, task: &str) -> Result<Vec<String>, String> {
        let c = database::open(&self.db_path)?;
        let mut statement = c
            .prepare("SELECT id FROM task_work_sessions WHERE task_id=?")
            .map_err(|e| e.to_string())?;
        let result = statement
            .query_map([task], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(result)
    }
    pub(crate) fn completion_preview(&self, input: &Value) -> Result<Value, String> {
        let mut c = database::open(&self.db_path)?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        let task = req(input, "taskId")?;
        let mut result = preview_on(&tx, task)?;
        if let Some((operation,request))=tx.query_row("SELECT operation_id,request_json FROM task_completion_stops WHERE task_id=? AND state='stopping'",[task],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional().map_err(|e|e.to_string())?{
            result["activeOperationId"]=json!(operation);result["activeEvidence"]=serde_json::from_str::<Value>(&request).map_err(|e|e.to_string())?["evidence"].clone();
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    }
    pub(crate) fn completion_begin(&self, input: &Value) -> Result<Value, String> {
        let task = req(input, "taskId")?;
        let operation = req(input, "operationId")?;
        let request_json = input.to_string();
        let mut c = database::open(&self.db_path)?;
        let tx = database::immediate_transaction(&mut c)?;
        if let Some(old) = tx
            .query_row(
                "SELECT request_json FROM task_completion_stops WHERE operation_id=?",
                [operation],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
        {
            if old != request_json {
                return Err("operation_conflict".into());
            }
            tx.commit().map_err(|e| e.to_string())?;
            return self.completion_status(input);
        }
        let review = preview_on(&tx, task)?;
        if review["reviewHash"] != input["reviewHash"]
            || review["taskRevision"] != input["expectedTaskRevision"]
        {
            return Err(
                "completion_review_stale: executions or queued requests changed; review again"
                    .into(),
            );
        }
        if review["taskState"] != "in_progress" {
            return Err("transition_invalid: Task must be in progress".into());
        }
        req(input, "evidence")?;
        crate::native::task_hierarchy::ensure_children_completed(&tx, task)?;
        ensure_execution_allowed(&tx, task)?;
        tx.execute("INSERT INTO task_completion_stops(operation_id,task_id,state,request_json,review_json) VALUES(?,?,'stopping',?,?)",params![operation,task,request_json,review.to_string()]).map_err(|e|e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO task_session_queue_control(session_id,mode,reason) SELECT id,'paused','Task completion requested' FROM task_work_sessions WHERE task_id=?",[task]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE task_session_queue_control SET mode='paused',reason='Task completion requested',stop_run_id=NULL,revision=revision+1 WHERE session_id IN (SELECT id FROM task_work_sessions WHERE task_id=?)",[task]).map_err(|e|e.to_string())?;
        tx.execute("UPDATE task_session_queue_items SET state='cancelled',revision=revision+1 WHERE task_id=? AND state='pending'",[task]).map_err(|e|e.to_string())?;
        // A never-dispatched Run has no external work to stop. Preserve its log and report.
        tx.execute("UPDATE task_work_session_runs SET status='cancelled',stop_requested=1,finished_at=CURRENT_TIMESTAMP,revision=revision+1 WHERE task_id=? AND status='queued' AND dispatch_state='not_dispatched'",[task]).map_err(|e|e.to_string())?;
        tx.execute(&format!("UPDATE task_work_session_runs SET stop_requested=1,revision=revision+1 WHERE task_id=? AND {ACTIVE}"),[task]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.completion_status(input)
    }
    pub(crate) fn completion_status(&self, input: &Value) -> Result<Value, String> {
        let c = database::open(&self.db_path)?;
        let task = req(input, "taskId")?;
        let operation = req(input, "operationId")?;
        let (state,review):(String,String)=c.query_row("SELECT state,review_json FROM task_completion_stops WHERE operation_id=? AND task_id=?",params![operation,task],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        Ok(
            json!({"state":state,"review":serde_json::from_str::<Value>(&review).map_err(|e|e.to_string())?,"remaining":preview_on(&c,task)?}),
        )
    }
    pub(crate) fn completion_cancel(&self, input: &Value) -> Result<Value, String> {
        let c = database::open(&self.db_path)?;
        c.execute("INSERT OR IGNORE INTO task_completion_stops(operation_id,task_id,state,request_json,review_json) VALUES(?,?,'cancelled','{}','{}')",params![req(input,"operationId")?,req(input,"taskId")?]).map_err(|e|e.to_string())?;
        c.execute("UPDATE task_completion_stops SET state='cancelled' WHERE operation_id=? AND task_id=? AND state='stopping'",params![req(input,"operationId")?,req(input,"taskId")?]).map_err(|e|e.to_string())?;
        // Cancellation never resumes a queue or reverses an already sent interrupt.
        self.completion_status(input)
    }
    pub(crate) fn completion_finalize(&self, input: &Value) -> Result<Value, String> {
        let mut c = database::open(&self.db_path)?;
        let tx = database::immediate_transaction(&mut c)?;
        let task = req(input, "taskId")?;
        let operation = req(input, "operationId")?;
        let (state,request):(String,String)=tx.query_row("SELECT state,request_json FROM task_completion_stops WHERE operation_id=? AND task_id=?",params![operation,task],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
        if state != "stopping" {
            tx.commit().map_err(|e| e.to_string())?;
            return self.completion_status(input);
        }
        if super::task_service::TaskApplicationService::has_active_execution(&tx, task)? {
            tx.commit().map_err(|e| e.to_string())?;
            return self.completion_status(input);
        }
        let mut request: Value = serde_json::from_str(&request).map_err(|e| e.to_string())?;
        request["operationId"] = json!(format!("{operation}:complete"));
        request["completionStopId"] = json!(operation);
        TaskApplicationService::new(&self.db_path).execute_tx_for_tracking(
            &tx,
            "task.completion.create",
            &request,
            None,
        )?;
        tx.execute("UPDATE task_completion_stops SET state='completed' WHERE operation_id=? AND state='stopping'",[operation]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        self.completion_status(input)
    }
    /// Only a terminal result for the exact saved provider turn can reconcile an uncertain Run.
    pub(crate) fn completion_reconcile(&self, run: &Value, turn: &Value) -> Result<(), String> {
        if turn["id"] != run["turnId"]
            || !matches!(
                turn["status"].as_str(),
                Some("completed" | "failed" | "interrupted" | "cancelled")
            )
        {
            return Err("uncertain_execution: exact terminal turn is unavailable".into());
        }
        let c = database::open(&self.db_path)?;
        let exact:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM task_work_session_runs WHERE id=? AND task_id=? AND session_id=? AND provider_thread_id=? AND provider_turn_id=?)",params![req(run,"id")?,req(run,"taskId")?,req(run,"sessionId")?,req(run,"threadId")?,req(run,"turnId")?],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !exact {
            return Err("ownership_failure: provider turn changed".into());
        }
        for (index, item) in turn["items"].as_array().into_iter().flatten().enumerate() {
            self.complete_item(req(run, "id")?, item, index as i64)?;
        }
        self.finish_turn(req(run, "id")?, turn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        _root: tempfile::TempDir,
        db: std::path::PathBuf,
        task: String,
        session: String,
        run: Value,
        service: TaskExecutionApplicationService,
    }
    fn fixture(dispatch: bool) -> Fixture {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".tmp");
        std::fs::create_dir_all(&directory).unwrap();
        let root = tempfile::tempdir_in(directory).unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let tasks = TaskApplicationService::new(&db);
        let task=tasks.execute("task.create",&json!({"operationId":"task","inputText":"Document work","title":"Safe completion"})).unwrap()["id"].as_str().unwrap().to_owned();
        let session = tasks
            .execute(
                "task.work-session.create",
                &json!({"operationId":"session","taskId":task,"title":"Work"}),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        tasks.execute("task.work-session.update",&json!({"operationId":"settings","taskId":task,"sessionId":session,"title":"Work","provider":"codex","model":"gpt-5.6-sol","approvalMode":"auto","workspacePath":root.path().to_str().unwrap()})).unwrap();
        tasks.execute("task.transition",&json!({"operationId":"start-task","taskId":task,"expectedTaskRevision":1,"to":"in_progress"})).unwrap();
        let service = TaskExecutionApplicationService::new(&db);
        let prepared = service.prepared(&task, &session).unwrap();
        service.bind_thread(&prepared,&json!({"id":"thread","model":"gpt-5.6-sol","cwd":prepared.cwd,"approvalPolicy":"on-request","sandbox":"workspace-write"}),true).unwrap();
        let (created,_)=service.create_run(&json!({"taskId":task,"sessionId":session,"operationId":"run","instruction":"Preserve results","settingsRevision":prepared.settings_revision})).unwrap();
        let mut run = created["selectedRun"].clone();
        let id = run["id"].as_str().unwrap();
        if dispatch {
            service.mark_dispatch_recorded(id).unwrap();
            service.accept_turn(id, "thread", "turn").unwrap();
        }
        run["threadId"] = json!("thread");
        run["turnId"] = json!("turn");
        Fixture {
            _root: root,
            db,
            task,
            session,
            run,
            service,
        }
    }
    fn request(f: &Fixture) -> Value {
        let review = f
            .service
            .completion_preview(&json!({"taskId":f.task}))
            .unwrap();
        json!({"action":"begin","operationId":"stop-complete","taskId":f.task,"expectedTaskRevision":1,"evidence":"Work is documented","reviewHash":review["reviewHash"]})
    }
    fn state(f: &Fixture) -> String {
        database::open(&f.db)
            .unwrap()
            .query_row("SELECT state FROM tasks WHERE id=?", [&f.task], |r| {
                r.get(0)
            })
            .unwrap()
    }
    fn queue(f: &Fixture) {
        database::open(&f.db).unwrap().execute("INSERT INTO task_session_queue_items(id,task_id,session_id,operation_id,payload_hash,instruction,settings_json,position) VALUES('queue',?,?,'queued','hash','Next action','{}',1)",params![f.task,f.session]).unwrap();
    }
    #[test]
    fn completion_review_is_exact_and_gate_prevents_queued_or_manual_dispatch() {
        let f = fixture(false);
        queue(&f);
        let old = request(&f);
        database::open(&f.db)
            .unwrap()
            .execute(
                "UPDATE task_session_queue_items SET revision=revision+1",
                [],
            )
            .unwrap();
        assert!(f
            .service
            .completion_begin(&old)
            .unwrap_err()
            .contains("stale"));
        let input = request(&f);
        let first = f.service.completion_begin(&input).unwrap();
        assert_eq!(first["state"], "stopping");
        assert_eq!(first, f.service.completion_begin(&input).unwrap());
        assert!(f.service.begin_formal_response(&json!({"taskId":f.task,"sessionId":f.session,"runId":f.run["id"],"requestId":"pending","response":{}}),1).unwrap_err().contains("completion_pending"));
        assert!(
            ensure_execution_allowed(&database::open(&f.db).unwrap(), &f.task)
                .unwrap_err()
                .contains("completion_pending")
        );
        assert!(f
            .service
            .mark_dispatch_recorded(f.run["id"].as_str().unwrap())
            .is_err());
        assert!(f
            .service
            .queue_mutate(&json!({"action":"resume","taskId":f.task,"sessionId":f.session}))
            .unwrap_err()
            .contains("completion_pending"));
        let done = f.service.completion_finalize(&input).unwrap();
        assert_eq!(done["state"], "completed");
        assert_eq!(state(&f), "completed");
        assert_eq!(
            f.service.completion_finalize(&input).unwrap()["state"],
            "completed"
        );
        let c = database::open(&f.db).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM task_completions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT state FROM task_session_queue_items", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
            "cancelled"
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM task_work_log_entries", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn completion_scope_ignores_stream_progress_but_rejects_a_replacement_run() {
        let f = fixture(true);
        let input = request(&f);
        let c = database::open(&f.db).unwrap();
        c.execute("UPDATE task_work_session_runs SET revision=revision+1,status='awaiting_response' WHERE id=?",[f.run["id"].as_str().unwrap()]).unwrap();
        assert_eq!(request(&f)["reviewHash"], input["reviewHash"]);
        f.service
            .finish_turn(
                f.run["id"].as_str().unwrap(),
                &json!({"status":"completed"}),
            )
            .unwrap();
        let revision = f
            .service
            .prepared(&f.task, &f.session)
            .unwrap()
            .settings_revision;
        f.service.create_run(&json!({"taskId":f.task,"sessionId":f.session,"operationId":"replacement","instruction":"New work started after review","settingsRevision":revision})).unwrap();
        assert!(f
            .service
            .completion_begin(&input)
            .unwrap_err()
            .contains("stale"));
        assert_eq!(state(&f), "in_progress");
    }

    #[test]
    fn completion_uncertain_execution_requires_exact_terminal_evidence_and_preserves_report() {
        let f = fixture(true);
        f.service.recover_nonterminal().unwrap();
        let input = request(&f);
        f.service.completion_begin(&input).unwrap();
        assert_eq!(
            f.service.completion_finalize(&input).unwrap()["state"],
            "stopping"
        );
        assert_eq!(state(&f), "in_progress");
        for turn in [
            json!({"id":"other","status":"completed"}),
            json!({"id":"turn","status":"inProgress"}),
        ] {
            assert!(f.service.completion_reconcile(&f.run, &turn).is_err());
        }
        let turn = json!({"id":"turn","status":"completed","items":[{"id":"report","type":"agentMessage","phase":"final_answer","text":"Preserved final result"}]});
        f.service.completion_reconcile(&f.run, &turn).unwrap();
        assert_eq!(
            f.service.completion_finalize(&input).unwrap()["state"],
            "completed"
        );
        let c = database::open(&f.db).unwrap();
        assert_eq!(
            c.query_row("SELECT final_report FROM task_work_session_runs", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Preserved final result"
        );
    }
    #[test]
    fn completion_cancel_wins_before_begin_and_after_terminal_without_resuming_queue() {
        let f = fixture(true);
        queue(&f);
        let input = request(&f);
        f.service.completion_cancel(&input).unwrap();
        assert!(f.service.completion_begin(&input).is_err());
        assert_eq!(state(&f), "in_progress");
        let mut input = request(&f);
        input["operationId"] = json!("second-attempt");
        f.service.completion_begin(&input).unwrap();
        f.service
            .completion_reconcile(&f.run, &json!({"id":"turn","status":"interrupted"}))
            .unwrap();
        assert_eq!(
            f.service.completion_cancel(&input).unwrap()["state"],
            "cancelled"
        );
        assert_eq!(
            f.service.completion_finalize(&input).unwrap()["state"],
            "cancelled"
        );
        assert_eq!(state(&f), "in_progress");
        let c = database::open(&f.db).unwrap();
        assert_eq!(
            c.query_row("SELECT mode FROM task_session_queue_control", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "paused"
        );
        assert_eq!(
            c.query_row("SELECT state FROM task_session_queue_items", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
            "cancelled"
        );
    }
    #[test]
    fn completion_stale_task_and_missing_provider_identity_never_force_completion() {
        let f = fixture(true);
        let input = request(&f);
        f.service.completion_begin(&input).unwrap();
        let preview = f
            .service
            .completion_preview(&json!({"taskId":f.task}))
            .unwrap();
        assert_eq!(preview["activeOperationId"], "stop-complete");
        f.service
            .completion_reconcile(&f.run, &json!({"id":"turn","status":"completed"}))
            .unwrap();
        TaskApplicationService::new(&f.db).execute("task.revision",&json!({"operationId":"edit","taskId":f.task,"expectedTaskRevision":1,"patch":{"detail":"New detail"}})).unwrap();
        assert!(f
            .service
            .completion_finalize(&input)
            .unwrap_err()
            .contains("head_conflict"));
        assert_eq!(state(&f), "in_progress");
    }
}
