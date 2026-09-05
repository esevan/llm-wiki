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
    assert!(response.status < 300, "{name}: {}", response.body);
    response.body
}

struct ActiveSolution {
    session_id: Value,
    problem_id: String,
    solution_id: String,
}

#[test]
fn linking_binds_the_actual_target_and_workspace_revision() {
    let root = tempdir().unwrap();
    let app = NativeApplication::isolated(
        &root.path().join("vault"),
        &root.path().join("state.sqlite"),
    )
    .unwrap();
    let active = active_solution(&app);
    let new = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"second","lineageKey":"second","mode":"create","capture":{"title":"Continue existing work","summary":"Explicit link review"}}),
    );
    let proposal = json!({"sessionId":new["sessionId"],"expectedHeadRevision":new["headRevision"],"sourceEventId":new["headEventId"],"action":"link_current_work","proposedPayload":{"entityType":"features","entityId":active.solution_id}});
    let preview = call(&app, "work_tracking.advance.preview", proposal.clone());
    assert_eq!(preview["target"]["entityId"], active.solution_id);
    assert!(preview["target"]["title"]
        .as_str()
        .is_some_and(|title| !title.is_empty()));
    let changed=app.execute_domain("workflow",NativeOperation{name:"item.update".into(),input:json!({"entityType":"features","entityId":active.solution_id,"title":"Changed after review","expectedSourceRevision":preview["target"]["entityRevision"]})});
    assert_eq!(changed.status, 204, "{}", changed.body);
    let stale=app.execute_work_tracking(NativeOperation{name:"work_tracking.advance.review".into(),input:json!({"proposal":proposal,"reviewState":preview["reviewState"],"decision":"accept"})});
    assert_eq!(stale.status, 409);
    assert_eq!(stale.body["error"]["code"], "head_conflict");
    let refreshed = call(&app, "work_tracking.advance.preview", proposal.clone());
    assert_eq!(refreshed["target"]["title"], "Changed after review");
    call(
        &app,
        "work_tracking.advance.review",
        json!({"proposal":proposal,"reviewState":refreshed["reviewState"],"decision":"accept"}),
    );
    let linked = call(
        &app,
        "work_tracking.session",
        json!({"sessionId":new["sessionId"]}),
    );
    assert_eq!(
        linked["linkedWorkflow"]["solution"]["id"],
        active.solution_id
    );
}

fn current_head(app: &NativeApplication, session_id: &Value) -> i64 {
    call(
        app,
        "work_tracking.session",
        json!({"sessionId":session_id}),
    )["headRevision"]
        .as_i64()
        .unwrap()
}

fn active_solution(app: &NativeApplication) -> ActiveSolution {
    active_solution_with_initial_checkpoint(app, false)
}

