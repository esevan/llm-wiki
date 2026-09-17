//! Native application-boundary journeys for the schema-8 Task workbench.
use llm_wiki_desktop::{NativeApplication, NativeOperation, NativeResponse};
use serde_json::{json, Value};
use std::time::Instant;
use tempfile::TempDir;

struct Harness {
    root: TempDir,
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
        Self { root, app }
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
    fn task(&self, title: &str) -> NativeResponse {
        self.call(
            "workflow",
            "task.create",
            json!({"operationId":format!("task-{title}"),"inputText":title,"title":title}),
        )
    }
}
fn id(r: &NativeResponse) -> String {
    r.body["id"].as_str().unwrap().into()
}
fn ok(r: &NativeResponse) {
    assert!((200..300).contains(&r.status), "{}", r.body);
}
fn get(h: &Harness, id: &str) -> NativeResponse {
    h.call("workflow", "task.get", json!({"taskId":id}))
}

#[test]
fn capture_is_canonical_workbench_item() {
    let h = Harness::new();
    ok(&h.call(
        "workflow",
        "capture.create",
        json!({"operationId":"capture","text":"Native thought"}),
    ));
    assert!(h
        .call("workflow", "workbench.get", json!({}))
        .body
        .to_string()
        .contains("Native thought"));
}
#[test]
fn direct_task_is_persisted() {
    let h = Harness::new();
    let t = h.task("Independent");
    ok(&t);
    let aggregate = get(&h, &id(&t));
    assert_eq!(aggregate.body["state"], "task");
    assert_eq!(
        aggregate.body["readinessEntries"].as_array().unwrap().len(),
        4
    );
    assert_eq!(aggregate.body["readinessEntries"][0]["key"], "outcome");
}

