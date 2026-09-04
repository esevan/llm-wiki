use llm_wiki_desktop::{NativeApplication, NativeOperation, NativeResponse};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
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

fn provider_once(content: Value) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        let payload = json!({"choices":[{"message":{"content":content.to_string()}}]}).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            payload.len(), payload
        );
        reader.get_mut().write_all(response.as_bytes()).unwrap();
    });
    format!("http://{address}/v1")
}

fn slow_provider() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(2));
        let _ =
            stream.write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
    });
    format!("http://{address}/v1")
}

fn wait_for_job(harness: &Harness, queued: &NativeResponse) -> NativeResponse {
    let job_id = id(queued);
    for _ in 0..200 {
        let job = harness.call("jobs", "jobs.get", json!({"jobId":job_id}));
        if job.body["status"] == "completed" {
            return harness.call("jobs", "jobs.result", json!({"jobId":job_id}));
        }
        assert_ne!(job.body["status"], "failed", "{}", job.body);
        thread::sleep(Duration::from_millis(10));
    }
    panic!("native AI job did not finish")
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
fn given_a_legacy_localization_database_when_opened_then_native_preserves_versions() {
    let root = tempfile::tempdir().unwrap();
    let db_path = root.path().join("state.sqlite3");
    let connection = rusqlite::Connection::open(&db_path).unwrap();
    connection.execute_batch(
        "CREATE TABLE captures(id TEXT PRIMARY KEY,text TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
         CREATE TABLE localized_content(
           entity_type TEXT NOT NULL,entity_id TEXT NOT NULL,field_name TEXT NOT NULL,
           locale TEXT NOT NULL,value TEXT NOT NULL,origin TEXT NOT NULL,source_hash TEXT NOT NULL DEFAULT '',
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           PRIMARY KEY(entity_type,entity_id,field_name,locale));
         INSERT INTO captures(id,text) VALUES ('legacy','Original');
         INSERT INTO localized_content(entity_type,entity_id,field_name,locale,value,origin)
           VALUES ('captures','legacy','text','ko','기존 데이터','user');",
    ).unwrap();
    drop(connection);
    let app = NativeApplication::isolated(&root.path().join("vault"), &db_path).unwrap();
    let board = app.execute_domain(
        "workflow",
        NativeOperation {
            name: "board.get".into(),
            input: json!({"locale":"ko"}),
        },
    );
    assert_eq!(board.status, 200, "{}", board.body);
    assert_eq!(board.body["captures"][0]["text"], "기존 데이터");
    let connection = rusqlite::Connection::open(&db_path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        2
    );
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
            .filter(|item| item["checked"] == true)
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
    let read = harness.call(
        "vault",
        "knowledge.read",
        json!({"path":"native.md","locale":"en"}),
    );
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
    assert!(harness.app.settings_path().is_file());
    let connection = rusqlite::Connection::open(harness.app.db_path()).unwrap();
    for removed_table in ["app_settings", "locale_settings", "provider_settings"] {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
                [removed_table],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists, "{removed_table} must not be created in SQLite");
    }
}

#[test]
fn given_reviewed_work_when_completed_then_native_records_and_dashboard_restore_it() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Finish safely"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Finish safely","detail":"Keep evidence"}),
    );
    assert_eq!(
        harness.call(
            "workflow",
            "problem.importance.save",
            json!({"problemId":id(&problem),"alignment":5,"impact":5,"urgency":5,"leverage":5,"evidence":"User impact"})
        ).body["importance"],
        100
    );
    assert_eq!(
        harness
            .call(
                "workflow",
                "problem.record",
                json!({"problemId":id(&problem)})
            )
            .body["statement"],
        "Finish safely"
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call(
        "workflow",
        "solution.create",
        json!({"problemId":id(&problem),"title":"Native completion","outcome":"Persist evidence","validation_criteria":"- [ ] Verified"}),
    );
    harness.call(
        "workflow",
        "solution.conflict.save",
        json!({"solutionId":id(&solution),"state":"clear","citation":"Reviewed locally"}),
    );
    harness.call(
        "workflow",
        "solution.approve",
        json!({"solutionId":id(&solution)}),
    );
    let completion = harness.call(
        "workflow",
        "solution.completion.create",
        json!({"solutionId":id(&solution),"evidence":"Tests passed","report":"Ready","no_update_reason":"No reusable change"}),
    );
    assert_eq!(completion.status, 201, "{}", completion.body);
    assert_eq!(completion.body["knowledge_status"], "not_needed");
    assert_eq!(
        harness
            .call(
                "workflow",
                "solution.completion.verify",
                json!({"solutionId":id(&solution)})
            )
            .status,
        204
    );
    let completed = harness.call(
        "workflow",
        "workbench.completed",
        json!({"limit":20,"locale":"en"}),
    );
    assert_eq!(completed.body["solutions"][0]["title"], "Native completion");
    assert_eq!(
        completed.body["solutions"][0]["completion_evidence"],
        "Tests passed"
    );
    let dashboard = harness.call("workflow", "compass.dashboard", json!({}));
    assert_eq!(dashboard.body["events"].as_array().unwrap().len(), 3);
    assert_eq!(dashboard.body["scores"][0]["points"], 100.0);
}