fn active_solution_with_initial_checkpoint(
    app: &NativeApplication,
    initial_checkpoint: bool,
) -> ActiveSolution {
    let opened = call(
        app,
        "work_tracking.open",
        json!({"operationId":"acceptance-open","lineageKey":"acceptance","mode":"create","capture":{"title":"Acceptance","summary":"Exercise the real tracked workflow"}}),
    );
    let session = opened["sessionId"].clone();
    if initial_checkpoint {
        call(
            app,
            "work_tracking.append",
            json!({"operationId":"before-solution","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"work_log_checkpoint","summary":"Early research","evidenceRefs":["early-evidence"],"validation":["reviewed"],"origin":"spoofed-external"}}),
        );
        call(app, "work_tracking.project", json!({"limit":100}));
        let pending = call(app, "work_tracking.session", json!({"sessionId":session}));
        assert!(pending.to_string().contains("waiting_solution"));
        assert_eq!(
            call(app, "work_tracking.project", json!({"limit":100}))["processed"],
            0
        );
    }
    let problem = call(
        app,
        "work_tracking.append",
        json!({"operationId":"acceptance-problem","sessionId":session,"expectedHeadRevision":current_head(app, &session),"event":{"kind":"problem_draft","statement":"The release needs executable acceptance coverage","detail":"Run every user path"}}),
    );
    let adopted_problem = call(
        app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":current_head(app, &session),"sourceEventId":problem["eventId"],"action":"adopt_problem","decision":"accept","proposedPayload":{"statement":"The release needs executable acceptance coverage","detail":"Run every user path"}}),
    );
    let problem_id = adopted_problem["resultEntityId"]
        .as_str()
        .expect("problem id")
        .to_owned();
    call(
        app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":current_head(app, &session),"sourceEventId":problem["eventId"],"action":"approve_problem","decision":"accept","proposedPayload":{}}),
    );
    let solution = call(
        app,
        "work_tracking.append",
        json!({"operationId":"acceptance-solution","sessionId":session,"expectedHeadRevision":current_head(app, &session),"event":{"kind":"solution_draft","title":"Executable release gate","outcome":"Every scenario has evidence","validationCriteria":"All acceptance probes pass"}}),
    );
    let adopted_solution = call(
        app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":current_head(app, &session),"sourceEventId":solution["eventId"],"action":"adopt_solution","decision":"accept","proposedPayload":{"title":"Executable release gate","outcome":"Every scenario has evidence","validationCriteria":"All acceptance probes pass"}}),
    );
    let solution_id = adopted_solution["resultEntityId"]
        .as_str()
        .expect("solution id")
        .to_owned();
    call(
        app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":current_head(app, &session),"sourceEventId":solution["eventId"],"action":"resolve_conflict","decision":"accept","proposedPayload":{"evidence":[]}}),
    );
    call(
        app,
        "work_tracking.advance",
        json!({"sessionId":session,"expectedHeadRevision":current_head(app, &session),"sourceEventId":solution["eventId"],"action":"approve_solution","decision":"accept","proposedPayload":{}}),
    );
    ActiveSolution {
        session_id: session,
        problem_id,
        solution_id,
    }
}

#[test]
fn checkpoint_before_solution_waits_then_materializes_once_with_trusted_origin() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let active = active_solution_with_initial_checkpoint(&app, true);
    for _ in 0..2 {
        call(&app, "work_tracking.project", json!({"limit":100}));
        let progress = app.execute_domain(
            "workflow",
            NativeOperation {
                name: "solution.progress.get".into(),
                input: json!({"solutionId":active.solution_id,"locale":"en"}),
            },
        );
        assert_eq!(progress.status, 200);
        let entries = progress.body["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["evidenceRefs"], json!(["early-evidence"]));
        assert_eq!(entries[0]["origin"], "in_app_chat");
        assert_eq!(entries[0]["sessionId"], active.session_id);
    }
}

#[test]
fn us1_capture_is_not_persisted_before_exact_user_consent() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let preview = app.execute_work_tracking(NativeOperation { name: "work_tracking.open".into(), input: json!({"operationId":"consent-open","lineageKey":"consent","mode":"create","capture":{"title":"Preview only","summary":"Do not store before acceptance"}}) });
    assert_eq!(preview.status, 200);
    assert_eq!(preview.body["decisionRequired"], true);
    let connection = rusqlite::Connection::open(db).unwrap();
    let captures: i64 = connection
        .query_row("SELECT count(*) FROM captures", [], |row| row.get(0))
        .unwrap();
    assert_eq!(captures, 0, "Capture was persisted before a user decision");
}

#[test]
fn us2_checkpoint_materializes_structured_work_log_evidence() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let active = active_solution(&app);
    let checkpoint = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"structured-checkpoint","sessionId":active.session_id,"expectedHeadRevision":current_head(&app, &active.session_id),"event":{"kind":"work_log_checkpoint","summary":"Validated behavior","decisions":["keep one store"],"evidenceRefs":["ev_release"],"artifactRefs":["report.txt"],"validation":["passed"],"origin":"in_app_chat"}}),
    );
    call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":active.session_id,"expectedHeadRevision":current_head(&app, &active.session_id),"sourceEventId":checkpoint["eventId"],"action":"accept_checkpoint","decision":"accept","proposedPayload":checkpoint}),
    );
    call(&app, "work_tracking.project", json!({"limit":100}));
    let progress = app.execute_domain(
        "workflow",
        NativeOperation {
            name: "solution.progress.get".into(),
            input: json!({"solutionId":active.solution_id,"locale":"en"}),
        },
    );
    assert_eq!(progress.status, 200, "{}", progress.body);
    let entry = &progress.body["entries"][0];
    assert_eq!(entry["evidenceRefs"], json!(["ev_release"]));
    assert_eq!(entry["validation"], json!(["passed"]));
    assert_eq!(entry["origin"], "in_app_chat");
}

#[test]
fn us3_publication_cannot_run_without_a_separate_user_challenge() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    let app = NativeApplication::isolated(&vault, &root.path().join("db.sqlite")).unwrap();
    let active = active_solution(&app);
    let completion = call(
        &app,
        "work_tracking.append",
        json!({"operationId":"completion","sessionId":active.session_id,"expectedHeadRevision":current_head(&app, &active.session_id),"event":{"kind":"completion_proposal","outcomes":["done"],"verification":["passed"]}}),
    );
    call(
        &app,
        "work_tracking.advance",
        json!({"sessionId":active.session_id,"expectedHeadRevision":current_head(&app, &active.session_id),"sourceEventId":completion["eventId"],"action":"verify_and_complete","decision":"accept","proposedPayload":{"verification":["passed"]}}),
    );
    let draft = call(
        &app,
        "work_tracking.knowledge.draft.save",
        json!({"operationId":"draft","sessionId":active.session_id,"completionEventId":completion["eventId"],"title":"Acceptance","summary":"Reviewed","bodyMarkdown":"# Acceptance\n\nReviewed body","evidenceRefs":[]}),
    );
    let response = app.execute_work_tracking(NativeOperation {
        name: "work_tracking.knowledge.publish".into(),
        input: json!({"operationId":"publish-without-decision","draftId":draft["draftId"],"expectedDraftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"]}),
    });
    assert!(
        response.status >= 400,
        "Knowledge published without a separate interface-bound user challenge: {}",
        response.body
    );
    assert_eq!(std::fs::read_dir(vault).unwrap().count(), 0);
}

