use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn call(app: &NativeApplication, name: &str, mut input: Value) -> Value {
    if name == "work_tracking.advance" {
        if input["action"] == "resolve_conflict" {
            // Explicit fixture: the user reviewed limited coverage in this empty test Vault.
            let payload = json!({"kind":"conflict_proposal","rationale":"Empty test Vault has no corroborating evidence","proposedResolution":"Proceed with the reviewed test plan","evidence":[],"coverage":"insufficient"});
            let event = call(
                app,
                "work_tracking.append",
                json!({"operationId":"fixture-conflict","sessionId":input["sessionId"],"expectedHeadRevision":input["expectedHeadRevision"],"event":payload}),
            );
            input["sourceEventId"] = event["eventId"].clone();
            input["expectedHeadRevision"] = event["headRevision"].clone();
            input["proposedPayload"] = payload;
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
    let response=app.execute_work_tracking(NativeOperation{name:"work_tracking.append".into(),input:json!({"operationId":"append-1","sessionId":first["sessionId"],"expectedHeadRevision":0,"event":{"kind":"problem_draft","statement":"Wrong head"}})});
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
    let problem = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"p","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"problem_draft","statement":"Publication is coupled","detail":"Split it"}}),
    );
    let adopted = call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":2,"sourceEventId":problem["eventId"],"action":"adopt_problem","decision":"accept","proposedPayload":{"statement":"Publication is coupled","detail":"Split it"}}),
    );
    let approved = call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":adopted["headRevision"],"sourceEventId":problem["eventId"],"action":"approve_problem","decision":"accept","proposedPayload":{}}),
    );
    let solution = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"s","sessionId":session,"expectedHeadRevision":approved["headRevision"],"event":{"kind":"solution_draft","title":"Two phases","outcome":"Private completion first"}}),
    );
    let adopted_solution = call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":solution["headRevision"],"sourceEventId":solution["eventId"],"action":"adopt_solution","decision":"accept","proposedPayload":{"title":"Two phases","outcome":"Private completion first"}}),
    );
    let resolved = call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":adopted_solution["headRevision"],"sourceEventId":solution["eventId"],"action":"resolve_conflict","decision":"accept","proposedPayload":{}}),
    );
    let approved_solution = call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":resolved["headRevision"],"sourceEventId":solution["eventId"],"action":"approve_solution","decision":"accept","proposedPayload":{}}),
    );
    let completion = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"c","sessionId":session,"expectedHeadRevision":approved_solution["headRevision"],"event":{"kind":"completion_proposal","outcomes":["done"],"verification":["tests pass"]}}),
    );
    call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":completion["headRevision"],"sourceEventId":completion["eventId"],"action":"verify_and_complete","decision":"accept","proposedPayload":{"verification":["tests pass"]}}),
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
    assert_eq!(deferred["nextActions"], json!([]));
    let draft = call(
        &app,
        "work_tracking.knowledge.draft.save",
        json!({"operationId":"draft","sessionId":session,"completionEventId":completion["eventId"],"title":"Separate completion","summary":"Publish later","bodyMarkdown":"# Separate completion\n\nVerified privately first.","evidenceRefs":[]}),
    );
    assert_eq!(draft["status"], "draft");
    assert_eq!(
        std::fs::read_dir(&vault).unwrap().count(),
        0,
        "draft save must not publish Knowledge"
    );
    // Crash boundary: the reviewed file is created, then publication-decision
    // persistence fails. The durable approval/job must recover after restart.
    let publish_input = json!({"operationId":"publish","draftId":draft["draftId"],"expectedDraftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]});
    let review = call(
        &app,
        "work_tracking.knowledge.publish.preview",
        publish_input.clone(),
    );
    let database = rusqlite::Connection::open(root.path().join("db.sqlite")).unwrap();
    database.execute_batch("CREATE TRIGGER fail_publication BEFORE INSERT ON knowledge_publication_decisions BEGIN SELECT RAISE(ABORT,'injected publication failure'); END;").unwrap();
    let failed=app.execute_work_tracking(NativeOperation{name:"work_tracking.knowledge.publish.review".into(),input:json!({"proposal":publish_input,"reviewState":review["reviewState"],"decision":"accept"})});
    assert!(failed.status >= 400);
    assert_eq!(
        database
            .query_row(
                "SELECT count(*) FROM knowledge_publication_decisions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        std::fs::read_dir(vault.join("Knowledge")).unwrap().count(),
        1
    );
    database
        .execute_batch("DROP TRIGGER fail_publication;")
        .unwrap();
    drop(database);
    drop(app);
    let app = NativeApplication::isolated(&vault, &root.path().join("db.sqlite")).unwrap();
    app.work_tracking_service().drain(100).unwrap();
    assert_eq!(
        rusqlite::Connection::open(root.path().join("db.sqlite"))
            .unwrap()
            .query_row(
                "SELECT count(*) FROM work_tracking_publication_jobs WHERE state='complete'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    let published = call(
        &app,
        "work_tracking.knowledge.publish",
        json!({"operationId":"publish","draftId":draft["draftId"],"expectedDraftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]}),
    );
    assert_eq!(published["status"], "published");
    assert!(vault.join(published["path"].as_str().unwrap()).is_file());
    let replay = call(
        &app,
        "work_tracking.knowledge.publish",
        json!({"operationId":"publish","draftId":draft["draftId"],"expectedDraftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]}),
    );
    assert_eq!(replay["deduplicated"], true);
    let service = app.work_tracking_service();
    let undo = json!({"operationId":"withdraw","draftId":draft["draftId"],"expectedDraftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]});
    let preview = service
        .begin_withdrawal("native-in-app-chat", &undo)
        .unwrap();
    let path = vault.join(published["path"].as_str().unwrap());
    let content = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, "External changed publication").unwrap();
    assert!(service
        .finish_withdrawal(
            "native-in-app-chat",
            &undo,
            preview["reviewState"].as_str().unwrap(),
            "accept"
        )
        .is_err());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "External changed publication"
    );
    std::fs::write(&path, &content).unwrap();
    let withdrawn = service
        .finish_withdrawal(
            "native-in-app-chat",
            &undo,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_to_string(vault.join(withdrawn["recoveryReference"].as_str().unwrap()))
            .unwrap(),
        content
    );
    assert_eq!(
        service
            .session("native-in-app-chat", session.as_str().unwrap())
            .unwrap()["state"],
        "completed"
    );
    assert!(service.begin_publish("native-in-app-chat", &undo).is_err());
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
