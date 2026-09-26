//! Atomic publication and targeted invalidation for Task Distillation projections.

use crate::application::task_distillation_service::{
    content_hash, source_set_hash, validate_result, DistillationInput, DistillationResult,
    RESULT_SCHEMA_VERSION,
};
use crate::native::workflow_foundation::{
    record_document_references, record_job_application_disposition, record_version_provenance,
};
use crate::workflow_foundation::{
    ApplicationDisposition, PromptId, SourceAttribution, VersionOwner, VersionProvenance,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap, VecDeque};

pub fn terminal_run_for_repair(
    connection: &Connection,
    task_id: &str,
    run_id: Option<&str>,
) -> Result<String, String> {
    if run_id.is_some_and(|id| id.trim().is_empty()) {
        return Err("runId must not be empty".into());
    }
    connection.query_row("SELECT id FROM task_work_session_runs WHERE task_id=? AND (? IS NULL OR id=?) AND status IN ('succeeded','failed','cancelled','interrupted','needs_attention') ORDER BY finished_at DESC,created_at DESC,id DESC LIMIT 1",params![task_id,run_id,run_id],|row|row.get(0)).optional().map_err(|e|e.to_string())?.ok_or_else(||"No completed Run is available to repair for this Task".into())
}

fn owner(input: &DistillationInput) -> Result<(&'static str, String), String> {
    match input.projection_kind.as_str() {
        "work_log" => Ok((
            "work_log",
            input
                .owner
                .work_log_entry_id
                .clone()
                .ok_or("work_log projection requires workLogEntryId")?,
        )),
        "task_journey" => Ok(("task_journey", input.owner.task_id.clone())),
        _ => Err("unknown projection kind".into()),
    }
}

fn source_attributions(result: &DistillationResult) -> Vec<SourceAttribution> {
    let mut unique = BTreeSet::new();
    let mut output = Vec::new();
    for claim in &result.claims {
        for source in claim.sources.iter().chain(&claim.contradicted_by) {
            let key = format!(
                "{}\0{}\0{}\0{}\0{}",
                source.source_type, source.id, source.revision, source.locator, claim.id
            );
            if unique.insert(key) {
                output.push(SourceAttribution {
                    document_id: format!("{}:{}", source.source_type, source.id),
                    document_version: source.revision.clone(),
                    section: Some(source.locator.clone()),
                    excerpt: Some(source.quote.chars().take(240).collect()),
                    claim_id: Some(claim.id.clone()),
                });
            }
        }
    }
    output
}

fn derived_dependencies(
    result: &DistillationResult,
) -> Vec<(
    crate::application::task_distillation_service::SourceCitation,
    String,
    String,
)> {
    let mut output = Vec::new();
    for claim in &result.claims {
        for source in claim.sources.iter().chain(&claim.contradicted_by) {
            output.push((source.clone(), claim.id.clone(), String::new()));
        }
    }
    for change in &result.node_changes {
        for source in change.sources.iter().chain(&change.detail.links) {
            for claim in &change.claim_ids {
                output.push((source.clone(), claim.clone(), String::new()));
            }
        }
    }
    for relationship in &result.relationship_changes {
        for source in &relationship.sources {
            output.push((source.clone(), String::new(), relationship.id.clone()));
        }
    }
    for topic in &result.topic_states {
        for source in &topic.sources {
            output.push((source.clone(), String::new(), String::new()));
        }
    }
    for snapshot in &result.completion_snapshots {
        for source in &snapshot.sources {
            for claim in &snapshot.claim_ids {
                output.push((source.clone(), claim.clone(), String::new()));
            }
        }
    }
    output.sort_by(|a, b| {
        (
            &a.0.source_type,
            &a.0.id,
            &a.0.revision,
            &a.0.locator,
            &a.1,
            &a.2,
        )
            .cmp(&(
                &b.0.source_type,
                &b.0.id,
                &b.0.revision,
                &b.0.locator,
                &b.1,
                &b.2,
            ))
    });
    output.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2);
    output
}

fn snapshot_node(
    tx: &Transaction<'_>,
    task_id: &str,
    node_id: &str,
    projection_revision: i64,
) -> Result<(), String> {
    let row = tx.query_row(
        "SELECT revision,kind,topic_key,status,claim_ids_json,detail_json,source_set_hash,active FROM task_distillation_nodes WHERE task_id=? AND node_id=?",
        params![task_id,node_id], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<String>>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,i64>(7)?)),
    ).optional().map_err(|error|error.to_string())?;
    if let Some((revision, kind, topic, status, claims, detail, source_hash, active)) = row {
        let snapshot = json!({"nodeId":node_id,"revision":revision,"kind":kind,"topicKey":topic,"status":status,"claimIds":serde_json::from_str::<Value>(&claims).unwrap_or(json!([])),"detail":serde_json::from_str::<Value>(&detail).unwrap_or(json!({})),"sourceSetHash":source_hash,"active":active==1});
        tx.execute("INSERT OR IGNORE INTO task_distillation_node_history(task_id,node_id,revision,snapshot_json,projection_revision) VALUES(?,?,?,?,?)",params![task_id,node_id,revision,snapshot.to_string(),projection_revision]).map_err(|error|error.to_string())?;
    }
    Ok(())
}

