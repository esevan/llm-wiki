use llm_wiki_desktop::{NativeApplication, NativeOperation};
use serde_json::{json, Value};
use tempfile::tempdir;

fn external_advance(
    service: &llm_wiki_desktop::application::work_tracking_service::WorkTrackingApplicationService,
    owner: &str,
    input: Value,
) -> Value {
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

#[test]
fn native_and_external_adapters_converge_through_solution_and_work_log() {
    let root = tempdir().unwrap();
    let native_app = NativeApplication::isolated(
        &root.path().join("native-vault"),
        &root.path().join("native.sqlite"),
    )
    .unwrap();
    let opened = native(
        &native_app,
        "work_tracking.open",
        json!({"operationId":"open","lineageKey":"parity","mode":"create","capture":{"title":"Parity","summary":"Same canonical state"}}),
    );
    let proposal = native(
        &native_app,
        "work_tracking.append",
        json!({"operationId":"problem","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"problem_draft","statement":"One workflow","detail":"Across adapters"}}),
    );
    let adopted = native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":2,"sourceEventId":proposal["eventId"],"action":"adopt_problem","decision":"accept","proposedPayload":{"statement":"One workflow","detail":"Across adapters"}}),
    );
    let approved = native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":adopted["headRevision"],"sourceEventId":proposal["eventId"],"action":"approve_problem","decision":"accept","proposedPayload":{}}),
    );
    let solution = native(
        &native_app,
        "work_tracking.append",
        json!({"operationId":"solution","sessionId":opened["sessionId"],"expectedHeadRevision":approved["headRevision"],"event":{"kind":"solution_draft","title":"One Solution","outcome":"One canonical lifecycle"}}),
    );
    let adopted_solution = native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":solution["headRevision"],"sourceEventId":solution["eventId"],"action":"adopt_solution","decision":"accept","proposedPayload":{"title":"One Solution","outcome":"One canonical lifecycle"}}),
    );
    let conflict = native(
        &native_app,
        "work_tracking.append",
        json!({"operationId":"conflict","sessionId":opened["sessionId"],"expectedHeadRevision":adopted_solution["headRevision"],"event":{"kind":"conflict_proposal","rationale":"Empty fixture Vault","proposedResolution":"Proceed with explicit insufficient coverage","evidence":[],"coverage":"insufficient"}}),
    );
    let resolved = native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":conflict["headRevision"],"sourceEventId":conflict["eventId"],"action":"resolve_conflict","decision":"accept","proposedPayload":{"rationale":"Empty fixture Vault","proposedResolution":"Proceed with explicit insufficient coverage","evidence":[],"coverage":"insufficient"}}),
    );
    let started = native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":resolved["headRevision"],"sourceEventId":solution["eventId"],"action":"approve_solution","decision":"accept","proposedPayload":{}}),
    );
    let checkpoint = native(
        &native_app,
        "work_tracking.append",
        json!({"operationId":"checkpoint","sessionId":opened["sessionId"],"expectedHeadRevision":started["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"Canonical checkpoint","changes":["same"]}}),
    );
    native(
        &native_app,
        "work_tracking.advance",
        json!({"sessionId":opened["sessionId"],"expectedHeadRevision":checkpoint["headRevision"],"sourceEventId":checkpoint["eventId"],"action":"accept_checkpoint","decision":"accept","proposedPayload":{}}),
    );
    native(&native_app, "work_tracking.project", json!({"limit":100}));
    let native_state = native(
        &native_app,
        "work_tracking.session",
        json!({"sessionId":opened["sessionId"]}),
    );

    let external_app = NativeApplication::isolated(
        &root.path().join("external-vault"),
        &root.path().join("external.sqlite"),
    )
    .unwrap();
    let service = external_app.work_tracking_service();
    let connection = service
        .create_connection(
            "External",
            &["session:read".into(), "session:write".into()],
            &[],
            "confirm_each",
        )
        .unwrap();
    let owner = connection["id"].as_str().unwrap();
    let input = json!({"operationId":"open","lineageKey":"parity","mode":"create","capture":{"title":"Parity","summary":"Same canonical state"}});
    let preview = service.open(owner, &input).unwrap();
    let opened = service
        .finish_open(
            owner,
            &input,
            preview["reviewState"].as_str().unwrap(),
            "accept",
        )
        .unwrap();
    let proposal=service.append(owner,&json!({"operationId":"problem","sessionId":opened["sessionId"],"expectedHeadRevision":1,"event":{"kind":"problem_draft","statement":"One workflow","detail":"Across adapters"}})).unwrap();
    let advance = json!({"operationId":"adopt","sessionId":opened["sessionId"],"expectedHeadRevision":2,"sourceEventId":proposal["eventId"],"action":"adopt_problem","proposedPayload":{"statement":"One workflow","detail":"Across adapters"}});
    let challenge = service.begin_advance(owner, &advance).unwrap();
    let adopted = service
        .finish_advance(owner, &challenge, &advance, "accept")
        .unwrap();
    let approved = external_advance(
        &service,
        owner,
        json!({"operationId":"approve","sessionId":opened["sessionId"],"expectedHeadRevision":adopted["headRevision"],"sourceEventId":proposal["eventId"],"action":"approve_problem","proposedPayload":{}}),
    );
    let solution=service.append(owner,&json!({"operationId":"solution","sessionId":opened["sessionId"],"expectedHeadRevision":approved["headRevision"],"event":{"kind":"solution_draft","title":"One Solution","outcome":"One canonical lifecycle"}})).unwrap();
    let adopted_solution = external_advance(
        &service,
        owner,
        json!({"operationId":"adopt-solution","sessionId":opened["sessionId"],"expectedHeadRevision":solution["headRevision"],"sourceEventId":solution["eventId"],"action":"adopt_solution","proposedPayload":{"title":"One Solution","outcome":"One canonical lifecycle"}}),
    );
    let conflict=service.append(owner,&json!({"operationId":"conflict","sessionId":opened["sessionId"],"expectedHeadRevision":adopted_solution["headRevision"],"event":{"kind":"conflict_proposal","rationale":"Empty fixture Vault","proposedResolution":"Proceed with explicit insufficient coverage","evidence":[],"coverage":"insufficient"}})).unwrap();
    let resolved = external_advance(
        &service,
        owner,
        json!({"operationId":"resolve","sessionId":opened["sessionId"],"expectedHeadRevision":conflict["headRevision"],"sourceEventId":conflict["eventId"],"action":"resolve_conflict","proposedPayload":{"rationale":"Empty fixture Vault","proposedResolution":"Proceed with explicit insufficient coverage","evidence":[],"coverage":"insufficient"}}),
    );
    let started = external_advance(
        &service,
        owner,
        json!({"operationId":"start","sessionId":opened["sessionId"],"expectedHeadRevision":resolved["headRevision"],"sourceEventId":solution["eventId"],"action":"approve_solution","proposedPayload":{}}),
    );
    let checkpoint=service.append(owner,&json!({"operationId":"checkpoint","sessionId":opened["sessionId"],"expectedHeadRevision":started["headRevision"],"event":{"kind":"work_log_checkpoint","summary":"Canonical checkpoint","changes":["same"]}})).unwrap();
    external_advance(
        &service,
        owner,
        json!({"operationId":"accept-checkpoint","sessionId":opened["sessionId"],"expectedHeadRevision":checkpoint["headRevision"],"sourceEventId":checkpoint["eventId"],"action":"accept_checkpoint","proposedPayload":{}}),
    );
    service.drain(100).unwrap();
    let external_state = service
        .session(owner, opened["sessionId"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        native_state["linkedWorkflow"]["problem"]["title"],
        external_state["linkedWorkflow"]["problem"]["title"]
    );
    assert_eq!(
        native_state["linkedWorkflow"]["problem"]["state"],
        external_state["linkedWorkflow"]["problem"]["state"]
    );
    assert_eq!(native_state["nextActions"], external_state["nextActions"]);
    assert_eq!(
        native_state["linkedWorkflow"]["solution"]["title"],
        external_state["linkedWorkflow"]["solution"]["title"]
    );
    assert_eq!(
        native_state["linkedWorkflow"]["solution"]["state"],
        external_state["linkedWorkflow"]["solution"]["state"]
    );
    assert_eq!(
        native_state["acceptedEvidenceIds"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        external_state["acceptedEvidenceIds"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        native_state["recentEvents"][0]["payload"]["summary"],
        external_state["recentEvents"][0]["payload"]["summary"]
    );
}
