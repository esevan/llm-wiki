//! Durable prerequisites for the single private-draft action. No publication authority.
use super::{database, jobs, knowledge_distillation, semantic::SemanticEngine, task_distillation};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::path::Path;
use tokio_util::sync::CancellationToken;

pub(crate) fn request_source(
    tx: &Transaction<'_>,
    task: &str,
    expected: i64,
) -> Result<String, String> {
    let (head, state): (i64, String) = tx.query_row(
        "SELECT current_revision,state FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=tasks.id)",
        [task], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|e| e.to_string())?.ok_or("Task not found")?;
    if head != expected {
        return Err("task_revision_conflict: Task changed before draft preparation".into());
    }
    if state != "completed" {
        return Err("task_not_completed".into());
    }
    let recorded = super::task_journey::build_tx(tx, task)?;
    let mut statement = tx.prepare("SELECT id,revision,status,final_report FROM task_work_session_runs WHERE task_id=? ORDER BY id").map_err(|e| e.to_string())?;
    let runs = statement
        .query_map([task], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?
            ]))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut statement = tx.prepare("SELECT i.run_id,i.provider_item_id,i.kind,i.status,i.content_json FROM task_work_session_run_items i JOIN task_work_session_runs r ON r.id=i.run_id WHERE r.task_id=? ORDER BY i.run_id,i.provider_item_id").map_err(|e|e.to_string())?;
    let items = statement
        .query_map([task], |row| {
            Ok(json!([
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?
            ]))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(crate::application::task_distillation_service::content_hash(
        &json!({"recorded":recorded,"runs":runs,"items":items,"revision":head}).to_string(),
    ))
}

fn ready(
    db: &Path,
    task: &str,
    revision: i64,
    locale: &str,
    source: Option<&str>,
) -> Result<bool, String> {
    let mut connection = database::open(db)?;
    let tx = connection.transaction().map_err(|e| e.to_string())?;
    let current = request_source(&tx, task, revision)?;
    if source.is_some_and(|expected| expected != current) {
        return Err("source_snapshot_stale: Task evidence changed during draft preparation".into());
    }
    match knowledge_distillation::capture_snapshot_tx(&tx, task, revision, locale) {
        Ok(_) => Ok(true),
        Err(error) if error.starts_with("journey_not_current:") => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) struct Preparation<'a> {
    pub db: &'a Path,
    pub settings: &'a Path,
    pub vault: &'a Path,
    pub registry: &'a jobs::JobRegistry,
    pub semantic: &'a SemanticEngine,
}

impl Preparation<'_> {
    pub(crate) async fn ensure(
        &self,
        token: &CancellationToken,
        worker: &str,
        job: &str,
        input: &Value,
    ) -> Result<(), String> {
        let Self {
            db,
            settings,
            vault,
            registry,
            semantic,
        } = *self;
        let task = input["taskId"].as_str().ok_or("taskId is required")?;
        let revision = input["expectedTaskRevision"]
            .as_i64()
            .ok_or("expectedTaskRevision is required")?;
        let locale = input["locale"].as_str().unwrap_or("en");
        let source = input["knowledgeSourceHash"].as_str();
        if ready(db, task, revision, locale, source)? {
            return Ok(());
        }

        // Reattach after restart to the durable dependency instead of creating another refresh.
        let connection = database::open(db)?;
        let stored: Option<String> = connection.query_row(
        "SELECT json_extract(input_json,'$.knowledgeRefreshJob') FROM ai_jobs_v2 WHERE id=?", [job], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
        let dependency = if let Some(id) = stored {
            id
        } else {
            let run = task_distillation::terminal_run_for_repair(&connection, task, None)?;
            let service =
                crate::application::task_execution_service::TaskExecutionApplicationService::new(
                    db,
                );
            let mut refresh = service.distillation_job_input(&run, locale)?;
            refresh["distillationInput"]["repairReason"] =
                json!("targeted source change: knowledge preparation");
            let completion: String = connection.query_row("SELECT id FROM task_completions WHERE task_id=? ORDER BY created_at DESC,id DESC LIMIT 1", [task], |r| r.get(0)).map_err(|e| e.to_string())?;
            let affected = task_distillation::affected_subgraph(
                &connection,
                task,
                "task_completion",
                &completion,
            )?;
            task_distillation::scope_generation_input(&mut refresh, &affected);
            // Reuse the current Run's matching refresh even if it originated at completion.
            let active: Option<String> = connection.query_row(
            "SELECT id FROM ai_jobs_v2 WHERE ((task_kind='run_report_distillation' AND entity_id=?) OR (task_kind='task_journey_increment' AND entity_id=?)) AND source_revision=? AND json_extract(input_json,'$.distillationInput.locale')=? AND status IN ('queued','running','retryable') ORDER BY rowid DESC LIMIT 1",
            params![run,task,refresh["sourceRevision"].as_str().unwrap_or(""),locale], |r| r.get(0),
        ).optional().map_err(|e| e.to_string())?;
            if let Some(id) = active {
                id
            } else {
                let queued = jobs::enqueue(
                    db.to_owned(),
                    settings.to_owned(),
                    vault.to_owned(),
                    registry.clone(),
                    semantic.clone(),
                    refresh,
                )
                .await?;
                queued["id"]
                    .as_str()
                    .ok_or("knowledge_preparation_failed: refresh was not queued")?
                    .to_owned()
            }
        };
        connection.execute(
        "UPDATE ai_jobs_v2 SET input_json=json_set(input_json,'$.knowledgeRefreshJob',?,'$.knowledgeStage','preparing'),progress_total=2,progress_completed=0 WHERE id=? AND status='running' AND worker_id=?",
        params![dependency,job,worker],
    ).map_err(|e| e.to_string())?;
        drop(connection);
        loop {
            if token.is_cancelled() {
                return Err("cancelled: draft preparation stopped".into());
            }
            // A changed canonical record never acquires authority from the old request.
            ready(db, task, revision, locale, source)?;
            let dependency = jobs::get(db, &dependency)?;
            match dependency["status"].as_str().unwrap_or("missing") {
                "queued" | "running" | "retryable" => {}
                "completed" => {
                    if ready(db, task, revision, locale, source)? {
                        return Ok(());
                    }
                    return Err("knowledge_preparation_failed: refreshed evidence is not ready for this completion".into());
                }
                _ => {
                    return Err(format!(
                        "knowledge_preparation_failed: {}",
                        dependency["error"]["message"]
                            .as_str()
                            .unwrap_or("The evidence update could not finish")
                    ))
                }
            }
            tokio::select! {
                _ = token.cancelled() => return Err("cancelled: draft preparation stopped".into()),
                _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;

    fn fixture(report: Option<&str>) -> (tempfile::TempDir, std::path::PathBuf, Value) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        database::initialize(&db).unwrap();
        let mut c = database::open(&db).unwrap();
        c.execute_batch("INSERT INTO tasks(id,current_revision,state,created_at,last_user_activity_at) VALUES('task',1,'completed','2026-01-01','2026-01-01');
            INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES('task',1,'Recorded work','Keep the original records','Recorded outcome','','','tests','hash','2026-01-01');
            INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion','task',1,'Recorded checks','Approval remains required','complete','2026-01-02');
            INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log','task','Saved original evidence','2026-01-01');
            INSERT INTO task_work_sessions(id,task_id,title,created_at,updated_at) VALUES('session','task','Work','2026-01-01','2026-01-01');
            INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES('entry','session','user','note','Inspect','2026-01-01');
            INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,revision,created_at,updated_at) VALUES('run','task','session','submission','hash','Inspect','entry','log','model','','context','failed',1,'2026-01-01','2026-01-01');").unwrap();
        c.execute("UPDATE task_work_session_runs SET final_report=?", [report])
            .unwrap();
        let tx = c.transaction().unwrap();
        let source = request_source(&tx, "task", 1).unwrap();
        tx.commit().unwrap();
        (
            root,
            db,
            json!({"taskKind":"knowledge_draft","entityType":"tasks","entityId":"task","taskId":"task","expectedTaskRevision":1,"locale":"en","knowledgeSourceHash":source}),
        )
    }

    fn pending_refresh(db: &Path) -> Value {
        let input =
            crate::application::task_execution_service::TaskExecutionApplicationService::new(db)
                .distillation_job_input("run", "en")
                .unwrap();
        database::open(db).unwrap().execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,prompt_id,prompt_version,source_revision) VALUES('refresh','run_report_distillation','task_work_session_runs','run','running',?,'refresh','run_report_distillation',5,?)",params![input.to_string(),input["sourceRevision"].as_str().unwrap()]).unwrap();
        input
    }

    struct Fake(std::process::Child);
    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn provider(settings: &Path) -> Fake {
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fakes/openai_server.mjs");
        let mut fake = Fake(
            std::process::Command::new("node")
                .arg(script)
                .args(["--port", "0"])
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut port = String::new();
        std::io::BufReader::new(fake.0.stdout.take().unwrap())
            .read_line(&mut port)
            .unwrap();
        super::super::settings::save_provider(settings,&json!({"base_url":format!("http://127.0.0.1:{}/v1",port.trim()),"model":"fixture","api_key":"fixture"})).unwrap();
        fake
    }
    async fn terminal(db: &Path, id: &str) -> Value {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let job = jobs::get(db, id).unwrap();
                if !["queued", "running", "retryable"].contains(&job["status"].as_str().unwrap()) {
                    return job;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("isolated generation must finish")
    }

    #[tokio::test]
    async fn completed_run_without_report_builds_private_draft_in_one_action() {
        for report in [
            None,
            Some("   "),
            Some("Saved final report: approval remains required."),
        ] {
            let (root, db, input) = fixture(report);
            if report.is_some_and(|report| !report.trim().is_empty()) {
                database::open(&db).unwrap().execute_batch("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('task_journey','task',1,'task','earlier','task_journey_increment',3,'rules','schema','en','{}','current','earlier');
                    INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('task_journey','task',1,'earlier','stale');
                    INSERT INTO task_distillation_nodes(task_id,node_id,revision,kind,status,claim_ids_json,detail_json,source_set_hash,active) VALUES('task','retained',1,'result','current','[]','{\"result\":\"Earlier retained result\"}','earlier',1);").unwrap();
            }
            let settings = root.path().join("settings.json");
            let _fake = provider(&settings);
            let job = jobs::enqueue(
                db.clone(),
                settings,
                root.path().join("vault"),
                jobs::JobRegistry::default(),
                SemanticEngine::new(None),
                input,
            )
            .await
            .unwrap();
            let result = terminal(&db, job["id"].as_str().unwrap()).await;
            assert_eq!(result["status"], "completed", "{result}");
            let c = database::open(&db).unwrap();
            assert_eq!(
                c.query_row("SELECT count(*) FROM knowledge_draft_versions", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
                1
            );
            assert_eq!(
                c.query_row("SELECT state FROM tasks WHERE id='task'", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                "completed"
            );
            assert_eq!(
                c.query_row(
                    "SELECT body FROM task_work_log_entries WHERE id='log'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "Saved original evidence"
            );
            assert_eq!(c.query_row("SELECT verification_state FROM (SELECT json_extract(snapshot_json,'$.verificationState') AS verification_state FROM task_distillation_completion_snapshots)",[],|r|r.get::<_,String>(0)).unwrap(),"recorded");
            if report.is_some_and(|report| !report.trim().is_empty()) {
                assert_eq!(
                    c.query_row(
                        "SELECT revision FROM task_distillation_nodes WHERE node_id='retained'",
                        [],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                    1
                );
                assert_eq!(
                    c.query_row("SELECT count(*) FROM task_distillation_nodes", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    1
                );
            }
            assert!(!root.path().join("vault/Knowledge").exists());
        }
    }

    #[tokio::test]
    async fn duplicate_clicks_reuse_running_refresh_and_failure_is_inline_retryable() {
        let (root, db, mut input) = fixture(None);
        pending_refresh(&db);
        let registry = jobs::JobRegistry::default();
        let semantic = SemanticEngine::new(None);
        let settings = root.path().join("settings.json");
        let first = jobs::enqueue(
            db.clone(),
            settings.clone(),
            root.path().join("vault"),
            registry.clone(),
            semantic.clone(),
            input.clone(),
        )
        .await
        .unwrap();
        input["operationId"] = json!("different-click");
        let second = jobs::enqueue(
            db.clone(),
            settings.clone(),
            root.path().join("vault"),
            registry.clone(),
            semantic.clone(),
            input.clone(),
        )
        .await
        .unwrap();
        assert_eq!(first["id"], second["id"]);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let c = database::open(&db).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM ai_jobs_v2 WHERE task_kind='run_report_distillation'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        c.execute("UPDATE ai_jobs_v2 SET status='failed',error_code='application_error',error_message='Scoped fixture failure' WHERE id='refresh'",[]).unwrap();
        let failed = terminal(&db, first["id"].as_str().unwrap()).await;
        assert!(
            failed["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Scoped fixture failure"),
            "{failed}"
        );
        let _fake = provider(&settings);
        let retry = jobs::enqueue(
            db.clone(),
            settings,
            root.path().join("vault"),
            registry,
            semantic,
            input,
        )
        .await
        .unwrap();
        assert_ne!(first["id"], retry["id"]);
        assert_eq!(
            terminal(&db, retry["id"].as_str().unwrap()).await["status"],
            "completed"
        );
    }

    #[tokio::test]
    async fn startup_reattaches_saved_dependency_and_source_changes_fail_closed() {
        let (root, db, input) = fixture(None);
        pending_refresh(&db);
        let registry = jobs::JobRegistry::default();
        let semantic = SemanticEngine::new(None);
        let settings = root.path().join("settings.json");
        let _fake = provider(&settings);
        let first = jobs::enqueue(
            db.clone(),
            settings.clone(),
            root.path().join("vault"),
            registry.clone(),
            semantic.clone(),
            input,
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        registry.cancel(first["id"].as_str().unwrap());
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        jobs::recover_stored(
            &db,
            &settings,
            &root.path().join("vault"),
            &jobs::JobRegistry::default(),
            &semantic,
        )
        .unwrap();
        let recovered = terminal(&db, first["id"].as_str().unwrap()).await;
        assert_eq!(recovered["status"], "completed", "{recovered}");
        assert_eq!(
            database::open(&db)
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM ai_jobs_v2 WHERE task_kind='run_report_distillation'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );

        let (root, db, input) = fixture(None);
        pending_refresh(&db);
        let job = jobs::enqueue(
            db.clone(),
            root.path().join("settings.json"),
            root.path().join("vault"),
            jobs::JobRegistry::default(),
            SemanticEngine::new(None),
            input,
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE task_work_log_entries SET body='New evidence' WHERE id='log'",
                [],
            )
            .unwrap();
        let failed = terminal(&db, job["id"].as_str().unwrap()).await;
        assert!(failed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("source_snapshot_stale"));
        assert_eq!(
            database::open(&db)
                .unwrap()
                .query_row("SELECT count(*) FROM knowledge_draft_versions", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
    }
}
