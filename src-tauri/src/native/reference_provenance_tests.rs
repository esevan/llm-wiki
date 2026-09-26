use super::*;
use crate::application::task_service::TaskApplicationService;
use crate::native::{
    database, knowledge_archive, knowledge_distillation, semantic::SemanticEngine, settings,
    task_assistance, vault,
};
use std::{
    io::{BufRead, Read, Write},
    net::TcpListener,
    path::Path,
    time::Duration,
};

fn temporary() -> tempfile::TempDir {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".tmp");
    std::fs::create_dir_all(&parent).unwrap();
    tempfile::tempdir_in(parent).unwrap()
}
fn preview_fields() -> Value {
    json!({"description":"Review approval conditions","background":"External requests need a separate approval.",
        "goal":"Prepare the reviewed contract","scope":"Contract approval","nonGoals":"Deployment",
        "constraints":["Internal approval is required."],"completionCriteria":["Recorded review"],
        "initialApproach":["Read the policy"],"assumptions":[]})
}
fn preview_provider(settings_path: &Path, responses: Vec<Value>) -> std::thread::JoinHandle<()> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    settings::save_provider(settings_path,&json!({"base_url":format!("http://{}",listener.local_addr().unwrap()),"model":"fixture","api_key":"fixture"})).unwrap();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buffer = [0; 8192];
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..end]);
                    let length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body =
                json!({"choices":[{"message":{"content":response.to_string()}}]}).to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
    })
}
struct Provider(std::process::Child);
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn knowledge_provider(settings_path: &Path) -> Provider {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fakes/openai_server.mjs");
    let mut child = Provider(
        std::process::Command::new("node")
            .arg(script)
            .args(["--port", "0"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut port = String::new();
    std::io::BufReader::new(child.0.stdout.take().unwrap())
        .read_line(&mut port)
        .unwrap();
    let port: u16 = port.trim().parse().unwrap();
    settings::save_provider(settings_path,&json!({"base_url":format!("http://127.0.0.1:{port}/v1"),"model":"fixture","api_key":"fixture"})).unwrap();
    child
}
fn binding(c: &Connection, id: &str) -> Value {
    c.query_row("SELECT document_id,source_revision,section,path,text FROM vault_search_units WHERE document_id=? AND aspect='content' ORDER BY rowid LIMIT 1",[id],|row|Ok(json!({
        "documentId":row.get::<_,String>(0)?,"documentVersion":row.get::<_,String>(1)?,"section":row.get::<_,String>(2)?,"path":row.get::<_,String>(3)?,"excerpt":row.get::<_,String>(4)?
    }))).unwrap()
}
async fn refinement(db: &Path, settings: &Path, root: &Path, name: &str, input: Value) -> Value {
    task_assistance::execute(db, settings, root, SemanticEngine::new(None), name, &input)
        .await
        .unwrap()
}

#[tokio::test]
async fn capture_adoption_exports_exact_scoped_use_through_knowledge_and_archive() {
    let root = temporary();
    let db = root.path().join("state.sqlite");
    let settings_path = root.path().join("settings.json");
    let wiki = root.path().join("wiki");
    std::fs::create_dir(&wiki).unwrap();
    database::initialize(&db).unwrap();
    for id in ["support", "counter", "viewed", "candidate", "excluded"] {
        std::fs::write(wiki.join(format!("{id}.md")),format!("---\ntitle: {id} approval\nllm_wiki:\n  schema: 1\n  document_id: {id}\n  information_type: knowledge\n  status: current\n---\n# Approval\n\napproval conditions for {id}; external requests need separate approval.\n")).unwrap();
    }
    vault::index(&db, &wiki, &SemanticEngine::new(None), false, false).unwrap();
    let mut c = database::open(&db).unwrap();
    c.execute(
        "INSERT INTO captures(id,text) VALUES('capture','Review the approval policy')",
        [],
    )
    .unwrap();
    let session = refinement(
        &db,
        &settings_path,
        &wiki,
        "task-refinement.open",
        json!({"operationId":"open","captureId":"capture"}),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let support = binding(&c, "support");
    let counter = binding(&c, "counter");
    let excluded = binding(&c, "excluded");
    let viewed = binding(&c, "viewed");
    let no_search = json!({"needed":false,"reason":"No further investigation","queries":[],"aspects":[],"filters":{},"requery":null});
    let claim = |reference: &Value, field: &str, role: &str| json!({"claimId":format!("{field}:approval"),"documentId":reference["documentId"],"documentVersion":reference["documentVersion"],"section":reference["section"],"role":role});
    let server = preview_provider(
        &settings_path,
        vec![
            json!({"retrieval":{"needed":true,"reason":"Read the approval evidence","queries":["approval"],"aspects":["content"],"filters":{},"requery":null},"preliminaryAssumptions":[]}),
            json!({"preview":preview_fields(),"claimSources":[claim(&support,"constraints","constraint"),claim(&counter,"background","counterevidence"),claim(&excluded,"goal","support")],"optionalInvestigation":no_search}),
        ],
    );
    refinement(&db,&settings_path,&wiki,"task-refinement.reference-generate",json!({"operationId":"generate","sessionId":session,"expectedContextRevision":"refinement:0","locale":"en"})).await;
    let mut workspace = Value::Null;
    for _ in 0..500 {
        workspace = refinement(
            &db,
            &settings_path,
            &wiki,
            "task-refinement.reference-workspace",
            json!({"sessionId":session}),
        )
        .await;
        if workspace["preview"]["currentVersion"] == 1
            || workspace["generation"]["status"] == "failed"
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(workspace["preview"]["currentVersion"], 1, "{workspace}");
    server.join().unwrap();
    let preview = workspace["preview"]["id"].as_str().unwrap();
    assert_eq!(
        for_task(&c, "capture").unwrap(),
        json!([]).as_array().unwrap().clone()
    );
    let mut open = viewed.clone();
    open["sessionId"] = json!(session);
    open["version"] = json!(1);
    refinement(
        &db,
        &settings_path,
        &wiki,
        "task-refinement.reference-open",
        open,
    )
    .await;
    let mut exclude = excluded.clone();
    exclude["sessionId"] = json!(session);
    exclude["kind"] = json!("excluded");
    exclude["reason"] = json!("Not applicable to this work");
    refinement(
        &db,
        &settings_path,
        &wiki,
        "task-refinement.reference-usage",
        exclude,
    )
    .await;
    let applied=refinement(&db,&settings_path,&wiki,"task-refinement.reference-apply",json!({"sessionId":session,"operationId":"apply","version":1,"expectedCurrentPreviewVersion":1,"expectedContentHash":workspace["preview"]["current"]["contentHash"]})).await;
    let task = applied["task"]["id"].as_str().unwrap();
    let revision = applied["task"]["taskRevision"].as_i64().unwrap();
    assert_eq!(
        c.query_row(
            "SELECT subject_type FROM work_previews WHERE id=?",
            [preview],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "capture"
    );
    let uses = for_task(&c, task).unwrap();
    assert_eq!(uses.len(), 2, "{uses:?}");
    let counter_use = uses
        .iter()
        .find(|item| item["documentId"] == "counter")
        .unwrap();
    assert_eq!(counter_use["disposition"], "counterevidence");
    assert_eq!(
        counter_use["reason"],
        "External requests need a separate approval."
    );
    assert_eq!(counter_use["documentVersion"], counter["documentVersion"]);
    assert_eq!(counter_use["section"], counter["section"]);
    assert!(!uses.iter().any(|item| ["viewed", "candidate", "excluded"]
        .iter()
        .any(|id| item["documentId"] == *id)));
    // A Task retaining the same origin, even with a rejected decision, owns none
    // of this preview's references. There is no origin-based export fallback.
    let service = TaskApplicationService::new(&db);
    let other=service.execute("task.create",&json!({"operationId":"other-task","title":"Unrelated work","inputText":"Unrelated work"})).unwrap();
    let other_id = other["id"].as_str().unwrap();
    c.execute(
        "UPDATE tasks SET origin_capture_id='capture' WHERE id=?",
        [other_id],
    )
    .unwrap();
    c.execute("INSERT INTO refinement_proposal_decisions(id,session_id,draft_revision,proposal_id,decision,result_json,operation_id) VALUES('rejected',?,0,'other-proposal','reject',?,'reject-other')",params![session,other.to_string()]).unwrap();
    assert!(for_task(&c, other_id).unwrap().is_empty());
    service.execute("task.transition",&json!({"operationId":"start","taskId":task,"expectedTaskRevision":revision,"to":"in_progress"})).unwrap();
    service.execute("task.work-log.create",&json!({"operationId":"log","taskId":task,"expectedTaskRevision":revision,"body":"Reviewed the internal and external approval conditions."})).unwrap();
    service.execute("task.completion.create",&json!({"operationId":"complete","taskId":task,"expectedTaskRevision":revision,"evidence":"Review recorded","report":"Approval conditions remain explicit"})).unwrap();
    let tx = c.transaction().unwrap();
    let snapshot = knowledge_distillation::capture_snapshot_tx(&tx, task, revision, "en").unwrap();
    tx.commit().unwrap();
    assert_eq!(snapshot["actualReferenceUsages"], json!(uses));
    let _provider = knowledge_provider(&settings_path);
    let generated = task_assistance::prepare_knowledge_draft(
        &db,
        &settings_path,
        &json!({"taskId":task,"expectedTaskRevision":revision,"locale":"en"}),
    )
    .await
    .unwrap();
    let tx = c.transaction().unwrap();
    let saved = knowledge_distillation::append_generated_tx(&tx, &generated).unwrap();
    tx.commit().unwrap();
    let request = json!({"operationId":"archive","taskId":task,"knowledgeRevision":1,"expectedKnowledgeContentHash":saved["contentHash"],"expectedGenerationSnapshotHash":saved["sourceHash"],"selectedIdeaRevisionIds":[],"locale":"en"});
    let proposal = knowledge_archive::prepare(
        &db,
        &settings_path,
        &wiki,
        &SemanticEngine::new(None),
        &request,
    )
    .await
    .unwrap();
    assert_eq!(proposal["referenceLinks"].as_array().unwrap().len(), 2);
    let link = proposal["referenceLinks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["documentId"] == "counter")
        .unwrap();
    assert_eq!(link["role"], "counterevidence");
    assert_eq!(link["rationale"], counter_use["reason"]);
    assert_eq!(link["documentVersion"], counter["documentVersion"]);
    assert!(proposal["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact["bytes"]
            .as_str()
            .is_some_and(|body| body.contains("[[counter#Approval]]"))));
    assert!(!wiki.join("Knowledge/deterministic.md").exists());
    // A later explicit exclusion changes the exact generation snapshot and
    // blocks both an old append and a newly requested archive preparation.
    let mut later = counter.clone();
    later["sessionId"] = json!(session);
    later["kind"] = json!("excluded");
    later["reason"] = json!("Reconsidered after review");
    refinement(
        &db,
        &settings_path,
        &wiki,
        "task-refinement.reference-usage",
        later,
    )
    .await;
    let tx = c.transaction().unwrap();
    assert!(knowledge_distillation::append_generated_tx(&tx, &generated)
        .unwrap_err()
        .contains("source_snapshot_stale"));
    drop(tx);
    let mut stale = request;
    stale["operationId"] = json!("archive-stale");
    assert!(knowledge_archive::prepare(
        &db,
        &settings_path,
        &wiki,
        &SemanticEngine::new(None),
        &stale
    )
    .await
    .unwrap_err()
    .contains("generation_snapshot_conflict"));
}

#[test]
fn task_owned_legacy_roles_resolve_from_exact_claims_and_candidates_never_export() {
    let root = temporary();
    let db = root.path().join("db");
    database::initialize(&db).unwrap();
    let c = database::open(&db).unwrap();
    c.execute_batch("INSERT INTO captures(id,text) VALUES('capture','recorded');
      INSERT INTO refinement_sessions(id,capture_id) VALUES('session','capture');
      INSERT INTO work_previews(id,session_id,subject_type,subject_id,current_version) VALUES('preview','session','task','task-one',1);").unwrap();
    c.execute("INSERT INTO work_preview_versions(preview_id,version,derivation_kind,context_revision,locale,fields_json,content_hash) VALUES('preview',1,'generated',0,'en',?,'hash')",[preview_fields().to_string()]).unwrap();
    c.execute_batch("INSERT INTO work_preview_references(id,preview_id,version,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,role,claim_ids_json) VALUES('ref','preview',1,'request','bundle','doc','v1','Approval','counterevidence','[\"background:approval\"]');
      INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,use_scope,reason,preview_version) VALUES('use','preview',0,'doc','v1','Approval','used','background:approval','counterevidence',1);
      INSERT INTO work_preview_references(id,preview_id,version,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,role) VALUES('candidate','preview',1,'request','bundle','candidate','v2','Other','counterevidence');").unwrap();
    let used = for_task(&c, "task-one").unwrap();
    assert_eq!(used.len(), 1);
    assert_eq!(
        used[0]["reason"],
        "External requests need a separate approval."
    );
    assert_eq!(used[0]["disposition"], "counterevidence");
    assert!(for_task(&c, "task-two").unwrap().is_empty());
    c.execute_batch("INSERT INTO captures(id,text) VALUES('capture-two','other');
      INSERT INTO refinement_sessions(id,capture_id) VALUES('session-two','capture-two');
      INSERT INTO work_previews(id,session_id,subject_type,subject_id,current_version) VALUES('preview-two','session-two','task','task-two',0);
      INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,reason) VALUES('other-use','preview-two',0,'other-doc','v2','Other','used','Other Task explicitly uses this condition');").unwrap();
    assert_eq!(
        for_task(&c, "task-two").unwrap()[0]["documentId"],
        "other-doc"
    );
    assert_eq!(for_task(&c, "task-one").unwrap()[0]["documentId"], "doc");
    c.execute(
        "UPDATE reference_interactions SET use_scope='missing:unknown' WHERE id='use'",
        [],
    )
    .unwrap();
    assert!(for_task(&c, "task-one")
        .unwrap_err()
        .contains("reference_provenance_unresolved"));
}