#[test]
fn reachable_queued_solution_conflict_decision_persists_without_restoring_other_retired_writes() {
    let h = Harness::new();
    let connection = rusqlite::Connection::open(h.root.path().join("state.sqlite3")).unwrap();
    connection.execute_batch(
        "INSERT INTO captures(id,text) VALUES('queued-capture','Queued review source');
         INSERT INTO problems(id,capture_id,statement,detail,state,current_revision) VALUES('queued-problem','queued-capture','Queued review Problem','','open',1);
         INSERT INTO features(id,problem_id,title,outcome,conflict_state,state) VALUES('queued-solution','queued-problem','Queued review Solution','Review the persisted result','conflicted','proposed');",
    ).unwrap();
    ok(&h.call(
        "workflow",
        "solution.conflict.save",
        json!({"solutionId":"queued-solution","state":"clear","citation":"Reviewed from the persisted queue result"}),
    ));
    let saved = h.call(
        "workflow",
        "item.get",
        json!({"entityType":"features","entityId":"queued-solution","locale":"en"}),
    );
    ok(&saved);
    assert_eq!(saved.body["conflict_state"], "clear");

    let retired = h.call(
        "workflow",
        "problem.approve",
        json!({"problemId":"queued-problem"}),
    );
    assert!(retired.status >= 400);
    assert!(retired.body.to_string().contains("not implemented"));
}
#[test]
fn stale_task_revision_is_rejected() {
    let h = Harness::new();
    let t = h.task("Revision");
    let x = id(&t);
    ok(&h.call(
        "workflow",
        "task.revision",
        json!({"operationId":"r1","taskId":x,"expectedTaskRevision":1,"patch":{"title":"new"}}),
    ));
    let stale = h.call(
        "workflow",
        "task.revision",
        json!({"operationId":"r2","taskId":x,"expectedTaskRevision":1,"patch":{"title":"old"}}),
    );
    assert_eq!(stale.status, 409);
}
#[test]
fn work_log_attachment_comment_checklist_and_decision_survive_readback() {
    let h = Harness::new();
    let t = h.task("Evidence");
    let x = id(&t);
    let log=h.call("workflow","task.work-log.create",json!({"operationId":"log","taskId":x,"expectedTaskRevision":1,"body":"Implemented","attachment":{"name":"evidence.txt","mediaType":"text/plain","data":"ZQ=="}}));
    ok(&log);
    ok(&h.call(
        "workflow",
        "work-log.comment.create",
        json!({"operationId":"comment","entryId":log.body["id"],"body":"Reviewed"}),
    ));
    ok(&h.call(
        "workflow",
        "task.checklist.create",
        json!({"operationId":"check","taskId":x,"expectedTaskRevision":1,"body":"Verified"}),
    ));
    ok(&h.call("workflow","task.decision.create",json!({"operationId":"decision","taskId":x,"expectedTaskRevision":1,"kind":"user","payload":{"body":"Ship"}})));
    let logs = h
        .call("workflow", "task.work-log.get", json!({"taskId":x}))
        .body
        .to_string();
    assert!(logs.contains("Implemented"));
}
#[test]
fn completion_problem_resolution_and_knowledge_are_explicit_separate_decisions() {
    let h = Harness::new();
    let t = h.task("Complete");
    let x = id(&t);
    let p = h.call(
        "workflow",
        "problem.create",
        json!({"operationId":"p","statement":"Open"}),
    );
    ok(&p);
    ok(&h.call("workflow","task.problem-link.create",json!({"operationId":"link","taskId":x,"expectedTaskRevision":1,"problemId":p.body["id"],"problemRevision":1})));
    ok(&h.call(
        "workflow",
        "task.transition",
        json!({"operationId":"start","taskId":x,"expectedTaskRevision":1,"to":"in_progress"}),
    ));
    ok(&h.call("workflow","task.completion.create",json!({"operationId":"complete","taskId":x,"expectedTaskRevision":1,"evidence":"Tests passed"})));
    assert_eq!(get(&h, &x).body["state"], "completed");
    let board = h.call("workflow", "workbench.get", json!({}));
    ok(&board);
    let completed = board.body["categories"].as_array().unwrap().iter()
        .flat_map(|category| category["items"].as_array().unwrap())
        .find(|item| item["id"] == x).unwrap();
    assert!(completed["completedAt"].as_str().is_some_and(|at| !at.is_empty()));

    ok(&h.call("workflow","problem.resolution.create",json!({"operationId":"resolve","problemId":p.body["id"],"expectedProblemRevision":1,"rationale":"explicit"})));
}
#[test]
fn exact_problem_revision_and_relationship_cycle_are_enforced() {
    let h = Harness::new();
    let a = h.task("A");
    let b = h.task("B");
    let aid = id(&a);
    let bid = id(&b);
    let p = h.call(
        "workflow",
        "problem.create",
        json!({"operationId":"p1","statement":"Original"}),
    );
    ok(&p);
    ok(&h.call(
        "workflow",
        "problem.revision",
        json!({"operationId":"p2","problemId":p.body["id"],"statement":"Changed"}),
    ));
    let linked=h.call("workflow","task.problem-link.create",json!({"operationId":"link","taskId":aid,"expectedTaskRevision":1,"problemId":p.body["id"],"problemRevision":1}));
    ok(&linked);
    assert_eq!(linked.body["problemRevision"], 1);
    ok(&h.call("workflow","task.relationship.create",json!({"operationId":"ab","taskId":aid,"expectedTaskRevision":1,"targetTaskId":bid,"kind":"prerequisite"})));
    assert_eq!(h.call("workflow","task.relationship.create",json!({"operationId":"ba","taskId":bid,"expectedTaskRevision":1,"targetTaskId":aid,"kind":"prerequisite"})).status,400);
    ok(&h.call("workflow", "task.relationship.create", json!({"operationId":"related-both","taskId":aid,"expectedTaskRevision":1,"targetTaskId":bid,"kind":"related"})));
    for (source, target) in [(&aid, &bid), (&bid, &aid)] {
        assert!(get(&h, source).body["relationships"]
            .as_array()
            .unwrap()
            .iter()
            .any(|link| link["kind"] == "related" && link["targetTaskId"] == *target));
    }
    let related_id = get(&h, &bid).body["relationships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["kind"] == "related")
        .unwrap()["id"]
        .clone();
    ok(&h.call(
        "workflow",
        "task.relationship.delete",
        json!({"operationId":"unlink-related","taskId":bid,"relationshipId":related_id}),
    ));
    for endpoint in [&aid, &bid] {
        assert!(get(&h, endpoint).body["relationships"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] != "related"));
    }
    assert!(get(&h, &aid).body["relationships"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link["kind"] == "prerequisite" && link["targetTaskId"] == bid));
}
#[tokio::test]
async fn refinement_workspace_restores_exact_draft_and_tab() {
    let h = Harness::new();
    let t = h.task("Refine");
    let x = id(&t);
    let s = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-refinement.open".into(),
            input: json!({"operationId":"open","taskId":x}),
        })
        .await;
    ok(&s);
    let saved=h.app.execute_workflow(NativeOperation{name:"task-refinement.workspace".into(),input:json!({"operationId":"save","sessionId":s.body["id"],"inputDraft":"unsent","activeTab":"proposals","scrollAnchor":"row-2","baseDraftRevision":0})}).await;
    ok(&saved);
    let restored = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-refinement.get".into(),
            input: json!({"taskId":x}),
        })
        .await;
    assert_eq!(restored.body["inputDraft"], "unsent");
    assert_eq!(restored.body["activeTab"], "proposals");
    let capture = h.call(
        "workflow",
        "capture.create",
        json!({"operationId":"capture-refining","text":"Capture shortcut"}),
    );
    ok(&capture);
    let capture_id = id(&capture);
    let capture_session = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-refinement.open".into(),
            input: json!({"operationId":"open-capture-refining","captureId":capture_id}),
        })
        .await;
    ok(&capture_session);
    let board = h.call("workflow", "workbench.get", json!({}));
    ok(&board);
    assert!(board.body["refiningShortcuts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "capture" && item["id"] == capture_id));
    ok(&h.call("workflow", "task.transition", json!({"operationId":"start-refining","taskId":x,"expectedTaskRevision":1,"to":"in_progress"})));
    let started = h.call("workflow", "workbench.get", json!({}));
    assert!(started.body["refiningShortcuts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["id"] != x));
}
#[tokio::test]
async fn explicit_knowledge_publish_and_external_edit_guard_work() {
    let h = Harness::new();
    let t = h.task("Knowledge");
    let x = id(&t);
    ok(&h.call(
        "workflow",
        "task.transition",
        json!({"operationId":"start","taskId":x,"expectedTaskRevision":1,"to":"in_progress"}),
    ));
    ok(&h.call(
        "workflow",
        "task.completion.create",
        json!({"operationId":"complete","taskId":x,"expectedTaskRevision":1,"evidence":"verified"}),
    ));
    let d = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-knowledge.draft".into(),
            input: json!({"operationId":"draft","taskId":x,"expectedTaskRevision":1}),
        })
        .await;
    ok(&d);
    let corrected = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-knowledge.correction".into(),
            input: json!({"operationId":"correct","taskId":x,"draftRevision":d.body["draftRevision"],"expectedContentHash":d.body["contentHash"],"expectedSourceHash":d.body["sourceHash"],"bodyMarkdown":"# Corrected Knowledge"}),
        })
        .await;
    ok(&corrected);
    let stale = h
        .app
        .execute_workflow(NativeOperation {
            name: "task-knowledge.correction".into(),
            input: json!({"operationId":"stale-correct","taskId":x,"draftRevision":d.body["draftRevision"],"expectedContentHash":d.body["contentHash"],"expectedSourceHash":d.body["sourceHash"],"bodyMarkdown":"# Stale"}),
        })
        .await;
    assert_eq!(stale.status, 409);
    let published=h.app.execute_workflow(NativeOperation{name:"task-knowledge.publish".into(),input:json!({"operationId":"publish","taskId":x,"draftRevision":d.body["draftRevision"],"expectedContentHash":corrected.body["contentHash"],"expectedSourceHash":corrected.body["sourceHash"]})}).await;
    ok(&published);
    let aggregate = h.call("workflow", "task.get", json!({"taskId":x}));
    assert_eq!(aggregate.body["publication"]["state"], "published");
    assert_eq!(
        aggregate.body["publication"]["contentHash"],
        corrected.body["contentHash"]
    );
    let regenerated=h.app.execute_workflow(NativeOperation{name:"task-knowledge.regenerate".into(),input:json!({"operationId":"regenerate","taskId":x,"draftRevision":d.body["draftRevision"],"expectedTaskRevision":1})}).await;
    ok(&regenerated);
    let republished=h.app.execute_workflow(NativeOperation{name:"task-knowledge.publish".into(),input:json!({"operationId":"republish","taskId":x,"draftRevision":regenerated.body["draftRevision"],"expectedContentHash":regenerated.body["contentHash"],"expectedSourceHash":regenerated.body["sourceHash"]})}).await;
    ok(&republished);
    let withdrawn_regenerated=h.app.execute_workflow(NativeOperation{name:"task-knowledge.withdraw".into(),input:json!({"operationId":"withdraw-regenerated","taskId":x,"draftRevision":regenerated.body["draftRevision"],"expectedContentHash":regenerated.body["contentHash"],"expectedSourceHash":regenerated.body["sourceHash"]})}).await;
    ok(&withdrawn_regenerated);
    assert_eq!(withdrawn_regenerated.body["state"], "withdrawn");
    let path = h
        .root
        .path()
        .join("vault")
        .join(republished.body["path"].as_str().unwrap());
    std::fs::write(path, "external").unwrap();
    let withdrawn=h.app.execute_workflow(NativeOperation{name:"task-knowledge.withdraw".into(),input:json!({"operationId":"withdraw","taskId":x,"draftRevision":d.body["draftRevision"],"expectedContentHash":corrected.body["contentHash"],"expectedSourceHash":corrected.body["sourceHash"]})}).await;
    assert_eq!(withdrawn.status, 409);
}
#[test]
fn vault_search_read_and_domain_boundary_remain_safe() {
    let h = Harness::new();
    std::fs::write(h.root.path().join("vault/native.md"), "No sidecar socket").unwrap();
    ok(&h.call("vault", "vault.index", json!({})));
    assert!(h
        .call(
            "vault",
            "vault.search",
            json!({"query":"sidecar","limit":20})
        )
        .body
        .to_string()
        .contains("native.md"));
    assert_eq!(
        h.call("vault", "knowledge.read", json!({"path":"../secret"}))
            .status,
        404
    );
    assert_eq!(h.call("vault", "task.create", json!({})).status, 400);
}
#[test]
fn locale_and_provider_secret_are_not_exposed() {
    let h = Harness::new();
    ok(&h.call("settings", "locale.save", json!({"locale":"ko"})));
    let p = h.call(
        "settings",
        "provider.save",
        json!({"base_url":"https://api.example.test/v1","model":"model","api_key":"secret"}),
    );
    ok(&p);
    assert!(p.body.get("api_key").is_none());
}

