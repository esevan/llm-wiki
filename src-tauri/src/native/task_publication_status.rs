//! Read-only publication facts and explicit, private user choices. Never publishes.
use super::database;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

pub(crate) fn project(c: &Connection) -> Result<BTreeMap<String, Value>, String> {
    let mut query = c.prepare("SELECT t.id,
      p.published_revision IS NOT NULL AND (p.publication_document_id IS NOT NULL OR NOT EXISTS(
        SELECT 1 FROM task_knowledge_drafts old WHERE old.task_id=t.id AND old.revision=p.published_revision AND old.state='withdrawn')),
      EXISTS(SELECT 1 FROM ai_jobs_v2 j WHERE j.entity_id=t.id AND j.entity_type='tasks' AND j.task_kind='knowledge_draft' AND j.status IN ('queued','running','retryable') AND j.execution_outcome='pending' AND j.application_disposition='pending' AND COALESCE(json_extract(j.input_json,'$.expectedTaskRevision'),t.current_revision)=t.current_revision),
      EXISTS(SELECT 1 FROM knowledge_revision_turns r WHERE r.task_id=t.id AND r.state='running'),
      EXISTS(SELECT 1 FROM knowledge_revision_turns r WHERE r.task_id=t.id AND r.state='proposed' AND r.expected_private_revision IS p.current_private_revision ),
      p.current_private_revision,v.content_hash,v.derivation_kind,
      ch.reviewed_revision,ch.reviewed_content_hash,
      p.published_revision IS p.current_private_revision AND p.current_private_revision IS NOT NULL,
      COALESCE(ch.wont_publish,0),COALESCE(ch.choice_revision,0)
      FROM tasks t LEFT JOIN knowledge_pointers p ON p.task_id=t.id
      LEFT JOIN knowledge_draft_versions v ON v.task_id=t.id AND v.revision=p.current_private_revision
      LEFT JOIN task_publication_choices ch ON ch.task_id=t.id").map_err(|e|e.to_string())?;
    let rows=query.query_map([], |r| {
        let published:bool=r.get(1)?;
        let generating:bool=r.get::<_,bool>(2)? || r.get::<_,bool>(3)?;
        let revision:Option<i64>=r.get(5)?;
        let hash:Option<String>=r.get(6)?;
        let kind:Option<String>=r.get(7)?;
        let reviewed=revision.is_some() && revision==r.get::<_,Option<i64>>(8)? && hash==r.get::<_,Option<String>>(9)?;
        // User-authored/applied revisions are already reviewed; a generated private
        // draft remains a proposal until explicit review or actual publication.
        let generated=matches!(kind.as_deref(),Some("generated"|"regenerated")) && !reviewed;
        let exact_published:bool=r.get(10)?;
        let awaiting=r.get::<_,bool>(4)? || (generated && !exact_published);
        Ok((r.get::<_,String>(0)?,json!({"published":published,"proposal":if generating {"generating"} else if awaiting {"awaiting_review"} else {"none"},"wontPublish":r.get::<_,bool>(11)?,"choiceRevision":r.get::<_,i64>(12)?,"privateRevision":revision,"privateContentHash":hash})))
    }).map_err(|e|e.to_string())?.collect::<Result<BTreeMap<_,_>,_>>().map_err(|e|e.to_string())?;
    Ok(rows)
}
fn task<'a>(input: &'a Value) -> Result<&'a str, String> {
    input["taskId"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("taskId is required".into())
}
fn ensure_visible(c: &Connection, id: &str) -> Result<(), String> {
    let visible:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM tasks t WHERE t.id=? AND t.archived_at IS NULL AND NOT EXISTS(SELECT 1 FROM deleted_entities d WHERE d.entity_type='tasks' AND d.entity_id=t.id))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if visible {
        Ok(())
    } else {
        Err("Task not found".into())
    }
}
pub(crate) fn choose(db: &Path, input: &Value) -> Result<Value, String> {
    let mut c = database::open(db)?;
    let tx = database::immediate_transaction(&mut c)?;
    let id = task(input)?;
    ensure_visible(&tx, id)?;
    let state: (String, i64) = tx
        .query_row(
            "SELECT state,current_revision FROM tasks WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if state.0 != "completed" || input["expectedTaskRevision"].as_i64() != Some(state.1) {
        return Err("stale_task: refresh the completed list".into());
    }
    let choice = input["wontPublish"]
        .as_bool()
        .ok_or("wontPublish is required")?;
    let status = project(&tx)?.remove(id).ok_or("Task not found")?;
    let expected = input["expectedChoiceRevision"]
        .as_i64()
        .ok_or("expectedChoiceRevision is required")?;
    if expected != status["choiceRevision"].as_i64().unwrap_or(0) {
        return Err("stale_choice: refresh the completed list".into());
    }
    if choice && (status["published"] == true || status["proposal"] != "none") {
        return Err("publication_changed: review the current publication proposal first".into());
    }
    tx.execute("INSERT INTO task_publication_choices(task_id,wont_publish,choice_revision) VALUES(?,?,1) ON CONFLICT(task_id) DO UPDATE SET wont_publish=excluded.wont_publish,choice_revision=task_publication_choices.choice_revision+1,updated_at=CURRENT_TIMESTAMP",params![id,choice]).map_err(|e|e.to_string())?;
    let result = project(&tx)?.remove(id).ok_or("Task not found")?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}