#[test]
fn given_a_completed_problem_then_native_archives_evidence_and_protects_external_edits() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Original evidence"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Archive safely","detail":"Keep all work"}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call("workflow", "solution.create", json!({"problemId":id(&problem),"title":"Preserve record","outcome":"Retain evidence","non_goals":"No loss","validation_criteria":"- [ ] Everything retained"}));
    let entry = harness.call("workflow", "solution.progress.add", json!({"solutionId":id(&solution),"body":"Captured UI","image_data":"aGVsbG8=","image_media_type":"image/png"}));
    harness.call(
        "workflow",
        "solution.comment.add",
        json!({"entryId":id(&entry),"body":"Reviewed by user"}),
    );
    let completed = harness.call(
        "workflow",
        "problem.complete",
        json!({"problemId":id(&problem),"reason":"Human review complete"}),
    );
    assert_eq!(completed.status, 200, "{}", completed.body);
    assert_eq!(completed.body["closed"]["problem"], id(&problem));
    assert_eq!(completed.body["closed"]["capture"], id(&capture));
    assert_eq!(completed.body["closed"]["solutions"][0], id(&solution));
    let problem_record = harness.call(
        "workflow",
        "problem.record",
        json!({"problemId":id(&problem)}),
    );
    assert_eq!(problem_record.body["state"], "completed");
    let completed_solutions = harness.call(
        "workflow",
        "workbench.completed",
        json!({"limit":20,"locale":"en"}),
    );
    assert_eq!(
        completed_solutions.body["solutions"][0]["state"],
        "completed"
    );
    let board = harness.call("workflow", "board.get", json!({}));
    assert!(board.body["captures"].as_array().unwrap().is_empty());
    assert!(board.body["problems"].as_array().unwrap().is_empty());
    assert!(board.body["features"].as_array().unwrap().is_empty());
    let path = completed.body["path"].as_str().unwrap();
    let playbook = harness._root.path().join("vault").join(path);
    let content = std::fs::read_to_string(&playbook).unwrap();
    assert!(content.contains("## Executive Summary"));
    assert!(content.contains("## Lineage"));
    assert!(content.contains("## Decision Changes"));
    assert!(content.contains("## Conflicts & Addresses"));
    assert!(content.contains("## Completion Evidence"));
    assert!(content.contains("[Raw work record](<assets/"));
    let assets = playbook.parent().unwrap().join("assets");
    let raw = std::fs::read_dir(&assets)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .unwrap();
    let raw_content = std::fs::read_to_string(&raw).unwrap();
    assert!(raw_content.contains("Captured UI"));
    assert!(raw_content.contains("Reviewed by user"));
    assert!(raw_content.contains("- [ ] Everything retained"));
    assert_eq!(
        std::fs::read(assets.join(format!("{}.png", id(&entry)))).unwrap(),
        b"hello"
    );
    std::fs::write(&playbook, format!("{content}\nExternal note\n")).unwrap();
    let blocked = harness.call(
        "workflow",
        "problem.playbook.delete",
        json!({"problemId":id(&problem),"force":false}),
    );
    assert_eq!(blocked.status, 409);
    assert!(playbook.exists());
    let removed = harness.call(
        "workflow",
        "problem.playbook.delete",
        json!({"problemId":id(&problem),"force":true}),
    );
    assert_eq!(removed.status, 204, "{}", removed.body);
    assert!(!playbook.exists());
    assert!(!raw.exists());
}

