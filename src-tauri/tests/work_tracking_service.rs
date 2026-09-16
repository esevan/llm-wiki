use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn call(app: &NativeApplication, name: &str, mut input: Value) -> Value {
    if name == "work_tracking.advance" {
        if input.get("operationId").and_then(Value::as_str).is_none() {
            input["operationId"] = json!(format!(
                "fixture-{}-{}",
                input["action"].as_str().unwrap_or("advance"),
                input["sourceEventId"].as_str().unwrap_or("none")
            ));
        }
        let mut proposal = input.clone();
        let decision = proposal
            .as_object_mut()
            .unwrap()
            .remove("decision")
            .unwrap();
        let preview = call(app, "work_tracking.advance.preview", proposal.clone());
        return call(
            app,
            "work_tracking.advance.review",
            json!({"proposal":proposal,"reviewState":preview["reviewState"],"decision":decision}),
        );
    }
    if name == "work_tracking.append"
        && input["event"]["kind"] == "completion_proposal"
        && input["event"]["selectedEvidence"].is_null()
    {
        // Build accepted, projected test evidence before the completion proposal.
        let evidence = call(
            app,
            "work_tracking.append",
            json!({"operationId":format!("evidence-{}",input["operationId"]),"sessionId":input["sessionId"],"expectedHeadRevision":input["expectedHeadRevision"],"event":{"kind":"work_log_checkpoint","summary":"Executed release verification","validation":["passed"]}}),
        );
        call(app, "work_tracking.project", json!({"limit":100}));
        let latest = call(
            app,
            "work_tracking.session",
            json!({"sessionId":input["sessionId"]}),
        );
        input["expectedHeadRevision"] = latest["headRevision"].clone();
        input["event"]["selectedEvidence"] = json!([evidence["eventId"]]);
    }
    if matches!(
        name,
        "work_tracking.knowledge.draft.save" | "work_tracking.knowledge.publish"
    ) {
        let preview_name = if name.ends_with("publish") {
            "work_tracking.knowledge.publish.preview"
        } else {
            name
        };
        let preview = app.execute_work_tracking(NativeOperation {
            name: preview_name.into(),
            input: input.clone(),
        });
        assert!(preview.status < 300, "{}", preview.body);
        if preview.body["decisionRequired"] != true {
            return preview.body;
        }
        let finish = if name.ends_with("publish") {
            "work_tracking.knowledge.publish.review"
        } else {
            "work_tracking.knowledge.draft.review"
        };
        return call(
            app,
            finish,
            json!({"proposal":input,"reviewState":preview.body["reviewState"],"decision":"accept"}),
        );
    }
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
    assert!(
        response.status < 300,
        "{}: {}",
        response.status,
        response.body
    );
    response.body
}

#[test]
fn open_is_exactly_idempotent_and_head_compare_and_swap_is_enforced() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let input = json!({"operationId":"open-1","lineageKey":"chat-1","mode":"create","capture":{"title":"Track MCP","summary":"Implement one durable workflow"}});
    let first = call(&app, "work_tracking.open", input.clone());
    let replay = call(&app, "work_tracking.open", input);
    assert_eq!(first["sessionId"], replay["sessionId"]);
    assert_eq!(replay["deduplicated"], true);
    let response=app.execute_work_tracking(NativeOperation{name:"work_tracking.append".into(),input:json!({"operationId":"append-1","sessionId":first["sessionId"],"expectedHeadRevision":0,"event":{"kind":"task_created","title":"Wrong head","outcome":"Wrong head"}})});
    assert_eq!(response.status, 409);
    assert_eq!(response.body["error"]["code"], "head_conflict");
    assert_eq!(response.body["error"]["currentRevision"], 1);
    let connection = rusqlite::Connection::open(db).unwrap();
    let activity=connection.prepare("SELECT source_interface,operation,outcome,COALESCE(safe_error_code,'') FROM work_tracking_activity_events").and_then(|mut statement|statement.query_map([],|row|Ok(format!("{}|{}|{}|{}",row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?)))?.collect::<Result<Vec<_>,_>>()).unwrap();
    assert!(
        !activity.join("\n").contains("durable workflow"),
        "activity logs must not contain work content"
    );
}

