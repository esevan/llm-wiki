use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tauri::command]
pub(crate) async fn desktop_e2e_resize_window(
    window: tauri::WebviewWindow,
    width: f64,
    height: f64,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    if !(900.0..=1600.0).contains(&width) || !(640.0..=1200.0).contains(&height) {
        return Err("Desktop E2E window size is outside the supported fixture bounds".into());
    }
    window
        .set_size(tauri::LogicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    for _ in 0..120 {
        tokio::time::sleep(Duration::from_millis(16)).await;
        let scale = window.scale_factor().map_err(|error| error.to_string())?;
        let size = window.inner_size().map_err(|error| error.to_string())?;
        let window_width = size.width as f64 / scale;
        let window_height = size.height as f64 / scale;
        if (window_width - width).abs() <= 2.0 && (window_height - height).abs() <= 2.0 {
            return Ok(
                json!({"windowWidth":window_width,"windowHeight":window_height,"requestedWidth":width,"requestedHeight":height}),
            );
        }
    }
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    Err(format!(
        "Desktop E2E window did not reach requested inner size {width}×{height}; native window inner size was {}×{}",
        size.width as f64 / scale,
        size.height as f64 / scale
    ))
}

#[tauri::command]
pub(crate) async fn desktop_e2e_provider_requests(base_url: String) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    let parsed = reqwest::Url::parse(&base_url).map_err(|error| error.to_string())?;
    if parsed.scheme() != "http" || !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost")) {
        return Err("Desktop E2E provider evidence is limited to the local fixture".into());
    }
    let evidence_url = format!(
        "{}/__test/requests",
        base_url.trim_end_matches("/v1").trim_end_matches('/')
    );
    reqwest::Client::new()
        .get(evidence_url)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_e2e_arm_one_shot_failure(
    application: tauri::State<'_, crate::native::NativeApplication>,
    operation: String,
) -> Result<(), String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    if !matches!(
        operation.as_str(),
        "refinement.context" | "knowledge.read" | "workbench.get" | "task-refinement.workspace" | "task.get"
    ) {
        return Err("Unsupported desktop E2E failure operation".into());
    }
    let connection = crate::native::database::open(&application.db_path())?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS desktop_e2e_failures(operation TEXT PRIMARY KEY);",
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT OR REPLACE INTO desktop_e2e_failures(operation) VALUES(?)",
            [operation],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) fn desktop_e2e_seed_legacy_refinement(
    application: tauri::State<'_, crate::native::NativeApplication>,
    statement: String,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    let statement = statement.trim();
    if statement.is_empty() {
        return Err("Legacy refinement statement is required".into());
    }
    let problem_id = uuid::Uuid::new_v4().to_string();
    let item_id = format!("legacy-problem-{problem_id}");
    let created_at = chrono::Utc::now().to_rfc3339();
    let mut connection = crate::native::database::open(&application.db_path())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO problems(id,capture_id,statement,detail,state,created_at,current_revision) VALUES(?,NULL,?,'','draft',?,2)",
            rusqlite::params![problem_id, statement, created_at],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO problem_revisions(problem_id,revision,statement,detail,content_hash,author,created_at) VALUES(?,1,?,'',?,'migration',?),(?,2,?,'',?,'migration',?)",
            rusqlite::params![problem_id, format!("Earlier {statement}"), format!("desktop-e2e-r1-{problem_id}"), created_at, problem_id, statement, format!("desktop-e2e-r2-{problem_id}"), created_at],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO refinement_items(id,problem_id,capture_id,problem_revision,source_kind,created_at) VALUES(?,?,NULL,2,'legacy_problem',?)",
            rusqlite::params![item_id, problem_id, created_at],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(json!({"problemId":problem_id,"problemRevision":2,"itemId":item_id}))
}

