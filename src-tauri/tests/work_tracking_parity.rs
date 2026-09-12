use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn external_advance(
    service: &llm_wiki_desktop::application::work_tracking_service::WorkTrackingApplicationService,
    owner: &str,
    mut input: Value,
) -> Value {
    if input.get("operationId").and_then(Value::as_str).is_none() {
        input["operationId"] = json!(format!(
            "external-{}-{}",
            input["action"].as_str().unwrap_or("advance"),
            input["sourceEventId"].as_str().unwrap_or("none")
        ));
    }
    let state = service.begin_advance(owner, &input).unwrap();
    service
        .finish_advance(owner, &state, &input, "accept")
        .unwrap()
}

fn native(app: &NativeApplication, name: &str, input: Value) -> Value {
    if name == "work_tracking.advance" {
        let mut proposal = input.clone();
        let decision = proposal
            .as_object_mut()
            .unwrap()
            .remove("decision")
            .unwrap();
        let preview = native(app, "work_tracking.advance.preview", proposal.clone());
        return native(
            app,
            "work_tracking.advance.review",
            json!({"proposal":proposal,"reviewState":preview["reviewState"],"decision":decision}),
        );
    }
    if name == "work_tracking.open" {
        let preview = app.execute_work_tracking(NativeOperation {
            name: name.into(),
            input: input.clone(),
        });
        assert!(preview.status < 300, "{}", preview.body);
        if preview.body["decisionRequired"] == true {
            return native(
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

fn canonical_native(app: &NativeApplication) -> Value {
    let opened = native(
        app,
        "work_tracking.open",
        json!({"operationId":"open","lineageKey":"parity","mode":"create","capture":{"title":"Parity","summary":"Same canonical Task state"}}),
    );
    let session = opened["sessionId"].clone();
    let task = native(
        app,
        "work_tracking.append",
        json!({"operationId":"task","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"task_created","title":"Parity Task","outcome":"Same canonical lifecycle"}}),
    );
    let created = native(
        app,
        "work_tracking.advance",
        json!({"operationId":"native-create","sessionId":session,"expectedHeadRevision":task["headRevision"],"sourceEventId":task["eventId"],"action":"create_task","decision":"accept","proposedPayload":{"title":"Parity Task","outcome":"Same canonical lifecycle","scope":"Adapter parity","validationCriteria":"Both paths converge"}}),
    );
    let task_id = created["resultEntityId"].clone();
    let transition = native(
        app,
        "work_tracking.append",
        json!({"operationId":"transition","sessionId":session,"expectedHeadRevision":created["headRevision"],"event":{"kind":"task_transition_proposed","to":"in_progress"}}),
    );
    let started = native(
        app,
        "work_tracking.advance",
        json!({"operationId":"native-transition","sessionId":session,"expectedHeadRevision":transition["headRevision"],"sourceEventId":transition["eventId"],"action":"transition_task","decision":"accept","proposedPayload":{"taskId":task_id,"expectedTaskRevision":1,"to":"in_progress"}}),
    );
    let checkpoint = native(
        app,
        "work_tracking.append",
        json!({"operationId":"checkpoint","sessionId":session,"expectedHeadRevision":started["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"Canonical checkpoint","changes":["same"]}}),
    );
    native(
        app,
        "work_tracking.advance",
        json!({"operationId":"native-checkpoint","sessionId":session,"expectedHeadRevision":checkpoint["headRevision"],"sourceEventId":checkpoint["eventId"],"action":"accept_checkpoint","decision":"accept","proposedPayload":{}}),
    );
    // Accepted checkpoints remain queued until the canonical projector links their
    // Work Log entry. The external path drains explicitly below, so do the same here.
    native(app, "work_tracking.project", json!({"limit":100}));
    native(app, "work_tracking.session", json!({"sessionId":session}))
}

#[test]
fn native_and_external_adapters_converge_through_task_and_work_log() {
    let root = tempdir().unwrap();
    let native_app = NativeApplication::isolated(
        &root.path().join("native-vault"),
        &root.path().join("native.sqlite"),
    )
    .unwrap();
    let native_state = canonical_native(&native_app);
    let external_app = NativeApplication::isolated(
        &root.path().join("external-vault"),
        &root.path().join("external.sqlite"),
    )
    .unwrap();
    let service = external_app.work_tracking_service();
    let connection = service
        .create_connection(
            "External",
            &[
                "session:read".into(),
                "session:write".into(),
                "workbench:current:read".into(),
            ],
            &[],
            "confirm_each",
        )
        .unwrap();
    let owner = connection["id"].as_str().unwrap();
    let input = json!({"operationId":"open","lineageKey":"parity","mode":"create","capture":{"title":"Parity","summary":"Same canonical Task state"}});
    let preview = service.open(owner, &input).unwrap();
    let opened = service
        .finish_open(
            owner,
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let session = opened["sessionId"].clone();
    let task = service.append(owner, &json!({"operationId":"task","sessionId":session,"expectedHeadRevision":1,"event":{"kind":"task_created","title":"Parity Task","outcome":"Same canonical lifecycle"}})).unwrap();
    let created = external_advance(
        &service,
        owner,
        json!({"operationId":"create","sessionId":session,"expectedHeadRevision":task["headRevision"],"sourceEventId":task["eventId"],"action":"create_task","proposedPayload":{"title":"Parity Task","outcome":"Same canonical lifecycle","scope":"Adapter parity","validationCriteria":"Both paths converge"}}),
    );
    let task_id = created["resultEntityId"].clone();
    let transition = service.append(owner, &json!({"operationId":"transition","sessionId":session,"expectedHeadRevision":created["headRevision"],"event":{"kind":"task_transition_proposed","to":"in_progress"}})).unwrap();
    let started = external_advance(
        &service,
        owner,
        json!({"operationId":"start","sessionId":session,"expectedHeadRevision":transition["headRevision"],"sourceEventId":transition["eventId"],"action":"transition_task","proposedPayload":{"taskId":task_id,"expectedTaskRevision":1,"to":"in_progress"}}),
    );
    let checkpoint = service.append(owner, &json!({"operationId":"checkpoint","sessionId":session,"expectedHeadRevision":started["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"Canonical checkpoint","changes":["same"]}})).unwrap();
    external_advance(
        &service,
        owner,
        json!({"operationId":"accept-checkpoint","sessionId":session,"expectedHeadRevision":checkpoint["headRevision"],"sourceEventId":checkpoint["eventId"],"action":"accept_checkpoint","proposedPayload":{}}),
    );
    service.drain(100).unwrap();
    let external_state = service.session(owner, session.as_str().unwrap()).unwrap();
    assert_eq!(
        native_state["linkedWorkflow"]["task"]["title"],
        external_state["linkedWorkflow"]["task"]["title"]
    );
    assert_eq!(
        native_state["linkedWorkflow"]["task"]["state"],
        external_state["linkedWorkflow"]["task"]["state"]
    );
    assert_eq!(native_state["nextActions"], external_state["nextActions"]);
    assert_eq!(
        native_state["acceptedEvidenceIds"]
            .as_array()
            .unwrap()
            .len(),
        external_state["acceptedEvidenceIds"]
            .as_array()
            .unwrap()
            .len()
    );
}