#[test]
fn reference_fixture_capture_and_workbench_p95_stay_within_local_budgets() {
    let h = Harness::new();
    let mut task_ids = Vec::new();
    // The warm projection contains canonical Captures and Tasks; linked Problems and
    // Work Logs make the fixture representative of an active local workbench.
    for index in 0..800 {
        ok(&h.call(
            "workflow",
            "capture.create",
            json!({"operationId":format!("fixture-capture-{index}"),"text":format!("reference capture {index}")}),
        ));
    }
    for index in 0..200 {
        let response = h.call(
            "workflow",
            "task.create",
            json!({"operationId":format!("fixture-task-{index}"),"inputText":format!("reference task {index}"),"title":format!("reference task {index}")}),
        );
        ok(&response);
        task_ids.push(id(&response));
    }
    for (index, task_id) in task_ids.iter().take(20).enumerate() {
        let problem = h.call(
            "workflow",
            "problem.create",
            json!({"operationId":format!("fixture-problem-{index}"),"statement":format!("reference problem {index}")}),
        );
        ok(&problem);
        ok(&h.call(
            "workflow",
            "task.problem-link.create",
            json!({"operationId":format!("fixture-link-{index}"),"taskId":task_id,"expectedTaskRevision":1,"problemId":problem.body["id"],"problemRevision":1}),
        ));
        let revision = get(&h, task_id).body["taskRevision"].as_u64().unwrap();
        ok(&h.call(
            "workflow",
            "task.work-log.create",
            json!({"operationId":format!("fixture-worklog-{index}"),"taskId":task_id,"expectedTaskRevision":revision,"body":format!("reference work log {index}")}),
        ));
    }
    let mut capture_ms = Vec::new();
    for index in 0..100 {
        let started = Instant::now();
        let response = h.call(
            "workflow",
            "capture.create",
            json!({"operationId":format!("bench-capture-{index}"),"text":format!("reference capture {index}")}),
        );
        ok(&response);
        capture_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    let mut task_ms = Vec::new();
    for index in 0..100 {
        let started = Instant::now();
        let response = h.call(
            "workflow",
            "task.create",
            json!({"operationId":format!("bench-task-{index}"),"inputText":format!("benchmark task {index}"),"title":format!("benchmark task {index}")}),
        );
        ok(&response);
        task_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    let mut workbench_ms = Vec::new();
    for _ in 0..100 {
        let started = Instant::now();
        ok(&h.call("workflow", "workbench.get", json!({})));
        workbench_ms.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    capture_ms.sort_by(f64::total_cmp);
    task_ms.sort_by(f64::total_cmp);
    workbench_ms.sort_by(f64::total_cmp);
    let capture_p95 = capture_ms[94];
    let task_p95 = task_ms[94];
    let workbench_p95 = workbench_ms[94];
    eprintln!(
        "reference fixture: machine={}-{} fixture={{captures:900,tasks:300,problems:20,workLogs:20}} samples={{capture:100,task:100,workbench:100}} p95={{capture:{capture_p95:.3}ms,task:{task_p95:.3}ms,workbench:{workbench_p95:.3}ms}}",
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
    assert!(
        capture_p95 < 50.0,
        "capture p95 exceeded 50ms: {capture_p95:.3}ms"
    );
    assert!(task_p95 < 50.0, "Task p95 exceeded 50ms: {task_p95:.3}ms");
    assert!(
        workbench_p95 < 100.0,
        "workbench p95 exceeded 100ms: {workbench_p95:.3}ms"
    );
}

#[tokio::test]
async fn image_work_log_save_queues_bilingual_summary_once_even_when_replayed() {
    let h = Harness::new();
    let created = h.call("workflow", "task.create", json!({"operationId":"image-task","title":"Screenshot","inputText":"Screenshot"}));
    ok(&created);
    let input = json!({"operationId":"save-image","taskId":created.body["id"],"expectedTaskRevision":1,"locale":"ko","attachment":{"name":"screen.png","mediaType":"image/png","data":"aGVsbG8="}});
    let saved = h.app.execute_workflow(NativeOperation { name:"task.work-log.create".into(), input:input.clone() }).await;
    ok(&saved);
    let replay = h.app.execute_workflow(NativeOperation { name:"task.work-log.create".into(), input }).await;
    ok(&replay);
    assert_eq!(saved.body["id"], replay.body["id"]);
    let connection = rusqlite::Connection::open(h.root.path().join("state.sqlite3")).unwrap();
    let count: i64 = connection.query_row("SELECT count(*) FROM ai_jobs_v2 WHERE task_kind='image_summary' AND entity_type='task_work_log_entries' AND entity_id=?", [saved.body["id"].as_str().unwrap()], |r| r.get(0)).unwrap();
    assert_eq!(count, 1);
    let data = get(&h, created.body["id"].as_str().unwrap());
    assert!(data.body["workLog"][0]["imageSummaryJob"]["id"].is_string());
}
