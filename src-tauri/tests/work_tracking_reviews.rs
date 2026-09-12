use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn open_input(id: &str) -> Value {
    json!({"operationId":id,"lineageKey":id,"mode":"create","capture":{"title":"Exact preview","summary":"Keep private until accepted"}})
}

#[test]
fn capture_review_binds_content_owner_and_revocation_without_side_effects() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let service = app.work_tracking_service();
    let scopes = vec!["session:read".into(), "session:write".into()];
    let a = service
        .create_connection("A", &scopes, &[], "confirm_each")
        .unwrap();
    let b = service
        .create_connection("B", &scopes, &[], "confirm_each")
        .unwrap();
    let a = a["id"].as_str().unwrap();
    let b = b["id"].as_str().unwrap();
    let input = open_input("bound");
    let preview = service.open(a, &input).unwrap();
    let state = preview["reviewState"].as_str().unwrap();
    assert!(service.finish_open(b, &input, state, "accept").is_err());
    let mut edited = input.clone();
    edited["capture"]["summary"] = json!("Unreviewed edit");
    assert!(service.finish_open(a, &edited, state, "accept").is_err());
    service.revoke_connection(a).unwrap();
    assert!(service.finish_open(a, &input, state, "accept").is_err());
    let db = rusqlite::Connection::open(db).unwrap();
    for table in ["captures", "work_tracking_sessions"] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn capture_reject_edit_accept_and_replay_are_exact() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let service = app.work_tracking_service();
    let owner = "native-in-app-chat";
    let input = open_input("reject");
    let preview = service.open(owner, &input).unwrap();
    let state = preview["reviewState"].as_str().unwrap();
    service.finish_open(owner, &input, state, "reject").unwrap();
    assert!(service.finish_open(owner, &input, state, "accept").is_err());
    let mut edited = open_input("edited");
    edited["capture"]["summary"] = json!("User edited and reviewed");
    let preview = service.open(owner, &edited).unwrap();
    let state = preview["reviewState"].as_str().unwrap();
    let first = service
        .finish_open(owner, &edited, state, "accept")
        .unwrap();
    let replay = service
        .finish_open(owner, &edited, state, "accept")
        .unwrap();
    assert_eq!(first["sessionId"], replay["sessionId"]);
    assert_eq!(
        rusqlite::Connection::open(db)
            .unwrap()
            .query_row("SELECT count(*) FROM captures", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn expired_capture_preview_cannot_create_a_session() {
    let root = tempdir().unwrap();
    let db = root.path().join("state.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let input = open_input("expire");
    let preview = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.open".into(),
        input: input.clone(),
    });
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE work_tracking_reviews SET expires_at='2000-01-01T00:00:00Z'",
            [],
        )
        .unwrap();
    let result=app.execute_work_tracking(NativeOperation{name:"work_tracking.open.review".into(),input:json!({"proposal":input,"reviewState":preview.body["reviewState"],"decision":"accept"})});
    assert!(result.status >= 400);
    assert_eq!(result.body["error"]["code"], "challenge_expired");
}

#[test]
fn capture_review_cannot_move_between_native_windows() {
    let root = tempdir().unwrap();
    let app = NativeApplication::isolated(
        &root.path().join("vault"),
        &root.path().join("state.sqlite"),
    )
    .unwrap();
    let service = app.work_tracking_service();
    let owner = "native-in-app-chat";
    let mut input = open_input("window-bound");
    input["reviewContext"] = json!({"window":"main"});
    let preview = service.open(owner, &input).unwrap();
    let state = preview["reviewState"].as_str().unwrap();
    let mut other = input.clone();
    other["reviewContext"] = json!({"window":"other"});
    assert!(service.finish_open(owner, &other, state, "accept").is_err());
    assert!(service.finish_open(owner, &input, state, "accept").is_ok());
}

#[test]
fn stale_governed_review_can_be_cancelled_but_not_accepted_or_replayed() {
    let root = tempdir().unwrap();
    let app = NativeApplication::isolated(
        &root.path().join("vault"),
        &root.path().join("state.sqlite"),
    )
    .unwrap();
    let service = app.work_tracking_service();
    let owner = "native-in-app-chat";
    let input = open_input("cancel");
    let preview = service.open(owner, &input).unwrap();
    let opened = service
        .finish_open(
            owner,
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let event=service.append(owner,&json!({"operationId":"p","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"task_created","title":"Pending Task","outcome":"Do not create on cancel"}})).unwrap();
    let proposal = json!({"operationId":"pending-task-create","sessionId":opened["sessionId"],"expectedHeadRevision":event["headRevision"],"sourceEventId":event["eventId"],"action":"create_task","proposedPayload":{"title":"Pending Task","outcome":"Do not create on cancel"}});
    let review = service.begin_advance(owner, &proposal).unwrap();
    service.append(owner,&json!({"operationId":"newer","sessionId":opened["sessionId"],"expectedHeadRevision":event["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"A newer change"}})).unwrap();
    let state = review.as_str();
    assert!(service
        .finish_advance(owner, state, &proposal, "accept")
        .is_err());
    assert_eq!(
        service
            .finish_advance(owner, state, &proposal, "cancel")
            .unwrap()["decision"],
        "cancel"
    );
    // Once the cancellation has consumed this review, any decision replay returns
    // the stored terminal cancellation. It must never revive the stale proposal.
    let replay = service
        .finish_advance(owner, state, &proposal, "accept")
        .unwrap();
    assert_eq!(replay["decision"], "cancel");
    let session = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.session".into(),
        input: json!({"sessionId":opened["sessionId"]}),
    });
    assert!(session.body["linkedWorkflow"]["task"].is_null());
    let connection = rusqlite::Connection::open(root.path().join("state.sqlite")).unwrap();
    let tasks: i64 = connection
        .query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))
        .unwrap();
    let accepted_decisions: i64 = connection
        .query_row(
            "SELECT count(*) FROM work_tracking_decisions WHERE event_id=? AND decision LIKE 'accepted%'",
            [event["eventId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tasks, 0);
    assert_eq!(accepted_decisions, 0);
}

#[test]
fn identical_operation_ids_are_isolated_between_connections_and_oversize_is_rejected() {
    let root = tempdir().unwrap();
    let app = NativeApplication::isolated(
        &root.path().join("vault"),
        &root.path().join("state.sqlite"),
    )
    .unwrap();
    let service = app.work_tracking_service();
    let scopes = vec!["session:read".into(), "session:write".into()];
    let a = service
        .create_connection("A", &scopes, &[], "confirm_each")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let b = service
        .create_connection("B", &scopes, &[], "confirm_each")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let input = open_input("same-operation");
    let pa = service.open(&a, &input).unwrap();
    let pb = service.open(&b, &input).unwrap();
    let sa = service
        .finish_open(&a, &input, pa["reviewState"].as_str().unwrap(), "accept")
        .unwrap();
    let sb = service
        .finish_open(&b, &input, pb["reviewState"].as_str().unwrap(), "accept")
        .unwrap();
    assert_ne!(sa["sessionId"], sb["sessionId"]);
    let mut oversized = open_input("oversized");
    oversized["capture"]["summary"] = json!("x".repeat(20_001));
    assert_eq!(
        service.open(&a, &oversized).unwrap_err().code,
        "content_too_large"
    );
}
