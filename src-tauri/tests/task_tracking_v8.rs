use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::json;

#[test]
fn reviewed_task_proposal_creates_the_same_task_seen_by_desktop_without_problem_approval() {
    let root = tempfile::tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("state.db"))
            .unwrap();
    let service = app.work_tracking_service();
    let input = json!({"operationId":"open-task","lineageKey":"task-flow","mode":"create","capture":{"title":"Task","summary":"Fix the installation command"}});
    let preview = service.open("native-in-app-chat", &input).unwrap();
    let opened = service
        .finish_open(
            "native-in-app-chat",
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let event = service.append("native-in-app-chat", &json!({"operationId":"propose-task","sessionId":opened["sessionId"],"expectedHeadRevision":opened["headRevision"],"event":{"kind":"task_created","title":"Fix the installation command","outcome":"The documented command works"}})).unwrap();
    let proposal = json!({"operationId":"adopt-task","sessionId":opened["sessionId"],"expectedHeadRevision":event["headRevision"],"sourceEventId":event["eventId"],"action":"create_task","proposedPayload":{"title":"Fix the installation command","outcome":"The documented command works"}});
    let state = service
        .begin_advance("native-in-app-chat", &proposal)
        .unwrap();
    let result = service
        .finish_advance("native-in-app-chat", &state, &proposal, "accept")
        .unwrap();
    let task = app.execute(NativeOperation {
        name: "task.get".into(),
        input: json!({"taskId":result["resultEntityId"]}),
    });
    assert_eq!(task.status, 200, "{}", task.body);
    assert_eq!(task.body["title"], "Fix the installation command");
    assert_eq!(task.body["state"], "task");
    let session = service
        .session("native-in-app-chat", opened["sessionId"].as_str().unwrap())
        .unwrap();
    assert_eq!(session["linkedWorkflow"]["task"]["id"], task.body["id"]);
}

#[test]
fn reviewed_task_proposal_keeps_the_exact_migrated_problem_revision() {
    let root = tempfile::tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("state.db"))
            .unwrap();
    let problem = app.execute(NativeOperation {
        name: "problem.create".into(),
        input: json!({"operationId":"migrated-problem","statement":"Preserved legacy Problem"}),
    });
    assert_eq!(problem.status, 200, "{}", problem.body);
    let problem_id = problem.body["id"].as_str().unwrap();
    let revised = app.execute(NativeOperation {
        name: "problem.revision".into(),
        input: json!({"operationId":"migrated-problem-r2","problemId":problem_id,"statement":"Preserved legacy Problem revision two"}),
    });
    assert_eq!(revised.status, 200, "{}", revised.body);
    assert_eq!(revised.body["problemRevision"], 2);
    let service = app.work_tracking_service();
    let input = json!({"operationId":"migrated-open","lineageKey":"migrated-problem","mode":"create","capture":{"title":"Legacy refinement","summary":"Preserved legacy Problem"}});
    let preview = service.open("native-in-app-chat", &input).unwrap();
    let opened = service
        .finish_open(
            "native-in-app-chat",
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let event = service.append("native-in-app-chat", &json!({"operationId":"migrated-event","sessionId":opened["sessionId"],"expectedHeadRevision":opened["headRevision"],"event":{"kind":"task_created","title":"Migrated Task","outcome":"Current Task service owns it"}})).unwrap();
    let proposal = json!({"operationId":"migrated-task","sessionId":opened["sessionId"],"expectedHeadRevision":event["headRevision"],"sourceEventId":event["eventId"],"action":"create_task","proposedPayload":{"title":"Migrated Task","outcome":"Current Task service owns it","problemId":problem_id,"problemRevision":2}});
    let review = service
        .begin_advance("native-in-app-chat", &proposal)
        .unwrap();
    let accepted = service
        .finish_advance("native-in-app-chat", &review, &proposal, "accept")
        .unwrap();
    let task = app.execute(NativeOperation {
        name: "task.get".into(),
        input: json!({"taskId":accepted["resultEntityId"]}),
    });
    assert_eq!(task.status, 200, "{}", task.body);
    assert!(task.body["problemLinks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link["problemId"] == problem_id && link["problemRevision"] == 2));
}

#[test]
fn migrated_problem_task_proposal_rejects_a_revision_that_does_not_exist() {
    let root = tempfile::tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("state.db"))
            .unwrap();
    let problem = app.execute(NativeOperation {
        name: "problem.create".into(),
        input: json!({"operationId":"missing-revision-problem","statement":"A migrated Problem"}),
    });
    assert_eq!(problem.status, 200, "{}", problem.body);
    let service = app.work_tracking_service();
    let input = json!({"operationId":"missing-revision-open","lineageKey":"missing-revision","mode":"create","capture":{"title":"Legacy refinement","summary":"A migrated Problem"}});
    let preview = service.open("native-in-app-chat", &input).unwrap();
    let opened = service
        .finish_open(
            "native-in-app-chat",
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let event = service.append("native-in-app-chat", &json!({"operationId":"missing-revision-event","sessionId":opened["sessionId"],"expectedHeadRevision":opened["headRevision"],"event":{"kind":"task_created","title":"Invalid migrated Task","outcome":"Must not be created"}})).unwrap();
    let proposal = json!({"operationId":"missing-revision-task","sessionId":opened["sessionId"],"expectedHeadRevision":event["headRevision"],"sourceEventId":event["eventId"],"action":"create_task","proposedPayload":{"title":"Invalid migrated Task","outcome":"Must not be created","problemId":problem.body["id"],"problemRevision":99}});
    let error = service
        .begin_advance("native-in-app-chat", &proposal)
        .unwrap_err();
    assert_eq!(error.code, "not_found_or_not_visible");
    assert!(
        error.message.to_lowercase().contains("problem target"),
        "{:?}",
        error
    );
    let tasks: i64 = rusqlite::Connection::open(root.path().join("state.db"))
        .unwrap()
        .query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))
        .unwrap();
    assert_eq!(tasks, 0);
}