#[test]
fn completion_is_separate_from_knowledge_publication() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"open","lineageKey":"complete-chat","mode":"create","capture":{"title":"Separate completion","summary":"Completion must remain private"}}),
    );
    let session = opened["sessionId"].clone();
    let task = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"task","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"task_created","title":"Two phases","outcome":"Private completion first"}}),
    );
    let created = call(
        &app,
        "work_tracking.advance",
        json!({"operationId":"completion-create","sessionId":session,"expectedHeadRevision":task["headRevision"],"sourceEventId":task["eventId"],"action":"create_task","decision":"accept","proposedPayload":{"title":"Two phases","outcome":"Private completion first","scope":"Local release","validationCriteria":"Tests pass"}}),
    );
    let task_id = created["resultEntityId"].as_str().unwrap().to_owned();
    let transition = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"transition","sessionId":session,"expectedHeadRevision":created["headRevision"],"event":{"kind":"task_transition_proposed","to":"in_progress"}}),
    );
    let started = call(
        &app,
        "work_tracking.advance",
        json!({"operationId":"completion-transition","sessionId":session,"expectedHeadRevision":transition["headRevision"],"sourceEventId":transition["eventId"],"action":"transition_task","decision":"accept","proposedPayload":{"taskId":task_id,"expectedTaskRevision":1,"to":"in_progress"}}),
    );
    let completion = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"c","sessionId":session,"expectedHeadRevision":started["headRevision"],"event":{"kind":"completion_proposal","outcomes":["done"],"verification":["tests pass"]}}),
    );
    call(
        &app,
        "work_tracking.advance",
        json!({"operationId":"completion-complete","sessionId":session,"expectedHeadRevision":completion["headRevision"],"sourceEventId":completion["eventId"],"action":"complete_task","decision":"accept","proposedPayload":{"taskId":task_id,"expectedTaskRevision":1,"evidence":"tests pass","report":"Verified privately"}}),
    );
    assert_eq!(
        std::fs::read_dir(&vault).unwrap().count(),
        0,
        "completion must not publish Knowledge"
    );
    let state = call(&app, "work_tracking.session", json!({"sessionId":session}));
    assert_eq!(state["state"], "completed");
    assert_eq!(state["publicationState"], "offered");
    call(
        &app,
        "work_tracking.knowledge.defer",
        json!({"sessionId":session,"completionRevision":state["headRevision"]}),
    );
    let deferred = call(&app, "work_tracking.session", json!({"sessionId":session}));
    assert_eq!(deferred["publicationState"], "deferred");
    assert_eq!(deferred["nextActions"], json!(["reopen_task"]));
    let draft = call(
        &app,
        "work_tracking.knowledge.draft.save",
        json!({"operationId":"draft","sessionId":session,"completionEventId":completion["eventId"],"title":"Separate completion","summary":"Publish later","bodyMarkdown":"# Separate completion\n\nVerified privately first.","evidenceRefs":[]}),
    );
    assert_eq!(draft["state"], "draft");
    assert_eq!(
        std::fs::read_dir(&vault).unwrap().count(),
        0,
        "draft save must not publish Knowledge"
    );
    let publish_input = json!({
        "operationId":"publish",
        "taskId":draft["taskId"],
        "draftRevision":draft["draftRevision"],
        "expectedContentHash":draft["contentHash"],
        "expectedSourceHash":draft["sourceHash"]
    });
    let queued = call(
        &app,
        "work_tracking.knowledge.publish",
        publish_input.clone(),
    );
    assert_eq!(queued["publicationStatus"], "queued");
    app.work_tracking_service().drain(100).unwrap();
    let database = rusqlite::Connection::open(root.path().join("db.sqlite")).unwrap();
    let (published_state, relative_path): (String, String) = database
        .query_row(
            "SELECT state,path FROM task_knowledge_drafts WHERE task_id=? AND revision=?",
            rusqlite::params![
                draft["taskId"].as_str().unwrap(),
                draft["draftRevision"].as_i64().unwrap()
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(published_state, "published");
    let path = vault.join(&relative_path);
    assert!(path.is_file());

    let undo = json!({
        "operationId":"withdraw",
        "taskId":draft["taskId"],
        "draftRevision":draft["draftRevision"],
        "expectedContentHash":draft["contentHash"],
        "expectedSourceHash":draft["sourceHash"]
    });
    let withdrawal = call(
        &app,
        "work_tracking.knowledge.withdraw.preview",
        undo.clone(),
    );
    let accepted = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.knowledge.withdraw.review".into(),
        input: json!({"proposal":undo,"reviewState":withdrawal["reviewState"],"decision":"accept"}),
    });
    assert_eq!(accepted.status, 200, "{}", accepted.body);
    app.work_tracking_service().drain(100).unwrap();
    let withdrawn: String = database
        .query_row(
            "SELECT state FROM task_knowledge_drafts WHERE task_id=? AND revision=?",
            rusqlite::params![
                draft["taskId"].as_str().unwrap(),
                draft["draftRevision"].as_i64().unwrap()
            ],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(withdrawn, "withdrawn");
    assert!(!path.exists());
}

