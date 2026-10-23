//! Persistent follow-ups. Provider execution remains in the existing runtime.
use super::task_execution_service::{PreparedExecution, TaskExecutionApplicationService};
use crate::{adapters::sqlite::task_repository, domain::task::content_hash, native::database};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

fn required<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("invalid_input: {key} is required"))
}
pub(super) fn validate_attachment(input: &Value) -> Result<(), String> {
    use base64::Engine;
    if let Some(a) = input.get("attachment").filter(|v| !v.is_null()) {
        let name = required(a, "name")?;
        let media = required(a, "mediaType")?;
        let data = required(a, "data")?;
        if name.len() > 1024 || media.len() > 200 || data.len() > 14_000_000 {
            return Err("invalid_input: attachment exceeds 10 MB".into());
        }
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|_| "invalid_input: invalid attachment encoding")?;
        if decoded.len() > 10 * 1024 * 1024 {
            return Err("invalid_input: attachment exceeds 10 MB".into());
        }
    }
    Ok(())
}
fn err(e: rusqlite::Error) -> String {
    e.to_string()
}
fn owned(c: &Connection, task: &str, session: &str) -> Result<(), String> {
    let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM task_work_sessions s JOIN tasks t ON t.id=s.task_id WHERE s.id=? AND t.id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=t.id))",params![session,task],|r|r.get(0)).map_err(err)?;
    if exists {
        Ok(())
    } else {
        Err("ownership_failure: work session not found".into())
    }
}
fn eligible(c: &Connection, task: &str, session: &str) -> Result<(), String> {
    owned(c, task, session)?;
    super::task_service::ensure_task_mutable(c, task)?;
    let held: bool = c
        .query_row(
            "SELECT state='on_hold' FROM tasks WHERE id=?",
            [task],
            |r| r.get(0),
        )
        .map_err(err)?;
    if held {
        return Err("task_on_hold: resume this Task before queueing work".into());
    }
    Ok(())
}
fn settings(c: &Connection, session: &str) -> Result<Value, String> {
    c.query_row("SELECT model,workspace_path,approvals_reviewer,reasoning_effort,effective_reasoning_effort,effective_model,effective_cwd,effective_settings_revision,effective_approval_policy,effective_sandbox FROM task_work_sessions WHERE id=?",[session],|r|Ok(json!({"model":r.get::<_,String>(0)?,"workspacePath":r.get::<_,String>(1)?,"approvalsReviewer":r.get::<_,String>(2)?,"reasoningEffort":r.get::<_,Option<String>>(3)?,"resolvedEffort":r.get::<_,Option<String>>(4)?,"effectiveModel":r.get::<_,Option<String>>(5)?,"cwd":r.get::<_,Option<String>>(6)?,"settingsRevision":r.get::<_,Option<String>>(7)?,"approvalPolicy":r.get::<_,Option<String>>(8)?,"sandbox":r.get::<_,Option<String>>(9)?}))).map_err(err)
}
fn uncertain(c: &Connection, session: &str) -> Result<bool, String> {
    c.query_row("SELECT EXISTS(SELECT 1 FROM task_work_session_runs WHERE session_id=? AND status='needs_attention' AND dispatch_state IN ('dispatch_recorded','accepted','uncertain'))",[session],|r|r.get(0)).map_err(err)
}
fn fingerprint(v: &Value) -> Value {
    json!([
        v["settingsRevision"],
        v["model"],
        v["effectiveModel"],
        v["cwd"],
        v["approvalsReviewer"],
        v["reasoningEffort"],
        v["resolvedEffort"],
        v["approvalPolicy"],
        v["sandbox"]
    ])
}

