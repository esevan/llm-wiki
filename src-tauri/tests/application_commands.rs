use llm_wiki_desktop::{NativeApplication, NativeOperation, NativeResponse};
use serde_json::{json, Value};
use tempfile::TempDir;

struct Harness {
    _root: TempDir,
    app: NativeApplication,
}

impl Harness {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let app = NativeApplication::isolated(
            &root.path().join("vault"),
            &root.path().join("state.sqlite3"),
        )
        .unwrap();
        Self { _root: root, app }
    }
    fn call(&self, domain: &str, name: &str, input: Value) -> NativeResponse {
        self.app.execute_domain(
            domain,
            NativeOperation {
                name: name.into(),
                input,
            },
        )
    }
}

fn id(response: &NativeResponse) -> String {
    response.body["id"].as_str().unwrap().into()
}

#[test]
fn given_a_capture_when_created_then_it_is_persisted_on_the_native_board() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Native thought"}),
    );
    let board = harness.call("workflow", "board.get", json!({}));
    assert_eq!(capture.status, 201);
    assert_eq!(board.body["captures"][0]["text"], "Native thought");
}

#[test]
fn given_a_workflow_when_advanced_then_rust_enforces_review_preconditions() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Need a native boundary"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Socket coupling","detail":"Remove it"}),
    );
    let invalid = harness.call("workflow", "solution.create", json!({"problemId":id(&problem),"title":"Direct commands","outcome":"No internal HTTP","validation_criteria":"- [ ] Native"}));
    assert_eq!(invalid.status, 400);
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call("workflow", "solution.create", json!({"problemId":id(&problem),"title":"Direct commands","outcome":"No internal HTTP","validation_criteria":"- [ ] Native"}));
    assert_eq!(solution.status, 201);
    let premature = harness.call(
        "workflow",
        "solution.approve",
        json!({"solutionId":id(&solution)}),
    );
    assert_eq!(premature.status, 400);
}

#[test]
fn given_a_solution_when_work_is_recorded_then_comments_and_checklist_survive_queries() {
    let harness = Harness::new();
    let capture = harness.call("workflow", "capture.create", json!({"text":"Evidence"}));
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Evidence","detail":""}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call("workflow", "solution.create", json!({"problemId":id(&problem),"title":"Evidence","outcome":"Recorded","validation_criteria":"- [ ] Command test"}));
    let entry = harness.call(
        "workflow",
        "solution.progress.add",
        json!({"solutionId":id(&solution),"body":"Implemented"}),
    );
    harness.call(
        "workflow",
        "solution.comment.add",
        json!({"entryId":id(&entry),"body":"Reviewed"}),
    );
    let check = harness.call(
        "workflow",
        "solution.checklist.add",
        json!({"solutionId":id(&solution),"body":"Verified"}),
    );
    harness.call(
        "workflow",
        "solution.checklist.update",
        json!({"itemId":id(&check),"body":"Verified","checked":true}),
    );
    let progress = harness.call(
        "workflow",
        "solution.progress.get",
        json!({"solutionId":id(&solution)}),
    );
    assert_eq!(
        progress.body["entries"][0]["comments"][0]["body"],
        "Reviewed"
    );
    assert_eq!(
        progress.body["checklist"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["checked"] == 1)
            .count(),
        1
    );
}

#[test]
fn given_a_soft_deleted_item_when_restored_then_visibility_returns() {
    let harness = Harness::new();
    let capture = harness.call("workflow", "capture.create", json!({"text":"Recover me"}));
    harness.call(
        "workflow",
        "item.delete",
        json!({"entityType":"captures","entityId":id(&capture)}),
    );
    assert!(
        harness.call("workflow", "board.get", json!({})).body["captures"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    harness.call(
        "workflow",
        "item.restore",
        json!({"entityType":"captures","entityId":id(&capture)}),
    );
    assert_eq!(
        harness.call("workflow", "board.get", json!({})).body["captures"][0]["text"],
        "Recover me"
    );
}

#[test]
fn given_a_vault_document_when_indexed_then_native_search_and_safe_read_work() {
    let harness = Harness::new();
    std::fs::write(
        harness._root.path().join("vault/native.md"),
        "# Native command\nNo sidecar socket.",
    )
    .unwrap();
    let indexed = harness.call("vault", "vault.index", json!({}));
    let found = harness.call(
        "vault",
        "vault.search",
        json!({"query":"sidecar","limit":20}),
    );
    let read = harness.call("vault", "knowledge.read", json!({"path":"native.md"}));
    assert_eq!(indexed.status, 200);
    assert_eq!(found.body["results"][0]["path"], "native.md");
    assert!(read.body["content"]
        .as_str()
        .unwrap()
        .contains("No sidecar"));
    assert_eq!(
        harness
            .call("vault", "knowledge.read", json!({"path":"../secret"}))
            .status,
        404
    );
}

#[test]
fn given_a_cross_domain_operation_when_invoked_then_it_is_rejected() {
    let harness = Harness::new();
    let response = harness.call("vault", "capture.create", json!({"text":"wrong boundary"}));
    assert_eq!(response.status, 400);
    assert!(response.body["detail"]
        .as_str()
        .unwrap()
        .contains("not available in the vault domain"));
}

#[test]
fn given_locale_and_provider_settings_then_secrets_are_not_returned_to_the_ui() {
    let harness = Harness::new();
    assert_eq!(
        harness
            .call("settings", "locale.save", json!({"locale":"ko"}))
            .body["locale"],
        "ko"
    );
    let provider = harness.call(
        "settings",
        "provider.save",
        json!({"base_url":"https://api.example.test/v1","model":"model"}),
    );
    assert_eq!(provider.status, 200);
    assert!(provider.body.get("api_key").is_none());
}
