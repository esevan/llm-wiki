//! Explicit ownership is separate from historical split_from provenance.
use crate::adapters::sqlite::task_repository::{new_id, record_activity_tx};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

pub(crate) fn parent(c: &Connection, task: &str) -> Result<Option<String>, String> {
    c.query_row(
        "SELECT parent_task_id FROM task_subtasks WHERE child_task_id=?",
        [task],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn snapshot(c: &Connection, task: &str) -> Result<Value, String> {
    c.query_row("SELECT t.current_revision,t.state,r.title,r.detail,r.outcome,r.scope,r.non_goals,r.validation_criteria FROM tasks t JOIN task_revisions r ON r.task_id=t.id AND r.revision=t.current_revision WHERE t.id=?", [task], |r| Ok(json!({
        "id":task,"taskRevision":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,
        "title":r.get::<_,String>(2)?,"detail":r.get::<_,String>(3)?,"outcome":r.get::<_,String>(4)?,
        "scope":r.get::<_,String>(5)?,"nonGoals":r.get::<_,String>(6)?,"validationCriteria":r.get::<_,String>(7)?
    }))).map_err(|e| e.to_string())
}

pub(crate) fn children(c: &Connection, task: &str) -> Result<Vec<Value>, String> {
    let mut query = c.prepare("SELECT child_task_id FROM task_subtasks WHERE parent_task_id=? ORDER BY created_at,child_task_id").map_err(|e| e.to_string())?;
    let ids = query
        .query_map([task], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    ids.iter().map(|id| snapshot(c, id)).collect()
}

pub(crate) fn context(c: &Connection, task: &str) -> Result<Value, String> {
    let parent_id = parent(c, task)?;
    let parent_task = parent_id.as_deref().map(|id| snapshot(c, id)).transpose()?;
    let siblings = parent_id
        .as_deref()
        .map(|id| children(c, id))
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .filter(|x| x["id"] != task)
        .collect::<Vec<_>>();
    Ok(json!({"parent":parent_task,"siblings":siblings,"children":children(c,task)?}))
}

pub(crate) fn context_hash(c: &Connection, task: &str) -> Result<String, String> {
    Ok(super::task_assistance::digest(
        &context(c, task)?.to_string(),
    ))
}

pub(crate) fn metadata(c: &Connection, task: &str) -> Result<Value, String> {
    let revision = c
        .query_row(
            "SELECT revision FROM task_refinements WHERE task_id=?",
            [task],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(json!({"parentTaskId":parent(c,task)?,"refinedRevision":revision}))
}

pub(crate) fn mark_refined(tx: &Transaction<'_>, task: &str, revision: i64) -> Result<(), String> {
    tx.execute("INSERT INTO task_refinements(task_id,revision) VALUES(?,?) ON CONFLICT(task_id) DO UPDATE SET revision=excluded.revision", params![task,revision]).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn reopen_ancestors(
    tx: &Transaction<'_>,
    task: &str,
    operation: &str,
    at: &str,
) -> Result<(), String> {
    let mut next = parent(tx, task)?;
    while let Some(id) = next {
        let changed = tx.execute("UPDATE tasks SET state='in_progress',completed_at=NULL,reopened_at=? WHERE id=? AND state='completed'", params![at,id]).map_err(|e| e.to_string())?;
        if changed > 0 {
            record_activity_tx(tx, "task", &id, "subtask_reopened", operation, at)?;
        }
        next = parent(tx, &id)?;
    }
    Ok(())
}

pub(crate) fn ensure_children_completed(c: &Connection, task: &str) -> Result<(), String> {
    if children(c, task)?
        .iter()
        .any(|child| child["state"] != "completed")
    {
        return Err("transition_invalid: complete all Subtasks first".into());
    }
    Ok(())
}

/// Runs within the initiating completion transaction, including its replay guard.
pub(crate) fn child_completed(
    tx: &Transaction<'_>,
    task: &str,
    completion_id: &str,
    operation: &str,
    at: &str,
) -> Result<(), String> {
    let Some(parent_id) = parent(tx, task)? else {
        return Ok(());
    };
    let mut child_snapshots = children(tx, &parent_id)?;
    let all_done =
        !child_snapshots.is_empty() && child_snapshots.iter().all(|x| x["state"] == "completed");
    let mut sections = Vec::new();
    for child in &mut child_snapshots {
        let child_id = child["id"].as_str().unwrap_or_default().to_owned();
        if child["state"] != "completed" {
            continue;
        }
        let evidence: Value = tx.query_row("SELECT id,evidence,report FROM task_completions WHERE task_id=? AND task_revision=? ORDER BY created_at DESC,rowid DESC LIMIT 1", params![child_id,child["taskRevision"].as_i64()], |r| Ok(json!({"id":r.get::<_,String>(0)?,"evidence":r.get::<_,String>(1)?,"report":r.get::<_,String>(2)?})))
            .optional().map_err(|e| e.to_string())?.ok_or("Completed Subtask evidence not found")?;
        sections.push(format!(
            "## {}\n\n{}\n\n{}",
            child["title"].as_str().unwrap_or_default(),
            evidence["evidence"].as_str().unwrap_or_default(),
            evidence["report"].as_str().unwrap_or_default()
        ));
        child["completion"] = evidence;
    }
    let parent_snapshot = snapshot(tx, &parent_id)?;
    let revision = parent_snapshot["taskRevision"]
        .as_i64()
        .ok_or("Missing parent revision")?;
    let summary = sections.join("\n\n");
    let mut parent_completion = None;
    if all_done && parent_snapshot["state"] != "completed" {
        let id = new_id();
        tx.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES(?,?,?,?,?,?,?)",params![id,parent_id,revision,summary,"All Subtasks completed",format!("{operation}:parent:{parent_id}"),at]).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE tasks SET state='completed',completed_at=? WHERE id=?",
            params![at, parent_id],
        )
        .map_err(|e| e.to_string())?;
        parent_completion = Some(id);
    }
    let source = json!({"kind":"subtask_aggregate","parent":snapshot(tx,&parent_id)?,"children":child_snapshots,"triggerTaskId":task,"triggerCompletionId":completion_id,"allSubtasksCompleted":all_done});
    let lineage = json!({"taskId":parent_id,"taskRevision":revision,"completionId":completion_id,"completionBinding":"triggering_subtask","sourceHash":super::task_assistance::digest(&source.to_string()),"source":source});
    let body = format!(
        "# {}\n\n{}\n\n{}\n\n{}",
        parent_snapshot["title"].as_str().unwrap_or_default(),
        parent_snapshot["outcome"].as_str().unwrap_or_default(),
        if all_done {
            "All Subtasks completed."
        } else {
            "Subtask progress — remaining work is open."
        },
        summary
    );
    let draft = super::task_assistance::insert_knowledge_draft_tx(
        tx,
        &parent_id,
        revision,
        completion_id,
        &body,
        &lineage,
        "subtask_aggregate",
        "",
    )?;
    let request = json!({"operationId":format!("subtask-publish:{parent_id}:{}",draft["draftRevision"]),"taskId":parent_id,"draftRevision":draft["draftRevision"],"expectedContentHash":draft["contentHash"],"expectedSourceHash":draft["sourceHash"]});
    tx.execute(
        "INSERT INTO task_auto_publications(task_id,revision,request_json) VALUES(?,?,?)",
        params![
            parent_id,
            draft["draftRevision"].as_i64(),
            request.to_string()
        ],
    )
    .map_err(|e| e.to_string())?;
    record_activity_tx(tx, "task", &parent_id, "subtask_completed", operation, at)?;
    if let Some(id) = parent_completion {
        child_completed(tx, &parent_id, &id, operation, at)?;
    }
    Ok(())
}

/// Recoverable outbox: only the exact auto-generated revision is authorized for publication.
pub(crate) fn publish_pending(db: &std::path::Path, vault: &std::path::Path) -> Result<(), String> {
    static PUBLISH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = PUBLISH_LOCK.lock().map_err(|e| e.to_string())?;
    let c = super::database::open(db)?;
    let mut query = c.prepare("SELECT task_id,revision,request_json FROM task_auto_publications WHERE state='pending' ORDER BY task_id,revision").map_err(|e|e.to_string())?;
    let pending = query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(query);
    let mut blocked = std::collections::HashSet::new();
    for (task, revision, request) in pending {
        if blocked.contains(&task) {
            continue;
        }
        let request: Value = serde_json::from_str(&request).map_err(|e| e.to_string())?;
        match super::task_assistance::publish_reviewed_knowledge(db, vault, &request) {
            Ok(_) => {
                c.execute("UPDATE task_auto_publications SET state='published',error='' WHERE task_id=? AND revision=?",params![task,revision]).map_err(|e|e.to_string())?;
            }
            Err(error) => {
                c.execute(
                    "UPDATE task_auto_publications SET error=? WHERE task_id=? AND revision=?",
                    params![error, task, revision],
                )
                .map_err(|e| e.to_string())?;
                blocked.insert(task);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::task_service::TaskApplicationService;

    #[test]
    fn completion_rollup_is_atomic_cumulative_idempotent_and_reopens() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.db");
        super::super::database::initialize(&db).unwrap();
        let service = TaskApplicationService::new(&db);
        let create = |name: &str| {
            service
                .execute(
                    "task.create",
                    &json!({"operationId":name,"title":name,"inputText":name}),
                )
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let grandparent = create("Grandparent");
        let parent = create("Parent");
        let first = create("First");
        let second = create("Second");
        for task in [&parent, &first, &second] {
            service.execute("task.transition",&json!({"operationId":format!("start-{task}"),"taskId":task,"expectedTaskRevision":1,"to":"in_progress"})).unwrap();
        }
        let c = super::super::database::open(&db).unwrap();
        for (child, parent) in [
            (&parent, &grandparent),
            (&first, &parent),
            (&second, &parent),
        ] {
            c.execute(
                "INSERT INTO task_subtasks VALUES(?,?,'Separate scope','2026-01-01')",
                params![child, parent],
            )
            .unwrap();
        }
        let complete = |id: &str, operation: &str| {
            service.execute("task.completion.create",&json!({"operationId":operation,"taskId":id,"expectedTaskRevision":1,"evidence":format!("Evidence for {id}")}))
        };
        complete(&first, "first-done").unwrap();
        let partial = service
            .execute("task.get", &json!({"taskId":parent}))
            .unwrap();
        assert_eq!(partial["state"], "in_progress");
        assert_eq!(partial["publication"]["draftRevision"], 1);
        assert!(partial.get("completion").is_none());
        assert!(complete(&parent, "premature")
            .unwrap_err()
            .contains("all Subtasks"));
        complete(&first, "first-done").unwrap();
        assert_eq!(
            service
                .execute("task.get", &json!({"taskId":parent}))
                .unwrap()["publication"]["draftRevision"],
            1
        );
        // A draft failure rolls back both the initiating child and parent completion.
        c.execute_batch("CREATE TRIGGER fail_aggregate BEFORE INSERT ON task_knowledge_drafts BEGIN SELECT RAISE(ABORT,'injected draft failure'); END;").unwrap();
        assert!(complete(&second, "second-done").is_err());
        assert_eq!(
            service
                .execute("task.get", &json!({"taskId":second}))
                .unwrap()["state"],
            "in_progress"
        );
        c.execute_batch("DROP TRIGGER fail_aggregate;").unwrap();
        complete(&second, "second-done").unwrap();
        let done = service
            .execute("task.get", &json!({"taskId":parent}))
            .unwrap();
        assert_eq!(done["state"], "completed");
        assert_eq!(done["publication"]["draftRevision"], 2);
        let body = done["publication"]["bodyMarkdown"].as_str().unwrap();
        assert!(body.contains(&first) && body.contains(&second));
        assert_eq!(
            service
                .execute("task.get", &json!({"taskId":grandparent}))
                .unwrap()["state"],
            "completed"
        );
        service
            .execute(
                "task.reopen",
                &json!({"operationId":"reopen","taskId":first,"expectedTaskRevision":1}),
            )
            .unwrap();
        for task in [&parent, &grandparent] {
            assert_eq!(
                service
                    .execute("task.get", &json!({"taskId":task}))
                    .unwrap()["state"],
                "in_progress"
            );
        }
        complete(&first, "first-done-again").unwrap();
        assert_eq!(
            service
                .execute("task.get", &json!({"taskId":parent}))
                .unwrap()["publication"]["draftRevision"],
            3
        );
        let vault = root.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        publish_pending(&db, &vault).unwrap();
        let published = service
            .execute("task.get", &json!({"taskId":parent}))
            .unwrap();
        assert_eq!(published["publication"]["state"], "published");
        let path: String = c.query_row("SELECT path FROM task_knowledge_drafts WHERE task_id=? ORDER BY revision DESC LIMIT 1",[&parent],|r|r.get(0)).unwrap();
        let document = std::fs::read_to_string(vault.join(&path)).unwrap();
        assert!(document.contains("llm_wiki_draft_revision: 3"));
        publish_pending(&db, &vault).unwrap();
        assert_eq!(
            document,
            std::fs::read_to_string(vault.join(&path)).unwrap()
        );
        // External edits survive a failed automatic update; the outbox remains retryable.
        std::fs::write(vault.join(&path), "External edit").unwrap();
        service
            .execute(
                "task.reopen",
                &json!({"operationId":"reopen-next","taskId":first,"expectedTaskRevision":1}),
            )
            .unwrap();
        complete(&first, "first-done-next").unwrap();
        publish_pending(&db, &vault).unwrap();
        assert_eq!(
            std::fs::read_to_string(vault.join(&path)).unwrap(),
            "External edit"
        );
        assert!(service
            .execute("task.get", &json!({"taskId":parent}))
            .unwrap()["autoPublicationError"]
            .as_str()
            .unwrap()
            .contains("source_changed"));
        // Startup cannot publish over the external edit. After it is restored,
        // opening Task detail must retry through the real command boundary.
        let app = super::super::NativeApplication::isolated(&vault, &db).unwrap();
        std::fs::write(vault.join(&path), &document).unwrap();
        let refreshed = app.execute(super::super::NativeOperation {
            name: "task.get".into(),
            input: json!({"taskId":parent}),
        });
        assert_eq!(refreshed.status, 200);
        assert_eq!(refreshed.body["publication"]["state"], "published");
        let updated = std::fs::read_to_string(vault.join(&path)).unwrap();
        app.execute(super::super::NativeOperation {
            name: "task.get".into(),
            input: json!({"taskId":parent}),
        });
        assert_eq!(updated, std::fs::read_to_string(vault.join(&path)).unwrap());
    }
}
