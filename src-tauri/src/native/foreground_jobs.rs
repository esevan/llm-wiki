//! Durable observations of foreground calls. Never schedules provider execution.
use super::{database, jobs::JobRegistry};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub(crate) fn observed(kind: &str) -> bool {
    matches!(kind, "session_context_summary" | "knowledge_revision")
}

pub(crate) struct Tracking {
    db: PathBuf,
    pub id: String,
    worker: String,
    registry: JobRegistry,
    pub token: CancellationToken,
}
impl Tracking {
    pub fn start(
        db: &Path,
        registry: &JobRegistry,
        kind: &str,
        model: &str,
        mut origin: Value,
    ) -> Result<Self, String> {
        if !observed(kind) {
            return Err("Unsupported foreground observation".into());
        }
        let id = if kind == "knowledge_revision" {
            revision_id(origin["turnId"].as_str().ok_or("Missing revision turn")?)
        } else {
            format!("context-summary-{}", uuid::Uuid::new_v4())
        };
        let connection = database::open(db)?;
        let label:Option<String>=connection.query_row("SELECT r.title FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=?",[origin["taskId"].as_str().unwrap_or("")],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        if let Some(label) = label {
            origin["label"] = json!(label);
        }

        let (worker, token) = registry.register(&id)?;
        let input = json!({"observationOnly":true,"model":model,"origin":origin,"callsStarted":0,"callsCompleted":0});
        let result=connection.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,input_json,idempotency_key,worker_id,result_interface,execution_outcome,application_disposition,started_at) VALUES(?,?,'tasks',?,'running',?,?,?,'foreground_origin','pending','not_applicable',CURRENT_TIMESTAMP)",params![id,kind,origin["taskId"].as_str().unwrap_or(""),input.to_string(),id,worker]).map_err(|e|e.to_string());
        if let Err(error) = result {
            registry.cancel(&id);
            return Err(error);
        }
        Ok(Self {
            db: db.to_owned(),
            id,
            worker,
            registry: registry.clone(),
            token,
        })
    }
    pub fn set_model(&self, model: &str) -> Result<(), String> {
        database::open(&self.db)?.execute("UPDATE ai_jobs_v2 SET input_json=json_set(input_json,'$.model',?) WHERE id=? AND worker_id=? AND status='running'",params![model,self.id,self.worker]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn call_started(&self) -> Result<(), String> {
        if self.token.is_cancelled() {
            return Err("cancelled".into());
        }
        let changed=database::open(&self.db)?.execute("UPDATE ai_jobs_v2 SET input_json=json_set(input_json,'$.callsStarted',COALESCE(json_extract(input_json,'$.callsStarted'),0)+1),progress_total=progress_total+1 WHERE id=? AND worker_id=? AND status='running'",params![self.id,self.worker]).map_err(|e|e.to_string())?;
        if changed != 1 {
            return Err("cancelled".into());
        }
        Ok(())
    }
    pub fn call_completed(&self) -> Result<(), String> {
        database::open(&self.db)?.execute("UPDATE ai_jobs_v2 SET input_json=json_set(input_json,'$.callsCompleted',COALESCE(json_extract(input_json,'$.callsCompleted'),0)+1),progress_completed=progress_completed+1 WHERE id=? AND worker_id=? AND status IN ('running','cancelled')",params![self.id,self.worker]).map_err(|e|e.to_string())?;
        Ok(())
    }
    pub fn finish(&self, error: Option<&str>) -> Result<(), String> {
        let cancelled = self.token.is_cancelled();
        let status = if cancelled {
            "cancelled"
        } else if error.is_some() {
            "failed"
        } else {
            "completed"
        };
        database::open(&self.db)?.execute("UPDATE ai_jobs_v2 SET status=?,execution_outcome=?,error_code=?,error_message=?,finished_at=CURRENT_TIMESTAMP WHERE id=? AND worker_id=? AND status='running'",params![status,if cancelled {"cancelled"}else if error.is_some(){"failed"}else{"succeeded"},if error.is_some(){"foreground_error"}else{""},error.unwrap_or(""),self.id,self.worker]).map_err(|e|e.to_string())?;
        self.registry.finish(&self.id, &self.worker);
        Ok(())
    }
}
impl Drop for Tracking {
    fn drop(&mut self) {
        self.registry.finish(&self.id, &self.worker);
    }
}
pub(crate) fn revision_id(turn: &str) -> String {
    format!("knowledge-revision-{turn}")
}

/// An interrupted observation cannot safely repeat billable calls on restart.
pub(crate) fn recover(db: &Path) -> Result<(), String> {
    let c = database::open(db)?;
    c.execute("UPDATE ai_jobs_v2 SET status=CASE (SELECT state FROM knowledge_revision_turns WHERE id=json_extract(ai_jobs_v2.input_json,'$.origin.turnId')) WHEN 'cancelled' THEN 'cancelled' WHEN 'failed' THEN 'failed' ELSE 'completed' END,execution_outcome=CASE (SELECT state FROM knowledge_revision_turns WHERE id=json_extract(ai_jobs_v2.input_json,'$.origin.turnId')) WHEN 'cancelled' THEN 'cancelled' WHEN 'failed' THEN 'failed' ELSE 'succeeded' END,finished_at=CURRENT_TIMESTAMP WHERE task_kind='knowledge_revision' AND status IN ('running','queued','retryable') AND EXISTS(SELECT 1 FROM knowledge_revision_turns WHERE id=json_extract(ai_jobs_v2.input_json,'$.origin.turnId') AND state<>'running')",[]).map_err(|e|e.to_string())?;
    c.execute("UPDATE ai_jobs_v2 SET error_code='foreground_error',error_message=COALESCE((SELECT COALESCE(NULLIF(error,''),json_extract(reply_json,'$.preparationError')) FROM knowledge_revision_turns WHERE id=json_extract(ai_jobs_v2.input_json,'$.origin.turnId')),'Request failed; review the original screen.') WHERE task_kind='knowledge_revision' AND status='failed' AND error_code=''",[]).map_err(|e|e.to_string())?;
    c.execute("UPDATE ai_jobs_v2 SET status='failed',execution_outcome='failed',error_code='interrupted',error_message='Request interrupted. Earlier API calls may have been charged. Open the original screen to review or send a new request; no automatic retry was performed.',finished_at=CURRENT_TIMESTAMP WHERE task_kind IN ('session_context_summary','knowledge_revision') AND status IN ('queued','running','retryable')",[]).map_err(|e|e.to_string())?;
    c.execute("UPDATE knowledge_revision_turns SET state='failed',error='Request interrupted; review before sending a new request.' WHERE state='running' AND EXISTS(SELECT 1 FROM ai_jobs_v2 j WHERE j.id='knowledge-revision-'||knowledge_revision_turns.id AND j.status='failed' AND j.error_code='interrupted')",[]).map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let parent = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".tmp");
        std::fs::create_dir_all(&parent).unwrap();
        let root = tempfile::tempdir_in(parent).unwrap();
        let db = root.path().join("fixture.sqlite");
        database::initialize(&db).unwrap();
        (root, db)
    }
    #[test]
    fn multi_call_tracking_is_observation_only_and_cancelled_without_reexecution() {
        let (root, db) = fixture();
        let registry = JobRegistry::default();
        let t = Tracking::start(
            &db,
            &registry,
            "session_context_summary",
            "summary-model",
            json!({"taskId":"task","sessionId":"session"}),
        )
        .unwrap();
        t.call_started().unwrap();
        t.call_completed().unwrap();
        t.call_started().unwrap();
        let job = super::super::jobs::get(&db, &t.id).unwrap();
        assert_eq!(job["calls_started"], 2);
        assert_eq!(job["calls_completed"], 1);
        assert_eq!(job["model"], "summary-model");
        assert!(super::super::jobs::retry(
            &db,
            &root.path().join("settings.json"),
            root.path(),
            &registry,
            &super::super::semantic::SemanticEngine::new(None),
            &t.id
        )
        .unwrap_err()
        .contains("original screen"));
        super::super::jobs::cancel(&db, &registry, &t.id).unwrap();
        assert!(t.token.is_cancelled());
        assert!(t.call_started().is_err());
        t.finish(None).unwrap();
        assert_eq!(
            super::super::jobs::get(&db, &t.id).unwrap()["status"],
            "cancelled"
        );
    }
    #[test]
    fn interrupted_calls_are_persisted_and_never_queued_on_recovery() {
        let (_root, db) = fixture();
        let registry = JobRegistry::default();
        let t = Tracking::start(
            &db,
            &registry,
            "session_context_summary",
            "model",
            json!({"taskId":"task"}),
        )
        .unwrap();
        t.call_started().unwrap();
        recover(&db).unwrap();
        let job = super::super::jobs::get(&db, &t.id).unwrap();
        assert_eq!(job["status"], "failed");
        assert_eq!(job["error"]["code"], "interrupted");
        assert_eq!(job["calls_started"], 1);
        t.finish(None).unwrap();
        assert_eq!(
            super::super::jobs::get(&db, &t.id).unwrap()["status"],
            "failed"
        );
    }
}