#[test]
fn us4_native_and_external_sessions_keep_distinct_provenance() {
    let root = tempdir().unwrap();
    let db = root.path().join("db.sqlite");
    let app = NativeApplication::isolated(&root.path().join("vault"), &db).unwrap();
    let opened = call(
        &app,
        "work_tracking.open",
        json!({"operationId":"native-provenance","lineageKey":"native-chat","mode":"create","capture":{"title":"Native","summary":"Native Chat source"}}),
    );
    let connection = rusqlite::Connection::open(db).unwrap();
    let source: String = connection
        .query_row(
            "SELECT source_interface FROM work_tracking_sessions WHERE id=?",
            [opened["sessionId"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(source, "in_app_chat");
}

#[test]
fn us5_workbench_edit_advances_the_shared_tracked_session_head() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let active = active_solution(&app);
    let before = current_head(&app, &active.session_id);
    let item = app.execute_domain(
        "workflow",
        NativeOperation {
            name: "item.get".into(),
            input: json!({"entityType":"problems","entityId":active.problem_id}),
        },
    );
    assert_eq!(item.status, 200, "{}", item.body);
    let updated = app.execute_domain(
        "workflow",
        NativeOperation {
            name: "item.update".into(),
            input: json!({"entityType":"problems","entityId":active.problem_id,"title":"Edited in Workbench","detail":"Same tracked work","expectedSourceRevision":item.body["sourceRevision"]}),
        },
    );
    assert_eq!(updated.status, 204, "{}", updated.body);
    let session = call(
        &app,
        "work_tracking.session",
        json!({"sessionId":active.session_id}),
    );
    assert!(
        session["headRevision"].as_i64().unwrap() > before,
        "Workbench edit did not advance the tracked session head"
    );
}

#[test]
fn us6_current_projection_contains_selection_and_resume_details() {
    let root = tempdir().unwrap();
    let app =
        NativeApplication::isolated(&root.path().join("vault"), &root.path().join("db.sqlite"))
            .unwrap();
    let active = active_solution(&app);
    // Selection must come from an actual user action, not an invented latest item.
    call(
        &app,
        "work_tracking.selection",
        json!({"entityType":"features","entityId":active.solution_id}),
    );
    let current = call(&app, "work_tracking.current", json!({"limit":10}));
    assert!(!current["activeSelection"].is_null());
    let first = &current["activeWork"][0];
    assert!(first.get("problem").is_some());
    assert!(first.get("solution").is_some());
    assert!(first.get("recentProgress").is_some());
    assert!(first.get("nextDecision").is_some());
}

#[test]
fn us7_topic_search_does_not_reveal_out_of_scope_overflow() {
    let root = tempdir().unwrap();
    let vault = root.path().join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    std::fs::write(
        vault.join("zz-Allowed.md"),
        "# Allowed\n\nrelease-scope-probe",
    )
    .unwrap();
    for index in 0..30 {
        std::fs::write(
            vault.join(format!("Secret-{index:02}.md")),
            "# Secret Allowed\n\nrelease-scope-probe Allowed",
        )
        .unwrap();
    }
    let app = NativeApplication::isolated(&vault, &root.path().join("db.sqlite")).unwrap();
    let indexed = app.execute_domain(
        "vault",
        NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        },
    );
    assert_eq!(indexed.status, 200, "{}", indexed.body);
    let service = app.work_tracking_service();
    service.set_topic_membership(&json!({"topicId":"Allowed","entityType":"vault","entityId":"zz-Allowed.md","included":true})).unwrap();
    let connection = service
        .create_connection(
            "Topic probe",
            &[
                "topic:read".into(),
                "vault:search:lexical".into(),
                "vault:evidence:read".into(),
            ],
            &["Allowed".into()],
            "confirm_each",
        )
        .unwrap();
    let result = service
        .lexical_search(
            connection["id"].as_str().unwrap(),
            "topic",
            "Allowed",
            "release-scope-probe",
            10,
        )
        .unwrap();
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["truncated"], false,
        "out-of-scope result count leaked through truncation"
    );
    assert_eq!(result["hits"][0]["sourceIdentity"], "zz-Allowed.md");
    assert!(!result["hits"][0]["matchedTerms"]
        .as_array()
        .unwrap()
        .is_empty());
    let evidence = json!({"evidenceId":result["hits"][0]["evidenceId"],"expectedRevision":result["hits"][0]["revision"]});
    service
        .evidence_read(
            connection["id"].as_str().unwrap(),
            std::slice::from_ref(&evidence),
        )
        .unwrap();
    service.set_topic_membership(&json!({"topicId":"Allowed","entityType":"vault","entityId":"zz-Allowed.md","included":false})).unwrap();
    assert!(
        service
            .evidence_read(connection["id"].as_str().unwrap(), &[evidence])
            .is_err(),
        "removed topic membership must invalidate issued evidence"
    );
}