impl TaskExecutionApplicationService {
    pub(crate) fn save_effective_effort(
        &self,
        task: &str,
        session: &str,
        effort: Option<&str>,
    ) -> Result<(), String> {
        let mut c = database::open(&self.db_path)?;
        let tx = database::immediate_transaction(&mut c)?;
        owned(&tx, task, session)?;
        super::task_service::ensure_task_mutable(&tx, task)?;
        let changed=tx.execute(
            "UPDATE task_work_sessions SET effective_reasoning_effort=? WHERE id=? AND effective_settings_revision IS NOT NULL",
            params![effort, session],
        )
        .map_err(err)?;
        if changed != 1 {
            return Err(
                "settings_revision_conflict: settings changed during reasoning preparation".into(),
            );
        }
        tx.commit().map_err(err)
    }

    pub(crate) fn queue_snapshot(&self, task: &str, session: &str) -> Result<Value, String> {
        let c = database::open(&self.db_path)?;
        owned(&c, task, session)?;
        let controller=c.query_row("SELECT mode,reason,revision,stop_run_id FROM task_session_queue_control WHERE session_id=?",[session],|r|Ok(json!({"mode":r.get::<_,String>(0)?,"reason":r.get::<_,String>(1)?,"revision":r.get::<_,i64>(2)?,"stopRunId":r.get::<_,Option<String>>(3)?}))).optional().map_err(err)?.unwrap_or(json!({"mode":"active","reason":"","revision":0}));
        let mut stmt=c.prepare("SELECT id,instruction,attachment_json,settings_json,position,revision,state,run_id FROM task_session_queue_items WHERE session_id=? AND state='pending' ORDER BY position,id").map_err(err)?;
        let items=stmt.query_map([session],|r|{let a:Option<String>=r.get(2)?;let s:String=r.get(3)?;Ok(json!({"id":r.get::<_,String>(0)?,"instruction":r.get::<_,String>(1)?,"attachment":a.and_then(|v|serde_json::from_str::<Value>(&v).ok()).map(|mut v|{v.as_object_mut().map(|m|m.remove("data"));v}),"settings":serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),"position":r.get::<_,i64>(4)?,"revision":r.get::<_,i64>(5)?,"state":r.get::<_,String>(6)?,"runId":r.get::<_,Option<String>>(7)?}))}).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
        Ok(json!({"control":controller,"items":items}))
    }
    pub(crate) fn queue_mutate(&self, input: &Value) -> Result<Value, String> {
        let task = required(input, "taskId")?;
        let session = required(input, "sessionId")?;
        let action = required(input, "action")?;
        let mut c = database::open(&self.db_path)?;
        let tx = database::immediate_transaction(&mut c)?;
        eligible(&tx, task, session)?;
        tx.execute(
            "INSERT OR IGNORE INTO task_session_queue_control(session_id) VALUES(?)",
            [session],
        )
        .map_err(err)?;
        match action {
            "enqueue" => {
                validate_attachment(input)?;
                let instruction = required(input, "instruction")?;
                if instruction.len() > 100_000 {
                    return Err("invalid_input: instruction too long".into());
                }
                let attachment = input
                    .get("attachment")
                    .filter(|v| !v.is_null())
                    .map(Value::to_string);
                if attachment.as_ref().is_some_and(|v| v.len() > 14_100_000) {
                    return Err("invalid_input: attachment exceeds 10 MB".into());
                }
                let hash = content_hash(&[instruction, attachment.as_deref().unwrap_or("")]);
                let operation = required(input, "operationId")?;
                let prior:Option<String>=tx.query_row("SELECT payload_hash FROM task_session_queue_items WHERE session_id=? AND operation_id=?",params![session,operation],|r|r.get(0)).optional().map_err(err)?;
                if let Some(prior) = prior {
                    if prior != hash {
                        return Err("operation_conflict: queue submission changed".into());
                    }
                } else {
                    let s = settings(&tx, session)?;
                    if s["settingsRevision"].is_null() || s["resolvedEffort"].is_null() {
                        return Err("execution_not_prepared: prepare and confirm model/reasoning before queueing".into());
                    }
                    let position:i64=tx.query_row("SELECT COALESCE(MAX(position),0)+1 FROM task_session_queue_items WHERE session_id=?",[session],|r|r.get(0)).map_err(err)?;
                    tx.execute("INSERT INTO task_session_queue_items(id,task_id,session_id,operation_id,payload_hash,instruction,attachment_json,settings_json,position) VALUES(?,?,?,?,?,?,?,?,?)",params![task_repository::new_id(),task,session,operation,hash,instruction,attachment,s.to_string(),position]).map_err(err)?;
                }
            }
            "edit" | "remove" | "move" => {
                let id = required(input, "itemId")?;
                let rev = input["expectedRevision"]
                    .as_i64()
                    .ok_or("invalid_input: expectedRevision is required")?;
                let mode: String = tx
                    .query_row(
                        "SELECT mode FROM task_session_queue_control WHERE session_id=?",
                        [session],
                        |r| r.get(0),
                    )
                    .map_err(err)?;
                if mode == "stop_then_drain" {
                    return Err("queue_conflict: stop confirmation is pending".into());
                }
                let position:i64=tx.query_row("SELECT position FROM task_session_queue_items WHERE id=? AND session_id=? AND state='pending' AND revision=?",params![id,session,rev],|r|r.get(0)).optional().map_err(err)?.ok_or("queue_conflict: request changed or already dispatched")?;
                if action == "remove" {
                    tx.execute("UPDATE task_session_queue_items SET state='cancelled',revision=revision+1 WHERE id=?",[id]).map_err(err)?;
                } else if action == "edit" {
                    let body = required(input, "instruction")?;
                    if body.len() > 100_000 {
                        return Err("invalid_input: instruction too long".into());
                    }
                    tx.execute("UPDATE task_session_queue_items SET instruction=?,revision=revision+1 WHERE id=?",params![body,id]).map_err(err)?;
                } else {
                    let direction = required(input, "direction")?;
                    if !["up", "down"].contains(&direction) {
                        return Err("invalid_input: direction".into());
                    }
                    let sql = if direction == "up" {
                        "SELECT id,position FROM task_session_queue_items WHERE session_id=? AND state='pending' AND position<? ORDER BY position DESC LIMIT 1"
                    } else {
                        "SELECT id,position FROM task_session_queue_items WHERE session_id=? AND state='pending' AND position>? ORDER BY position LIMIT 1"
                    };
                    if let Some((other, p)) = tx
                        .query_row(sql, params![session, position], |r| {
                            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
                        })
                        .optional()
                        .map_err(err)?
                    {
                        tx.execute("UPDATE task_session_queue_items SET position=?,revision=revision+1 WHERE id=?",params![p,id]).map_err(err)?;
                        tx.execute("UPDATE task_session_queue_items SET position=?,revision=revision+1 WHERE id=?",params![position,other]).map_err(err)?;
                    }
                }
            }
            "pause" | "resume" | "cancel_drain" | "stop_then_drain" => {
                if action == "resume" && uncertain(&tx, session)? {
                    return Err(
                        "uncertain_dispatch: review the uncertain Run before continuing".into(),
                    );
                }
                let stop = if action == "stop_then_drain" {
                    let run = required(input, "runId")?;
                    let live:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_work_session_runs WHERE id=? AND session_id=? AND task_id=? AND status IN ('running','awaiting_response'))",params![run,session,task],|r|r.get(0)).map_err(err)?;
                    if !live {
                        return Err("stale_run: current Run has changed".into());
                    }
                    Some(run)
                } else {
                    None
                };
                let mode = if action == "resume" {
                    "active"
                } else if stop.is_some() {
                    "stop_then_drain"
                } else {
                    "paused"
                };
                tx.execute("UPDATE task_session_queue_control SET mode=?,stop_run_id=?,reason=CASE WHEN ?='active' THEN 'reviewed' ELSE '' END WHERE session_id=?",params![mode,stop,mode,session]).map_err(err)?;
            }
            _ => return Err("invalid_input: unknown queue action".into()),
        }
        tx.execute(
            "UPDATE task_session_queue_control SET revision=revision+1 WHERE session_id=?",
            [session],
        )
        .map_err(err)?;
        tx.execute(
            "UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=?",
            [session],
        )
        .map_err(err)?;
        tx.commit().map_err(err)?;
        self.snapshot(task, session, None)
    }
    pub(crate) fn queue_sessions(&self) -> Result<Vec<(String, String)>, String> {
        let c = database::open(&self.db_path)?;
        let mut s=c.prepare("SELECT DISTINCT q.task_id,q.session_id FROM task_session_queue_items q JOIN task_session_queue_control c ON c.session_id=q.session_id WHERE q.state='pending' AND c.mode IN ('active','stop_then_drain')").map_err(err)?;
        let rows = s
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        Ok(rows)
    }
    pub(crate) fn pause_queue(&self, session: &str, reason: &str) -> Result<(), String> {
        let c = database::open(&self.db_path)?;
        c.execute("UPDATE task_session_queue_control SET mode='paused',reason=?,revision=revision+1 WHERE session_id=?",params![reason,session]).map_err(err)?;
        c.execute(
            "UPDATE task_work_sessions SET execution_revision=execution_revision+1 WHERE id=?",
            [session],
        )
        .map_err(err)?;
        Ok(())
    }
    pub(crate) fn queue_next(&self, task: &str, session: &str) -> Result<Option<Value>, String> {
        let c = database::open(&self.db_path)?;
        eligible(&c, task, session)?;
        if uncertain(&c, session)? {
            return Err("Outcome uncertain; review saved evidence before continuing".into());
        }
        let active:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM task_work_session_runs WHERE session_id=? AND status IN ('queued','running','awaiting_response'))",[session],|r|r.get(0)).map_err(err)?;
        if active {
            return Ok(None);
        }
        let (mode, stop): (String, Option<String>) = c
            .query_row(
                "SELECT mode,stop_run_id FROM task_session_queue_control WHERE session_id=?",
                [session],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(err)?;
        if !["active", "stop_then_drain"].contains(&mode.as_str()) {
            return Ok(None);
        }
        if let Some(stop) = stop {
            let status: String = c
                .query_row(
                    "SELECT status FROM task_work_session_runs WHERE id=? AND session_id=?",
                    params![stop, session],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if !["succeeded", "cancelled"].contains(&status.as_str()) {
                return Err("Stop is not confirmed; review saved evidence".into());
            }
        } else {
            let latest:Option<String>=c.query_row("SELECT status FROM task_work_session_runs WHERE session_id=? ORDER BY created_at DESC,rowid DESC LIMIT 1",[session],|r|r.get(0)).optional().map_err(err)?;
            // Explicit resume authorizes retrying pending work after review; never retry the failed Run itself.
            if latest.as_deref().is_some_and(|s| s != "succeeded") {
                let reason: String = c
                    .query_row(
                        "SELECT reason FROM task_session_queue_control WHERE session_id=?",
                        [session],
                        |r| r.get(0),
                    )
                    .map_err(err)?;
                if reason != "reviewed" {
                    return Err(
                        "Previous Run did not succeed; review before continuing pending requests"
                            .into(),
                    );
                }
            }
        }
        let row:Option<(String,String,Option<String>,String,i64)>=c.query_row("SELECT id,instruction,attachment_json,settings_json,revision FROM task_session_queue_items WHERE session_id=? AND state='pending' ORDER BY position,id LIMIT 1",[session],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(err)?;
        let Some((id, body, attachment, s, revision)) = row else {
            return Ok(None);
        };
        let s: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
        if fingerprint(&s) != fingerprint(&settings(&c, session)?) {
            return Err(
                "Execution settings changed; remove and requeue with explicitly reviewed settings"
                    .into(),
            );
        }
        let prepared = self.prepared(task, session)?;
        Ok(Some(
            json!({"taskId":task,"sessionId":session,"operationId":format!("queue:{id}"),"instruction":body,"attachment":attachment.and_then(|v|serde_json::from_str::<Value>(&v).ok()),"settingsRevision":prepared.settings_revision,"queueItemId":id,"queueItemRevision":revision,"resolvedEffort":s["resolvedEffort"]}),
        ))
    }
}

pub(super) fn validate_claim(
    tx: &Transaction<'_>,
    input: &Value,
    prepared: &PreparedExecution,
) -> Result<(), String> {
    let Some(id) = input.get("queueItemId").and_then(Value::as_str) else {
        return Ok(());
    };
    if uncertain(tx, &prepared.session_id)? {
        return Err("uncertain_dispatch: queue paused".into());
    }
    let row:Option<(String,i64,String,Option<String>)>=tx.query_row("SELECT settings_json,revision,instruction,attachment_json FROM task_session_queue_items WHERE id=? AND task_id=? AND session_id=? AND state='pending'",params![id,prepared.task_id,prepared.session_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(err)?;
    let (s, rev, body, attachment) = row.ok_or("queue_conflict: request was already claimed")?;
    if input["queueItemRevision"].as_i64() != Some(rev) {
        return Err("queue_conflict: request changed".into());
    }
    let allowed:bool=tx.query_row("SELECT mode IN ('active','stop_then_drain') FROM task_session_queue_control WHERE session_id=?",[&prepared.session_id],|r|r.get(0)).map_err(err)?;
    if !allowed {
        return Err("queue_conflict: queue paused".into());
    }
    let expected_revision: Option<String> = tx
        .query_row(
            "SELECT effective_settings_revision FROM task_work_sessions WHERE id=?",
            [&prepared.session_id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if expected_revision.as_deref() != Some(prepared.settings_revision.as_str()) {
        return Err("settings_revision_conflict: queued preparation is stale".into());
    }
    let head:String=tx.query_row("SELECT id FROM task_session_queue_items WHERE session_id=? AND state='pending' ORDER BY position,id LIMIT 1",[&prepared.session_id],|r|r.get(0)).map_err(err)?;
    if head != id
        || input["instruction"].as_str() != Some(body.as_str())
        || input
            .get("attachment")
            .filter(|v| !v.is_null())
            .map(Value::to_string)
            != attachment
    {
        return Err("queue_conflict: pending request payload/order changed".into());
    }
    let stop: Option<String> = tx
        .query_row(
            "SELECT stop_run_id FROM task_session_queue_control WHERE session_id=?",
            [&prepared.session_id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if let Some(stop) = stop {
        let confirmed:bool=tx.query_row("SELECT status IN ('succeeded','cancelled') FROM task_work_session_runs WHERE id=? AND session_id=?",params![stop,prepared.session_id],|r|r.get(0)).map_err(err)?;
        if !confirmed {
            return Err("queue_conflict: target termination unconfirmed".into());
        }
    }
    let s: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    if input["resolvedEffort"] != s["resolvedEffort"] {
        return Err("queue_conflict: reasoning snapshot changed".into());
    }
    if fingerprint(&s) != fingerprint(&settings(tx, &prepared.session_id)?) {
        return Err("settings_revision_conflict: queued execution settings changed".into());
    }
    Ok(())
}
pub(super) fn link_claim(tx: &Transaction<'_>, input: &Value, run: &str) -> Result<(), String> {
    if let Some(id) = input.get("queueItemId").and_then(Value::as_str) {
        tx.execute("UPDATE task_session_queue_items SET state='linked_run',run_id=?,revision=revision+1 WHERE id=? AND state='pending'",params![run,id]).map_err(err)?;
        tx.execute("UPDATE task_session_queue_control SET mode='active',stop_run_id=NULL,reason='',revision=revision+1 WHERE session_id=?",[required(input,"sessionId")?]).map_err(err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::task_service::TaskApplicationService;
    struct Fixture {
        _root: tempfile::TempDir,
        service: TaskExecutionApplicationService,
        tasks: TaskApplicationService,
        task: String,
        session: String,
    }
    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let tasks = TaskApplicationService::new(&db);
        let task = tasks
            .execute(
                "task.create",
                &json!({"operationId":"task","inputText":"Preserve context","title":"Queue"}),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let session = tasks
            .execute(
                "task.work-session.create",
                &json!({"operationId":"session","taskId":task,"title":"Work"}),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        tasks.execute("task.work-session.update",&json!({"operationId":"settings","taskId":task,"sessionId":session,"title":"Work","provider":"codex","model":"gpt-5.6-sol","workspacePath":root.path().to_string_lossy()})).unwrap();
        let service = TaskExecutionApplicationService::new(&db);
        let p = service.prepared(&task, &session).unwrap();
        service.bind_thread(&p,&json!({"id":"thread","model":p.model,"cwd":p.cwd,"approvalPolicy":"on-request","sandbox":"workspace-write"}),true).unwrap();
        service
            .save_effective_effort(&task, &session, Some("medium"))
            .unwrap();
        Fixture {
            _root: root,
            service,
            tasks,
            task,
            session,
        }
    }
    impl Fixture {
        fn action(&self, action: &str) -> Value {
            json!({"taskId":self.task,"sessionId":self.session,"action":action})
        }
        fn enqueue(&self, key: &str, body: &str) -> Value {
            let mut i = self.action("enqueue");
            i["operationId"] = json!(key);
            i["instruction"] = json!(body);
            self.service.queue_mutate(&i).unwrap()
        }
        fn next(&self) -> Option<Value> {
            self.service.queue_next(&self.task, &self.session).unwrap()
        }
        fn create(&self) -> String {
            self.service.create_run(&self.next().unwrap()).unwrap().0["selectedRun"]["id"]
                .as_str()
                .unwrap()
                .to_owned()
        }
    }
    #[test]
    fn task_projection_refresh_hint_covers_runnable_queue_between_runs_only() {
        let f = fixture();
        let tasks =
            crate::application::task_service::TaskApplicationService::new(&f.service.db_path);
        f.enqueue("pending", "Later work");
        let read = || {
            tasks
                .execute("task.get", &json!({"taskId":f.task}))
                .unwrap()
        };
        assert_eq!(read()["workLogRefreshPending"], true);
        assert!(read()["workLog"].as_array().unwrap().is_empty());
        f.service
            .queue_mutate(&json!({"taskId":f.task,"sessionId":f.session,"action":"pause"}))
            .unwrap();
        assert_eq!(read()["workLogRefreshPending"], false);
        f.service
            .queue_mutate(&json!({"taskId":f.task,"sessionId":f.session,"action":"resume"}))
            .unwrap();
        assert_eq!(read()["workLogRefreshPending"], true);
    }

    #[test]
    fn enqueue_is_persistent_idempotent_and_creates_no_run_or_log() {
        let f = fixture();
        let one = f.enqueue("one", "First");
        let two = f.enqueue("one", "First");
        assert_eq!(one["queue"]["items"], two["queue"]["items"]);
        assert!(two["runs"].as_array().unwrap().is_empty());
        let c = database::open(&f.service.db_path).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM task_work_log_entries", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let reopened = TaskExecutionApplicationService::new(&f.service.db_path);
        assert_eq!(
            reopened.queue_snapshot(&f.task, &f.session).unwrap()["items"][0]["settings"]
                ["resolvedEffort"],
            "medium"
        );
        let mut conflict = f.action("enqueue");
        conflict["operationId"] = json!("one");
        conflict["instruction"] = json!("Different");
        assert!(f
            .service
            .queue_mutate(&conflict)
            .unwrap_err()
            .contains("operation_conflict"));
    }
    #[test]
    fn claim_is_atomic_head_only_and_replays_one_run() {
        let f = fixture();
        f.enqueue("one", "First");
        f.enqueue("two", "Second");
        let input = f.next().unwrap();
        let (first, replay) = f.service.create_run(&input).unwrap();
        assert!(!replay);
        let (same, replay) = f.service.create_run(&input).unwrap();
        assert!(replay);
        assert_eq!(first["selectedRun"]["id"], same["selectedRun"]["id"]);
        assert!(f.next().is_none());
        assert_eq!(same["queue"]["items"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn edits_and_reorders_invalidate_a_stale_dispatch_without_losing_composer_payload() {
        let f = fixture();
        let snap = f.enqueue("one", "First");
        f.enqueue("two", "Second");
        let stale = f.next().unwrap();
        let item = &snap["queue"]["items"][0];
        let mut edit = f.action("edit");
        edit["itemId"] = item["id"].clone();
        edit["expectedRevision"] = item["revision"].clone();
        edit["instruction"] = json!("Changed");
        f.service.queue_mutate(&edit).unwrap();
        assert!(f
            .service
            .create_run(&stale)
            .unwrap_err()
            .contains("queue_conflict"));
        assert!(f.service.queue_mutate(&edit).is_err());
        let mut moved = f.action("move");
        moved["itemId"] = item["id"].clone();
        moved["expectedRevision"] = json!(2);
        moved["direction"] = json!("down");
        f.service.queue_mutate(&moved).unwrap();
        assert_eq!(f.next().unwrap()["instruction"], "Second");
    }
    #[test]
    fn stop_ack_does_not_release_queue_but_confirmed_target_cancellation_does() {
        let f = fixture();
        f.enqueue("one", "Current");
        let run = f.create();
        f.service.mark_dispatch_recorded(&run).unwrap();
        f.service.accept_turn(&run, "thread", "turn").unwrap();
        f.enqueue("two", "Follow up");
        let mut stop = f.action("stop_then_drain");
        stop["runId"] = json!(run);
        f.service.queue_mutate(&stop).unwrap();
        f.service
            .request_interrupt(&f.task, &f.session, &run)
            .unwrap();
        assert!(f.next().is_none());
        f.service
            .finish_turn(&run, &json!({"status":"interrupted"}))
            .unwrap();
        assert_eq!(f.next().unwrap()["instruction"], "Follow up");
        let next = f.create();
        assert_ne!(next, run);
        assert_eq!(
            f.service.queue_snapshot(&f.task, &f.session).unwrap()["control"]["stopRunId"],
            Value::Null
        );
    }
    #[test]
    fn failures_and_uncertain_dispatch_never_automatically_resend() {
        let f = fixture();
        f.enqueue("one", "Current");
        let run = f.create();
        f.service.mark_dispatch_recorded(&run).unwrap();
        f.service
            .mark_dispatch_uncertain(&run, "response lost")
            .unwrap();
        f.enqueue("two", "Next");
        assert!(f
            .service
            .queue_next(&f.task, &f.session)
            .unwrap_err()
            .contains("uncertain"));
        assert!(f.service.queue_mutate(&f.action("resume")).is_err());
        f.service.recover_nonterminal().unwrap();
        assert_eq!(
            f.service.queue_snapshot(&f.task, &f.session).unwrap()["control"]["mode"],
            "reconcile_needed"
        );
    }
    #[test]
    fn changed_settings_do_not_mutate_or_dispatch_captured_work() {
        let f = fixture();
        let original = f.enqueue("one", "First")["queue"]["items"][0]["settings"].clone();
        f.service
            .save_effective_effort(&f.task, &f.session, Some("high"))
            .unwrap();
        assert!(f
            .service
            .queue_next(&f.task, &f.session)
            .unwrap_err()
            .contains("settings changed"));
        assert_eq!(
            f.service.queue_snapshot(&f.task, &f.session).unwrap()["items"][0]["settings"],
            original
        );
    }
    #[test]
    fn queue_blocks_completion_and_hold_but_cancelled_pending_items_release_guards() {
        let f = fixture();
        let snap = f.enqueue("one", "First");
        assert!(TaskApplicationService::has_active_execution(
            &database::open(&f.service.db_path).unwrap(),
            &f.task
        )
        .unwrap());
        let item = &snap["queue"]["items"][0];
        let mut remove = f.action("remove");
        remove["itemId"] = item["id"].clone();
        remove["expectedRevision"] = item["revision"].clone();
        f.service.queue_mutate(&remove).unwrap();
        assert!(!TaskApplicationService::has_active_execution(
            &database::open(&f.service.db_path).unwrap(),
            &f.task
        )
        .unwrap());
        database::open(&f.service.db_path)
            .unwrap()
            .execute("UPDATE tasks SET state='in_progress' WHERE id=?", [&f.task])
            .unwrap();
        f.tasks.execute("task.completion.create",&json!({"operationId":"complete","taskId":f.task,"expectedTaskRevision":1,"evidence":"Checked"})).unwrap();
        assert!(f.service.queue_mutate(&f.action("resume")).is_err());
    }
    #[test]
    fn tampered_claim_payload_cannot_reuse_pending_item() {
        let f = fixture();
        f.enqueue("one", "Original");
        let mut i = f.next().unwrap();
        i["instruction"] = json!("Unrelated");
        assert!(f
            .service
            .create_run(&i)
            .unwrap_err()
            .contains("queue_conflict"));
        assert_eq!(f.next().unwrap()["instruction"], "Original");
    }
    #[test]
    fn revision_only_consent_change_blocks_old_queue_and_stale_claim() {
        let f = fixture();
        f.enqueue("one", "Original");
        let stale = f.next().unwrap();
        let c = database::open(&f.service.db_path).unwrap();
        c.execute("UPDATE task_work_sessions SET settings_epoch=settings_epoch+1,effective_settings_revision=NULL WHERE id=?",[&f.session]).unwrap();
        let p = f.service.prepared(&f.task, &f.session).unwrap();
        f.service.bind_thread(&p,&json!({"id":"thread","model":p.model,"cwd":p.cwd,"approvalPolicy":"on-request","sandbox":"workspace-write"}),true).unwrap();
        f.service
            .save_effective_effort(&f.task, &f.session, Some("medium"))
            .unwrap();
        assert!(f
            .service
            .queue_next(&f.task, &f.session)
            .unwrap_err()
            .contains("settings changed"));
        assert!(f
            .service
            .create_run(&stale)
            .unwrap_err()
            .contains("settings_revision_conflict"));
        assert_eq!(
            f.service.queue_snapshot(&f.task, &f.session).unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn settings_save_between_preparation_and_binding_cannot_restore_stale_consent() {
        let f = fixture();
        let p = f.service.prepared(&f.task, &f.session).unwrap();
        let c = database::open(&f.service.db_path).unwrap();
        c.execute("UPDATE task_work_sessions SET settings_epoch=settings_epoch+1,effective_settings_revision=NULL WHERE id=?",[&f.session]).unwrap();
        assert!(f.service.bind_thread(&p,&json!({"id":"thread","model":p.model,"cwd":p.cwd,"approvalPolicy":"on-request","sandbox":"workspace-write"}),true).unwrap_err().contains("settings_revision_conflict"));
        assert!(c
            .query_row(
                "SELECT effective_settings_revision FROM task_work_sessions WHERE id=?",
                [&f.session],
                |r| r.get::<_, Option<String>>(0)
            )
            .unwrap()
            .is_none());
    }
    #[test]
    fn note_append_does_not_change_queue_consent_revision() {
        let f = fixture();
        f.enqueue("one", "First");
        let before = f.next().unwrap();
        f.tasks.execute("task.work-session.entry.create",&json!({"operationId":"note","taskId":f.task,"sessionId":f.session,"body":"A separate local note"})).unwrap();
        assert_eq!(
            f.next().unwrap()["settingsRevision"],
            before["settingsRevision"]
        );
    }
    #[test]
    fn reasoning_confirmation_cannot_restore_settings_invalidated_by_a_concurrent_save() {
        let f = fixture();
        let c = database::open(&f.service.db_path).unwrap();
        c.execute("UPDATE task_work_sessions SET settings_epoch=settings_epoch+1,effective_settings_revision=NULL,effective_reasoning_effort=NULL WHERE id=?",[&f.session]).unwrap();
        assert!(f
            .service
            .save_effective_effort(&f.task, &f.session, Some("high"))
            .unwrap_err()
            .contains("settings_revision_conflict"));
        assert!(c
            .query_row(
                "SELECT effective_reasoning_effort FROM task_work_sessions WHERE id=?",
                [&f.session],
                |r| r.get::<_, Option<String>>(0)
            )
            .unwrap()
            .is_none());
    }
}
