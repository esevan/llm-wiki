use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};

fn execute(app: &NativeApplication, name: &str, input: Value) -> Value {
    let response = app.execute(NativeOperation {
        name: name.into(),
        input,
    });
    assert_eq!(response.status, 200, "{}", response.body);
    response.body
}

#[test]
fn scoped_context_returns_canonical_children_without_binary_contents() {
    let root = tempfile::tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("state.db"))
            .unwrap();
    let service = app.work_tracking_service();
    let connection = service
        .create_connection(
            "context",
            &["session:read".into(), "workbench:overview:read".into()],
            &[],
            "confirm_each",
        )
        .unwrap();
    let owner = connection["id"].as_str().unwrap();
    let task = execute(
        &app,
        "task.create",
        json!({"operationId":"task","inputText":"Maintenance","title":"Context Task","detail":"Actual detail","outcome":"Useful outcome"}),
    );
    let id = task["id"].as_str().unwrap();
    let work = execute(
        &app,
        "task.work-log.create",
        json!({"operationId":"work","taskId":id,"expectedTaskRevision":1,"body":"a".repeat(5000),"imageData":"SECRET_IMAGE","attachment":{"name":"evidence.txt","mediaType":"text/plain","data":"SECRET_ATTACHMENT"}}),
    );
    execute(
        &app,
        "work-log.comment.create",
        json!({"operationId":"comment","entryId":work["id"],"body":"Verified comment"}),
    );
    execute(
        &app,
        "task.readiness.decision",
        json!({"operationId":"ready","taskId":id,"expectedTaskRevision":1,"key":"scope","status":"not_applicable","reason":"Bounded maintenance"}),
    );
    let context = service.task_context_read(owner, id).unwrap();
    assert_eq!(context["task"]["detail"], "Actual detail");
    assert_eq!(context["comments"][0]["body"], "Verified comment");
    assert_eq!(context["attachments"][0]["name"], "evidence.txt");
    assert_eq!(context["workLog"][0]["body"].as_str().unwrap().len(), 4000);
    assert_eq!(context["truncated"], true);
    assert!(!context.to_string().contains("SECRET_"));
    assert!(context["readiness"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["key"] == "scope" && entry["status"] == "not_applicable"));
    let hidden = service
        .create_connection("hidden", &["session:read".into()], &[], "confirm_each")
        .unwrap();
    assert!(service
        .task_context_read(hidden["id"].as_str().unwrap(), id)
        .is_err());
}

#[test]
fn child_only_external_comment_change_invalidates_exact_continuation_review() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.db");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let service = app.work_tracking_service();
    let task = execute(
        &app,
        "task.create",
        json!({"operationId":"task","inputText":"Maintenance","title":"Freshness"}),
    );
    let work = execute(
        &app,
        "task.work-log.create",
        json!({"operationId":"work","taskId":task["id"],"expectedTaskRevision":1,"body":"Evidence"}),
    );
    let comment = execute(
        &app,
        "work-log.comment.create",
        json!({"operationId":"comment","entryId":work["id"],"body":"Before"}),
    );
    let input = json!({"operationId":"continue","lineageKey":"comment-freshness","mode":"continue_task","taskId":task["id"]});
    let preview = service.open("native-in-app-chat", &input).unwrap();
    // Bypass activity/revision counters to prove the actual child content is hashed.
    let connection = rusqlite::Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE task_work_log_comments SET body='After' WHERE id=?",
            [comment["id"].as_str().unwrap()],
        )
        .unwrap();
    assert!(service
        .finish_open(
            "native-in-app-chat",
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept"
        )
        .is_err());
    let count: i64 = connection
        .query_row("SELECT count(*) FROM work_tracking_sessions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn failed_canonical_draft_mutation_rolls_back_review_acceptance() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.db");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let task = execute(
        &app,
        "task.create",
        json!({"operationId":"already-used","inputText":"Maintenance","title":"Completed work"}),
    );
    execute(
        &app,
        "task.transition",
        json!({"operationId":"start","taskId":task["id"],"expectedTaskRevision":1,"to":"in_progress"}),
    );
    execute(
        &app,
        "task.completion.create",
        json!({"operationId":"complete","taskId":task["id"],"expectedTaskRevision":1,"evidence":"Verified","report":"Done"}),
    );
    let service = app.work_tracking_service();
    let open = json!({"operationId":"continue","lineageKey":"draft-rollback","mode":"continue_task","taskId":task["id"]});
    let preview = service.open("native-in-app-chat", &open).unwrap();
    let session = service
        .finish_open(
            "native-in-app-chat",
            &open,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    // The reused canonical operation ID is deliberately incompatible. Failure must leave
    // both the draft and consent unchanged, rather than accepting before applying.
    rusqlite::Connection::open(&db).unwrap().execute("INSERT INTO task_assistance_operations(operation_id,payload_hash,result_json) VALUES('already-used','different-payload','{}')", []).unwrap();
    let proposal = json!({"operationId":"already-used","sessionId":session["sessionId"],"bodyMarkdown":"# Private draft","title":"Private","summary":"Verified"});
    let review = service
        .save_knowledge_draft("native-in-app-chat", &proposal)
        .unwrap();
    let state = review["reviewState"].as_str().unwrap();
    assert!(service
        .finish_knowledge_draft("native-in-app-chat", &proposal, state, "accept")
        .is_err());
    let connection = rusqlite::Connection::open(&db).unwrap();
    let review_state: String = connection
        .query_row(
            "SELECT state FROM work_tracking_reviews WHERE id=?",
            [state],
            |row| row.get(0),
        )
        .unwrap();
    let drafts: i64 = connection
        .query_row("SELECT count(*) FROM task_knowledge_drafts", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(review_state, "pending");
    assert_eq!(drafts, 0);
}