/// Creates an isolated, already-completed tracking session so the packaged UI
/// can exercise its review-only Knowledge controls without writing a user vault.
#[tauri::command]
pub(crate) fn desktop_e2e_seed_completed_tracking(
    application: tauri::State<'_, crate::native::NativeApplication>,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    fn call(
        application: &crate::native::NativeApplication,
        name: &str,
        input: Value,
    ) -> Result<Value, String> {
        if name == "work_tracking.advance" {
            let mut proposal = input;
            proposal
                .as_object_mut()
                .ok_or("Invalid fixture proposal")?
                .remove("decision");
            proposal
                .as_object_mut()
                .ok_or("Invalid fixture proposal")?
                .entry("operationId")
                .or_insert_with(|| Value::String(uuid::Uuid::new_v4().to_string()));
            let preview = call(
                application,
                "work_tracking.advance.preview",
                proposal.clone(),
            )?;
            return call(
                application,
                "work_tracking.advance.review",
                json!({"proposal":proposal,"reviewState":preview["reviewState"],"decision":"accept"}),
            );
        }
        let response = application.execute_work_tracking(crate::native::NativeOperation {
            name: name.into(),
            input,
        });
        if response.status >= 300 {
            return Err(response.body.to_string());
        }
        Ok(response.body)
    }
    let open_input = json!({"operationId":uuid::Uuid::new_v4().to_string(),"lineageKey":format!("desktop-e2e-knowledge-{}",uuid::Uuid::new_v4()),"mode":"create","capture":{"title":"E2E Knowledge review","summary":"An isolated completed Task for review controls"}});
    let open = call(&application, "work_tracking.open", open_input.clone())?;
    let opened = if open["decisionRequired"] == true {
        call(
            &application,
            "work_tracking.open.review",
            json!({"proposal":open_input,"reviewState":open["reviewState"],"decision":"accept"}),
        )?
    } else {
        open
    };
    let session_id = opened["sessionId"]
        .as_str()
        .ok_or("Tracking session was not opened")?
        .to_owned();
    let task_event = call(
        &application,
        "work_tracking.append",
        json!({"operationId":uuid::Uuid::new_v4().to_string(),"sessionId":session_id,"expectedHeadRevision":opened["headRevision"],"event":{"kind":"task_created","title":"E2E Knowledge Task","outcome":"Exercise explicit Knowledge review"}}),
    )?;
    let task = call(
        &application,
        "work_tracking.advance",
        json!({"sessionId":session_id,"expectedHeadRevision":task_event["headRevision"],"sourceEventId":task_event["eventId"],"action":"create_task","decision":"accept","proposedPayload":{"title":"E2E Knowledge Task","outcome":"Exercise explicit Knowledge review","scope":"Isolated fixture","validationCriteria":"UI controls are visible"}}),
    )?;
    let transition = call(
        &application,
        "work_tracking.append",
        json!({"operationId":uuid::Uuid::new_v4().to_string(),"sessionId":session_id,"expectedHeadRevision":task["headRevision"],"event":{"kind":"task_transition_proposed","taskId":task["resultEntityId"],"expectedTaskRevision":1,"to":"in_progress"}}),
    )?;
    let started = call(
        &application,
        "work_tracking.advance",
        json!({"sessionId":session_id,"expectedHeadRevision":transition["headRevision"],"sourceEventId":transition["eventId"],"action":"transition_task","decision":"accept","proposedPayload":{"taskId":task["resultEntityId"],"expectedTaskRevision":1,"to":"in_progress"}}),
    )?;
    let completion = call(
        &application,
        "work_tracking.append",
        json!({"operationId":uuid::Uuid::new_v4().to_string(),"sessionId":session_id,"expectedHeadRevision":started["headRevision"],"event":{"kind":"completion_proposal","outcomes":["reviewed"],"verification":["isolated fixture"]}}),
    )?;
    call(
        &application,
        "work_tracking.advance",
        json!({"sessionId":session_id,"expectedHeadRevision":completion["headRevision"],"sourceEventId":completion["eventId"],"action":"complete_task","decision":"accept","proposedPayload":{"taskId":task["resultEntityId"],"expectedTaskRevision":1,"evidence":"isolated fixture","report":"Completed only to expose review controls"}}),
    )?;
    Ok(json!({"sessionId":session_id}))
}