#[test]
fn checkpoint_and_closed_state_survive_a_full_application_restart() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&vault, &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"restart-open","lineageKey":"restart","mode":"create","capture":{"title":"Restart","summary":"Keep the checkpoint"}}),
    );
    let appended = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"restart-append","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":"Persisted before restart"}}),
    );
    let session = opened["sessionId"].clone();
    drop(app);
    let restarted = NativeApplication::isolated(&vault, &db).unwrap();
    let restored = call(
        &restarted,
        "work_tracking.session",
        json!({"sessionId":session}),
    );
    assert_eq!(restored["headRevision"], appended["headRevision"]);
    assert_eq!(
        restored["recentEvents"][0]["payload"]["summary"],
        "Persisted before restart"
    );
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE work_tracking_sessions SET state='completed' WHERE id=?",
            [session.as_str().unwrap()],
        )
        .unwrap();
    let closed=restarted.execute_work_tracking(NativeOperation{name:"work_tracking.append".into(),input:json!({"operationId":"after-close","sessionId":session,"expectedHeadRevision":appended["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"must fail"}})});
    assert_eq!(closed.status, 409);
    assert_eq!(closed.body["error"]["code"], "session_closed");
}

#[test]
fn append_and_replay_recheck_transactional_authorization_and_keep_audit() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"auth-open","lineageKey":"auth","mode":"create","capture":{"title":"Authorization","summary":"Keep writes protected"}}),
    );
    let service = app.work_tracking_service();
    let input = json!({"operationId":"auth-append","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":"Private checkpoint"}});
    service.append("native-in-app-chat", &input).unwrap();
    assert_eq!(
        service.append("native-in-app-chat", &input).unwrap()["deduplicated"],
        true
    );
    let connection = rusqlite::Connection::open(&db).unwrap();
    for update in [
        "UPDATE mcp_connections SET scopes_json='[]' WHERE id='native-in-app-chat'",
        "UPDATE mcp_connections SET scopes_json='[\"session:write\"]',state='revoked' WHERE id='native-in-app-chat'",
    ] {
        connection.execute(update, []).unwrap();
        assert_eq!(service.append("native-in-app-chat", &input).unwrap_err().code, "not_found_or_not_visible");
        let mut fresh = input.clone();
        fresh["operationId"] = json!("unauthorized-fresh");
        fresh["expectedHeadRevision"] = json!(2);
        assert_eq!(service.append("native-in-app-chat", &fresh).unwrap_err().code, "not_found_or_not_visible");
    }
    let head: i64 = connection
        .query_row(
            "SELECT head_revision FROM work_tracking_sessions WHERE id=?",
            [opened["sessionId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(head, 2);
    for (outcome, count) in [("accepted", 2), ("rejected", 4)] {
        let actual: i64 = connection.query_row("SELECT count(*) FROM work_tracking_activity_events WHERE operation='inbound_work_append' AND outcome=?", [outcome], |row| row.get(0)).unwrap();
        assert_eq!(actual, count);
    }
}