#[test]
fn given_workflow_context_when_refining_then_native_returns_bounded_visible_evidence() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"A context-bearing problem"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"detail":"People need the earlier evidence while refining."}),
    );
    let context = harness.call(
        "workflow",
        "refinement.context",
        json!({"entityType":"problems","entityId":id(&problem),"locale":"en"}),
    );
    assert_eq!(context.status, 200, "{}", context.body);
    assert_eq!(context.body["has_context"], true);
    assert_eq!(context.body["entries"][0]["label"], "Current item");
    assert_eq!(
        context.body["entries"][0]["text"],
        "A context-bearing problem"
    );
    assert_eq!(context.body["entries"][1]["label"], "Current context");
    assert!(context.body["entries"][1]["text"]
        .as_str()
        .unwrap()
        .contains("earlier evidence"));
    assert!(context.body["refinement_draft"].is_null());
    assert!(context.body["next_draft"].is_null());
}

#[tokio::test]
async fn given_completed_work_when_lineage_is_inferred_then_evidence_and_corrections_are_auditable()
{
    let harness = Harness::new();
    let capture = harness.call("workflow", "capture.create", json!({"text":"Trace origin"}));
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Keep lineage","detail":"Preserve evidence"}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call("workflow", "solution.create", json!({"problemId":id(&problem),"title":"Trace decisions","outcome":"Auditable history","validation_criteria":"- [ ] Four stages"}));
    harness.call(
        "workflow",
        "problem.complete",
        json!({"problemId":id(&problem),"reason":"Reviewed"}),
    );
    let initial = harness.call(
        "workflow",
        "solution.lineage",
        json!({"solutionId":id(&solution)}),
    );
    assert_eq!(initial.status, 200, "{}", initial.body);
    assert_eq!(
        initial.body["lineage"]["stages"].as_array().unwrap().len(),
        4
    );
    assert_eq!(
        initial.body["lineage"]["transitions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let evidence_id = initial.body["evidence"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let provider = provider_once(
        json!({"claims":[{"claim_key":"inferred:rationale","text":"Likely rationale","confidence":"medium","evidence_ids":[evidence_id]}]}),
    );
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":provider,"model":"test-model"}),
    );
    let queued = harness.app.enqueue_job(json!({"taskKind":"lineage_inference","entityType":"features","entityId":id(&solution),"locale":"en"})).await;
    let result = wait_for_job(&harness, &queued);
    let inferred = result.body["result"]["claims"]
        .as_object()
        .unwrap()
        .values()
        .find(|claim| claim["classification"] == "inferred")
        .unwrap();
    let corrected = harness.call("workflow", "solution.lineage.correct", json!({"solutionId":id(&solution),"claimId":inferred["id"],"text":"User-confirmed rationale","reason":"Audit correction","current_revision_id":inferred["current_revision_id"]}));
    assert_eq!(corrected.status, 201, "{}", corrected.body);
    let current = harness.call(
        "workflow",
        "solution.lineage",
        json!({"solutionId":id(&solution)}),
    );
    let claim = &current.body["claims"][inferred["id"].as_str().unwrap()];
    assert_eq!(claim["text"], "User-confirmed rationale");
    assert_eq!(claim["revisions"].as_array().unwrap().len(), 2);
    let evidence = harness.call(
        "workflow",
        "solution.lineage.evidence",
        json!({"solutionId":id(&solution),"evidenceId":evidence_id}),
    );
    assert_eq!(evidence.status, 200, "{}", evidence.body);
}

#[test]
fn given_a_projection_when_the_file_changes_then_native_blocks_overwrite_and_archive() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Reusable record"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Reusable record","detail":"Preserve ownership"}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let projected = harness.call(
        "workflow",
        "item.project",
        json!({"entityType":"problems","entityId":id(&problem)}),
    );
    assert_eq!(projected.status, 201, "{}", projected.body);
    let relative = projected.body["path"].as_str().unwrap();
    let file = harness._root.path().join("vault").join(relative);
    assert!(file.is_file());
    std::fs::write(&file, "# externally changed").unwrap();
    let overwrite = harness.call(
        "workflow",
        "item.project",
        json!({"entityType":"problems","entityId":id(&problem)}),
    );
    assert_eq!(overwrite.status, 409);
    let archive = harness.call(
        "workflow",
        "item.archive",
        json!({"entityType":"problems","entityId":id(&problem)}),
    );
    assert_eq!(archive.status, 409);
    assert_eq!(
        std::fs::read_to_string(file).unwrap(),
        "# externally changed"
    );
}