#[tauri::command]
pub(crate) fn desktop_e2e_seed_queue_notifications(
    application: tauri::State<'_, crate::native::NativeApplication>,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    let queued = uuid::Uuid::new_v4().to_string();
    let running = uuid::Uuid::new_v4().to_string();
    let failed = uuid::Uuid::new_v4().to_string();
    let missing_key = uuid::Uuid::new_v4().to_string();
    let completed = uuid::Uuid::new_v4().to_string();
    let notification_open = uuid::Uuid::new_v4().to_string();
    let notification_dismiss = uuid::Uuid::new_v4().to_string();
    crate::native::vault::atomic_write(
        &application.vault_path(),
        "queue-recovery.md",
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Queue recovery evidence\n\nA reusable local result.",
    )?;
    let mut connection = crate::native::database::open(&application.db_path())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO captures(id,text,created_at) VALUES('fixture-capture','Queue conflict-review source',CURRENT_TIMESTAMP)",
        [],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,error_code,error_message,finished_at,created_at) VALUES(?,'knowledge_translation','knowledge','queue-recovery.md','failed',?,?,'provider_error','Configure an API key in AI setup before using AI',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        rusqlite::params![missing_key, json!({"path":"queue-recovery.md","locale":"ko"}).to_string(), format!("desktop-e2e-{missing_key}")],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO problems(id,capture_id,statement,detail,state,created_at,current_revision) VALUES('fixture-problem','fixture-capture','Queue conflict-review Problem','Fixture for persisted review decisions','open',CURRENT_TIMESTAMP,1)",
        [],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO problem_revisions(problem_id,revision,statement,detail,content_hash,author,created_at) VALUES('fixture-problem',1,'Queue conflict-review Problem','Fixture for persisted review decisions','desktop-e2e-queue-problem','desktop-e2e',CURRENT_TIMESTAMP)",
        [],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO features(id,problem_id,title,outcome,non_goals,conflict_state,validation_criteria,state,created_at) VALUES('fixture-solution','fixture-problem','Queue conflict-review Solution','Persist a reviewed conflict decision','','conflicted','The explicit decision is visible after readback','proposed',CURRENT_TIMESTAMP)",
        [],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,created_at) VALUES(?,'workflow_draft','captures','fixture-queued','queued','{}',?,CURRENT_TIMESTAMP)",
        rusqlite::params![queued, format!("desktop-e2e-{queued}")],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,worker_id,started_at,created_at) VALUES(?,'workflow_refinement','captures','fixture-running','running','{}',?,'desktop-e2e-worker',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        rusqlite::params![running, format!("desktop-e2e-{running}")],
    ).map_err(|error| error.to_string())?;
    transaction.execute(
        "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,error_code,error_message,finished_at,created_at) VALUES(?,'workflow_draft','captures','fixture-failed','failed','{}',?,'provider_error','deterministic fixture failure',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        rusqlite::params![failed, format!("desktop-e2e-{failed}")],
    ).map_err(|error| error.to_string())?;
    let result = json!({"feature_id":"fixture-solution","summary":"Deterministic completed review","conflicts":[],"scope":{"documents":0,"semantic_ready":0,"embedding_coverage":0}});
    transaction.execute(
        "INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,result_json,idempotency_key,result_interface,progress_completed,finished_at,created_at) VALUES(?,'conflict_review','features','fixture-solution','completed','{}',?,?,'conflict_review',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
        rusqlite::params![completed, result.to_string(), format!("desktop-e2e-{completed}")],
    ).map_err(|error| error.to_string())?;
    for (id, kind, title) in [
        (
            &notification_open,
            "fixture_open",
            "Open deterministic review",
        ),
        (
            &notification_dismiss,
            "fixture_dismiss",
            "Dismiss deterministic review",
        ),
    ] {
        transaction.execute(
            "INSERT INTO notifications(id,job_id,kind,title,target_json,created_at) VALUES(?,?,?,?,?,CURRENT_TIMESTAMP)",
            rusqlite::params![id, completed, kind, title, json!({"entity_type":"features","entity_id":"fixture-solution"}).to_string()],
        ).map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(
        json!({"queued":queued,"running":running,"failed":failed,"missingKey":missing_key,"completed":completed,"notificationOpen":notification_open,"notificationDismiss":notification_dismiss}),
    )
}

