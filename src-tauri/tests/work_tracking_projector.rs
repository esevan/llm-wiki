use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn call(app: &NativeApplication, name: &str, input: Value) -> Value {
    // Workflow fixtures explicitly accept the exact server preview. Consent
    // boundary tests invoke the native command directly without this helper.
    if name == "work_tracking.open" {
        let preview = app.execute_work_tracking(NativeOperation {
            name: name.into(),
            input: input.clone(),
        });
        assert!(preview.status < 300, "{}", preview.body);
        if preview.body["decisionRequired"] == true {
            return call(
                app,
                "work_tracking.open.review",
                json!({"proposal":input,"reviewState":preview.body["reviewState"],"decision":"accept"}),
            );
        }
        return preview.body;
    }
    let response = app.execute_work_tracking(NativeOperation {
        name: name.into(),
        input,
    });
    assert!(response.status < 300, "{}", response.body);
    response.body
}

#[test]
fn checkpoints_are_durable_before_projection_and_projection_is_idempotent() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"o","lineageKey":"project","mode":"create","capture":{"title":"Projection","summary":"Test the outbox"}}),
    );
    let appended = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"a","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":"Durably saved"}}),
    );
    assert_eq!(appended["persistenceStatus"], "durable");
    assert_eq!(appended["projectionStatus"], "queued");
    call(&app, "work_tracking.project", json!({"limit":100}));
    call(&app, "work_tracking.project", json!({"limit":100}));
    let connection = rusqlite::Connection::open(db).unwrap();
    let results: i64 = connection
        .query_row(
            "SELECT count(*) FROM work_tracking_projection_results WHERE event_id=?",
            [appended["eventId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(results, 1);
}

#[test]
fn a_late_event_stays_auditable_without_rewinding_the_stream_watermark() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"late-o","lineageKey":"late","mode":"create","capture":{"title":"Late events","summary":"Do not rewind state"}}),
    );
    let first = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"late-1","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":"older"}}),
    );
    let second = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"late-2","sessionId":opened["sessionId"],"expectedHeadRevision":2,"event":{"kind":"work_log_checkpoint","summary":"newer"}}),
    );
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE work_tracking_projection_jobs SET state='pending_review' WHERE event_id=?",
            [first["eventId"].as_str().unwrap()],
        )
        .unwrap();
    connection.execute("UPDATE work_tracking_events SET occurred_at='2026-01-01T00:00:00.000000000Z' WHERE id=?",[first["eventId"].as_str().unwrap()]).unwrap();
    connection.execute("UPDATE work_tracking_events SET occurred_at='2026-01-02T00:00:00.000000000Z' WHERE id=?",[second["eventId"].as_str().unwrap()]).unwrap();
    drop(connection);
    call(&app, "work_tracking.project", json!({"limit":100}));
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE work_tracking_projection_jobs SET state='pending' WHERE event_id=?",
            [first["eventId"].as_str().unwrap()],
        )
        .unwrap();
    drop(connection);
    call(&app, "work_tracking.project", json!({"limit":100}));
    let connection = rusqlite::Connection::open(db).unwrap();
    let state: String = connection
        .query_row(
            "SELECT state FROM work_tracking_projection_results WHERE event_id=?",
            [first["eventId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "ignored_late");
    let events: i64 = connection
        .query_row(
            "SELECT count(*) FROM work_tracking_events WHERE id=?",
            [first["eventId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(events, 1);
}

#[test]
fn durable_append_latency_stays_below_the_release_budget() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"perf-o","lineageKey":"perf","mode":"create","capture":{"title":"Performance","summary":"Measure durable append"}}),
    );
    let mut samples = Vec::new();
    for index in 0..100 {
        let revision = index + 1;
        let started = std::time::Instant::now();
        call(
            &app,
            "work_tracking.append",
            json!({"operationId":format!("perf-{index}"),"sessionId":opened["sessionId"],"expectedHeadRevision":revision,"event":{"kind":"work_log_checkpoint","summary":format!("checkpoint {index}")}}),
        );
        samples.push(started.elapsed());
    }
    samples.sort();
    assert!(
        samples[94] < std::time::Duration::from_millis(50),
        "p95 append latency was {:?}",
        samples[94]
    );
    let started = std::time::Instant::now();
    call(&app, "work_tracking.project", json!({"limit":200}));
    assert!(
        started.elapsed() < std::time::Duration::from_millis(500),
        "projection visibility exceeded 500 ms"
    );
}

#[test]
fn ten_thousand_out_of_order_events_converge_on_the_same_watermark() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"bulk-open","lineageKey":"bulk","mode":"create","capture":{"title":"Bulk replay","summary":"Deterministic projection"}}),
    );
    let session = opened["sessionId"].as_str().unwrap();
    let mut connection = rusqlite::Connection::open(&db).unwrap();
    let transaction = connection.transaction().unwrap();
    for index in 0..10_000u64 {
        let order = (index.wrapping_mul(7_919)) % 10_000;
        let event_id = format!("bulk-{index:05}");
        let occurred_at = format!(
            "2026-01-01T00:{:02}:{:02}.{:03}Z",
            order / 60_000,
            (order / 1_000) % 60,
            order % 1_000
        );
        transaction.execute("INSERT INTO work_tracking_events(id,session_id,revision,stream_id,source_sequence,kind,payload_json,payload_hash,occurred_at,ingested_at) VALUES (?,?,?,?,?,'capture','{\"kind\":\"capture\",\"summary\":\"bulk\"}','bulk-hash',?,?)",rusqlite::params![event_id,session,index as i64+2,"bulk-stream",index as i64+2,occurred_at,occurred_at]).unwrap();
        transaction.execute("INSERT INTO work_tracking_projection_jobs(event_id,projection_name,state,created_at,updated_at) VALUES (?,'workflow','pending','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",[event_id]).unwrap();
    }
    transaction.commit().unwrap();
    let started = std::time::Instant::now();
    let projected = call(&app, "work_tracking.project", json!({"limit":10_000}));
    assert_eq!(projected["processed"], 10_000);
    let connection = rusqlite::Connection::open(&db).unwrap();
    let results: i64 = connection
        .query_row(
            "SELECT count(*) FROM work_tracking_projection_results WHERE event_id LIKE 'bulk-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(results, 10_000);
    let watermark:String=connection.query_row("SELECT last_occurred_at FROM work_tracking_stream_watermarks WHERE session_id=? AND stream_id='bulk-stream'",[session],|row|row.get(0)).unwrap();
    assert_eq!(watermark, "2026-01-01T00:00:09.999Z");
    eprintln!("10k projection lag metric: {:?}", started.elapsed());
}

#[test]
fn expired_claim_is_recovered_and_two_workers_materialize_once() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"lease-open","lineageKey":"lease","mode":"create","capture":{"title":"Lease","summary":"Recover work"}}),
    );
    let event_id = opened["headEventId"].as_str().unwrap();
    rusqlite::Connection::open(&db).unwrap().execute("UPDATE work_tracking_projection_jobs SET state='claimed',lease_owner='crashed',lease_expires_at='2020-01-01T00:00:00Z',attempts=1 WHERE event_id=?",[event_id]).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let workers = (0..2)
        .map(|_| {
            let app = app.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                app.execute_work_tracking(NativeOperation {
                    name: "work_tracking.project".into(),
                    input: json!({"limit":10}),
                })
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    for worker in workers {
        assert!(worker.join().unwrap().status < 300);
    }
    let connection = rusqlite::Connection::open(&db).unwrap();
    let results: i64 = connection
        .query_row(
            "SELECT count(*) FROM work_tracking_projection_results WHERE event_id=?",
            [event_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(results, 1);
    let state: String = connection
        .query_row(
            "SELECT state FROM work_tracking_projection_jobs WHERE event_id=?",
            [event_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "applied");
}
