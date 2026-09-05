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
                input: json!({"text":format!("capture {index}")}),
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
            input: json!({"text":"changes snapshot"}),
        },
    );
    let stale = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.overview".into(),
        input: json!({"limit":1,"cursor":cursor,"snapshotRevision":snapshot}),
    });
    assert_eq!(stale.status, 409, "{}", stale.body);
    assert_eq!(stale.body["error"]["code"], "snapshot_stale");
}