/// Packaged acceptance only: a real stdio child calls back into this GUI's IPC.
/// No test helper opens the database or substitutes an application service here.
#[tauri::command]
pub(crate) async fn desktop_e2e_mcp_probe(
    connection_id: String,
    revoked: bool,
    task_id: Option<String>,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    let mut child =
        tokio::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .args(["--mcp", "--connection", &connection_id])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("Missing child input")?;
    let mut output = BufReader::new(child.stdout.take().ok_or("Missing child output")?);
    async fn exchange(
        input: &mut tokio::process::ChildStdin,
        output: &mut BufReader<tokio::process::ChildStdout>,
        request: Value,
    ) -> Result<Value, String> {
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        input.flush().await.map_err(|e| e.to_string())?;
        let mut line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            output.read_line(&mut line),
        )
        .await
        .map_err(|_| "MCP child timed out")?
        .map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|_| "MCP child returned no valid response".into())
    }
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"packaged-acceptance","version":"1"},"io.modelcontextprotocol/clientCapabilities":{"elicitation":{}}});
    let discovered = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta}}),
    )
    .await;
    if revoked {
        if discovered.is_ok() {
            return Err("Revoked connection still received protocol data".into());
        }
        return Ok(json!({"revoked":true}));
    }
    discovered?;
    let tools = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta}}),
    )
    .await?;
    if !tools["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools.iter().any(|t| t["name"] == "inbound_work_open"))
    {
        return Err("Packaged MCP tools missing".into());
    }
    if let Some(task_id) = task_id {
        fn task_snapshot(value: &Value, task_id: &str) -> Option<Value> {
            if let Some(object) = value.as_object() {
                let matches = ["id", "taskId", "entityRef"]
                    .iter()
                    .any(|key| object.get(*key).and_then(Value::as_str) == Some(task_id));
                if matches
                    && object.get("problemLinks").is_some_and(Value::is_array)
                    && object.get("relationships").is_some_and(Value::is_array)
                {
                    return Some(value.clone());
                }
                return object
                    .values()
                    .find_map(|item| task_snapshot(item, task_id));
            }
            value
                .as_array()?
                .iter()
                .find_map(|item| task_snapshot(item, task_id))
        }
        fn exact_problem_links(task: &Value) -> Result<Vec<(String, i64)>, String> {
            let mut links = task["problemLinks"]
                .as_array()
                .ok_or("MCP Task has no Problem links")?
                .iter()
                .map(|link| {
                    Ok((
                        link["problemId"]
                            .as_str()
                            .ok_or("MCP Problem link has no identity")?
                            .to_owned(),
                        link["problemRevision"]
                            .as_i64()
                            .ok_or("MCP Problem link has no exact revision")?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            links.sort();
            if links.is_empty() {
                return Err("MCP Task lost the desktop Problem link".into());
            }
            Ok(links)
        }
        fn exact_relationships(
            task: &Value,
        ) -> Result<Vec<(String, String, String, String)>, String> {
            let mut relationships = task["relationships"]
                .as_array()
                .ok_or("MCP Task has no relationship array")?
                .iter()
                .map(|link| {
                    Ok((
                        link["id"]
                            .as_str()
                            .ok_or("MCP relationship has no identity")?
                            .to_owned(),
                        link["sourceTaskId"]
                            .as_str()
                            .ok_or("MCP relationship has no source")?
                            .to_owned(),
                        link["targetTaskId"]
                            .as_str()
                            .ok_or("MCP relationship has no target")?
                            .to_owned(),
                        link["kind"]
                            .as_str()
                            .ok_or("MCP relationship has no kind")?
                            .to_owned(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            relationships.sort();
            if relationships.is_empty() {
                return Err("MCP Task lost the desktop relationship".into());
            }
            Ok(relationships)
        }
        let current = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"workbench_current","arguments":{},"_meta":meta}})).await?;
        let current_task = task_snapshot(&current["result"]["structuredContent"], &task_id)
            .ok_or("Direct desktop Task missing from scoped current MCP projection")?;
        let overview = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"workbench_overview","arguments":{"limit":50},"_meta":meta}})).await?;
        let overview_task = task_snapshot(&overview["result"]["structuredContent"], &task_id)
            .ok_or("Direct desktop Task missing from MCP overview")?;
        let links = exact_problem_links(&current_task)?;
        let relationships = exact_relationships(&current_task)?;
        if links != exact_problem_links(&overview_task)? {
            return Err("MCP current and overview disagree on exact Problem revisions".into());
        }
        if relationships != exact_relationships(&overview_task)? {
            return Err("MCP current and overview disagree on exact Task relationships".into());
        }
        let rejected_args = json!({"operationId":"packaged-task-cancel","lineageKey":"packaged-task","mode":"continue_task","taskId":task_id});
        let rejected_preview = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"inbound_work_open","arguments":rejected_args,"_meta":meta}})).await?;
        if rejected_preview["result"]["resultType"] != "input_required" {
            return Err("Direct Task continuation bypassed exact review".into());
        }
        let rejected = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"inbound_work_open","arguments":rejected_args,"requestState":rejected_preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"reject"}}},"_meta":meta}})).await?;
        let rejected = &rejected["result"]["structuredContent"];
        if rejected["decision"] != "reject"
            || rejected["sessionId"].as_str().is_some()
            || rejected["created"] == true
        {
            return Err("Rejected Task continuation created durable work".into());
        }
        let args = json!({"operationId":"packaged-task-accept","lineageKey":"packaged-task","mode":"continue_task","taskId":task_id});
        let preview = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"_meta":meta}})).await?;
        if preview["result"]["resultType"] != "input_required" {
            return Err("Accepted Task continuation had no exact review".into());
        }
        let accepted = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"requestState":preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta}})).await?;
        let accepted = &accepted["result"]["structuredContent"];
        let session_id = accepted["sessionId"]
            .as_str()
            .ok_or("Reviewed Task continuation failed")?;
        if accepted["created"] != true || accepted["headRevision"] != 1 {
            return Err(
                "Cancelled continuation had already created a session or work event".into(),
            );
        }
        if !accepted["captureId"].is_null() {
            return Err("Direct Task continuation fabricated a Capture".into());
        }
        let session = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"inbound_work_session_read","arguments":{"sessionId":session_id},"_meta":meta}})).await?;
        let session = &session["result"]["structuredContent"];
        if !session["capture"].is_null() {
            return Err("Captureless session was projected as a Capture".into());
        }
        if !session["recentEvents"]
            .as_array()
            .is_some_and(|events| events.len() == 1 && events[0]["kind"] == "task_binding")
        {
            return Err(
                "Task continuation created events other than the existing-Task binding".into(),
            );
        }
        let bound_task = task_snapshot(session, &task_id)
            .ok_or("Continued MCP session lost its canonical Task")?;
        if links != exact_problem_links(&bound_task)?
            || relationships != exact_relationships(&bound_task)?
        {
            return Err("Continued MCP session lost exact desktop relationships".into());
        }
        let task_revision = bound_task["taskRevision"]
            .as_i64()
            .ok_or("Continued Task has no exact revision")?;
        let detail = "Edited through the packaged stdio MCP child after captureless continuation";
        let append_args = json!({"operationId":"packaged-task-revision-proposal","sessionId":session_id,"expectedHeadRevision":session["headRevision"],"event":{"kind":"task_revision_proposed","taskId":task_id,"expectedTaskRevision":task_revision,"patch":{"detail":detail}}});
        let appended = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"inbound_work_append","arguments":append_args,"_meta":meta}})).await?;
        let appended = &appended["result"]["structuredContent"];
        let source_event = appended["eventId"]
            .as_str()
            .ok_or("Task revision proposal was not durably appended")?;
        let advance_args = json!({"operationId":"packaged-task-revision-accept","sessionId":session_id,"sourceEventId":source_event,"expectedHeadRevision":appended["headRevision"],"action":"task.revision","proposedPayload":{"taskId":task_id,"expectedTaskRevision":task_revision,"patch":{"detail":detail}}});
        let advance_preview = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":19,"method":"tools/call","params":{"name":"inbound_work_advance","arguments":advance_args,"_meta":meta}})).await?;
        if advance_preview["result"]["resultType"] != "input_required" {
            return Err("Canonical Task revision bypassed exact action review".into());
        }
        let advanced = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"inbound_work_advance","arguments":advance_args,"requestState":advance_preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta}})).await?;
        if advanced["result"]["structuredContent"]["decision"] != "accept" {
            return Err("Reviewed canonical Task revision failed through MCP".into());
        }
        let context = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"task_context_read","arguments":{"taskId":task_id},"_meta":meta}})).await?;
        let context = &context["result"]["structuredContent"];
        if context["task"]["detail"] != detail
            || context["task"]["taskRevision"] != task_revision + 1
            || context["readiness"]["entries"].as_array().is_none()
        {
            return Err("Scoped Task context did not reflect the exact MCP mutation".into());
        }
        let session_before_replay = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":211,"method":"tools/call","params":{"name":"inbound_work_session_read","arguments":{"sessionId":session_id},"_meta":meta}})).await?;
        let head_before_replay = session_before_replay["result"]["structuredContent"]
            ["headRevision"]
            .as_i64()
            .ok_or("Task session has no head revision before replay")?;
        let lineage = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"task_lineage_read","arguments":{"taskId":task_id},"_meta":meta}})).await?;
        if lineage["result"]["structuredContent"]["taskId"] != task_id
            || lineage["result"]["structuredContent"]["nodes"]
                .as_array()
                .is_none()
        {
            return Err("Active Task lineage could not be read through MCP".into());
        }
        let replay = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"inbound_work_advance","arguments":advance_args,"_meta":meta}})).await?;
        if replay["result"]["resultType"] == "input_required"
            || replay["result"]["structuredContent"]["decision"] != "accept"
        {
            return Err("Completed Task operation did not replay without a second approval".into());
        }
        let session_after_replay = exchange(&mut input, &mut output,
            json!({"jsonrpc":"2.0","id":212,"method":"tools/call","params":{"name":"inbound_work_session_read","arguments":{"sessionId":session_id},"_meta":meta}})).await?;
        if session_after_replay["result"]["structuredContent"]["headRevision"] != head_before_replay
        {
            return Err("Task operation replay appended a duplicate session event".into());
        }
        child.kill().await.map_err(|e| e.to_string())?;
        return Ok(
            json!({"captureless":true,"taskId":task_id,"problemLinksVerified":true,"relationshipsVerified":true,"cancelledWithoutSession":true,"changedDetail":detail,"expectedTaskRevision":task_revision+1}),
        );
    }
    let args = json!({"operationId":"packaged-open","lineageKey":"packaged-external","mode":"create","capture":{"title":"Packaged MCP Capture","summary":"Created through a real stdio child and the GUI owner"}});
    let preview=exchange(&mut input,&mut output,json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"_meta":meta}})).await?;
    if preview["result"]["resultType"] != "input_required" {
        return Err("MCP Capture bypassed review".into());
    }
    let result=exchange(&mut input,&mut output,json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"requestState":preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta}})).await?;
    let result = result["result"]["structuredContent"].clone();
    if result["sessionId"].as_str().is_none() {
        return Err("MCP Capture acceptance failed".into());
    }
    child.kill().await.map_err(|e| e.to_string())?;
    Ok(result)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopE2eResult {
    status: String,
    steps: Vec<String>,
    error: Option<String>,
    #[serde(default)]
    capture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coverage: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopE2eState {
    provider_url: String,
    scenario: String,
    restore_capture: Option<String>,
    restore_steps: Vec<String>,
}

#[tauri::command]
pub(crate) fn desktop_e2e_mode() -> Option<DesktopE2eState> {
    if let Some(path) = std::env::var_os("LLM_WIKI_E2E_RESULT").map(PathBuf::from) {
        let _ = std::fs::write(path.with_extension("started"), b"webview started");
        let provider_url = std::env::var("LLM_WIKI_E2E_PROVIDER_URL").ok()?;
        let scenario =
            std::env::var("LLM_WIKI_E2E_SCENARIO").unwrap_or_else(|_| "task-capture".into());
        let restore_capture = std::env::var("LLM_WIKI_E2E_RESTORE_CAPTURE").ok();
        let restore_steps = std::env::var("LLM_WIKI_E2E_RESTORE_STEPS")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_default();
        Some(DesktopE2eState {
            provider_url,
            scenario,
            restore_capture,
            restore_steps,
        })
    } else {
        None
    }
}

#[tauri::command]
pub(crate) fn desktop_e2e_complete(
    app: tauri::AppHandle,
    result: DesktopE2eResult,
) -> Result<(), String> {
    let path = std::env::var_os("LLM_WIKI_E2E_RESULT")
        .map(PathBuf::from)
        .ok_or("Desktop E2E mode is disabled")?;
    if result.status == "progress" {
        let payload =
            serde_json::to_vec_pretty(&result.steps).map_err(|error| error.to_string())?;
        return std::fs::write(path.with_extension("progress"), payload)
            .map_err(|error| error.to_string());
    }
    let payload = serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?;
    std::fs::write(path, payload).map_err(|error| error.to_string())?;
    app.exit(if matches!(result.status.as_str(), "passed" | "relaunch") {
        0
    } else {
        1
    });
    Ok(())
}
