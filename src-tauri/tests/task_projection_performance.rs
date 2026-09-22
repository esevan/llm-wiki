//! Keep the wall-clock budget in its own Cargo test target: unrelated parallel
//! unit tests create/migrate databases and otherwise measure runner contention.
//! Cargo still runs this target as part of the ordinary full `cargo test` command.
use llm_wiki_desktop::{NativeApplication, NativeOperation};
use rusqlite::params;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

fn call(app: &NativeApplication, name: &str, input: Value) -> Value {
    let response = app.execute(NativeOperation {
        name: name.into(),
        input,
    });
    assert_eq!(response.status, 200, "{}", response.body);
    response.body
}

#[test]
fn normal_task_projection_with_execution_work_log_stays_under_100_ms_p95() {
    let temporary = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.tmp");
    std::fs::create_dir_all(&temporary).unwrap();
    let root = tempfile::tempdir_in(temporary).unwrap();
    let db = root.path().join("state.sqlite3");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let task = call(&app, "task.create", json!({
        "operationId": "task", "inputText": "Project execution history", "title": "Execution history"
    }))["id"].as_str().unwrap().to_owned();
    let session = call(
        &app,
        "task.work-session.create",
        json!({
            "operationId": "session", "taskId": task, "title": "Codex work"
        }),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Seed persisted provider results without starting a real provider. Ten runs
    // with 25 items each exercise the indexed lookup and 20-item evidence bound;
    // one manual entry verifies the execution join preserves ordinary Work Log.
    let mut connection = rusqlite::Connection::open(&db).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let tx = connection.transaction().unwrap();
    tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('manual',?,'Manual note',CURRENT_TIMESTAMP)", [&task]).unwrap();
    for run in 0..10 {
        let run_id = format!("run-{run}");
        let entry_id = format!("entry-{run}");
        let log_id = format!("log-{run}");
        tx.execute("INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES(?,?,'user','note','Review this change',CURRENT_TIMESTAMP)", params![entry_id, session]).unwrap();
        tx.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES(?,?,'Execution summary',CURRENT_TIMESTAMP)", params![log_id, task]).unwrap();
        tx.execute("INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,dispatch_state,final_report,work_log_sync_state,created_at,updated_at) VALUES(?,?,?,?,'hash','Review this change',?,?,'gpt-5.6-sol',?,'context','succeeded','accepted','Review completed','synced',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", params![run_id, task, session, run_id, entry_id, log_id, root.path().to_string_lossy()]).unwrap();
        tx.execute("INSERT INTO task_work_session_run_logs(run_id,work_log_entry_id,sync_state,created_at,updated_at) VALUES(?,?,'synced',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", params![run_id, log_id]).unwrap();
        for item in 0..25 {
            let content = json!({"type":"commandExecution", "command":"cargo test", "aggregatedOutput":"Focused checks passed", "exitCode":0});
            tx.execute("INSERT INTO task_work_session_run_items(run_id,provider_item_id,provider_order,kind,status,content_json,created_at,completed_at) VALUES(?,?,?,'commandExecution','completed',?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)", params![run_id, format!("item-{item}"), item, content.to_string()]).unwrap();
        }
    }
    tx.commit().unwrap();
    drop(connection);

    let mut samples = Vec::new();
    for _ in 0..25 {
        let started = Instant::now();
        let detail = call(&app, "task.get", json!({"taskId":task}));
        samples.push(started.elapsed());
        let entries = detail["workLog"].as_array().unwrap();
        assert_eq!(entries.len(), 11);
        let executions = entries
            .iter()
            .filter_map(|entry| entry.get("execution"))
            .collect::<Vec<_>>();
        assert_eq!(executions.len(), 10);
        for execution in executions {
            assert_eq!(execution["status"], "succeeded");
            assert_eq!(execution["reportExcerpt"], "Review completed");
            assert_eq!(execution["evidence"].as_array().unwrap().len(), 20);
        }
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry["id"] == "manual")
                .unwrap()["body"],
            "Manual note"
        );
    }
    samples.sort();
    let p95 = samples[23]; // Nearest rank: ceil(25 * 0.95) - 1.
    eprintln!("Task projection p95={p95:?} (10 execution logs, 250 provider items)");
    assert!(
        p95 < Duration::from_millis(100),
        "task projection p95 {p95:?} exceeded 100 ms"
    );
}