pub(crate) fn review(db: &Path, input: &Value) -> Result<Value, String> {
    let mut c = database::open(db)?;
    let tx = database::immediate_transaction(&mut c)?;
    let id = task(input)?;
    ensure_visible(&tx, id)?;
    let current:Option<(i64,String)>=tx.query_row("SELECT p.current_private_revision,v.content_hash FROM knowledge_pointers p JOIN knowledge_draft_versions v ON v.task_id=p.task_id AND v.revision=p.current_private_revision WHERE p.task_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
    let (n, hash) = current.ok_or("Knowledge not found")?;
    if input["revision"].as_i64() != Some(n)
        || input["expectedContentHash"].as_str() != Some(hash.as_str())
    {
        return Err("stale_knowledge: review the current draft".into());
    }
    tx.execute("INSERT INTO task_publication_choices(task_id,reviewed_revision,reviewed_content_hash) VALUES(?,?,?) ON CONFLICT(task_id) DO UPDATE SET reviewed_revision=excluded.reviewed_revision,reviewed_content_hash=excluded.reviewed_content_hash,updated_at=CURRENT_TIMESTAMP",params![id,n,hash]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(json!({"reviewedRevision":n}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::task_service::TaskApplicationService;
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, String) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("test.db");
        database::initialize(&db).unwrap();
        let task = TaskApplicationService::new(&db)
            .execute(
                "task.create",
                &json!({"operationId":"create","title":"Demo task","inputText":"Demo task"}),
            )
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE tasks SET state='completed',completed_at=CURRENT_TIMESTAMP WHERE id=?",
                [&task],
            )
            .unwrap();
        (root, db, task)
    }
    fn status(db: &Path, task: &str) -> Value {
        project(&database::open(db).unwrap())
            .unwrap()
            .remove(task)
            .unwrap()
    }
    fn draft(db: &Path, task: &str, n: i64, kind: &str) {
        let c = database::open(db).unwrap();
        c.execute("INSERT OR IGNORE INTO knowledge_applicability(id,summary,representative_questions_json,helps_with_json,conditions_json,exclusions_json) VALUES('a','','[]','[]','[]','[]')",[]).unwrap();
        c.execute("INSERT INTO knowledge_draft_versions(task_id,revision,derivation_kind,article_type,title,body_markdown,content_hash,applicability_id,result_json,quality_state,model_status) VALUES(?,?,?,'concept','Demo','Demo',?,'a','{}','valid','complete')",params![task,n,kind,format!("hash-{n}")]).unwrap();
        c.execute("INSERT INTO knowledge_pointers(task_id,current_private_revision) VALUES(?,?) ON CONFLICT(task_id) DO UPDATE SET current_private_revision=excluded.current_private_revision",params![task,n]).unwrap();
    }
    #[test]
    fn explicit_choice_survives_restart_and_guards_stale_double_clicks_and_reopen() {
        let (_root, db, t) = fixture();
        assert_eq!(status(&db, &t)["proposal"], "none");
        assert_eq!(status(&db, &t)["wontPublish"], false);
        let input = json!({"taskId":t,"expectedTaskRevision":1,"expectedChoiceRevision":0,"wontPublish":true});
        assert_eq!(choose(&db, &input).unwrap()["wontPublish"], true);
        database::initialize(&db).unwrap();
        assert_eq!(status(&db, &t)["wontPublish"], true);
        let board = TaskApplicationService::new(&db)
            .execute("workbench.get", &json!({}))
            .unwrap();
        assert_eq!(
            board["categories"][0]["items"][0]["knowledgePublication"]["wontPublish"],
            true
        );
        assert!(choose(&db, &input).unwrap_err().starts_with("stale_choice"));
        assert_eq!(choose(&db,&json!({"taskId":t,"expectedTaskRevision":1,"expectedChoiceRevision":1,"wontPublish":false})).unwrap()["wontPublish"],false);
        database::open(&db)
            .unwrap()
            .execute(
                "UPDATE tasks SET state='in_progress',current_revision=2 WHERE id=?",
                [&t],
            )
            .unwrap();
        assert!(choose(&db, &input).unwrap_err().starts_with("stale_task"));
        assert_eq!(status(&db, &t)["wontPublish"], false);
    }
    #[test]
    fn generated_review_applied_and_publication_are_distinct() {
        let (_root, db, t) = fixture();
        draft(&db, &t, 1, "generated");
        assert_eq!(status(&db, &t)["proposal"], "awaiting_review");
        assert!(choose(&db,&json!({"taskId":t,"expectedTaskRevision":1,"expectedChoiceRevision":0,"wontPublish":true})).is_err());
        assert!(review(
            &db,
            &json!({"taskId":t,"revision":1,"expectedContentHash":"wrong"})
        )
        .is_err());
        review(
            &db,
            &json!({"taskId":t,"revision":1,"expectedContentHash":"hash-1"}),
        )
        .unwrap();
        assert_eq!(status(&db, &t)["proposal"], "none");
        assert_eq!(status(&db, &t)["published"], false);
        draft(&db, &t, 2, "generated");
        assert_eq!(status(&db, &t)["proposal"], "awaiting_review");
        let c = database::open(&db).unwrap();
        c.execute(
            "UPDATE knowledge_pointers SET published_revision=1 WHERE task_id=?",
            [&t],
        )
        .unwrap();
        assert_eq!(status(&db, &t)["published"], true);
        assert_eq!(status(&db, &t)["proposal"], "awaiting_review");
        c.execute(
            "UPDATE knowledge_pointers SET published_revision=2 WHERE task_id=?",
            [&t],
        )
        .unwrap();
        assert_eq!(status(&db, &t)["proposal"], "none");
        draft(&db, &t, 3, "edited");
        assert_eq!(status(&db, &t)["proposal"], "none");
        assert_eq!(status(&db, &t)["published"], true);
        c.execute(
            "UPDATE knowledge_pointers SET published_revision=NULL WHERE task_id=?",
            [&t],
        )
        .unwrap();
        assert_eq!(status(&db, &t)["published"], false);
    }
    #[test]
    fn jobs_and_revision_candidates_clear_on_failure_cancel_apply_and_stale_head() {
        let (_root, db, t) = fixture();
        let c = database::open(&db).unwrap();
        c.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status) VALUES('job','knowledge_draft','tasks',?,'queued')",[&t]).unwrap();
        assert_eq!(status(&db, &t)["proposal"], "generating");
        for state in ["failed", "cancelled", "stale", "succeeded"] {
            c.execute("UPDATE ai_jobs_v2 SET status=? WHERE id='job'", [state])
                .unwrap();
            assert_eq!(status(&db, &t)["proposal"], "none");
        }
        draft(&db, &t, 1, "edited");
        c.execute("INSERT INTO knowledge_revision_turns(id,task_id,base_revision,base_content_hash,expected_private_revision,request_hash,user_message,state) VALUES('turn',?,1,'hash-1',1,'request','Demo','running')",[&t]).unwrap();
        assert_eq!(status(&db, &t)["proposal"], "generating");
        c.execute(
            "UPDATE knowledge_revision_turns SET state='proposed' WHERE id='turn'",
            [],
        )
        .unwrap();
        assert_eq!(status(&db, &t)["proposal"], "awaiting_review");
        for state in ["failed", "needs_evidence", "cancelled", "applied"] {
            c.execute(
                "UPDATE knowledge_revision_turns SET state=? WHERE id='turn'",
                [state],
            )
            .unwrap();
            assert_eq!(status(&db, &t)["proposal"], "none");
            assert_eq!(status(&db, &t)["published"], false);
        }
        c.execute(
            "UPDATE knowledge_revision_turns SET state='proposed' WHERE id='turn'",
            [],
        )
        .unwrap();
        draft(&db, &t, 2, "edited");
        assert_eq!(status(&db, &t)["proposal"], "none");
    }
}