#[test]
fn direct_task_continuation_is_captureless_reviewed_and_stale_aware() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.db");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let task = app.execute(NativeOperation {
        name: "task.create".into(),
        input: json!({"operationId":"direct-task","inputText":"Desktop Task","title":"Desktop Task","outcome":"Visible to MCP"}),
    });
    assert_eq!(task.status, 200, "{}", task.body);
    let task_id = task.body["id"].as_str().unwrap();
    let service = app.work_tracking_service();

    let cancel = json!({"operationId":"continue-cancel","lineageKey":"desktop-task-cancel","mode":"continue_task","taskId":task_id});
    let preview = service.open("native-in-app-chat", &cancel).unwrap();
    assert_eq!(preview["stage"], "task_continuation");
    assert_eq!(preview["preview"]["task"]["taskId"], task_id);
    let cancelled = service
        .finish_open(
            "native-in-app-chat",
            &cancel,
            preview["reviewState"].as_str().unwrap(),
            "cancel",
        )
        .unwrap();
    assert_eq!(cancelled["created"], false);
    let sessions: i64 = rusqlite::Connection::open(&db)
        .unwrap()
        .query_row("SELECT count(*) FROM work_tracking_sessions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(sessions, 0, "cancellation must not create a session");

    let stale = json!({"operationId":"continue-stale","lineageKey":"desktop-task-stale","mode":"continue_task","taskId":task_id});
    let review = service.open("native-in-app-chat", &stale).unwrap();
    let revised = app.execute(NativeOperation { name:"task.revision".into(), input:json!({"operationId":"direct-task-r2","taskId":task_id,"expectedTaskRevision":1,"patch":{"detail":"child snapshot changed"}}) });
    assert_eq!(revised.status, 200, "{}", revised.body);
    assert_eq!(
        service
            .finish_open(
                "native-in-app-chat",
                &stale,
                review["reviewState"].as_str().unwrap(),
                "accept"
            )
            .unwrap_err()
            .code,
        "head_conflict"
    );

    let accept = json!({"operationId":"continue-accept","lineageKey":"desktop-task-accept","mode":"continue_task","taskId":task_id});
    let review = service.open("native-in-app-chat", &accept).unwrap();
    let opened = service
        .finish_open(
            "native-in-app-chat",
            &accept,
            review["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    assert!(opened["captureId"].is_null());
    let session = service
        .session("native-in-app-chat", opened["sessionId"].as_str().unwrap())
        .unwrap();
    assert!(session["capture"].is_null());
    assert_eq!(session["linkedWorkflow"]["task"]["id"], task_id);
    let connection = rusqlite::Connection::open(db).unwrap();
    let capture: Option<String> = connection
        .query_row(
            "SELECT capture_id FROM work_tracking_sessions WHERE id=?",
            [opened["sessionId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(capture.is_none());
    let event: String = connection
        .query_row(
            "SELECT kind FROM work_tracking_events WHERE session_id=?",
            [opened["sessionId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(event, "task_binding");
}
