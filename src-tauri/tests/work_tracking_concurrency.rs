use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::json;
use std::sync::{Arc, Barrier};
use tempfile::tempdir;

#[test]
fn concurrent_writers_share_one_head_and_the_loser_refreshes() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let input = json!({"operationId":"o","lineageKey":"race","mode":"create","capture":{"title":"CAS","summary":"Only one writer advances the head"}});
    let preview = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.open".into(),
        input: input.clone(),
    });
    let opened=app.execute_work_tracking(NativeOperation{name:"work_tracking.open.review".into(),input:json!({"proposal":input,"reviewState":preview.body["reviewState"],"decision":"accept"})});
    assert_eq!(opened.status, 200);
    let session = opened.body["sessionId"].as_str().unwrap().to_owned();
    let barrier = Arc::new(Barrier::new(3));
    let handles=(0..2).map(|index|{let app=app.clone();let barrier=barrier.clone();let session=session.clone();std::thread::spawn(move||{barrier.wait();app.execute_work_tracking(NativeOperation{name:"work_tracking.append".into(),input:json!({"operationId":format!("write-{index}"),"sessionId":session,"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":format!("writer {index}")}})})})}).collect::<Vec<_>>();
    barrier.wait();
    let responses = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.iter().filter(|r| r.status == 202).count(), 1);
    let loser = responses.iter().find(|r| r.status == 409).unwrap();
    assert_eq!(loser.body["error"]["code"], "head_conflict");
    assert_eq!(loser.body["error"]["currentRevision"], 2);
}

#[test]
fn overview_cursor_rejects_a_changed_snapshot() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    for index in 0..3 {
        let response = app.execute_domain(
            "workflow",
            NativeOperation {
                name: "capture.create".into(),
                input: json!({"operationId":format!("capture-{index}"),"text":format!("capture {index}")}),
            },
        );
        assert_eq!(response.status, 201, "{}", response.body);
    }
    let first = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":1}),
    });
    let cursor = first.body["nextCursor"].as_str().unwrap().to_owned();
    let snapshot = first.body["snapshotRevision"].as_i64().unwrap();
    let second = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":1,"cursor":cursor,"snapshotRevision":snapshot}),
    });
    assert_eq!(second.status, 200, "{}", second.body);
    app.execute_domain(
        "workflow",
        NativeOperation {
            name: "capture.create".into(),
            input: json!({"operationId":"changes-snapshot","text":"changes snapshot"}),
        },
    );
    let stale = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":1,"cursor":cursor,"snapshotRevision":snapshot}),
    });
    assert_eq!(stale.status, 409, "{}", stale.body);
    assert_eq!(stale.body["error"]["code"], "snapshot_stale");
}

#[test]
fn overview_orders_items_by_their_actual_latest_update() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let first = app.execute_domain("workflow", NativeOperation { name: "task.create".into(), input: json!({"operationId":"first-task","inputText":"first task","title":"first task"}) });
    let second = app.execute_domain("workflow", NativeOperation { name: "task.create".into(), input: json!({"operationId":"second-task","inputText":"second task","title":"second task"}) });
    std::thread::sleep(std::time::Duration::from_millis(2));
    let revised = app.execute_domain("workflow", NativeOperation { name: "task.revision".into(), input: json!({"operationId":"first-revise","taskId":first.body["id"],"expectedTaskRevision":1,"patch":{"title":"first task updated"}}) });
    assert_eq!(revised.status, 200, "{}", revised.body);

    let overview = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":10}),
    });

    assert_eq!(overview.status, 200, "{}", overview.body);
    assert_eq!(overview.body["items"][0]["entityRef"], first.body["id"]);
    assert_ne!(
        overview.body["items"][0]["updatedAt"],
        overview.body["items"][1]["updatedAt"]
    );
    assert_eq!(overview.body["items"][1]["entityRef"], second.body["id"]);
}

#[test]
fn overview_bounds_and_pages_attention_and_normalizes_context() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    for index in 0..55 {
        let response = app.execute_domain("workflow", NativeOperation { name: "task.create".into(), input: json!({"operationId":format!("task-{index}"),"inputText":format!("task {index}"),"title":format!("task {index}")}) });
        assert_eq!(response.status, 200, "{}", response.body);
    }

    let first = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":50}),
    });
    assert_eq!(first.status, 200, "{}", first.body);
    assert_eq!(first.body["attentionTotal"], 0);
    assert!(first.body["attention"].as_array().unwrap().is_empty());
    assert_eq!(first.body["attentionTruncated"], false);

    let second = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({
            "limit":50,
            "cursor":first.body["nextCursor"],
            "snapshotRevision":first.body["snapshotRevision"]
        }),
    });
    assert_eq!(second.status, 200, "{}", second.body);
    assert!(second.body["attention"].as_array().unwrap().is_empty());
    assert_eq!(second.body["attentionTruncated"], false);
    assert!(second.body["nextCursor"].is_null());
}

#[test]
fn overview_updates_a_task_timestamp_when_a_work_log_entry_changes() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let task = app.execute_domain(
        "workflow",
        NativeOperation {
            name: "task.create".into(),
            input: json!({"operationId":"task","inputText":"task","title":"task"}),
        },
    );
    let task_id = task.body["id"].as_str().unwrap().to_owned();
    let before = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":10}),
    });
    let before_updated = before.body["items"][0]["updatedAt"].clone();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let work = app.execute_domain("workflow", NativeOperation { name: "task.work-log.create".into(), input: json!({"operationId":"work","taskId":task_id,"expectedTaskRevision":1,"body":"progress"}) });
    assert_eq!(work.status, 200, "{}", work.body);

    let after = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":10}),
    });
    assert_eq!(after.status, 200, "{}", after.body);
    assert_eq!(after.body["items"][0]["entityRef"], task_id);
    assert_ne!(after.body["items"][0]["updatedAt"], before_updated);
}