fn check_expected_targets(
    tx: &Transaction<'_>,
    task_id: &str,
    targets: &HashMap<String, i64>,
) -> Result<(), String> {
    for (node_id, expected) in targets {
        let actual = tx
            .query_row(
                "SELECT revision FROM task_distillation_nodes WHERE task_id=? AND node_id=? AND active=1",
                params![task_id, node_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("unknown semantic target {node_id}"))?;
        if actual != *expected {
            return Err(format!(
                "stale_projection: target {node_id} expected revision {expected}, found {actual}"
            ));
        }
    }
    Ok(())
}

fn apply_semantics(
    tx: &Transaction<'_>,
    input: &DistillationInput,
    result: &DistillationResult,
    projection_revision: i64,
) -> Result<(), String> {
    let task = &input.owner.task_id;
    let claims = result
        .claims
        .iter()
        .map(|claim| (claim.id.as_str(), claim))
        .collect::<HashMap<_, _>>();
    for change in &result.node_changes {
        let expected = change
            .expected_target_revisions
            .iter()
            .map(|(key, value)| (key.clone(), *value))
            .collect::<HashMap<_, _>>();
        check_expected_targets(tx, task, &expected)?;
        match change.action.as_str() {
            "omit" => continue,
            "add" | "supersede" => {
                let first_claim = change
                    .claim_ids
                    .iter()
                    .find_map(|id| claims.get(id.as_str()))
                    .ok_or("new semantic node requires a known claim")?;
                let exists=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_distillation_nodes WHERE task_id=? AND node_id=?)",params![task,&change.candidate_id],|row|row.get::<_,bool>(0)).map_err(|error|error.to_string())?;
                if exists {
                    return Err(format!(
                        "semantic node {} already exists; use enrich with its expected revision",
                        change.candidate_id
                    ));
                } else {
                    tx.execute("INSERT INTO task_distillation_nodes(task_id,node_id,revision,kind,topic_key,status,claim_ids_json,detail_json,source_set_hash,active) VALUES(?,?,1,?,?,?,?,?,?,1)",params![task,&change.candidate_id,&first_claim.kind,&first_claim.topic_key,&change.detail.current_status,serde_json::to_string(&change.claim_ids).map_err(|e|e.to_string())?,serde_json::to_string(&change.detail).map_err(|e|e.to_string())?,&input.source_set_hash]).map_err(|error|error.to_string())?;
                }
                if change.action == "supersede" {
                    for target in &change.target_ids {
                        snapshot_node(tx, task, target, projection_revision)?;
                        tx.execute("UPDATE task_distillation_nodes SET revision=revision+1,status='superseded',active=0,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND node_id=?",params![task,target]).map_err(|error|error.to_string())?;
                    }
                }
            }
            "enrich" => {
                let target = &change.target_ids[0];
                snapshot_node(tx, task, target, projection_revision)?;
                tx.execute("UPDATE task_distillation_nodes SET revision=revision+1,claim_ids_json=?,detail_json=?,source_set_hash=?,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND node_id=?",params![serde_json::to_string(&change.claim_ids).map_err(|e|e.to_string())?,serde_json::to_string(&change.detail).map_err(|e|e.to_string())?,&input.source_set_hash,task,target]).map_err(|error|error.to_string())?;
            }
            "merge" => {
                let canonical = &change.target_ids[0];
                snapshot_node(tx, task, canonical, projection_revision)?;
                tx.execute("UPDATE task_distillation_nodes SET revision=revision+1,claim_ids_json=?,detail_json=?,source_set_hash=?,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND node_id=?",params![serde_json::to_string(&change.claim_ids).map_err(|e|e.to_string())?,serde_json::to_string(&change.detail).map_err(|e|e.to_string())?,&input.source_set_hash,task,canonical]).map_err(|error|error.to_string())?;
                for alias in change
                    .target_ids
                    .iter()
                    .skip(1)
                    .chain(std::iter::once(&change.candidate_id))
                {
                    if alias != canonical {
                        tx.execute("INSERT INTO task_distillation_redirects(task_id,alias_node_id,canonical_node_id,projection_revision) VALUES(?,?,?,?) ON CONFLICT(task_id,alias_node_id) DO UPDATE SET canonical_node_id=excluded.canonical_node_id,projection_revision=excluded.projection_revision",params![task,alias,canonical,projection_revision]).map_err(|error|error.to_string())?;
                        tx.execute("UPDATE task_distillation_nodes SET active=0,status='historical',updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND node_id=?",params![task,alias]).map_err(|error|error.to_string())?;
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    for relationship in &result.relationship_changes {
        for endpoint in [&relationship.from, &relationship.to] {
            let exists=tx.query_row("SELECT EXISTS(SELECT 1 FROM task_distillation_nodes WHERE task_id=? AND node_id=?)",params![task,endpoint],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())?;
            if !exists {
                return Err("relationship references unknown semantic node".into());
            }
        }
        tx.execute("INSERT INTO task_distillation_relationships(task_id,relationship_id,kind,from_node_id,to_node_id,reason,sources_json,active,projection_revision) VALUES(?,?,?,?,?,?,?,1,?) ON CONFLICT(task_id,relationship_id) DO UPDATE SET kind=excluded.kind,from_node_id=excluded.from_node_id,to_node_id=excluded.to_node_id,reason=excluded.reason,sources_json=excluded.sources_json,active=1,projection_revision=excluded.projection_revision",params![task,&relationship.id,&relationship.kind,&relationship.from,&relationship.to,&relationship.reason,serde_json::to_string(&relationship.sources).map_err(|e|e.to_string())?,projection_revision]).map_err(|error|error.to_string())?;
    }
    for topic in &result.topic_states {
        let prior:Option<String>=tx.query_row("SELECT current_node_id FROM task_distillation_topics WHERE task_id=? AND topic_key=?",params![task,&topic.topic_key],|row|row.get(0)).optional().map_err(|e|e.to_string())?.flatten();
        if let Some(prior) = prior.filter(|id| Some(id) != topic.current_node_id.as_ref()) {
            if !result.node_changes.iter().any(|change| {
                change.target_ids.contains(&prior)
                    && change.expected_target_revisions.contains_key(&prior)
            }) {
                return Err("topic replacement requires an explicit target revision".into());
            }
        }
        tx.execute("INSERT INTO task_distillation_topics(task_id,topic_key,status,current_node_id,replacement_node_id,sources_json,projection_revision) VALUES(?,?,?,?,?,?,?) ON CONFLICT(task_id,topic_key) DO UPDATE SET status=excluded.status,current_node_id=excluded.current_node_id,replacement_node_id=excluded.replacement_node_id,sources_json=excluded.sources_json,projection_revision=excluded.projection_revision",params![task,&topic.topic_key,&topic.status,&topic.current_node_id,&topic.replacement_node_id,serde_json::to_string(&topic.sources).map_err(|e|e.to_string())?,projection_revision]).map_err(|error|error.to_string())?;
    }
    for snapshot in &result.completion_snapshots {
        if snapshot.status == "current" {
            tx.execute("UPDATE task_distillation_completion_snapshots SET status='superseded_by_later_completion' WHERE task_id=? AND status='current' AND snapshot_id<>?",params![task,&snapshot.id]).map_err(|error|error.to_string())?;
        }
        tx.execute("INSERT OR IGNORE INTO task_distillation_completion_snapshots(task_id,snapshot_id,completion_id,completion_revision,status,snapshot_json,projection_revision) VALUES(?,?,?,?,?,?,?)",params![task,&snapshot.id,&snapshot.completion_id,&snapshot.completion_revision,&snapshot.status,serde_json::to_string(snapshot).map_err(|e|e.to_string())?,projection_revision]).map_err(|error|error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
pub fn publish(
    connection: &mut Connection,
    job_id: &str,
    input: &DistillationInput,
    result: &DistillationResult,
    prompt_id: PromptId,
    prompt_version: u32,
) -> Result<Value, String> {
    publish_inner(
        connection,
        job_id,
        input,
        result,
        prompt_id,
        prompt_version,
        None,
    )
}

pub fn publish_for_worker(
    connection: &mut Connection,
    job_id: &str,
    input: &DistillationInput,
    result: &DistillationResult,
    prompt_id: PromptId,
    prompt_version: u32,
    worker: &str,
) -> Result<Value, String> {
    publish_inner(
        connection,
        job_id,
        input,
        result,
        prompt_id,
        prompt_version,
        Some(worker),
    )
}

fn publish_inner(
    connection: &mut Connection,
    job_id: &str,
    input: &DistillationInput,
    result: &DistillationResult,
    prompt_id: PromptId,
    prompt_version: u32,
    expected_worker: Option<&str>,
) -> Result<Value, String> {
    validate_result(input, result)?;
    let (owner_type, owner_id) = owner(input)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    if let Some(revision)=tx.query_row("SELECT revision FROM task_distillation_revisions WHERE job_id=? AND owner_type=? AND owner_id=?",params![job_id,owner_type,&owner_id],|row|row.get::<_,i64>(0)).optional().map_err(|error|error.to_string())? {
        return Ok(json!({"ownerType":owner_type,"ownerId":owner_id,"revision":revision,"freshness":"current","applicationDisposition":"applied","replayed":true}));
    }
    let status = tx
        .query_row(
            "SELECT status FROM ai_jobs_v2 WHERE id=?",
            [job_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if status.as_deref() != Some("running") {
        return Err("stale_job: publication requires the active running job".into());
    }
    if let Some(worker) = expected_worker {
        let owns:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ai_jobs_v2 WHERE id=? AND worker_id=? AND status='running')",params![job_id,worker],|row|row.get(0)).map_err(|e|e.to_string())?;
        if !owns {
            return Err("stale_job: worker no longer owns publication".into());
        }
    }
    let task_head = tx
        .query_row(
            "SELECT current_revision FROM tasks WHERE id=? AND NOT EXISTS(SELECT 1 FROM deleted_entities WHERE entity_type='tasks' AND entity_id=tasks.id)",
            [&input.owner.task_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if task_head != Some(input.owner.task_revision) {
        let actual = format!(
            "task-head:{}",
            task_head.map_or_else(|| "deleted".into(), |value| value.to_string())
        );
        record_job_application_disposition(
            &tx,
            job_id,
            &input.source_set_hash,
            &actual,
            ApplicationDisposition::Superseded,
        )?;
        tx.execute("UPDATE ai_jobs_v2 SET status='stale',execution_outcome='succeeded',finished_at=CURRENT_TIMESTAMP WHERE id=?",[job_id]).map_err(|error|error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        return Err(format!(
            "stale_source: Task head changed from revision {}",
            input.owner.task_revision
        ));
    }
    if let (Some(run_id), Some(expected_run)) = (&input.owner.run_id, input.owner.run_revision) {
        let run_head = tx
            .query_row(
                "SELECT revision FROM task_work_session_runs WHERE id=?",
                [run_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if run_head != Some(expected_run) {
            let actual = format!(
                "run-head:{}",
                run_head.map_or_else(|| "deleted".into(), |value| value.to_string())
            );
            record_job_application_disposition(
                &tx,
                job_id,
                &input.source_set_hash,
                &actual,
                ApplicationDisposition::Superseded,
            )?;
            tx.execute("UPDATE ai_jobs_v2 SET status='stale',execution_outcome='succeeded',finished_at=CURRENT_TIMESTAMP WHERE id=?",[job_id]).map_err(|error|error.to_string())?;
            tx.commit().map_err(|error| error.to_string())?;
            return Err(format!(
                "stale_source: Run head changed from revision {expected_run}"
            ));
        }
    }
    let actual_source_hash = actual_source_set_hash(&tx, input)?;
    if actual_source_hash != input.source_set_hash {
        record_job_application_disposition(
            &tx,
            job_id,
            &input.source_set_hash,
            &actual_source_hash,
            ApplicationDisposition::Superseded,
        )?;
        tx.execute("UPDATE ai_jobs_v2 SET status='stale',execution_outcome='succeeded',finished_at=CURRENT_TIMESTAMP WHERE id=?",[job_id]).map_err(|error|error.to_string())?;
        tx.commit().map_err(|error| error.to_string())?;
        return Err("stale_source: exact source content changed before publication".into());
    }
    let current = tx
        .query_row(
            "SELECT revision FROM task_distillation_current WHERE owner_type=? AND owner_id=?",
            params![owner_type, &owner_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .unwrap_or(0);
    if current != input.expected_projection_revision {
        return Err(format!(
            "stale_projection: expected {}, found {current}",
            input.expected_projection_revision
        ));
    }
    for snapshot in &result.completion_snapshots {
        let (state, latest,reopened):(String,Option<String>,Option<String>)=tx.query_row("SELECT state,(SELECT id FROM task_completions WHERE task_id=tasks.id ORDER BY rowid DESC LIMIT 1),reopened_at FROM tasks WHERE id=?",[&input.owner.task_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(|e|e.to_string())?;
        let is_latest = latest.as_deref() == Some(snapshot.completion_id.as_str());
        let correct = match snapshot.status.as_str() {
            "current" => state == "completed" && is_latest,
            "superseded_by_later_completion" => !is_latest && latest.is_some(),
            "historical_after_reopen" => state != "completed" && is_latest && reopened.is_some(),
            _ => false,
        };
        if !correct {
            return Err("completion finality disagrees with canonical Task completion".into());
        }
    }
    let revision = current + 1;
    for change in &result.node_changes {
        check_expected_targets(
            &tx,
            &input.owner.task_id,
            &change
                .expected_target_revisions
                .iter()
                .map(|(id, revision)| (id.clone(), *revision))
                .collect(),
        )?;
    }
    if owner_type == "task_journey" {
        apply_semantics(&tx, input, result, revision)?;
    }
    tx.execute("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,run_id,work_log_entry_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,'current',?)",params![owner_type,&owner_id,revision,&input.owner.task_id,&input.owner.run_id,&input.owner.work_log_entry_id,&input.source_set_hash,prompt_id.as_str(),prompt_version,&input.rules_version,RESULT_SCHEMA_VERSION,&input.locale,serde_json::to_string(result).map_err(|e|e.to_string())?,job_id]).map_err(|error|error.to_string())?;
    tx.execute("INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES(?,?,?,?,'current') ON CONFLICT(owner_type,owner_id) DO UPDATE SET revision=excluded.revision,source_set_hash=excluded.source_set_hash,freshness='current',updated_at=CURRENT_TIMESTAMP",params![owner_type,&owner_id,revision,&input.source_set_hash]).map_err(|error|error.to_string())?;
    for (source, claim, relationship) in derived_dependencies(result) {
        tx.execute("INSERT OR IGNORE INTO task_distillation_dependencies(owner_type,owner_id,projection_revision,source_type,source_id,source_revision,locator,claim_id,relationship_id) VALUES(?,?,?,?,?,?,?,?,?)",params![owner_type,&owner_id,revision,&source.source_type,&source.id,&source.revision,&source.locator,claim,relationship]).map_err(|error|error.to_string())?;
    }
    let references = source_attributions(result);
    let version_owner = VersionOwner {
        entity_type: "task_distillation_revisions".into(),
        entity_id: format!("{owner_type}:{owner_id}"),
        version: revision.to_string(),
    };
    record_version_provenance(
        &tx,
        &VersionProvenance {
            owner: version_owner.clone(),
            source: None,
            prompt_id: Some(prompt_id),
            prompt_version: Some(prompt_version),
            operation_id: Some(job_id.into()),
            restored_from_version: None,
            source_references: references.clone(),
        },
    )?;
    record_document_references(&tx, &version_owner, &references)?;
    let disposition = record_job_application_disposition(
        &tx,
        job_id,
        &input.source_set_hash,
        &input.source_set_hash,
        ApplicationDisposition::Applied,
    )?;
    tx.commit().map_err(|error| error.to_string())?;
    Ok(
        json!({"ownerType":owner_type,"ownerId":owner_id,"revision":revision,"freshness":"current","applicationDisposition":disposition.as_str()}),
    )
}

fn actual_source_set_hash(
    connection: &Connection,
    input: &DistillationInput,
) -> Result<String, String> {
    // New immutable decisions/completions also invalidate an in-flight result:
    // their insertion need not advance the Task definition revision.
    for (source_type, table) in [
        ("task_decision", "task_decisions"),
        ("task_completion", "task_completions"),
    ] {
        let sql =
            format!("SELECT id,COALESCE(task_revision,0) FROM {table} WHERE task_id=? ORDER BY id");
        let current = connection
            .prepare(&sql)
            .map_err(|e| e.to_string())?
            .query_map([&input.owner.task_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?.to_string()))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|e| e.to_string())?;
        let expected = input
            .sources
            .iter()
            .filter(|s| s.source_type == source_type)
            .map(|s| (s.id.clone(), s.revision.clone()))
            .collect::<BTreeSet<_>>();
        if current != expected {
            return Ok(content_hash(&format!("changed {source_type}: {current:?}")));
        }
    }
    if let Some(run) = &input.owner.run_id {
        let rows=connection.prepare("SELECT provider_item_id,kind,content_json FROM task_work_session_run_items WHERE run_id=? AND status='completed'").map_err(|e|e.to_string())?.query_map([run],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        let current = rows
            .into_iter()
            .filter(|(_, kind, raw)| {
                kind == "fileChange"
                    || (kind == "commandExecution"
                        && serde_json::from_str::<Value>(raw)
                            .ok()
                            .and_then(|v| v["exitCode"].as_i64())
                            .is_some())
            })
            .map(|(id, _, _)| id)
            .collect::<BTreeSet<_>>();
        let expected = input
            .sources
            .iter()
            .filter(|s| s.source_type == "run_completed_item")
            .map(|s| s.id.clone())
            .collect::<BTreeSet<_>>();
        if current != expected {
            return Ok(content_hash(&format!(
                "changed completed evidence: {current:?}"
            )));
        }
    }
    let mut all = input
        .primary_source
        .iter()
        .chain(&input.sources)
        .cloned()
        .collect::<Vec<_>>();
    for source in &mut all {
        let raw=match source.source_type.as_str(){
            "run_final_report"=>connection.query_row("SELECT final_report FROM task_work_session_runs WHERE id=? AND revision=?",params![&source.id,source.revision.parse::<i64>().unwrap_or(-1)],|row|row.get::<_,Option<String>>(0)).optional().map_err(|e|e.to_string())?.flatten(),
            "run_completed_item"=>connection.query_row("SELECT content_json FROM task_work_session_run_items WHERE provider_item_id=? AND run_id=?",params![&source.id,input.owner.run_id.as_deref().unwrap_or("")],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())?,
            "task_revision"=>connection.query_row("SELECT json_object('title',r.title,'detail',r.detail,'outcome',r.outcome,'scope',r.scope,'nonGoals',r.non_goals,'validationCriteria',r.validation_criteria,'author',r.author,'state',t.state) FROM task_revisions r JOIN tasks t ON t.id=r.task_id WHERE r.task_id=? AND r.revision=?",params![&source.id,source.revision.parse::<i64>().unwrap_or(-1)],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())?,
            "task_decision"=>connection.query_row("SELECT kind||': '||payload_json FROM task_decisions WHERE id=? AND task_id=? AND COALESCE(task_revision,0)=?",params![&source.id,&input.owner.task_id,source.revision.parse::<i64>().unwrap_or(-1)],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())?,
            "task_completion"=>connection.query_row("SELECT 'Evidence: '||evidence||char(10)||'Report: '||report FROM task_completions WHERE id=? AND task_id=? AND task_revision=?",params![&source.id,&input.owner.task_id,source.revision.parse::<i64>().unwrap_or(-1)],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())?,
            _=>None,
        };
        source.content_hash = raw
            .as_deref()
            .map(content_hash)
            .unwrap_or_else(|| "deleted".into());
    }
    let primary = if input.primary_source.is_some() {
        all.first()
    } else {
        None
    };
    let sources = if primary.is_some() {
        &all[1..]
    } else {
        &all[..]
    };
    Ok(source_set_hash(primary, sources))
}

pub fn current(
    connection: &Connection,
    owner_type: &str,
    owner_id: &str,
) -> Result<Option<Value>, String> {
    connection.query_row("SELECT r.revision,r.source_set_hash,c.freshness,r.prompt_id,r.prompt_version,r.rules_version,r.result_schema_version,r.locale,r.result_json,r.created_at,r.job_id FROM task_distillation_current c JOIN task_distillation_revisions r USING(owner_type,owner_id,revision) WHERE c.owner_type=? AND c.owner_id=?",params![owner_type,owner_id],|row|Ok(json!({"projectionRevision":row.get::<_,i64>(0)?,"sourceSetHash":row.get::<_,String>(1)?,"freshness":row.get::<_,String>(2)?,"prompt":{"id":row.get::<_,String>(3)?,"version":row.get::<_,i64>(4)?},"rulesVersion":row.get::<_,String>(5)?,"resultSchemaVersion":row.get::<_,String>(6)?,"locale":row.get::<_,String>(7)?,"result":serde_json::from_str::<Value>(&row.get::<_,String>(8)?).unwrap_or(json!({})),"createdAt":row.get::<_,String>(9)?,"jobId":row.get::<_,String>(10)?}))).optional().map_err(|error|error.to_string())
}

pub fn task_projection(connection: &Connection, task_id: &str) -> Result<Option<Value>, String> {
    let Some(mut projection) = current(connection, "task_journey", task_id)? else {
        return Ok(None);
    };
    let nodes=connection.prepare("SELECT node_id,revision,kind,topic_key,status,claim_ids_json,detail_json,active FROM task_distillation_nodes WHERE task_id=? ORDER BY active DESC,updated_at,node_id").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"revision":row.get::<_,i64>(1)?,"kind":row.get::<_,String>(2)?,"topicKey":row.get::<_,Option<String>>(3)?,"status":row.get::<_,String>(4)?,"claimIds":serde_json::from_str::<Value>(&row.get::<_,String>(5)?).unwrap_or(json!([])),"detail":serde_json::from_str::<Value>(&row.get::<_,String>(6)?).unwrap_or(json!({})),"active":row.get::<_,i64>(7)?==1}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let relationships=connection.prepare("SELECT relationship_id,kind,from_node_id,to_node_id,reason,sources_json,active FROM task_distillation_relationships WHERE task_id=? ORDER BY projection_revision,relationship_id").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"kind":row.get::<_,String>(1)?,"from":row.get::<_,String>(2)?,"to":row.get::<_,String>(3)?,"reason":row.get::<_,Option<String>>(4)?,"sources":serde_json::from_str::<Value>(&row.get::<_,String>(5)?).unwrap_or(json!([])),"active":row.get::<_,i64>(6)?==1}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let topics=connection.prepare("SELECT topic_key,status,current_node_id,replacement_node_id,sources_json FROM task_distillation_topics WHERE task_id=? ORDER BY topic_key").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok(json!({"topicKey":row.get::<_,String>(0)?,"status":row.get::<_,String>(1)?,"currentNodeId":row.get::<_,Option<String>>(2)?,"replacementNodeId":row.get::<_,Option<String>>(3)?,"sources":serde_json::from_str::<Value>(&row.get::<_,String>(4)?).unwrap_or(json!([]))}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let snapshots=connection.prepare("SELECT status,snapshot_json FROM task_distillation_completion_snapshots WHERE task_id=? ORDER BY projection_revision,snapshot_id").map_err(|e|e.to_string())?.query_map([task_id],|row|{let status=row.get::<_,String>(0)?;let mut value=serde_json::from_str::<Value>(&row.get::<_,String>(1)?).unwrap_or(json!({}));value["status"]=json!(status);Ok(value)}).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    projection["nodes"] = json!(nodes);
    projection["relationships"] = json!(relationships);
    projection["topicStates"] = json!(topics);
    projection["completionSnapshots"] = json!(snapshots);
    Ok(Some(projection))
}

pub fn mark_source_changed(
    connection: &Connection,
    source_type: &str,
    source_id: &str,
) -> Result<Vec<Value>, String> {
    let mut statement=connection.prepare("SELECT DISTINCT owner_type,owner_id,projection_revision FROM task_distillation_dependencies WHERE source_type=? AND source_id=?").map_err(|error|error.to_string())?;
    let affected = statement
        .query_map(params![source_type, source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    let mut values = Vec::new();
    for (owner_type, owner_id, revision) in affected {
        connection.execute("UPDATE task_distillation_current SET freshness='stale',updated_at=CURRENT_TIMESTAMP WHERE owner_type=? AND owner_id=? AND revision=?",params![&owner_type,&owner_id,revision]).map_err(|error|error.to_string())?;
        values
            .push(json!({"ownerType":owner_type,"ownerId":owner_id,"projectionRevision":revision}));
    }
    Ok(values)
}

pub fn mark_task_changed(connection: &Connection, task_id: &str) -> Result<(), String> {
    connection.execute("UPDATE task_distillation_current SET freshness='stale',updated_at=CURRENT_TIMESTAMP WHERE owner_type='task_journey' AND owner_id=?",[task_id]).map_err(|error|error.to_string())?;
    let mut statement=connection.prepare("SELECT owner_id FROM task_distillation_revisions WHERE owner_type='work_log' AND task_id=? GROUP BY owner_id").map_err(|error|error.to_string())?;
    let owners = statement
        .query_map([task_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for owner in owners {
        connection.execute("UPDATE task_distillation_current SET freshness='stale',updated_at=CURRENT_TIMESTAMP WHERE owner_type='work_log' AND owner_id=?",[owner]).map_err(|error|error.to_string())?;
    }
    Ok(())
}

pub fn mark_task_reopened(connection: &Connection, task_id: &str) -> Result<(), String> {
    connection.execute("UPDATE task_distillation_completion_snapshots SET status='historical_after_reopen' WHERE task_id=? AND status='current'",[task_id]).map_err(|error|error.to_string())?;
    mark_task_changed(connection, task_id)
}

pub fn affected_subgraph(
    connection: &Connection,
    task_id: &str,
    source_type: &str,
    source_id: &str,
) -> Result<Vec<String>, String> {
    let mut seed=connection.prepare("SELECT DISTINCT d.claim_id FROM task_distillation_dependencies d WHERE d.owner_type='task_journey' AND d.owner_id=? AND d.source_type=? AND d.source_id=? AND d.claim_id<>''").map_err(|e|e.to_string())?.query_map(params![task_id,source_type,source_id],|row|row.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut nodes=connection.prepare("SELECT node_id,claim_ids_json FROM task_distillation_nodes WHERE task_id=? AND active=1").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?.into_iter().filter_map(|(id,raw)|{let ids=serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default();ids.iter().any(|item|seed.contains(item)).then_some(id)}).collect::<BTreeSet<_>>();
    let relationships=connection.prepare("SELECT from_node_id,to_node_id FROM task_distillation_relationships WHERE task_id=? AND active=1").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mut queue = nodes.iter().cloned().collect::<VecDeque<_>>();
    while let Some(node) = queue.pop_front() {
        for (from, to) in &relationships {
            if from == &node && nodes.insert(to.clone()) {
                queue.push_back(to.clone());
            }
        }
    }
    seed.clear();
    Ok(nodes.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::task_distillation_service::*;
    use crate::native::{database, migrations};
    use std::collections::BTreeMap;

    fn fixture() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        DistillationInput,
        DistillationResult,
    ) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db.sqlite");
        database::initialize(&db).unwrap();
        let c = database::open(&db).unwrap();
        c.execute("INSERT INTO captures(id,text) VALUES('capture','raw')", [])
            .unwrap();
        c.execute("INSERT INTO tasks(id,origin_capture_id,created_at,last_user_activity_at) VALUES('task','capture',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_revisions(task_id,revision,title,content_hash,created_at) VALUES('task',1,'Export','hash',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log','task','original',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_sessions(id,task_id,title,created_at,updated_at) VALUES('session','task','Work',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES('entry','session','user','note','Inspect',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,final_report,revision,created_at,updated_at) VALUES('run','task','session','submission','hash','Inspect','entry','log','model','','context','succeeded','Implemented export',2,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",[]).unwrap();
        let owner = DistillationOwner {
            task_id: "task".into(),
            task_revision: 1,
            run_id: Some("run".into()),
            run_revision: Some(2),
            work_log_entry_id: Some("log".into()),
        };
        let mut input =
            build_run_input(owner, Some("Implemented export"), None, [], 0, "en").unwrap();
        input.projection_kind = "task_journey".into();
        input.source_set_hash = source_set_hash(input.primary_source.as_ref(), &input.sources);
        let source = input.primary_source.as_ref().unwrap();
        let citation = SourceCitation {
            source_type: source.source_type.clone(),
            id: source.id.clone(),
            revision: source.revision.clone(),
            locator: source.locator.clone(),
            quote: source.excerpt.clone(),
        };
        let claim = DistilledClaim {
            id: "claim".into(),
            kind: "result".into(),
            statement: "Implemented export".into(),
            actor: "ai".into(),
            epistemic_state: "performed".into(),
            status: "current".into(),
            topic_key: Some("export".into()),
            sources: vec![citation.clone()],
            contradicted_by: vec![],
        };
        let result = DistillationResult {
            claims: vec![claim],
            node_changes: vec![NodeChange {
                action: "add".into(),
                candidate_id: "node".into(),
                target_ids: vec![],
                expected_target_revisions: BTreeMap::new(),
                claim_ids: vec!["claim".into()],
                detail: PreparedDetail {
                    current_status: "unresolved".into(),
                    ..Default::default()
                },
                sources: vec![citation],
            }],
            work_log_view: WorkLogView {
                sections: vec![WorkLogSection {
                    kind: "outcome".into(),
                    claim_ids: vec!["claim".into()],
                }],
            },
            dependencies: vec![],
            ..Default::default()
        };
        (root, db, input, result)
    }

    fn job(c: &Connection, input: &DistillationInput, id: &str) {
        c.execute("INSERT INTO ai_jobs_v2(id,task_kind,entity_type,entity_id,status,source_revision,prompt_id,prompt_version) VALUES(?,'task_journey_increment','tasks','task','running',?,'task_journey_increment',1)",params![id,&input.source_set_hash]).unwrap();
    }

    #[test]
    fn publication_is_atomic_idempotent_and_preserves_originals() {
        let (_root, db, input, result) = fixture();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "job");
        let published = publish(
            &mut c,
            "job",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            1,
        )
        .unwrap();
        assert_eq!(published["revision"], 1);
        for _ in 0..100 {
            assert_eq!(
                publish(
                    &mut c,
                    "job",
                    &input,
                    &result,
                    PromptId::TaskJourneyIncrement,
                    1
                )
                .unwrap()["revision"],
                1
            );
        }
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM task_distillation_revisions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row(
                "SELECT body FROM task_work_log_entries WHERE id='log'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "original"
        );
        assert_eq!(
            c.query_row(
                "SELECT application_disposition FROM ai_jobs_v2 WHERE id='job'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "applied"
        );
    }

    #[test]
    fn source_change_marks_last_good_stale_and_targets_only_dependents() {
        let (_root, db, input, result) = fixture();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "job");
        publish(
            &mut c,
            "job",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            1,
        )
        .unwrap();
        assert_eq!(
            mark_source_changed(&c, "run_final_report", "run")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            current(&c, "task_journey", "task").unwrap().unwrap()["freshness"],
            "stale"
        );
        assert_eq!(
            affected_subgraph(&c, "task", "run_final_report", "run").unwrap(),
            vec!["node"]
        );
    }

    #[test]
    fn publication_rejects_a_changed_live_task_head_inside_transaction() {
        let (_root, db, input, result) = fixture();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "job");
        c.execute("UPDATE tasks SET current_revision=2 WHERE id='task'", [])
            .unwrap();
        assert!(publish(
            &mut c,
            "job",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            1
        )
        .unwrap_err()
        .contains("Task head changed"));
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM task_distillation_revisions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            c.query_row("SELECT status FROM ai_jobs_v2 WHERE id='job'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "stale"
        );
    }

    #[test]
    fn provenance_uses_the_exact_bound_quote() {
        let (_root, _db, _input, result) = fixture();
        let refs = source_attributions(&result);
        assert_eq!(refs[0].excerpt.as_deref(), Some("Implemented export"));
    }

    #[test]
    fn later_same_work_enriches_the_stable_node_and_preserves_unrelated_nodes() {
        let (_root, db, input, mut first) = fixture();
        canonicalize_semantic_ids(&input, &mut first).unwrap();
        let stable = first.node_changes[0].candidate_id.clone();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "job-1");
        publish(
            &mut c,
            "job-1",
            &input,
            &first,
            PromptId::TaskJourneyIncrement,
            3,
        )
        .unwrap();
        c.execute("INSERT INTO task_distillation_nodes(task_id,node_id,revision,kind,topic_key,status,claim_ids_json,detail_json,source_set_hash,active) VALUES('task','unrelated',5,'decision','other','current','[]','{\"result\":\"keep\"}','other',1)",[]).unwrap();
        c.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log-next','task','original later report',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES('entry-next','session','user','note','Continue export',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,final_report,revision,created_at,updated_at) SELECT 'run-next',task_id,session_id,'next','next-hash','Continue export','entry-next','log-next',model,workspace_path,context_hash,'succeeded','Added JSON export alongside CSV export',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP FROM task_work_session_runs WHERE id='run'",[]).unwrap();
        let prepared =
            crate::application::task_execution_service::TaskExecutionApplicationService::new(&db)
                .distillation_job_input("run-next", "en")
                .unwrap();
        assert!(prepared["existingSemanticGraph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["id"] == stable && node["revision"] == 1));
        let mut second_input: DistillationInput =
            serde_json::from_value(prepared["distillationInput"].clone()).unwrap();
        second_input.projection_kind = "task_journey".into();
        second_input.expected_projection_revision = second_input.expected_journey_revision;
        second_input.affected_semantic_ids = vec![stable.clone()];
        let mut second = first.clone();
        let source = second_input.primary_source.as_ref().unwrap();
        let citation = SourceCitation {
            source_type: source.source_type.clone(),
            id: source.id.clone(),
            revision: source.revision.clone(),
            locator: source.locator.clone(),
            quote: source.excerpt.clone(),
        };
        second.claims[0].sources = vec![citation.clone()];
        second.claims[0].statement = source.excerpt.clone();
        second.node_changes[0].sources = vec![citation];
        second.node_changes[0].action = "enrich".into();
        second.node_changes[0].candidate_id = "different-model-label".into();
        second.node_changes[0].target_ids = vec![stable.clone()];
        second.node_changes[0]
            .expected_target_revisions
            .insert(stable.clone(), 1);
        second.node_changes[0].detail.result = Some(source.excerpt.clone());
        canonicalize_semantic_ids(&second_input, &mut second).unwrap();
        job(&c, &second_input, "job-2");
        publish(
            &mut c,
            "job-2",
            &second_input,
            &second,
            PromptId::TaskJourneyIncrement,
            3,
        )
        .unwrap();
        assert_eq!(
            c.query_row(
                "SELECT revision FROM task_distillation_nodes WHERE task_id='task' AND node_id=?",
                [&stable],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(c.query_row("SELECT revision||':'||detail_json FROM task_distillation_nodes WHERE task_id='task' AND node_id='unrelated'",[],|r|r.get::<_,String>(0)).unwrap(),"5:{\"result\":\"keep\"}");
    }

    #[test]
    fn cancelled_jobs_changed_run_heads_and_missing_target_cas_never_apply() {
        for mode in [
            "cancelled",
            "run_head",
            "content",
            "new_decision",
            "missing_target",
        ] {
            let (_root, db, mut input, mut result) = fixture();
            let mut c = database::open(&db).unwrap();
            job(&c, &input, "negative");
            match mode {
                "cancelled" => {
                    c.execute(
                        "UPDATE ai_jobs_v2 SET status='cancelled' WHERE id='negative'",
                        [],
                    )
                    .unwrap();
                }
                "run_head" => {
                    c.execute(
                        "UPDATE task_work_session_runs SET revision=revision+1 WHERE id='run'",
                        [],
                    )
                    .unwrap();
                }
                "content" => {
                    c.execute("UPDATE task_work_session_runs SET final_report='Changed exact source' WHERE id='run'",[]).unwrap();
                }
                "new_decision" => {
                    c.execute("INSERT INTO task_decisions(id,task_id,task_revision,kind,payload_json,operation_id,created_at) VALUES('late','task',1,'choice','{}','late',CURRENT_TIMESTAMP)",[]).unwrap();
                }
                _ => {
                    c.execute("INSERT INTO task_distillation_nodes(task_id,node_id,revision,kind,status,claim_ids_json,detail_json,source_set_hash,active) VALUES('task','existing',1,'result','current','[]','{}','original',1)",[]).unwrap();
                    input.semantic_target_revisions.insert("existing".into(), 1);
                    result.node_changes[0].action = "enrich".into();
                    result.node_changes[0].target_ids = vec!["existing".into()];
                }
            }
            assert!(
                publish(
                    &mut c,
                    "negative",
                    &input,
                    &result,
                    PromptId::TaskJourneyIncrement,
                    3
                )
                .is_err(),
                "{mode}"
            );
            assert!(current(&c, "task_journey", "task").unwrap().is_none());
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM task_distillation_revisions",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
    }

    #[test]
    fn distillation_repair_selects_requested_terminal_run_and_enforces_task_ownership() {
        let (_root, db, _, _) = fixture();
        let c = database::open(&db).unwrap();
        c.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log-new','task','new report',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES('entry-new','session','user','note','New work',CURRENT_TIMESTAMP)",[]).unwrap();
        c.execute("INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,final_report,revision,created_at,updated_at,finished_at) SELECT 'run-new',task_id,session_id,'new','new','New work','entry-new','log-new',model,workspace_path,context_hash,'succeeded','New report',1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,'2099-01-01' FROM task_work_session_runs WHERE id='run'",[]).unwrap();
        assert_eq!(
            terminal_run_for_repair(&c, "task", None).unwrap(),
            "run-new"
        );
        assert_eq!(
            terminal_run_for_repair(&c, "task", Some("run")).unwrap(),
            "run"
        );
        assert!(terminal_run_for_repair(&c, "other-task", Some("run")).is_err());
        assert!(terminal_run_for_repair(&c, "task", Some("missing")).is_err());
        assert!(terminal_run_for_repair(&c, "task", Some("")).is_err());
        c.execute(
            "UPDATE task_work_session_runs SET status='running' WHERE id='run'",
            [],
        )
        .unwrap();
        assert!(terminal_run_for_repair(&c, "task", Some("run")).is_err());
        assert_eq!(
            terminal_run_for_repair(&c, "task", None).unwrap(),
            "run-new"
        );
    }

    #[test]
    fn distillation_rejects_superseded_worker_even_when_job_is_running() {
        let (_root, db, input, result) = fixture();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "leased");
        c.execute(
            "UPDATE ai_jobs_v2 SET worker_id='new-worker' WHERE id='leased'",
            [],
        )
        .unwrap();
        assert!(publish_for_worker(
            &mut c,
            "leased",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3,
            "old-worker"
        )
        .unwrap_err()
        .contains("worker no longer owns"));
        assert!(current(&c, "task_journey", "task").unwrap().is_none());
        publish_for_worker(
            &mut c,
            "leased",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3,
            "new-worker",
        )
        .unwrap();
    }

    #[test]
    fn saved_failed_check_cannot_be_omitted_or_bound_to_an_unrelated_subject() {
        let (_root, db, _, _) = fixture();
        let mut c = database::open(&db).unwrap();
        c.execute("INSERT INTO task_work_session_run_items(run_id,provider_item_id,provider_order,kind,status,content_json,created_at,completed_at) VALUES('run','check',1,'commandExecution','completed',?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",[json!({"command":"cargo test export","aggregatedOutput":"export test failed","exitCode":1}).to_string()]).unwrap();
        let service =
            crate::application::task_execution_service::TaskExecutionApplicationService::new(&db);
        let job_input = service.distillation_job_input("run", "en").unwrap();
        let mut input: DistillationInput =
            serde_json::from_value(job_input["distillationInput"].clone()).unwrap();
        input.projection_kind = "task_journey".into();
        let mut result = factual_fallback(&input, "succeeded");
        result.claims.retain(|claim| claim.kind != "attempt");
        job(&c, &input, "omitted");
        assert!(publish(
            &mut c,
            "omitted",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("omitted"));
        let failed = input.sources.iter().find(|s| s.id == "check").unwrap();
        result.claims[0].status = "contradicted".into();
        result.claims[0].topic_key = Some("unrelated migration".into());
        result.claims[0].contradicted_by = vec![SourceCitation {
            source_type: failed.source_type.clone(),
            id: failed.id.clone(),
            revision: failed.revision.clone(),
            locator: failed.locator.clone(),
            quote: failed.excerpt.clone(),
        }];
        assert!(publish(
            &mut c,
            "omitted",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("same canonical"));
        let fallback = factual_fallback(&input, "succeeded");
        publish(
            &mut c,
            "omitted",
            &input,
            &fallback,
            PromptId::TaskJourneyIncrement,
            3,
        )
        .unwrap();
        assert_eq!(
            current(&c, "task_journey", "task").unwrap().unwrap()["result"]["claims"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn failed_attempt_followed_by_success_is_historical_not_a_blanket_contradiction() {
        let (_root, db, _, _) = fixture();
        let mut c = database::open(&db).unwrap();
        for (id, order, exit, output) in [
            ("first", 1, 1, "export test failed"),
            ("retry", 2, 0, "export test passed"),
        ] {
            c.execute("INSERT INTO task_work_session_run_items(run_id,provider_item_id,provider_order,kind,status,content_json,created_at,completed_at) VALUES('run',?,?,'commandExecution','completed',?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",params![id,order,json!({"command":"cargo test export","aggregatedOutput":output,"exitCode":exit}).to_string()]).unwrap();
        }
        let value =
            crate::application::task_execution_service::TaskExecutionApplicationService::new(&db)
                .distillation_job_input("run", "en")
                .unwrap();
        let mut input: DistillationInput =
            serde_json::from_value(value["distillationInput"].clone()).unwrap();
        input.projection_kind = "task_journey".into();
        assert_eq!(
            input
                .sources
                .iter()
                .find(|s| s.id == "first")
                .unwrap()
                .evidence_state,
            "failed_approach"
        );
        assert_eq!(
            input
                .sources
                .iter()
                .find(|s| s.id == "retry")
                .unwrap()
                .evidence_state,
            "observed_exit"
        );
        let result = factual_fallback(&input, "succeeded");
        job(&c, &input, "retry-facts");
        publish(
            &mut c,
            "retry-facts",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3,
        )
        .unwrap();
        assert!(result
            .claims
            .iter()
            .all(|claim| claim.status != "contradicted" && claim.epistemic_state != "verified"));
    }

    #[test]
    fn fabricated_completion_and_verification_are_rejected_at_publication() {
        let (_root, db, mut input, mut result) = fixture();
        let mut c = database::open(&db).unwrap();
        job(&c, &input, "fabricated");
        let citation = result.claims[0].sources[0].clone();
        result.completion_snapshots.push(CompletionSnapshot {
            id: "final".into(),
            completion_id: "invented".into(),
            completion_revision: "1".into(),
            status: "current".into(),
            claim_ids: vec!["claim".into()],
            unresolved: vec![],
            verification_state: "verified".into(),
            sources: vec![citation],
        });
        assert!(publish(
            &mut c,
            "fabricated",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("canonical completion"));
        c.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion','task',1,'Recorded tests','finished','complete',CURRENT_TIMESTAMP)",[]).unwrap();
        let value =
            crate::application::task_execution_service::TaskExecutionApplicationService::new(&db)
                .distillation_job_input("run", "en")
                .unwrap();
        input = serde_json::from_value(value["distillationInput"].clone()).unwrap();
        input.projection_kind = "task_journey".into();
        let source = input
            .sources
            .iter()
            .find(|s| s.source_type == "task_completion")
            .unwrap();
        let bound = SourceCitation {
            source_type: source.source_type.clone(),
            id: source.id.clone(),
            revision: source.revision.clone(),
            locator: source.locator.clone(),
            quote: source.excerpt.clone(),
        };
        result.claims[0].sources = vec![bound.clone()];
        result.claims[0].statement = bound.quote.clone();
        result.node_changes.clear();
        result.completion_snapshots[0].sources = vec![bound];
        result.completion_snapshots[0].completion_id = "completion".into();
        assert!(publish(
            &mut c,
            "fabricated",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("does not establish verification"));
        result.completion_snapshots[0].verification_state = "recorded".into();
        assert!(publish(
            &mut c,
            "fabricated",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("finality"));
        result.completion_snapshots[0].status = "historical_after_reopen".into();
        assert!(publish(
            &mut c,
            "fabricated",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3
        )
        .unwrap_err()
        .contains("finality"));
        assert!(current(&c, "task_journey", "task").unwrap().is_none());
    }

    #[test]
    fn two_facts_from_one_saved_report_have_distinct_nodes() {
        let (_root, db, mut input, mut result) = fixture();
        let mut c = database::open(&db).unwrap();
        let report = "Added CSV export. Added JSON export.";
        c.execute(
            "UPDATE task_work_session_runs SET final_report=? WHERE id='run'",
            [report],
        )
        .unwrap();
        let source = input.primary_source.as_mut().unwrap();
        source.excerpt = report.into();
        source.content_hash = content_hash(report);
        input.source_set_hash = source_set_hash(input.primary_source.as_ref(), &input.sources);
        for (index, statement) in ["Added CSV export.", "Added JSON export."]
            .iter()
            .enumerate()
        {
            let mut claim = result.claims[0].clone();
            claim.id = format!("fact-{index}");
            claim.statement = statement.to_string();
            claim.sources[0].quote = statement.to_string();
            let mut node = result.node_changes[0].clone();
            node.candidate_id = format!("candidate-{index}");
            node.claim_ids = vec![claim.id.clone()];
            node.sources = claim.sources.clone();
            if index == 0 {
                result.claims = vec![claim];
                result.node_changes = vec![node];
            } else {
                result.claims.push(claim);
                result.node_changes.push(node);
            }
        }
        canonicalize_semantic_ids(&input, &mut result).unwrap();
        job(&c, &input, "two-facts");
        publish(
            &mut c,
            "two-facts",
            &input,
            &result,
            PromptId::TaskJourneyIncrement,
            3,
        )
        .unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM task_distillation_nodes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
    }

    #[test]
    fn schema_head_includes_distillation_migration() {
        let (_root, db, _, _) = fixture();
        let c = database::open(&db).unwrap();
        assert_eq!(migrations::schema_version(&c).unwrap(), migrations::CURRENT_SCHEMA_VERSION);
        assert_eq!(c.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='task_distillation_revisions'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    }
}