#[test]
fn given_a_reviewed_knowledge_patch_when_source_changes_then_apply_and_undo_are_safe() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Patch knowledge"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture)}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call(
        "workflow",
        "solution.create",
        json!({"problemId":id(&problem),"title":"Review patch","outcome":"Preserve source","validation_criteria":"- [ ] Reviewed"}),
    );
    let relative = "Knowledge/review.md";
    let file = harness._root.path().join("vault").join(relative);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "# Existing\n\nStable.\n").unwrap();
    let first = harness.call(
        "workflow",
        "solution.patch.create",
        json!({"solutionId":id(&solution),"path":relative,"operation":"append_section","heading":"Evidence","content":"Verified."}),
    );
    assert_eq!(first.status, 201, "{}", first.body);
    std::fs::write(&file, "# External\n").unwrap();
    let blocked = harness.call(
        "workflow",
        "solution.patch.apply",
        json!({"patchId":id(&first)}),
    );
    assert_eq!(blocked.status, 409);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "# External\n");

    std::fs::write(&file, "# Existing\n\nStable.\n").unwrap();
    let second = harness.call(
        "workflow",
        "solution.patch.create",
        json!({"solutionId":id(&solution),"path":relative,"operation":"append_section","heading":"Evidence","content":"Verified."}),
    );
    assert_eq!(
        harness
            .call(
                "workflow",
                "solution.patch.apply",
                json!({"patchId":id(&second)})
            )
            .status,
        204
    );
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("Verified."));
    assert_eq!(
        harness
            .call(
                "workflow",
                "solution.patch.undo",
                json!({"patchId":id(&second)})
            )
            .status,
        204
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "# Existing\n\nStable.\n"
    );
}

#[tokio::test]
async fn given_a_native_conflict_job_when_resolved_then_decisions_are_durable() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Conflicting scope"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture),"statement":"Conflicting scope","detail":"Review evidence"}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call(
        "workflow",
        "solution.create",
        json!({"problemId":id(&problem),"title":"Offline only","outcome":"No network","validation_criteria":"- [ ] Reviewed"}),
    );
    let provider = provider_once(json!({
        "conflicts":[{
            "id":"conflict-1","target_id":"decisions.md","target_title":"Prior decision",
            "severity":"high","category":"Scope","summary":"The scopes differ",
            "current_claim":"No network","existing_claim":"Remote sync required",
            "impact":"User choice required","recommendation":"Choose one","evidence":[]
        }]
    }));
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":provider,"model":"test-model"}),
    );
    let queued = harness
        .app
        .enqueue_job(json!({
            "taskKind":"conflict_review","entityType":"features","entityId":id(&solution),"locale":"en"
        }))
        .await;
    assert_eq!(queued.status, 202, "{}", queued.body);
    let result = wait_for_job(&harness, &queued);
    let run_id = result.body["result"]["run_id"].as_str().unwrap();
    let saved = harness.call(
        "workflow",
        "solution.conflict.resolve",
        json!({"runId":run_id,"resolutions":[{"conflict_id":"conflict-1","action":"accept_conflict","rationale":"Offline is intentional"}]}),
    );
    assert_eq!(saved.body["state"], "clear", "{}", saved.body);
    let restored = harness.call("jobs", "jobs.conflict.get", json!({"runId":run_id}));
    assert_eq!(
        restored.body["conflicts"][0]["resolution"]["rationale"],
        "Offline is intentional"
    );
}

#[tokio::test]
async fn given_a_work_image_when_summarized_then_both_languages_are_persisted() {
    let harness = Harness::new();
    let capture = harness.call(
        "workflow",
        "capture.create",
        json!({"text":"Visible result"}),
    );
    let problem = harness.call(
        "workflow",
        "capture.promote",
        json!({"captureId":id(&capture)}),
    );
    harness.call(
        "workflow",
        "problem.approve",
        json!({"problemId":id(&problem)}),
    );
    let solution = harness.call(
        "workflow",
        "solution.create",
        json!({"problemId":id(&problem),"title":"Visual work","outcome":"Document it","validation_criteria":"- [ ] Visible"}),
    );
    let entry = harness.call(
        "workflow",
        "solution.progress.add",
        json!({"solutionId":id(&solution),"body":"Screenshot","image_data":"aW1hZ2U=","image_media_type":"image/png"}),
    );
    let provider = provider_once(json!({
        "ko":{"summary":"완료 상태가 보입니다."},
        "en":{"summary":"The completed state is visible."}
    }));
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":provider,"model":"test-model"}),
    );
    let queued = harness.app.enqueue_job(json!({
        "taskKind":"image_summary","entityType":"solution_progress_entries","entityId":id(&entry),"locale":"ko"
    })).await;
    let result = wait_for_job(&harness, &queued);
    assert_eq!(result.body["result"]["summary"], "완료 상태가 보입니다.");
    let english = harness.call(
        "workflow",
        "solution.progress.get",
        json!({"solutionId":id(&solution),"locale":"en"}),
    );
    assert_eq!(
        english.body["entries"][0]["image_summary"],
        "The completed state is visible."
    );
}

