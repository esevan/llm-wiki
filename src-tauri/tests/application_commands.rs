use llm_wiki_desktop::{ApplicationGateway, ApplicationRequest};
use serde_json::Value;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

struct Backend {
    child: Child,
    _state: TempDir,
    gateway: ApplicationGateway,
}

struct ProcessGuard(Child);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn command(path: &str, method: &str, body: Option<&str>) -> ApplicationRequest {
    ApplicationRequest {
        path: path.into(),
        method: method.into(),
        headers: HashMap::from([
            ("Content-Type".into(), "application/json".into()),
            ("X-LLM-Wiki-Locale".into(), "en".into()),
        ]),
        body: body.map(str::to_owned),
    }
}

fn json(response: &llm_wiki_desktop::ApplicationResponse) -> Value {
    serde_json::from_str(&response.body).expect("JSON application response")
}

async fn execute(
    backend: &Backend,
    path: &str,
    method: &str,
    body: Option<&str>,
) -> llm_wiki_desktop::ApplicationResponse {
    backend
        .gateway
        .request(command(path, method, body))
        .await
        .expect("application command response")
}

async fn wait_for_job(backend: &Backend, job_id: &str) -> Value {
    for _ in 0..800 {
        let response = execute(backend, &format!("/jobs/{job_id}"), "GET", None).await;
        let job = json(&response);
        if matches!(
            job["status"].as_str(),
            Some("completed" | "awaiting_review" | "failed" | "cancelled" | "stale")
        ) {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("application job did not reach a terminal state")
}

async fn wait_for_result(backend: &Backend, job_id: &str) -> Value {
    let job = wait_for_job(backend, job_id).await;
    assert!(
        matches!(
            job["status"].as_str(),
            Some("completed" | "awaiting_review")
        ),
        "job did not complete: {job}"
    );
    let response = execute(backend, &format!("/jobs/{job_id}/result"), "GET", None).await;
    json(&response)["result"].clone()
}

async fn wait_for_job_status(backend: &Backend, job_id: &str, expected: &str) -> Value {
    for _ in 0..800 {
        let response = execute(backend, &format!("/jobs/{job_id}"), "GET", None).await;
        let job = json(&response);
        if job["status"] == expected {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("application job did not reach {expected}")
}

async fn start_backend(workers: bool, test_provider: bool) -> Backend {
    let state = tempfile::tempdir().expect("temporary desktop state");
    let vault = state.path().join("vault");
    std::fs::create_dir(&vault).expect("vault directory");
    std::fs::write(
        vault.join("context.md"),
        "---\nllm_wiki_managed: true\ncanonical_locale: en\n---\n# Existing context\n\nReusable evidence.\n",
    )
    .expect("vault note");
    let db = state.path().join("llm-wiki.sqlite3");
    let listener = TcpListener::bind("127.0.0.1:0").expect("available port");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_owned();
    let executable = root.join(".venv/bin/llm-wiki");
    let mut process = Command::new(executable);
    process.args([
        "desktop-backend",
        "--vault",
        vault.to_str().expect("vault path"),
        "--db",
        db.to_str().expect("database path"),
        "--port",
        &port.to_string(),
    ]);
    if !workers {
        process.arg("--no-workers");
    }
    if test_provider {
        process
            .env("LLM_WIKI_TEST_MODE", "1")
            .env("LLM_WIKI_TEST_API_KEY", "deterministic-test-key")
            .env("LLM_WIKI_TEST_PROVIDER_TIMEOUT", "0.3");
    }
    let mut child = process
        .current_dir(&root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("desktop backend starts");
    let gateway =
        ApplicationGateway::new(format!("http://127.0.0.1:{port}")).expect("loopback gateway");
    for _ in 0..100 {
        if gateway
            .request(command("/health", "GET", None))
            .await
            .is_ok()
        {
            return Backend {
                child,
                _state: state,
                gateway,
            };
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("desktop backend did not become ready")
}

#[tokio::test]
async fn given_a_capture_command_when_executed_then_real_application_state_changes() {
    let backend = start_backend(false, false).await;

    let created = backend
        .gateway
        .request(command(
            "/captures",
            "POST",
            Some(r#"{"text":"Preserve this thought"}"#),
        ))
        .await
        .expect("capture command response");
    let board = backend
        .gateway
        .request(command("/board", "GET", None))
        .await
        .expect("board query response");

    assert_eq!(created.status, 201);
    assert_eq!(board.status, 200);
    assert!(board.body.contains("Preserve this thought"));
}

#[tokio::test]
async fn given_invalid_capture_input_when_executed_then_validation_is_preserved() {
    let backend = start_backend(false, false).await;

    let response = backend
        .gateway
        .request(command("/captures", "POST", Some(r#"{"text":""}"#)))
        .await
        .expect("validation response");

    assert_eq!(response.status, 422);
    assert!(response.body.contains("string_too_short"));
}

#[tokio::test]
async fn given_vault_search_when_executed_then_filesystem_backed_behavior_is_preserved() {
    let backend = start_backend(false, false).await;

    let indexed = backend
        .gateway
        .request(command("/index", "POST", Some("{}")))
        .await
        .expect("index command response");
    let searched = backend
        .gateway
        .request(command("/search?q=Reusable&limit=20", "GET", None))
        .await
        .expect("search query response");

    assert_eq!(indexed.status, 200);
    assert_eq!(searched.status, 200);
    assert!(searched.body.contains("context.md"));
}

#[tokio::test]
async fn given_the_workflow_command_surface_when_executed_then_state_and_side_effects_are_preserved(
) {
    let backend = start_backend(false, false).await;

    let localized = execute(
        &backend,
        "/settings/locale?browser_locale=ko-KR",
        "GET",
        None,
    )
    .await;
    assert_eq!(json(&localized)["locale"], "ko");
    let saved_locale = execute(
        &backend,
        "/settings/locale",
        "PUT",
        Some(r#"{"locale":"en"}"#),
    )
    .await;
    assert_eq!(json(&saved_locale)["explicit"], true);

    let capture = execute(
        &backend,
        "/captures",
        "POST",
        Some(r#"{"text":"Command lifecycle"}"#),
    )
    .await;
    let capture_id = json(&capture)["id"].as_str().unwrap().to_owned();
    let problem = execute(
        &backend,
        &format!("/captures/{capture_id}/promote"),
        "POST",
        Some(r#"{"statement":"Command-owned problem","detail":"Two\nlines"}"#),
    )
    .await;
    let problem_id = json(&problem)["id"].as_str().unwrap().to_owned();
    let board = execute(&backend, "/board", "GET", None).await;
    assert_eq!(json(&board)["captures"].as_array().unwrap().len(), 0);
    assert_eq!(
        json(&board)["problems"][0]["statement"],
        "Command-owned problem"
    );

    let update = execute(
        &backend,
        &format!("/items/problems/{problem_id}"),
        "PUT",
        Some(r#"{"title":"Updated command problem","detail":"Updated detail"}"#),
    )
    .await;
    assert_eq!(update.status, 204);
    let record = execute(
        &backend,
        &format!("/items/problems/{problem_id}"),
        "GET",
        None,
    )
    .await;
    assert_eq!(json(&record)["title"], "Updated command problem");

    assert_eq!(
        execute(
            &backend,
            &format!("/items/problems/{problem_id}"),
            "DELETE",
            None
        )
        .await
        .status,
        204
    );
    assert!(
        json(&execute(&backend, "/board", "GET", None).await)["problems"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        execute(
            &backend,
            &format!("/items/problems/{problem_id}/restore"),
            "POST",
            None,
        )
        .await
        .status,
        204
    );

    assert_eq!(
        execute(
            &backend,
            &format!("/problems/{problem_id}/approve"),
            "POST",
            None
        )
        .await
        .status,
        204
    );
    let projected = execute(
        &backend,
        &format!("/problems/{problem_id}/project"),
        "POST",
        None,
    )
    .await;
    assert_eq!(projected.status, 201);
    let solution = execute(
        &backend,
        &format!("/problems/{problem_id}/features"),
        "POST",
        Some(r#"{"title":"Command solution","outcome":"Behavior is preserved","non_goals":"None","validation_criteria":"- [ ] Verified"}"#),
    )
    .await;
    let solution_id = json(&solution)["id"].as_str().unwrap().to_owned();
    let conflict = execute(
        &backend,
        &format!("/features/{solution_id}/conflict"),
        "PUT",
        Some(r#"{"state":"clear","citation":"Command test"}"#),
    )
    .await;
    assert_eq!(json(&conflict)["state"], "clear");
    assert_eq!(
        execute(
            &backend,
            &format!("/features/{solution_id}/approve"),
            "POST",
            None
        )
        .await
        .status,
        204
    );

    let progress = execute(
        &backend,
        &format!("/features/{solution_id}/progress"),
        "POST",
        Some(r#"{"body":"Command evidence"}"#),
    )
    .await;
    let progress_id = json(&progress)["id"].as_str().unwrap().to_owned();
    assert_eq!(
        execute(
            &backend,
            &format!("/progress/{progress_id}/comments"),
            "POST",
            Some(r#"{"body":"Reviewed by command test"}"#),
        )
        .await
        .status,
        201
    );
    let checklist = execute(
        &backend,
        &format!("/features/{solution_id}/checklist"),
        "POST",
        Some(r#"{"body":"Desktop parity"}"#),
    )
    .await;
    let checklist_id = json(&checklist)["id"].as_str().unwrap().to_owned();
    assert_eq!(
        execute(
            &backend,
            &format!("/checklist/{checklist_id}"),
            "PUT",
            Some(r#"{"body":"Desktop parity","checked":true}"#),
        )
        .await
        .status,
        204
    );
    let restored_progress = execute(
        &backend,
        &format!("/features/{solution_id}/progress"),
        "GET",
        None,
    )
    .await;
    assert!(restored_progress.body.contains("Reviewed by command test"));
    assert!(restored_progress.body.contains("Desktop parity"));

    let queued = execute(&backend, "/workbench/organize", "POST", Some("{}")).await;
    assert_eq!(queued.status, 202);
    let job_id = json(&queued)["id"].as_str().unwrap().to_owned();
    let cancelled = execute(&backend, &format!("/jobs/{job_id}/cancel"), "POST", None).await;
    assert_eq!(json(&cancelled)["status"], "cancelled");
    let jobs = execute(&backend, "/jobs", "GET", None).await;
    assert!(jobs.body.contains(&job_id));

    let knowledge = execute(
        &backend,
        "/knowledge?path=context.md&locale=en&translate=false",
        "GET",
        None,
    )
    .await;
    assert_eq!(knowledge.status, 200);
    assert!(knowledge.body.contains("Reusable evidence"));
    assert_eq!(
        execute(&backend, "/provider/config", "GET", None)
            .await
            .status,
        200
    );
}

fn start_deterministic_provider() -> (ProcessGuard, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("available provider port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root")
        .to_owned();
    let child = Command::new(root.join(".venv/bin/python"))
        .args(["tests/fakes/openai_server.py", "--port", &port.to_string()])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("deterministic provider starts");
    let guard = ProcessGuard(child);
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return (guard, port);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("deterministic provider did not become ready")
}

#[tokio::test]
async fn given_a_deterministic_llm_when_chat_runs_then_streamed_application_behavior_is_preserved()
{
    let (_provider, provider_port) = start_deterministic_provider();
    let backend = start_backend(true, true).await;
    let configured = execute(
        &backend,
        "/provider/config",
        "PUT",
        Some(&format!(
            r#"{{"base_url":"http://127.0.0.1:{provider_port}/v1","model":"deterministic-test-model","async_worker_count":1}}"#
        )),
    )
    .await;
    assert_eq!(configured.status, 200);
    let capture = execute(
        &backend,
        "/captures",
        "POST",
        Some(r#"{"text":"Explore through native command"}"#),
    )
    .await;
    let capture_id = json(&capture)["id"].as_str().unwrap().to_owned();

    let chat = execute(
        &backend,
        &format!("/captures/{capture_id}/chat"),
        "POST",
        Some(r#"{"message":"What should I consider?"}"#),
    )
    .await;

    assert_eq!(chat.status, 200);
    assert!(chat.body.contains("data: Deterministic"));
    assert!(chat.body.contains("data: desktop response"));
    assert!(chat.body.contains("event: done"));
}

#[tokio::test]
async fn given_deterministic_ai_jobs_when_invoked_then_results_notifications_and_recovery_are_preserved(
) {
    let (_provider, provider_port) = start_deterministic_provider();
    let backend = start_backend(true, true).await;
    let provider_url = format!("http://127.0.0.1:{provider_port}/v1");
    let configured = execute(
        &backend,
        "/provider/config",
        "PUT",
        Some(&format!(
            r#"{{"base_url":"{provider_url}","model":"deterministic-test-model","async_worker_count":1}}"#
        )),
    )
    .await;
    assert_eq!(configured.status, 200);

    let capture = execute(
        &backend,
        "/captures",
        "POST",
        Some(r#"{"text":"Turn this into a deterministic workflow"}"#),
    )
    .await;
    let capture_id = json(&capture)["id"].as_str().unwrap().to_owned();
    let draft_job = execute(
        &backend,
        &format!("/captures/{capture_id}/draft"),
        "POST",
        None,
    )
    .await;
    let draft = wait_for_result(&backend, json(&draft_job)["id"].as_str().unwrap()).await;
    assert_eq!(draft["title"], "Clear problem");
    assert_eq!(draft["localized_versions"]["ko"]["title"], "명확한 문제");

    let problem = execute(
        &backend,
        &format!("/captures/{capture_id}/promote"),
        "POST",
        Some(r#"{"statement":"Command AI problem","detail":"Needs refinement"}"#),
    )
    .await;
    let problem_id = json(&problem)["id"].as_str().unwrap().to_owned();
    let refine_job = execute(
        &backend,
        &format!("/problems/{problem_id}/refine"),
        "POST",
        None,
    )
    .await;
    let refinement = wait_for_result(&backend, json(&refine_job)["id"].as_str().unwrap()).await;
    assert_eq!(refinement["title"], "Refined problem");
    assert_eq!(
        refinement["localized_versions"]["ko"]["title"],
        "정제된 문제"
    );

    assert_eq!(
        execute(
            &backend,
            &format!("/problems/{problem_id}/approve"),
            "POST",
            None,
        )
        .await
        .status,
        204
    );
    let solution = execute(
        &backend,
        &format!("/problems/{problem_id}/features"),
        "POST",
        Some(
            r#"{"title":"AI command solution","outcome":"Reusable evidence remains covered","non_goals":"None","validation_criteria":"- [ ] Verified"}"#,
        ),
    )
    .await;
    let solution_id = json(&solution)["id"].as_str().unwrap().to_owned();

    let conflict_job = execute(
        &backend,
        &format!("/features/{solution_id}/conflict-review"),
        "POST",
        None,
    )
    .await;
    let conflict_review =
        wait_for_result(&backend, json(&conflict_job)["id"].as_str().unwrap()).await;
    assert_eq!(conflict_review["feature_id"], solution_id);
    let conflicts = conflict_review["conflicts"].as_array().unwrap();
    assert!(!conflicts.is_empty());
    let run_id = conflict_review["run_id"].as_str().unwrap();
    let invalid_resolution = serde_json::json!({
        "resolutions": [{
            "conflict_id": conflicts[0]["id"],
            "action": "accept_conflict",
            "rationale": ""
        }]
    })
    .to_string();
    assert_eq!(
        execute(
            &backend,
            &format!("/conflict-reviews/{run_id}/resolutions"),
            "PUT",
            Some(&invalid_resolution),
        )
        .await
        .status,
        400
    );
    let resolutions = conflicts
        .iter()
        .map(|conflict| {
            serde_json::json!({
                "conflict_id": conflict["id"],
                "action": "accept_conflict",
                "rationale": "The compatibility exception is intentional."
            })
        })
        .collect::<Vec<_>>();
    let resolution_body = serde_json::json!({"resolutions": resolutions}).to_string();
    let resolved = execute(
        &backend,
        &format!("/conflict-reviews/{run_id}/resolutions"),
        "PUT",
        Some(&resolution_body),
    )
    .await;
    assert_eq!(json(&resolved)["state"], "clear");
    assert_eq!(
        execute(
            &backend,
            &format!("/items/features/{solution_id}"),
            "PUT",
            Some(
                r#"{"title":"Updated AI command solution","detail":"Reusable evidence remains covered"}"#,
            ),
        )
        .await
        .status,
        204
    );
    assert_eq!(
        execute(
            &backend,
            &format!("/conflict-reviews/{run_id}/resolutions"),
            "PUT",
            Some(&resolution_body),
        )
        .await
        .status,
        400
    );
    execute(
        &backend,
        &format!("/features/{solution_id}/conflict"),
        "PUT",
        Some(r#"{"state":"clear","citation":"Deterministic command review"}"#),
    )
    .await;
    assert_eq!(
        execute(
            &backend,
            &format!("/features/{solution_id}/approve"),
            "POST",
            None,
        )
        .await
        .status,
        204
    );

    let authored_progress = execute(
        &backend,
        &format!("/features/{solution_id}/progress"),
        "POST",
        Some(r#"{"body":"Command evidence"}"#),
    )
    .await;
    assert_eq!(authored_progress.status, 201);
    let image_progress = execute(
        &backend,
        &format!("/features/{solution_id}/progress"),
        "POST",
        Some(
            r#"{"body":"Visual evidence","image_data":"aGVsbG8=","image_media_type":"image/png"}"#,
        ),
    )
    .await;
    let image_job = execute(
        &backend,
        &format!(
            "/progress/{}/summarize-image",
            json(&image_progress)["id"].as_str().unwrap()
        ),
        "POST",
        None,
    )
    .await;
    let image_summary = wait_for_result(&backend, json(&image_job)["id"].as_str().unwrap()).await;
    assert_eq!(image_summary["summary"], "Deterministic image summary");
    assert_eq!(
        image_summary["localized_versions"]["ko"]["image_summary"],
        "결정론적 이미지 요약"
    );

    let review_job = execute(
        &backend,
        &format!("/features/{solution_id}/completion-review"),
        "POST",
        None,
    )
    .await;
    let review = wait_for_result(&backend, json(&review_job)["id"].as_str().unwrap()).await;
    assert_eq!(review["report"]["resolution"], "complete");
    assert_eq!(review["report"]["criteria_review"][0]["status"], "met");

    let notifications = execute(&backend, "/notifications?unread_only=true", "GET", None).await;
    let notification = &json(&notifications)["notifications"][0];
    let notification_id = notification["id"].as_str().unwrap();
    assert_eq!(notification["job_id"], json(&review_job)["id"]);
    assert_eq!(
        execute(
            &backend,
            &format!("/notifications/{notification_id}/read"),
            "POST",
            None,
        )
        .await
        .status,
        200
    );
    assert_eq!(
        json(&execute(&backend, "/notifications?unread_only=true", "GET", None).await)
            ["unread_count"],
        0
    );

    let translation_job = execute(
        &backend,
        "/knowledge/translate?path=context.md",
        "GET",
        None,
    )
    .await;
    let translated =
        wait_for_result(&backend, json(&translation_job)["id"].as_str().unwrap()).await;
    assert_eq!(translated["locale"], "ko");
    let korean = execute(
        &backend,
        "/knowledge?path=context.md&locale=ko&translate=false",
        "GET",
        None,
    )
    .await;
    assert_eq!(json(&korean)["cache_status"], "hit");
    assert!(korean.body.contains("기존 맥락"));

    let completed = execute(
        &backend,
        &format!("/problems/{problem_id}/complete"),
        "POST",
        Some(&format!(
            r#"{{"reason":"Deterministic human review","review_id":"{}"}}"#,
            review["review_id"].as_str().unwrap()
        )),
    )
    .await;
    assert_eq!(completed.status, 200);
    let report = wait_for_result(
        &backend,
        json(&completed)["report_job_id"].as_str().unwrap(),
    )
    .await;
    assert!(report["path"].as_str().unwrap().ends_with(".md"));
    let lineage = execute(
        &backend,
        &format!("/features/{solution_id}/lineage"),
        "GET",
        None,
    )
    .await;
    let stages = json(&lineage)["lineage"]["stages"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(stages.len(), 4);
    assert_eq!(stages[0]["kind"], "capture");
    assert_eq!(stages[3]["kind"], "complete");
    let follow_up = execute(
        &backend,
        &format!("/features/{solution_id}/follow-up-problem"),
        "POST",
        None,
    )
    .await;
    assert_eq!(follow_up.status, 201);

    let patch = execute(
        &backend,
        &format!("/features/{solution_id}/patches"),
        "POST",
        Some(
            r#"{"path":"context.md","operation":"insert_after_heading","heading":"Existing context","content":"Command-reviewed addition."}"#,
        ),
    )
    .await;
    assert_eq!(patch.status, 201);
    let context_path = backend._state.path().join("vault/context.md");
    let context = std::fs::read_to_string(&context_path).unwrap();
    std::fs::write(&context_path, format!("{context}\nExternal edit.\n")).unwrap();
    let conflicted_patch = execute(
        &backend,
        &format!("/patches/{}/apply", json(&patch)["id"].as_str().unwrap()),
        "POST",
        None,
    )
    .await;
    assert_eq!(conflicted_patch.status, 409);

    let completion_path = backend
        ._state
        .path()
        .join("vault")
        .join(json(&completed)["path"].as_str().unwrap());
    let document = std::fs::read_to_string(&completion_path).unwrap();
    std::fs::write(&completion_path, format!("{document}\nExternal edit.\n")).unwrap();
    let conflicted_regeneration = execute(
        &backend,
        &format!("/problems/{problem_id}/completion-playbook/regenerate"),
        "POST",
        None,
    )
    .await;
    assert_eq!(conflicted_regeneration.status, 409);

    let failed_config = execute(
        &backend,
        "/provider/config",
        "PUT",
        Some(&format!(
            r#"{{"base_url":"{provider_url}","model":"deterministic-failure","async_worker_count":1}}"#
        )),
    )
    .await;
    assert_eq!(failed_config.status, 200);
    let failing_job = execute(
        &backend,
        &format!("/problems/{problem_id}/draft"),
        "POST",
        None,
    )
    .await;
    let failing_job_id = json(&failing_job)["id"].as_str().unwrap().to_owned();
    assert_eq!(
        wait_for_job(&backend, &failing_job_id).await["status"],
        "failed"
    );

    execute(
        &backend,
        "/provider/config",
        "PUT",
        Some(&format!(
            r#"{{"base_url":"{provider_url}","model":"deterministic-test-model","async_worker_count":1}}"#
        )),
    )
    .await;
    let retried = execute(
        &backend,
        &format!("/jobs/{failing_job_id}/retry"),
        "POST",
        None,
    )
    .await;
    assert_eq!(retried.status, 200);
    let retry_result = wait_for_result(&backend, &failing_job_id).await;
    assert_eq!(retry_result["title"], "Deterministic solution");

    execute(
        &backend,
        "/provider/config",
        "PUT",
        Some(&format!(
            r#"{{"base_url":"{provider_url}","model":"deterministic-timeout","async_worker_count":1}}"#
        )),
    )
    .await;
    let timeout_capture = execute(
        &backend,
        "/captures",
        "POST",
        Some(r#"{"text":"Exercise provider timeout mapping"}"#),
    )
    .await;
    let timeout_job = execute(
        &backend,
        &format!(
            "/captures/{}/draft",
            json(&timeout_capture)["id"].as_str().unwrap()
        ),
        "POST",
        None,
    )
    .await;
    let timeout_job_id = json(&timeout_job)["id"].as_str().unwrap().to_owned();
    let timed_out = wait_for_job_status(&backend, &timeout_job_id, "retryable").await;
    assert_eq!(timed_out["error"]["code"], "transient");
    let cancelled = execute(
        &backend,
        &format!("/jobs/{timeout_job_id}/cancel"),
        "POST",
        None,
    )
    .await;
    assert!(matches!(
        json(&cancelled)["status"].as_str(),
        Some("cancelling" | "cancelled")
    ));

    let jobs = json(&execute(&backend, "/jobs", "GET", None).await)["jobs"]
        .as_array()
        .unwrap()
        .clone();
    assert!(jobs.iter().any(|job| {
        job["task_kind"] == "derived_translation"
            && matches!(
                job["status"].as_str(),
                Some("completed" | "awaiting_review")
            )
    }));
}
