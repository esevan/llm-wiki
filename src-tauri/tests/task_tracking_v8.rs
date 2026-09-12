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
    let review = service
        .begin_advance("native-in-app-chat", &proposal)
        .unwrap();
    let error = service
        .finish_advance("native-in-app-chat", &review, &proposal, "accept")
        .unwrap_err();
    assert_eq!(error.code, "not_found_or_not_visible");
    assert!(
        error.message.to_lowercase().contains("problem revision"),
        "{:?}",
        error
    );
}