#[tokio::test]
async fn given_workbench_items_when_ai_organizes_then_attention_order_is_persisted() {
    let harness = Harness::new();
    let first = harness.call("workflow", "capture.create", json!({"text":"Later"}));
    let second = harness.call("workflow", "capture.create", json!({"text":"Now"}));
    let provider = provider_once(json!({"entries":[
        {"entity_type":"captures","entity_id":id(&first),"category":"General","attention_rank":10,"rationale":"Can wait"},
        {"entity_type":"captures","entity_id":id(&second),"category":"Product","attention_rank":95,"rationale":"Needs attention"}
    ]}));
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":provider,"model":"test-model"}),
    );
    let queued = harness.app.enqueue_job(json!({
        "taskKind":"workbench_organization","entityType":"workbench","entityId":"active","locale":"en"
    })).await;
    let result = wait_for_job(&harness, &queued);
    assert_eq!(result.body["result"]["organized"], 2);
    let board = harness.call("workflow", "board.get", json!({}));
    assert_eq!(board.body["captures"][0]["id"], id(&second));
    assert_eq!(board.body["captures"][0]["category"], "Product");
    assert_eq!(board.body["captures"][0]["attention_rank"], 95);
    assert_eq!(
        board.body["captures"][0]["attention_rationale"],
        "Needs attention"
    );
}

#[tokio::test]
async fn given_a_running_native_job_when_cancelled_then_provider_work_is_aborted() {
    let harness = Harness::new();
    let capture = harness.call("workflow", "capture.create", json!({"text":"Cancel work"}));
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":slow_provider(),"model":"test-model"}),
    );
    let queued = harness.app.enqueue_job(json!({
        "taskKind":"workflow_refinement","entityType":"captures","entityId":id(&capture),"locale":"en"
    })).await;
    thread::sleep(Duration::from_millis(30));
    let cancelled = harness.call("jobs", "jobs.cancel", json!({"jobId":id(&queued)}));
    assert_eq!(cancelled.status, 200, "{}", cancelled.body);
    assert_eq!(cancelled.body["status"], "cancelled");
    thread::sleep(Duration::from_millis(30));
    let current = harness.call("jobs", "jobs.get", json!({"jobId":id(&queued)}));
    assert_eq!(current.body["status"], "cancelled");
    assert!(current.body["finished_at"].is_string());
}

#[tokio::test]
async fn given_managed_knowledge_when_translated_then_hash_current_korean_is_restored() {
    let harness = Harness::new();
    let relative = "Knowledge/result.md";
    let file = harness._root.path().join("vault").join(relative);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(
        &file,
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Result\n",
    )
    .unwrap();
    let pending = harness.call(
        "vault",
        "knowledge.read",
        json!({"path":relative,"locale":"ko"}),
    );
    assert_eq!(pending.body["cache_status"], "pending");
    let provider = provider_once(json!({"markdown":"# 결과\n\n재사용 가능한 증거."}));
    harness.call(
        "settings",
        "provider.save",
        json!({"base_url":provider,"model":"test-model"}),
    );
    let queued = harness.app.enqueue_job(json!({
        "taskKind":"knowledge_translation","entityType":"knowledge","entityId":relative,"path":relative,"locale":"ko"
    })).await;
    wait_for_job(&harness, &queued);
    let translated = harness.call(
        "vault",
        "knowledge.read",
        json!({"path":relative,"locale":"ko"}),
    );
    assert_eq!(translated.body["cache_status"], "hit");
    assert!(translated.body["markdown"]
        .as_str()
        .unwrap()
        .contains("재사용 가능한 증거"));

    std::fs::write(
        file,
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Changed\n",
    )
    .unwrap();
    let stale = harness.call(
        "vault",
        "knowledge.read",
        json!({"path":relative,"locale":"ko"}),
    );
    assert_eq!(stale.body["cache_status"], "pending");
}
