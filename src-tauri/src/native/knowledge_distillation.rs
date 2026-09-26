//! Evidence-bound final Knowledge and separate idea generation.

use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn hash_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn hash_value(value: &Value) -> String {
    hash_bytes(&serde_json::to_vec(value).unwrap_or_default())
}
fn stored_idea_id(task_id: &str, provider_id: &str) -> String {
    hash_bytes(format!("{task_id}\0{provider_id}").as_bytes()).replacen("sha256:", "idea:", 1)
}
fn canonical_hash(values: &mut Vec<Value>) -> String {
    values.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());
    hash_value(&json!(values))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    #[serde(rename = "type")]
    pub source_type: String,
    pub id: String,
    pub revision: String,
    pub locator: String,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalOutcome {
    pub topic_key: String,
    pub statement: String,
    #[serde(default)]
    pub claim_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimBinding {
    pub claim_id: String,
    pub statement: String,
    pub epistemic_state: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionBinding {
    pub assumption_id: String,
    pub body_locator: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Applicability {
    pub summary: String,
    pub representative_questions: Vec<String>,
    pub helps_with: Vec<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    #[serde(rename = "type")]
    pub article_type: String,
    pub title: String,
    #[serde(default)]
    pub final_outcomes: Vec<FinalOutcome>,
    pub body_markdown: String,
    pub applicability: Applicability,
    #[serde(default)]
    pub claim_bindings: Vec<ClaimBinding>,
    #[serde(default)]
    pub assumption_bindings: Vec<AssumptionBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    pub id: String,
    pub title: String,
    pub body_markdown: String,
    pub disposition: String,
    pub reconsideration_conditions: Vec<String>,
    pub source_refs: Vec<SourceRef>,
    #[serde(default)]
    pub related_topic_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityFinding {
    pub kind: String,
    pub severity: String,
    pub locator: String,
    #[serde(default)]
    pub source_refs: Vec<SourceRef>,
    #[serde(default)]
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationResult {
    pub article: Article,
    #[serde(default)]
    pub ideas: Vec<Idea>,
    #[serde(default)]
    pub quality_findings: Vec<QualityFinding>,
}

fn source_key(source: &SourceRef) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        source.source_type, source.id, source.revision, source.locator
    )
}

fn source_citation_key(source: &SourceRef) -> String {
    format!("{}\0{}", source_key(source), source.quote.trim())
}
fn manifest_key(source: &Value) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        source["type"].as_str().unwrap_or(""),
        source["id"].as_str().unwrap_or(""),
        source["revision"].as_str().unwrap_or(""),
        source["locator"].as_str().unwrap_or("")
    )
}
fn full_source(
    source_type: &str,
    id: &str,
    revision: &str,
    locator: &str,
    role: &str,
    content: Value,
) -> Value {
    let bytes = serde_json::to_vec(&content).unwrap_or_default();
    json!({"type":source_type,"id":id,"revision":revision,"locator":locator,"contentHash":hash_bytes(&bytes),"role":role,"content":content})
}

/// Capture every generation input in one SQLite snapshot. Nothing is truncated for the model.
pub(crate) fn capture_snapshot_tx(
    tx: &Transaction<'_>,
    task_id: &str,
    expected_revision: i64,
    locale: &str,
) -> Result<Value, String> {
    let (head, state): (i64, String) = tx
        .query_row(
            "SELECT current_revision,state FROM tasks WHERE id=?",
            [task_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;
    if head != expected_revision {
        return Err(format!("task_revision_conflict: currentRevision={head}"));
    }
    if state != "completed" {
        return Err("task_not_completed".into());
    }
    let task:Value=tx.query_row("SELECT title,detail,outcome,scope,non_goals,validation_criteria,content_hash FROM task_revisions WHERE task_id=? AND revision=?",params![task_id,head],|row|Ok(json!({"id":task_id,"revision":head,"title":row.get::<_,String>(0)?,"detail":row.get::<_,String>(1)?,"outcome":row.get::<_,String>(2)?,"scope":row.get::<_,String>(3)?,"nonGoals":row.get::<_,String>(4)?,"validationCriteria":row.get::<_,String>(5)?,"contentHash":row.get::<_,String>(6)?}))).map_err(|e|e.to_string())?;
    let completion:Value=tx.query_row("SELECT id,task_revision,evidence,report,created_at FROM task_completions WHERE task_id=? ORDER BY created_at DESC,id DESC LIMIT 1",[task_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"taskRevision":row.get::<_,i64>(1)?,"evidence":row.get::<_,String>(2)?,"report":row.get::<_,String>(3)?,"createdAt":row.get::<_,String>(4)?}))).optional().map_err(|e|e.to_string())?.ok_or("task_not_completed: completion evidence missing")?;
    if completion["taskRevision"].as_i64() != Some(head) {
        return Err(
            "source_snapshot_stale: completion is not for the current Task revision".into(),
        );
    }
    // Only Run-backed work has final-report Distillation. Manual work remains
    // usable as recorded evidence without manufacturing semantic authority.
    let has_run: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM task_work_session_runs WHERE task_id=?)",
        [task_id], |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    let basis = if has_run { "distilled" } else { "recorded_only" };
    let journey = if has_run {
        let journey = crate::native::task_distillation::task_projection(tx, task_id)?
            .ok_or("journey_not_current: Distillation is unavailable")?;
        if journey["freshness"] != "current" {
            return Err(format!(
                "journey_not_current: {}",
                journey["freshness"].as_str().unwrap_or("unavailable")
            ));
        }
        let completion_snapshot = journey["completionSnapshots"]
            .as_array()
            .and_then(|items| items.last())
            .cloned()
            .ok_or("journey_not_current: completion snapshot is unavailable")?;
        if completion_snapshot["completionId"] != completion["id"] {
            return Err(
                "journey_not_current: completion snapshot does not match canonical completion".into(),
            );
        }
        journey
    } else {
        json!({"basis":"recorded_only","freshness":"recorded_only",
            "projectionRevision":0,"nodes":[],"topicStates":[],
            "completionSnapshots":[],"result":{"claims":[],"warnings":[]}})
    };
    let recorded_journey = crate::native::task_journey::build_tx(tx, task_id)?;
    let mut distillations = Vec::new();
    let mut statement=tx.prepare("SELECT r.owner_id,r.revision,r.source_set_hash,r.run_id,r.result_json FROM task_distillation_current c JOIN task_distillation_revisions r USING(owner_type,owner_id,revision) WHERE c.owner_type='work_log' AND r.task_id=? AND c.freshness='current' ORDER BY r.created_at,r.owner_id").map_err(|e|e.to_string())?;
    let rows = statement
        .query_map([task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(statement);
    let mut manifest = vec![
        full_source(
            "task_revision",
            task_id,
            &head.to_string(),
            "definition",
            "definition",
            task.clone(),
        ),
        full_source(
            "task_completion",
            completion["id"].as_str().unwrap_or(""),
            &head.to_string(),
            "completion",
            "completion",
            completion.clone(),
        ),
        full_source(
            "task_journey",
            task_id,
            &journey["projectionRevision"].to_string(),
            "projection",
            "journey",
            journey.clone(),
        ),
        full_source(
            "recorded_journey",
            task_id,
            &head.to_string(),
            "events",
            "history",
            recorded_journey.clone(),
        ),
    ];
    let mut decisions=tx.prepare("SELECT id,task_revision,kind,payload_json FROM task_decisions WHERE task_id=? ORDER BY created_at,id").map_err(|e|e.to_string())?;
    let decision_rows = decisions
        .query_map([task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(decisions);
    for (decision_id, decision_revision, kind, payload) in decision_rows {
        manifest.push(full_source("task_decision",&decision_id,&decision_revision.to_string(),"payload_json","decision",json!({"kind":kind,"payload":serde_json::from_str::<Value>(&payload).unwrap_or(Value::String(payload))})));
    }
    let mut logs = tx.prepare("SELECT id,body,image_summary,created_at FROM task_work_log_entries WHERE task_id=? ORDER BY created_at,id").map_err(|error| error.to_string())?;
    let recorded_logs = logs.query_map([task_id], |row| Ok(json!({
        "id":row.get::<_,String>(0)?,"body":row.get::<_,String>(1)?,
        "imageSummary":row.get::<_,String>(2)?,"createdAt":row.get::<_,String>(3)?
    }))).map_err(|error| error.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
    drop(logs);
    for log in recorded_logs {
        manifest.push(full_source("task_work_log",log["id"].as_str().unwrap_or(""),
            &hash_value(&log),"record","reported",log.clone()));
    }
    for (owner, revision, source_hash, run_id, result_json) in rows.into_iter().filter(|_| has_run) {
        let result: Value = serde_json::from_str(&result_json).unwrap_or(json!({}));
        let mut item = json!({"id":owner,"revision":revision,"sourceSetHash":source_hash,"runId":run_id,"result":result});
        if let Some(run) = item["runId"].as_str().map(str::to_owned) {
            let report = tx
                .query_row(
                    "SELECT final_report FROM task_work_session_runs WHERE id=?",
                    [&run],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .flatten();
            item["primaryFinalReport"] = report.clone().map(Value::String).unwrap_or(Value::Null);
            if let Some(report) = report {
                manifest.push(full_source(
                    "run_final_report",
                    &run,
                    &revision.to_string(),
                    "final_report",
                    "primary",
                    Value::String(report),
                ));
            }
            let mut evidence=tx.prepare("SELECT provider_item_id,kind,content_json FROM task_work_session_run_items WHERE run_id=? AND status='completed' AND kind IN ('commandExecution','fileChange') ORDER BY provider_order,provider_item_id").map_err(|e|e.to_string())?;
            let evidence_rows = evidence
                .query_map([&run], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            drop(evidence);
            for (evidence_id, kind, raw) in evidence_rows {
                let content = serde_json::from_str::<Value>(&raw).unwrap_or(Value::String(raw));
                let role = match (kind.as_str(), content["exitCode"].as_i64()) {
                    ("commandExecution", Some(0)) => "successful_check",
                    ("commandExecution", Some(_)) => "failed_check",
                    ("fileChange", _) => "performed_change",
                    _ => "reported",
                };
                manifest.push(full_source(
                    "run_completed_item",
                    &evidence_id,
                    &hash_value(&content),
                    "content_json",
                    role,
                    content,
                ));
            }
        }
        manifest.push(full_source(
            "work_log_distillation",
            &owner,
            &revision.to_string(),
            "result",
            "distillation",
            item.clone(),
        ));
        distillations.push(item);
    }
    if has_run && distillations.is_empty() {
        return Err("journey_not_current: final-report Distillation is unavailable".into());
    }
    let actual_references = super::reference_provenance::for_task(tx, task_id)?;
    for reference in &actual_references {
        manifest.push(full_source(
            "reference",
            reference["documentId"].as_str().unwrap_or(""),
            reference["documentVersion"].as_str().unwrap_or(""),
            reference["section"].as_str().unwrap_or(""),
            reference["disposition"].as_str().unwrap_or("used"),
            reference.clone(),
        ));
    }
    let mut hash_manifest = manifest.clone();
    let generation_snapshot_hash = canonical_hash(&mut hash_manifest);
    Ok(
        json!({"basis":basis,"task":task,"completion":completion,"distillations":distillations,"journey":journey,"recordedJourney":recorded_journey,"actualReferenceUsages":actual_references,"assumptions":[],"sourceManifest":manifest,"generationSnapshotHash":generation_snapshot_hash,"locale":locale}),
    )
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn value_contains_quote(value: &Value, quote: &str) -> bool {
    let quote = quote.trim();
    if quote.is_empty() {
        return false;
    }
    match value {
        Value::String(text) => text.contains(quote),
        Value::Array(items) => items.iter().any(|item| value_contains_quote(item, quote)),
        Value::Object(items) => items.values().any(|item| value_contains_quote(item, quote)),
        _ => false,
    }
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(crate) fn validate_result(snapshot: &Value, result: &GenerationResult) -> Result<(), String> {
    let manifest = snapshot["sourceManifest"]
        .as_array()
        .ok_or("invalid generation snapshot")?;
    let manifest_by_key = manifest
        .iter()
        .map(|source| (manifest_key(source), source))
        .collect::<BTreeMap<_, _>>();
    let check_refs = |refs: &[SourceRef], label: &str| -> Result<(), String> {
        if refs.is_empty() {
            return Err(format!("missing_reference: {label}"));
        }
        for source in refs {
            let Some(manifest_source) = manifest_by_key.get(&source_key(source)) else {
                return Err(format!(
                    "missing_reference: {label} cites a source outside the snapshot"
                ));
            };
            if !value_contains_quote(&manifest_source["content"], &source.quote) {
                return Err(format!(
                    "missing_reference: {label} requires an exact quote present in its source"
                ));
            }
        }
        Ok(())
    };
    let mut authoritative_claims = BTreeMap::<String, &Value>::new();
    for claim in snapshot["distillations"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|distillation| {
            distillation["result"]["claims"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .chain(
            snapshot["journey"]["result"]["claims"]
                .as_array()
                .into_iter()
                .flatten(),
        )
    {
        if let Some(claim_id) = claim["id"].as_str() {
            authoritative_claims.insert(claim_id.to_owned(), claim);
        }
    }
    if !matches!(
        result.article.article_type.as_str(),
        "concept" | "guide" | "comparison_decision" | "research_result"
    ) {
        return Err("invalid article type".into());
    }
    if result.article.title.trim().is_empty() || result.article.body_markdown.trim().is_empty() {
        return Err("Knowledge title and body are required".into());
    }
    let applicability = &result.article.applicability;
    if applicability.summary.trim().is_empty()
        || applicability.representative_questions.is_empty()
        || applicability.helps_with.is_empty()
    {
        return Err("applicability_overreach: complete supported applicability is required".into());
    }
    check_refs(&applicability.source_refs, "applicability")?;
    let article_body = normalize(&result.article.body_markdown);
    for supported_text in std::iter::once(&applicability.summary)
        .chain(&applicability.helps_with)
        .chain(&applicability.conditions)
        .chain(&applicability.exclusions)
    {
        if !article_body.contains(&normalize(supported_text)) {
            return Err(format!(
                "applicability_overreach: applicability statement is absent from the article: {supported_text}"
            ));
        }
        if !applicability
            .source_refs
            .iter()
            .any(|source| normalize(&source.quote).contains(&normalize(supported_text)))
        {
            return Err(format!(
                "applicability_overreach: applicability statement lacks exact quoted support: {supported_text}"
            ));
        }
    }
    let topics = snapshot["journey"]["topicStates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|topic| Some((topic["topicKey"].as_str()?.to_owned(), topic.clone())))
        .collect::<BTreeMap<_, _>>();
    let nodes = snapshot["journey"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|node| Some((node["id"].as_str()?.to_owned(), node)))
        .collect::<BTreeMap<_, _>>();
    if !result.article.final_outcomes.is_empty() {
        let completion_snapshot = snapshot["journey"]["completionSnapshots"]
            .as_array()
            .and_then(|items| items.last())
            .ok_or("invalid_finality: canonical completion snapshot is unavailable")?;
        if completion_snapshot["status"] != "current" {
            return Err("invalid_finality: canonical completion snapshot is not current".into());
        }
    }
    for outcome in &result.article.final_outcomes {
        let topic = topics.get(&outcome.topic_key).ok_or_else(|| {
            format!(
                "invalid_finality: topic {} is absent from the exact journey",
                outcome.topic_key
            )
        })?;
        if topic["status"] != "adopted" {
            return Err(format!(
                "invalid_finality: topic {} is not adopted in the exact journey",
                outcome.topic_key
            ));
        }
        if outcome.claim_ids.len() != 1 {
            return Err("invalid_finality: final outcome requires one exact decided claim".into());
        }
        let node_id = topic["currentNodeId"].as_str().ok_or_else(|| {
            format!(
                "invalid_finality: topic {} has no current semantic node",
                outcome.topic_key
            )
        })?;
        let node = nodes.get(node_id).ok_or_else(|| {
            format!("invalid_finality: current semantic node {node_id} is unavailable")
        })?;
        if node["active"] != true || node["status"] != "adopted" {
            return Err(format!(
                "invalid_finality: topic {} does not point to an active adopted node",
                outcome.topic_key
            ));
        }
        let node_claims = string_set(&node["claimIds"]);
        if outcome
            .claim_ids
            .iter()
            .any(|claim| !node_claims.contains(claim))
        {
            return Err(format!(
                "invalid_finality: topic {} claim is not in its current adopted node",
                outcome.topic_key
            ));
        }
    }
    if result.article.article_type == "comparison_decision"
        && result.article.final_outcomes.is_empty()
    {
        return Err("invalid_finality: comparison/decision requires a final topic decision".into());
    }
    let bindings = result
        .article
        .claim_bindings
        .iter()
        .map(|binding| (binding.claim_id.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    for outcome in &result.article.final_outcomes {
        for claim in &outcome.claim_ids {
            let Some(binding) = bindings.get(claim.as_str()) else {
                return Err("missing_reference: final outcome claim is unbound".into());
            };
            if normalize(&outcome.statement) != normalize(&binding.statement)
                || !article_body.contains(&normalize(&outcome.statement))
            {
                return Err(
                    "invalid_finality: final outcome must reproduce its exact decided claim in the article"
                        .into(),
                );
            }
            let authority = authoritative_claims.get(claim).ok_or_else(|| {
                "invalid_finality: final outcome requires an authoritative Distillation claim"
                    .to_owned()
            })?;
            if authority["topicKey"].as_str() != Some(outcome.topic_key.as_str())
                || authority["status"] != "current"
                || authority["epistemicState"] != "decided"
                || binding.epistemic_state != authority["epistemicState"]
            {
                return Err(
                    "invalid_finality: final outcome claim lacks matching current topic authority"
                        .into(),
                );
            }
        }
    }
    for binding in &result.article.claim_bindings {
        check_refs(&binding.source_refs, &format!("claim {}", binding.claim_id))?;
        let statement = normalize(&binding.statement);
        if statement.is_empty()
            || (!normalize(&result.article.body_markdown).contains(&statement)
                && !result.article.final_outcomes.iter().any(|outcome| {
                    outcome.claim_ids.contains(&binding.claim_id)
                        && normalize(&outcome.statement).contains(&statement)
                }))
        {
            return Err(format!(
                "missing_reference: bound claim {} is not present in the article",
                binding.claim_id
            ));
        }
        if let Some(authority) = authoritative_claims.get(&binding.claim_id) {
            if binding.statement.trim() != authority["statement"].as_str().unwrap_or("").trim()
                || binding.epistemic_state != authority["epistemicState"].as_str().unwrap_or("")
            {
                return Err(format!(
                    "invalid_verification: claim {} changes its authoritative statement or epistemic state",
                    binding.claim_id
                ));
            }
            let authoritative_sources = authority["sources"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|source| {
                    serde_json::from_value::<SourceRef>(source.clone())
                        .ok()
                        .map(|source| source_citation_key(&source))
                })
                .collect::<BTreeSet<_>>();
            if binding
                .source_refs
                .iter()
                .any(|source| !authoritative_sources.contains(&source_citation_key(source)))
            {
                return Err(format!(
                    "invalid_verification: claim {} changes its authoritative quote binding",
                    binding.claim_id
                ));
            }
        } else {
            if binding.epistemic_state != "reported" {
                return Err(format!(
                    "invalid_verification: synthesized claim {} must remain reported",
                    binding.claim_id
                ));
            }
            if !binding
                .source_refs
                .iter()
                .any(|source| source.quote.trim() == binding.statement.trim())
            {
                return Err(format!(
                    "invalid_verification: reported claim {} must retain an exact source quote",
                    binding.claim_id
                ));
            }
        }
    }
    for assumption in &result.article.assumption_bindings {
        check_refs(
            &assumption.source_refs,
            &format!("assumption {}", assumption.assumption_id),
        )?;
    }
    let binding_ids = result
        .article
        .claim_bindings
        .iter()
        .map(|binding| binding.claim_id.as_str())
        .collect::<BTreeSet<_>>();
    let finding_locators = result
        .quality_findings
        .iter()
        .filter(|finding| finding.kind == "material_omission")
        .map(|finding| finding.locator.as_str())
        .collect::<BTreeSet<_>>();
    for claim in snapshot["journey"]["result"]["claims"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|claim| claim["status"] == "contradicted" || claim["kind"] == "unresolved")
    {
        let Some(claim_id) = claim["id"].as_str() else {
            continue;
        };
        if !binding_ids.contains(claim_id) && !finding_locators.contains(claim_id) {
            return Err(format!(
                "validation_material_omission: material journey claim {claim_id} is neither retained nor disclosed"
            ));
        }
    }
    for warning in snapshot["journey"]["result"]["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !finding_locators.contains(warning) {
            return Err(format!(
                "validation_material_omission: journey warning {warning} is undisclosed"
            ));
        }
    }
    if let Some(completion) = snapshot["journey"]["completionSnapshots"]
        .as_array()
        .and_then(|items| items.last())
    {
        for unresolved in completion["unresolved"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !normalize(&result.article.body_markdown).contains(&normalize(unresolved))
                && !finding_locators.contains(unresolved)
            {
                return Err(format!(
                    "validation_material_omission: unresolved completion item {unresolved} is absent"
                ));
            }
        }
    }
    let article = normalize(&result.article.body_markdown);
    let mut idea_ids = BTreeSet::new();
    for idea in &result.ideas {
        if idea.id.trim().is_empty()
            || idea.title.trim().is_empty()
            || idea.body_markdown.trim().is_empty()
            || !idea_ids.insert(idea.id.as_str())
        {
            return Err("invalid idea identity or content".into());
        }
        if !matches!(
            idea.disposition.as_str(),
            "unverified" | "deferred" | "out_of_scope" | "rejected"
        ) {
            return Err("invalid idea disposition".into());
        }
        if idea.reconsideration_conditions.is_empty() {
            return Err("idea requires reconsideration conditions".into());
        }
        check_refs(&idea.source_refs, &format!("idea {}", idea.id))?;
        let idea_body = normalize(&idea.body_markdown);
        if !idea_body.is_empty() && article.contains(&idea_body) {
            return Err("idea_leak: idea text appears in final Knowledge".into());
        }
    }
    for finding in &result.quality_findings {
        if !finding.source_refs.is_empty() {
            check_refs(
                &finding.source_refs,
                &format!("quality finding {}", finding.locator),
            )?;
        }
    }
    if result
        .quality_findings
        .iter()
        .any(|finding| finding.severity == "blocking")
    {
        return Err("Knowledge candidate has blocking quality findings".into());
    }
    Ok(())
}

pub(crate) fn prepared(
    snapshot: Value,
    result: GenerationResult,
    model_status: &str,
    model_error: &str,
) -> Result<Value, String> {
    validate_result(&snapshot, &result)?;
    let body_markdown = result.article.body_markdown.clone();
    Ok(
        json!({"taskId":snapshot["task"]["id"],"taskRevision":snapshot["task"]["revision"],"completionId":snapshot["completion"]["id"],"sourceHash":snapshot["generationSnapshotHash"],"generationSnapshot":snapshot,"generationResult":result,"bodyMarkdown":body_markdown,"modelStatus":model_status,"modelError":model_error}),
    )
}

pub(crate) fn append_generated_tx(tx: &Transaction<'_>, prepared: &Value) -> Result<Value, String> {
    let task_id = prepared["taskId"]
        .as_str()
        .ok_or("Prepared Knowledge task is unavailable")?;
    let task_revision = prepared["taskRevision"]
        .as_i64()
        .ok_or("Prepared Knowledge revision is unavailable")?;
    let snapshot = &prepared["generationSnapshot"];
    let locale = snapshot["locale"].as_str().unwrap_or("en");
    let current = capture_snapshot_tx(tx, task_id, task_revision, locale)?;
    if current["generationSnapshotHash"] != snapshot["generationSnapshotHash"] {
        return Err("source_snapshot_stale".into());
    }
    let mut result: GenerationResult = serde_json::from_value(prepared["generationResult"].clone())
        .map_err(|e| format!("invalid Knowledge result: {e}"))?;
    validate_result(snapshot, &result)?;
    for idea in &mut result.ideas {
        idea.id = stored_idea_id(task_id, &idea.id);
    }
    let revision: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM knowledge_draft_versions WHERE task_id=?",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let parent = tx
        .query_row(
            "SELECT current_private_revision FROM knowledge_pointers WHERE task_id=?",
            [task_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let snapshot_hash = snapshot["generationSnapshotHash"].as_str().unwrap_or("");
    let snapshot_id = tx
        .query_row(
            "SELECT id FROM knowledge_evidence_snapshots WHERE generation_snapshot_hash=?",
            [snapshot_hash],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(id);
    let applicability_id = id();
    tx.execute("INSERT OR IGNORE INTO knowledge_evidence_snapshots(id,task_id,task_revision,generation_snapshot_hash,journey_projection_revision,journey_source_set_hash,completion_snapshot_id,completion_snapshot_revision,manifest_json,freshness) VALUES(?,?,?,?,?,?,?,?,?,'current')",params![snapshot_id,task_id,task_revision,snapshot_hash,snapshot["journey"]["projectionRevision"].as_i64().unwrap_or(0),snapshot["journey"]["sourceSetHash"].as_str().unwrap_or(""),snapshot["completion"]["id"].as_str().unwrap_or(""),snapshot["completion"]["taskRevision"].to_string(),snapshot.to_string()]).map_err(|e|e.to_string())?;
    let app = &result.article.applicability;
    tx.execute("INSERT INTO knowledge_applicability(id,summary,representative_questions_json,helps_with_json,conditions_json,exclusions_json) VALUES(?,?,?,?,?,?)",params![applicability_id,app.summary,serde_json::to_string(&app.representative_questions).unwrap(),serde_json::to_string(&app.helps_with).unwrap(),serde_json::to_string(&app.conditions).unwrap(),serde_json::to_string(&app.exclusions).unwrap()]).map_err(|e|e.to_string())?;
    let content_hash = hash_value(
        &json!({"article":&result.article,"snapshot":snapshot["generationSnapshotHash"]}),
    );
    let body_hash = hash_bytes(result.article.body_markdown.as_bytes());
    tx.execute("INSERT INTO knowledge_draft_versions(task_id,revision,parent_revision,derivation_kind,article_type,title,body_markdown,content_hash,generation_snapshot_id,applicability_id,result_json,quality_state,model_status,model_error) VALUES(?,?,?,'generated',?,?,?,?,?,?,?,'valid',?,?)",params![task_id,revision,parent,result.article.article_type,result.article.title,result.article.body_markdown,content_hash,snapshot_id,applicability_id,serde_json::to_string(&result).unwrap(),prepared["modelStatus"].as_str().unwrap_or("enhanced"),prepared["modelError"].as_str().unwrap_or("")]).map_err(|e|e.to_string())?;
    for idea in &result.ideas {
        let idea_hash = hash_value(&serde_json::to_value(idea).unwrap_or(Value::Null));
        let idea_revision: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(revision),0)+1 FROM knowledge_idea_revisions WHERE id=?",
                [&idea.id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO knowledge_idea_revisions(id,task_id,revision,knowledge_revision,title,body_markdown,disposition,reconsideration_conditions_json,source_refs_json,related_topic_keys_json,content_hash) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![idea.id,task_id,idea_revision,revision,idea.title,idea.body_markdown,idea.disposition,serde_json::to_string(&idea.reconsideration_conditions).unwrap(),serde_json::to_string(&idea.source_refs).unwrap(),serde_json::to_string(&idea.related_topic_keys).unwrap(),idea_hash]).map_err(|e|e.to_string())?;
    }
    for (index, finding) in result.quality_findings.iter().enumerate() {
        tx.execute("INSERT INTO knowledge_quality_findings(task_id,knowledge_revision,id,kind,severity,locator,source_refs_json,message) VALUES(?,?,?,?,?,?,?,?)",params![task_id,revision,format!("finding-{index}"),finding.kind,finding.severity,finding.locator,serde_json::to_string(&finding.source_refs).unwrap(),finding.message]).map_err(|e|e.to_string())?;
    }
    tx.execute("INSERT INTO knowledge_pointers(task_id,current_private_revision) VALUES(?,?) ON CONFLICT(task_id) DO UPDATE SET current_private_revision=excluded.current_private_revision,updated_at=CURRENT_TIMESTAMP",params![task_id,revision]).map_err(|e|e.to_string())?;
    // Compatibility projection: old readers continue to see the same immutable body.
    tx.execute("INSERT INTO task_knowledge_drafts(task_id,revision,task_revision,completion_id,body_markdown,content_hash,lineage_json,state,model_status,model_error) VALUES(?,?,?,?,?,?,?,'draft',?,?)",params![task_id,revision,task_revision,snapshot["completion"]["id"].as_str().unwrap_or(""),result.article.body_markdown,body_hash,snapshot.to_string(),prepared["modelStatus"].as_str().unwrap_or("enhanced"),prepared["modelError"].as_str().unwrap_or("")]).map_err(|e|e.to_string())?;
    Ok(
        json!({"taskId":task_id,"draftRevision":revision,"taskRevision":task_revision,"completionId":snapshot["completion"]["id"],"bodyMarkdown":result.article.body_markdown,"contentHash":content_hash,"bodyHash":body_hash,"sourceHash":snapshot["generationSnapshotHash"],"state":"draft","modelStatus":prepared["modelStatus"],"modelError":prepared["modelError"],"generationSnapshotId":snapshot_id,"ideas":result.ideas,"article":result.article}),
    )
}

pub(crate) fn review_projection(
    connection: &Transaction<'_>,
    task_id: &str,
    selected_revision: Option<i64>,
) -> Result<Value, String> {
    let head = connection
        .query_row(
            "SELECT current_revision FROM tasks WHERE id=?",
            [task_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Task not found")?;
    let current_snapshot_hash = capture_snapshot_tx(connection, task_id, head, "en")
        .ok()
        .and_then(|snapshot| {
            snapshot["generationSnapshotHash"]
                .as_str()
                .map(str::to_owned)
        });
    let pointers = connection
        .query_row(
            "SELECT current_private_revision,published_revision,publication_document_id,publication_path,publication_content_hash FROM knowledge_pointers WHERE task_id=?",
            [task_id],
            |row| Ok(json!({"currentPrivateRevision":row.get::<_,Option<i64>>(0)?,"publishedRevision":row.get::<_,Option<i64>>(1)?,"publicationDocumentId":row.get::<_,Option<String>>(2)?,"publicationPath":row.get::<_,Option<String>>(3)?,"publicationContentHash":row.get::<_,Option<String>>(4)?})),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| json!({"currentPrivateRevision":null,"publishedRevision":null}));
    let selected = selected_revision.or_else(|| pointers["currentPrivateRevision"].as_i64());
    let mut statement = connection.prepare("SELECT v.revision,v.parent_revision,v.derived_from_revision,v.derivation_kind,v.article_type,v.title,v.body_markdown,v.content_hash,v.result_json,v.quality_state,v.model_status,v.model_error,v.created_at,s.generation_snapshot_hash,s.freshness,a.summary,a.representative_questions_json,a.helps_with_json,a.conditions_json,a.exclusions_json FROM knowledge_draft_versions v LEFT JOIN knowledge_evidence_snapshots s ON s.id=v.generation_snapshot_id JOIN knowledge_applicability a ON a.id=v.applicability_id WHERE v.task_id=? ORDER BY v.revision").map_err(|e|e.to_string())?;
    let versions = statement.query_map([task_id],|row|{
        let revision=row.get::<_,i64>(0)?;
        let generation_hash=row.get::<_,Option<String>>(13)?;
        let freshness=if generation_hash.as_ref()==current_snapshot_hash.as_ref() && generation_hash.is_some(){"current"}else{"stale"};
        Ok(json!({"revision":revision,"parentRevision":row.get::<_,Option<i64>>(1)?,"derivedFromRevision":row.get::<_,Option<i64>>(2)?,"derivationKind":row.get::<_,String>(3)?,"articleType":row.get::<_,String>(4)?,"title":row.get::<_,String>(5)?,"bodyMarkdown":row.get::<_,String>(6)?,"contentHash":row.get::<_,String>(7)?,"result":serde_json::from_str::<Value>(&row.get::<_,String>(8)?).unwrap_or(json!({})),"qualityState":row.get::<_,String>(9)?,"modelStatus":row.get::<_,String>(10)?,"modelError":row.get::<_,String>(11)?,"createdAt":row.get::<_,String>(12)?,"generationSnapshotHash":generation_hash,"freshness":freshness,"storedFreshness":row.get::<_,Option<String>>(14)?.unwrap_or_else(||"historical".into()),"applicability":{"summary":row.get::<_,String>(15)?,"representativeQuestions":serde_json::from_str::<Value>(&row.get::<_,String>(16)?).unwrap_or(json!([])),"helpsWith":serde_json::from_str::<Value>(&row.get::<_,String>(17)?).unwrap_or(json!([])),"conditions":serde_json::from_str::<Value>(&row.get::<_,String>(18)?).unwrap_or(json!([])),"exclusions":serde_json::from_str::<Value>(&row.get::<_,String>(19)?).unwrap_or(json!([]))},"selected":selected==Some(revision)}))
    }).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let ideas = connection.prepare("SELECT id,revision,knowledge_revision,title,body_markdown,disposition,reconsideration_conditions_json,source_refs_json,related_topic_keys_json,content_hash,publication_state FROM knowledge_idea_revisions WHERE task_id=? ORDER BY knowledge_revision,id,revision").map_err(|e|e.to_string())?.query_map([task_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"revision":row.get::<_,i64>(1)?,"knowledgeRevision":row.get::<_,i64>(2)?,"title":row.get::<_,String>(3)?,"bodyMarkdown":row.get::<_,String>(4)?,"disposition":row.get::<_,String>(5)?,"reconsiderationConditions":serde_json::from_str::<Value>(&row.get::<_,String>(6)?).unwrap_or(json!([])),"sourceRefs":serde_json::from_str::<Value>(&row.get::<_,String>(7)?).unwrap_or(json!([])),"relatedTopicKeys":serde_json::from_str::<Value>(&row.get::<_,String>(8)?).unwrap_or(json!([])),"contentHash":row.get::<_,String>(9)?,"publicationState":row.get::<_,String>(10)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(json!({"taskId":task_id,"pointers":pointers,"versions":versions,"ideas":ideas}))
}

pub(crate) fn restore_as_new_tx(
    tx: &Transaction<'_>,
    task_id: &str,
    source_revision: i64,
    expected_current: i64,
) -> Result<Value, String> {
    let current = tx
        .query_row(
            "SELECT current_private_revision FROM knowledge_pointers WHERE task_id=?",
            [task_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .ok_or("version_not_found")?;
    if current != expected_current {
        return Err(format!(
            "current_private_conflict: currentRevision={current}"
        ));
    }
    let source = tx.query_row("SELECT article_type,title,body_markdown,content_hash,generation_snapshot_id,applicability_id,result_json,quality_state,model_status,model_error FROM knowledge_draft_versions WHERE task_id=? AND revision=?",params![task_id,source_revision],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,Option<String>>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,String>(8)?,row.get::<_,String>(9)?))).optional().map_err(|e|e.to_string())?.ok_or("version_not_found")?;
    let revision: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM knowledge_draft_versions WHERE task_id=?",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO knowledge_draft_versions(task_id,revision,parent_revision,derived_from_revision,derivation_kind,article_type,title,body_markdown,content_hash,generation_snapshot_id,applicability_id,result_json,quality_state,model_status,model_error) VALUES(?,?,?,?,'restored',?,?,?,?,?,?,?,?,?,?)",params![task_id,revision,current,source_revision,source.0,source.1,source.2,source.3,source.4,source.5,source.6,source.7,source.8,source.9]).map_err(|e|e.to_string())?;
    let mut ideas=tx.prepare("SELECT id,title,body_markdown,disposition,reconsideration_conditions_json,source_refs_json,related_topic_keys_json,content_hash FROM knowledge_idea_revisions WHERE task_id=? AND knowledge_revision=? ORDER BY id,revision").map_err(|e|e.to_string())?;
    let idea_rows = ideas
        .query_map(params![task_id, source_revision], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(ideas);
    for idea in idea_rows {
        let idea_revision: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(revision),0)+1 FROM knowledge_idea_revisions WHERE id=?",
                [&idea.0],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO knowledge_idea_revisions(id,task_id,revision,knowledge_revision,title,body_markdown,disposition,reconsideration_conditions_json,source_refs_json,related_topic_keys_json,content_hash) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![idea.0,task_id,idea_revision,revision,idea.1,idea.2,idea.3,idea.4,idea.5,idea.6,idea.7]).map_err(|e|e.to_string())?;
    }
    tx.execute("UPDATE knowledge_pointers SET current_private_revision=?,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND current_private_revision=?",params![revision,task_id,current]).map_err(|e|e.to_string())?;
    let task_revision = tx
        .query_row(
            "SELECT task_revision FROM task_knowledge_drafts WHERE task_id=? AND revision=?",
            params![task_id, source_revision],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| {
            tx.query_row(
                "SELECT current_revision FROM tasks WHERE id=?",
                [task_id],
                |row| row.get(0),
            )
            .unwrap_or(0)
        });
    let completion_id = tx
        .query_row(
            "SELECT completion_id FROM task_knowledge_drafts WHERE task_id=? AND revision=?",
            params![task_id, source_revision],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    tx.execute("INSERT INTO task_knowledge_drafts(task_id,revision,task_revision,completion_id,body_markdown,content_hash,lineage_json,state,model_status,model_error) VALUES(?,?,?,?,?,?,?,'draft',?,?)",params![task_id,revision,task_revision,completion_id,source.2,hash_bytes(source.2.as_bytes()),source.6,source.8,source.9]).map_err(|e|e.to_string())?;
    Ok(
        json!({"taskId":task_id,"draftRevision":revision,"derivedFromRevision":source_revision,"contentHash":source.3,"bodyMarkdown":source.2,"state":"draft"}),
    )
}

pub(crate) fn edit_as_new_tx(
    tx: &Transaction<'_>,
    task_id: &str,
    source_revision: i64,
    expected_content_hash: &str,
    expected_source_hash: &str,
    body_markdown: &str,
) -> Result<Value, String> {
    if body_markdown.trim().is_empty() {
        return Err("invalid_input: bodyMarkdown is required".into());
    }
    let current = tx
        .query_row(
            "SELECT current_private_revision FROM knowledge_pointers WHERE task_id=?",
            [task_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .ok_or("version_not_found")?;
    if current != source_revision {
        return Err(format!(
            "current_private_conflict: currentRevision={current}"
        ));
    }
    let source=tx.query_row("SELECT v.article_type,v.title,v.content_hash,v.generation_snapshot_id,v.applicability_id,v.result_json,v.model_status,v.model_error,s.generation_snapshot_hash,s.manifest_json FROM knowledge_draft_versions v JOIN knowledge_evidence_snapshots s ON s.id=v.generation_snapshot_id WHERE v.task_id=? AND v.revision=?",params![task_id,source_revision],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,String>(8)?,row.get::<_,String>(9)?))).optional().map_err(|e|e.to_string())?.ok_or("version_not_found")?;
    if source.2 != expected_content_hash {
        return Err("content_hash_conflict".into());
    }
    if source.8 != expected_source_hash {
        return Err("source_snapshot_stale".into());
    }
    let snapshot: Value = serde_json::from_str(&source.9).map_err(|e| e.to_string())?;
    let mut result: GenerationResult =
        serde_json::from_str(&source.5).map_err(|e| e.to_string())?;
    result.article.body_markdown = body_markdown.to_owned();
    validate_result(&snapshot, &result)?;
    let revision: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM knowledge_draft_versions WHERE task_id=?",
            [task_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let content_hash = hash_value(&json!({"article":&result.article,"snapshot":source.8}));
    let body_hash = hash_bytes(body_markdown.as_bytes());
    tx.execute("INSERT INTO knowledge_draft_versions(task_id,revision,parent_revision,derived_from_revision,derivation_kind,article_type,title,body_markdown,content_hash,generation_snapshot_id,applicability_id,result_json,quality_state,model_status,model_error) VALUES(?,?,?,?,'edited',?,?,?,?,?,?,?,'valid',?,?)",params![task_id,revision,current,source_revision,source.0,source.1,body_markdown,content_hash,source.3,source.4,serde_json::to_string(&result).map_err(|e|e.to_string())?,source.6,source.7]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE knowledge_pointers SET current_private_revision=?,updated_at=CURRENT_TIMESTAMP WHERE task_id=? AND current_private_revision=?",params![revision,task_id,current]).map_err(|e|e.to_string())?;
    let task_revision = snapshot["task"]["revision"].as_i64().unwrap_or(0);
    let completion_id = snapshot["completion"]["id"].as_str().unwrap_or("");
    tx.execute("INSERT INTO task_knowledge_drafts(task_id,revision,task_revision,completion_id,body_markdown,content_hash,lineage_json,state,model_status,model_error) VALUES(?,?,?,?,?,?,?,'draft','corrected','')",params![task_id,revision,task_revision,completion_id,body_markdown,body_hash,snapshot.to_string()]).map_err(|e|e.to_string())?;
    Ok(
        json!({"taskId":task_id,"draftRevision":revision,"derivedFromRevision":source_revision,"bodyMarkdown":body_markdown,"contentHash":content_hash,"bodyHash":body_hash,"sourceHash":expected_source_hash,"state":"draft","article":result.article}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{database, migrations};

    fn article(reference: SourceRef) -> GenerationResult {
        let claim_statement = reference.quote.clone();
        GenerationResult {
            article: Article {
                article_type: "concept".into(),
                title: "Offline approval".into(),
                final_outcomes: vec![],
                body_markdown: format!("# Offline approval\n\n{claim_statement}"),
                applicability: Applicability {
                    summary: claim_statement.clone(),
                    representative_questions: vec!["When is approval required?".into()],
                    helps_with: vec![claim_statement.clone()],
                    conditions: vec![],
                    exclusions: vec![],
                    source_refs: vec![reference.clone()],
                },
                claim_bindings: vec![ClaimBinding {
                    claim_id: "claim".into(),
                    statement: claim_statement,
                    epistemic_state: "reported".into(),
                    source_refs: vec![reference],
                }],
                assumption_bindings: vec![],
            },
            ideas: vec![],
            quality_findings: vec![],
        }
    }
    fn minimal_snapshot() -> Value {
        json!({"journey":{"topicStates":[]},"sourceManifest":[{"type":"run_final_report","id":"run","revision":"1","locator":"final_report","contentHash":"hash","role":"primary","content":"Receipt recorded; approval remains required."}]})
    }
    #[test]
    fn receipt_report_cannot_be_promoted_to_approval() {
        let source = SourceRef {
            source_type: "run_final_report".into(),
            id: "run".into(),
            revision: "1".into(),
            locator: "final_report".into(),
            quote: "Receipt recorded; approval remains required.".into(),
        };
        let mut result = article(source);
        result.article.claim_bindings[0].statement = "Approved for release".into();
        result
            .article
            .body_markdown
            .push_str("\n\nApproved for release");
        assert!(validate_result(&minimal_snapshot(), &result)
            .unwrap_err()
            .contains("exact source quote"));
    }
    #[test]
    fn opposite_polarity_cannot_rewrite_an_authoritative_claim() {
        let source = SourceRef {
            source_type: "run_completed_item".into(),
            id: "check".into(),
            revision: "1".into(),
            locator: "content_json".into(),
            quote: "Export check failed".into(),
        };
        let mut snapshot = json!({
            "journey":{"topicStates":[],"result":{"claims":[{
                "id":"check-claim","kind":"result","statement":"Export check failed",
                "epistemicState":"observed","status":"contradicted","topicKey":null,
                "sources":[{"type":"run_completed_item","id":"check","revision":"1","locator":"content_json","quote":"Export check failed"}]
            }]}},
            "distillations":[],
            "sourceManifest":[{"type":"run_completed_item","id":"check","revision":"1","locator":"content_json","role":"failed_check","content":"Export check failed"}]
        });
        snapshot["journey"]["completionSnapshots"] = json!([]);
        let mut result = article(source);
        result.article.claim_bindings[0].claim_id = "check-claim".into();
        result.article.claim_bindings[0].statement = "Export check passed".into();
        result.article.claim_bindings[0].epistemic_state = "verified".into();
        result
            .article
            .body_markdown
            .push_str("\n\nExport check passed");
        assert!(validate_result(&snapshot, &result)
            .unwrap_err()
            .contains("authoritative statement"));
    }
    #[test]
    fn ideas_remain_separate_and_require_revisit_conditions() {
        let source = SourceRef {
            source_type: "run_final_report".into(),
            id: "run".into(),
            revision: "1".into(),
            locator: "final_report".into(),
            quote: "Receipt recorded; approval remains required.".into(),
        };
        let mut result = article(source.clone());
        result.ideas.push(Idea {
            id: "reminder".into(),
            title: "Reminder automation".into(),
            body_markdown: "Try reminder automation".into(),
            disposition: "unverified".into(),
            reconsideration_conditions: vec![],
            source_refs: vec![source],
            related_topic_keys: vec![],
        });
        assert!(validate_result(&minimal_snapshot(), &result)
            .unwrap_err()
            .contains("reconsideration"));
        result.ideas[0].reconsideration_conditions =
            vec!["A sandbox test demonstrates delivery".into()];
        result
            .article
            .body_markdown
            .push_str("\nTry reminder automation");
        assert!(validate_result(&minimal_snapshot(), &result)
            .unwrap_err()
            .contains("idea_leak"));
    }

    #[test]
    fn all_idea_dispositions_preserve_metadata_without_final_article_leakage() {
        let source = SourceRef {
            source_type: "run_final_report".into(),
            id: "run".into(),
            revision: "1".into(),
            locator: "final_report".into(),
            quote: "Receipt recorded; approval remains required.".into(),
        };
        let mut result = article(source.clone());
        for (index, disposition) in ["unverified", "deferred", "out_of_scope", "rejected"]
            .into_iter()
            .enumerate()
        {
            result.ideas.push(Idea {
                id: format!("idea-{index}"),
                title: format!("Idea {index}"),
                body_markdown: format!("Idea body {index}"),
                disposition: disposition.into(),
                reconsideration_conditions: vec![format!("Condition {index}")],
                source_refs: vec![source.clone()],
                related_topic_keys: vec![format!("topic-{index}")],
            });
        }
        validate_result(&minimal_snapshot(), &result).unwrap();
        let round_trip: GenerationResult =
            serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
        assert_eq!(
            round_trip
                .ideas
                .iter()
                .map(|idea| idea.disposition.as_str())
                .collect::<Vec<_>>(),
            vec!["unverified", "deferred", "out_of_scope", "rejected"]
        );
        for (index, idea) in round_trip.ideas.iter().enumerate() {
            assert_eq!(idea.body_markdown, format!("Idea body {index}"));
            assert_eq!(idea.reconsideration_conditions, vec![format!("Condition {index}")]);
            assert!(!round_trip.article.body_markdown.contains(&idea.body_markdown));
        }
    }
    #[test]
    fn adopted_topic_without_authoritative_decision_is_not_final() {
        let source = SourceRef {
            source_type: "run_final_report".into(),
            id: "run".into(),
            revision: "1".into(),
            locator: "final_report".into(),
            quote: "Receipt recorded; approval remains required.".into(),
        };
        let mut snapshot = minimal_snapshot();
        snapshot["journey"] = json!({
            "topicStates":[{"topicKey":"release","status":"adopted","currentNodeId":"decision"}],
            "nodes":[{"id":"decision","active":true,"status":"adopted","claimIds":["decision-claim"]}],
            "completionSnapshots":[{"status":"current","claimIds":[]}],
            "result":{"claims":[]}
        });
        let mut result = article(source);
        result.article.final_outcomes.push(FinalOutcome {
            topic_key: "release".into(),
            statement: "Receipt recorded; approval remains required.".into(),
            claim_ids: vec!["decision-claim".into()],
        });
        result.article.claim_bindings[0].claim_id = "decision-claim".into();
        assert!(validate_result(&snapshot, &result)
            .unwrap_err()
            .contains("authoritative Distillation claim"));
    }
    #[test]
    fn exact_current_decision_can_become_its_topic_outcome() {
        let source = SourceRef {
            source_type: "task_decision".into(),
            id: "decision".into(),
            revision: "1".into(),
            locator: "payload_json".into(),
            quote: "Approve offline".into(),
        };
        let snapshot = json!({
            "journey":{
                "topicStates":[{"topicKey":"release","status":"adopted","currentNodeId":"decision-node"}],
                "nodes":[{"id":"decision-node","active":true,"status":"adopted","claimIds":["decision-claim"]}],
                "completionSnapshots":[{"status":"current","claimIds":["completion-claim"]}],
                "result":{"claims":[{"id":"decision-claim","kind":"decision","statement":"Approve offline","actor":"user","epistemicState":"decided","status":"current","topicKey":"release","sources":[{"type":"task_decision","id":"decision","revision":"1","locator":"payload_json","quote":"Approve offline"}]}],"warnings":[]}
            },
            "distillations":[],
            "sourceManifest":[{"type":"task_decision","id":"decision","revision":"1","locator":"payload_json","role":"decision","content":{"body":"Approve offline"}}]
        });
        let mut result = article(source);
        result.article.claim_bindings[0].claim_id = "decision-claim".into();
        result.article.claim_bindings[0].epistemic_state = "decided".into();
        result.article.final_outcomes = vec![FinalOutcome {
            topic_key: "release".into(),
            statement: "Approve offline".into(),
            claim_ids: vec!["decision-claim".into()],
        }];
        validate_result(&snapshot, &result).unwrap();
    }
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Value) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db.sqlite");
        database::initialize(&db).unwrap();
        let c = database::open(&db).unwrap();
        c.execute("INSERT INTO captures(id,text) VALUES('capture','raw')", [])
            .unwrap();
        c.execute("INSERT INTO tasks(id,origin_capture_id,current_revision,state,created_at,last_user_activity_at) VALUES('task','capture',1,'completed','2026-01-01','2026-01-01')",[]).unwrap();
        c.execute("INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES('task',1,'Approval flow','Keep approval explicit','Approved behavior','offline','online','tests','task-hash','2026-01-01')",[]).unwrap();
        c.execute("INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion','task',1,'cargo test passed','Approval remains required','complete-op','2026-01-02')",[]).unwrap();
        c.execute("INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log','task','work','2026-01-02')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,work_log_entry_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('work_log','log',1,'task','log','work-hash','run_report_distillation',2,'rules','schema','en','{}','current','work-job')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('work_log','log',1,'work-hash','current')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('task_journey','task',1,'task','journey-hash','task_journey_increment',2,'rules','schema','en','{}','current','journey-job')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('task_journey','task',1,'journey-hash','current')",[]).unwrap();
        c.execute("INSERT INTO task_distillation_completion_snapshots(task_id,snapshot_id,completion_id,completion_revision,status,snapshot_json,projection_revision) VALUES('task','snapshot','completion','1','current','{\"id\":\"snapshot\",\"completionId\":\"completion\",\"completionRevision\":\"1\",\"status\":\"current\",\"claimIds\":[]}',1)",[]).unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let snapshot = capture_snapshot_tx(&tx, "task", 1, "en").unwrap();
        tx.commit().unwrap();
        (root, db, snapshot)
    }
    fn remove_semantic_projections(db: &std::path::Path) {
        database::open(db).unwrap().execute_batch(
            "DELETE FROM task_distillation_completion_snapshots;
             DELETE FROM task_distillation_current;
             DELETE FROM task_distillation_revisions;"
        ).unwrap();
    }

    #[test]
    fn manual_recorded_snapshot_tracks_log_edits_and_never_grants_finality() {
        let (_root, db, _) = fixture();
        remove_semantic_projections(&db);
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let snapshot = capture_snapshot_tx(&tx, "task", 1, "en").unwrap();
        tx.commit().unwrap();
        assert_eq!(snapshot["basis"], "recorded_only");
        assert_eq!(snapshot["journey"]["topicStates"], json!([]));
        assert_eq!(snapshot["distillations"], json!([]));
        let log = snapshot["sourceManifest"].as_array().unwrap().iter()
            .find(|source| source["type"] == "task_work_log").unwrap();
        let reference = SourceRef { source_type:"task_work_log".into(),id:"log".into(),
            revision:log["revision"].as_str().unwrap().into(),locator:"record".into(),quote:"work".into() };
        let valid = article(reference);
        validate_result(&snapshot, &valid).unwrap();
        let prepared = prepared(snapshot.clone(), valid.clone(), "enhanced", "").unwrap();
        let mut inflated = valid;
        inflated.article.claim_bindings[0].epistemic_state = "verified".into();
        assert!(validate_result(&snapshot, &inflated).is_err());
        c.execute("UPDATE task_work_log_entries SET body='Corrected work' WHERE id='log'", []).unwrap();
        let tx = c.transaction().unwrap();
        assert_ne!(capture_snapshot_tx(&tx,"task",1,"en").unwrap()["generationSnapshotHash"],snapshot["generationSnapshotHash"]);
        assert!(append_generated_tx(&tx,&prepared).unwrap_err().contains("source_snapshot_stale"));
    }

    #[test]
    fn any_run_requires_current_distillation_even_for_manual_completion() {
        let (_root, db, _) = fixture();
        remove_semantic_projections(&db);
        let mut c = database::open(&db).unwrap();
        c.execute_batch("INSERT INTO task_work_sessions(id,task_id,title,created_at,updated_at) VALUES('session','task','Work',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
            INSERT INTO task_work_session_entries(id,session_id,author,kind,body,created_at) VALUES('entry','session','user','note','Inspect',CURRENT_TIMESTAMP);
            INSERT INTO task_work_session_runs(id,task_id,session_id,submission_key,payload_hash,instruction,user_entry_id,work_log_entry_id,model,workspace_path,context_hash,status,final_report,revision,created_at,updated_at) VALUES('run','task','session','submission','hash','Inspect','entry','log','model','','context','failed',NULL,1,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);").unwrap();
        let tx = c.transaction().unwrap();
        assert!(capture_snapshot_tx(&tx,"task",1,"en").unwrap_err().contains("journey_not_current"));
    }

    // Exercise the exact packaged-desktop fake over HTTP through the real
    // registry, source snapshot, result validator and immutable append path.
    #[tokio::test]
    async fn desktop_provider_generates_from_real_manual_sources() {
        use std::io::BufRead;
        struct ChildGuard(std::process::Child);
        impl Drop for ChildGuard {
            fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
        }
        let (root, db, _) = fixture();
        remove_semantic_projections(&db);
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fakes/openai_server.mjs");
        let mut server = ChildGuard(std::process::Command::new("node").arg(script)
            .args(["--port","0"]).stdout(std::process::Stdio::piped()).spawn().unwrap());
        let mut port = String::new();
        std::io::BufReader::new(server.0.stdout.take().unwrap()).read_line(&mut port).unwrap();
        let port: u16 = port.trim().parse().unwrap();
        let settings = root.path().join("settings.json");
        crate::native::settings::save_provider(&settings,&json!({"base_url":format!("http://127.0.0.1:{port}/v1"),"model":"fixture","api_key":"fixture"})).unwrap();
        let generated = crate::native::task_assistance::prepare_knowledge_draft(&db,&settings,
            &json!({"taskId":"task","expectedTaskRevision":1,"locale":"en"})).await.unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let saved = append_generated_tx(&tx,&generated).unwrap();
        tx.commit().unwrap();
        assert_eq!(saved["draftRevision"],1);
        assert!(saved["bodyMarkdown"].as_str().unwrap().contains("Approval remains required"));
        assert_eq!(saved["article"]["finalOutcomes"],json!([]));
        let vault = root.path().join("wiki");
        std::fs::create_dir(&vault).unwrap();
        let proposal = crate::native::knowledge_archive::prepare(&db,&settings,&vault,
            &crate::native::semantic::SemanticEngine::new(None),
            &json!({"operationId":"fixture-prepare","taskId":"task","knowledgeRevision":1,
                "expectedKnowledgeContentHash":saved["contentHash"],
                "expectedGenerationSnapshotHash":saved["sourceHash"],
                "selectedIdeaRevisionIds":[],"locale":"en"})).await.unwrap();
        assert_eq!(proposal["state"],"review_needed");
        assert_eq!(proposal["target"]["path"],"Knowledge/deterministic.md");
        assert!(proposal["artifacts"].as_array().unwrap().iter().any(|artifact|
            artifact["bytes"].as_str().is_some_and(|bytes| bytes.contains(saved["bodyMarkdown"].as_str().unwrap()))));
        assert!(!vault.join("Knowledge/deterministic.md").exists());

    }

    #[test]
    fn append_is_atomic_and_recomputes_the_exact_snapshot() {
        let (_root, db, snapshot) = fixture();
        let manifest = snapshot["sourceManifest"].as_array().unwrap();
        let task_source = manifest
            .iter()
            .find(|source| source["type"] == "task_revision")
            .unwrap();
        let reference = SourceRef {
            source_type: "task_revision".into(),
            id: "task".into(),
            revision: "1".into(),
            locator: "definition".into(),
            quote: "Approval flow".into(),
        };
        assert_eq!(manifest_key(task_source), source_key(&reference));
        let prepared = prepared(snapshot.clone(), article(reference), "enhanced", "").unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let saved = append_generated_tx(&tx, &prepared).unwrap();
        tx.commit().unwrap();
        assert_eq!(saved["draftRevision"], 1);
        assert_eq!(
            c.query_row("SELECT count(*) FROM knowledge_draft_versions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM task_knowledge_drafts", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            1
        );
        c.execute("UPDATE tasks SET current_revision=2 WHERE id='task'", [])
            .unwrap();
        let tx = c.transaction().unwrap();
        assert!(append_generated_tx(&tx, &prepared)
            .unwrap_err()
            .contains("task_revision_conflict"));
        drop(tx);
        assert_eq!(
            c.query_row("SELECT count(*) FROM knowledge_draft_versions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            1
        );
    }
    #[test]
    fn provider_idea_identity_is_stable_and_scoped_to_its_task() {
        let (_root, db, first_snapshot) = fixture();
        let mut connection = database::open(&db).unwrap();
        connection.execute_batch(
            "INSERT INTO captures(id,text) VALUES('capture-2','raw');
             INSERT INTO tasks(id,origin_capture_id,current_revision,state,created_at,last_user_activity_at) VALUES('task-2','capture-2',1,'completed','2026-01-01','2026-01-01');
             INSERT INTO task_revisions(task_id,revision,title,detail,outcome,scope,non_goals,validation_criteria,content_hash,created_at) VALUES('task-2',1,'Approval flow','Keep approval explicit','Approved behavior','offline','online','tests','task-2-hash','2026-01-01');
             INSERT INTO task_completions(id,task_id,task_revision,evidence,report,operation_id,created_at) VALUES('completion-2','task-2',1,'cargo test passed','Approval remains required','complete-op-2','2026-01-02');
             INSERT INTO task_work_log_entries(id,task_id,body,created_at) VALUES('log-2','task-2','work','2026-01-02');
             INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,work_log_entry_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('work_log','log-2',1,'task-2','log-2','work-hash-2','run_report_distillation',2,'rules','schema','en','{}','current','work-job-2');
             INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('work_log','log-2',1,'work-hash-2','current');
             INSERT INTO task_distillation_revisions(owner_type,owner_id,revision,task_id,source_set_hash,prompt_id,prompt_version,rules_version,result_schema_version,locale,result_json,freshness,job_id) VALUES('task_journey','task-2',1,'task-2','journey-hash-2','task_journey_increment',2,'rules','schema','en','{}','current','journey-job-2');
             INSERT INTO task_distillation_current(owner_type,owner_id,revision,source_set_hash,freshness) VALUES('task_journey','task-2',1,'journey-hash-2','current');
             INSERT INTO task_distillation_completion_snapshots(task_id,snapshot_id,completion_id,completion_revision,status,snapshot_json,projection_revision) VALUES('task-2','snapshot-2','completion-2','1','current','{\"id\":\"snapshot-2\",\"completionId\":\"completion-2\",\"completionRevision\":\"1\",\"status\":\"current\",\"claimIds\":[]}',1);",
        ).unwrap();
        let tx = connection.transaction().unwrap();
        let second_snapshot = capture_snapshot_tx(&tx, "task-2", 1, "en").unwrap();
        tx.commit().unwrap();
        let prepare = |snapshot: Value, task_id: &str| {
            let source = SourceRef {
                source_type: "task_revision".into(), id: task_id.into(), revision: "1".into(),
                locator: "definition".into(), quote: "Approval flow".into(),
            };
            let mut result = article(source.clone());
            result.ideas.push(Idea {
                id: "idea-1".into(), title: "Reminder".into(), body_markdown: "Try a reminder later".into(),
                disposition: "unverified".into(), reconsideration_conditions: vec!["A scheduler is available".into()],
                source_refs: vec![source], related_topic_keys: vec![],
            });
            prepared(snapshot, result, "enhanced", "").unwrap()
        };
        let first = prepare(first_snapshot, "task");
        let second = prepare(second_snapshot, "task-2");
        let tx = connection.transaction().unwrap();
        let first_saved = append_generated_tx(&tx, &first).unwrap();
        tx.commit().unwrap();
        let tx = connection.transaction().unwrap();
        let second_saved = append_generated_tx(&tx, &second).unwrap();
        tx.commit().unwrap();
        let first_id = first_saved["ideas"][0]["id"].as_str().unwrap();
        let second_id = second_saved["ideas"][0]["id"].as_str().unwrap();
        assert_ne!(first_id, second_id);
        assert!(first_id.starts_with("idea:"));
        assert_eq!(
            connection.query_row("SELECT revision FROM knowledge_idea_revisions WHERE id=?", [first_id], |row| row.get::<_, i64>(0)).unwrap(),
            1
        );
        assert_eq!(
            connection.query_row("SELECT revision FROM knowledge_idea_revisions WHERE id=?", [second_id], |row| row.get::<_, i64>(0)).unwrap(),
            1
        );
        let stored_result: String = connection.query_row(
            "SELECT result_json FROM knowledge_draft_versions WHERE task_id='task-2' AND revision=1", [], |row| row.get(0),
        ).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&stored_result).unwrap()["ideas"][0]["id"], second_id);
    }
    #[test]
    fn schema_head_includes_immutable_knowledge_tables() {
        let (_root, db, _) = fixture();
        let c = database::open(&db).unwrap();
        assert_eq!(
            migrations::schema_version(&c).unwrap(),
            migrations::CURRENT_SCHEMA_VERSION
        );
    }

    #[test]
    fn repeated_generation_reuses_snapshot_and_restore_appends_without_history_changes() {
        let (_root, db, snapshot) = fixture();
        let reference = SourceRef {
            source_type: "task_revision".into(),
            id: "task".into(),
            revision: "1".into(),
            locator: "definition".into(),
            quote: "Approval flow".into(),
        };
        let prepared = prepared(snapshot, article(reference), "enhanced", "").unwrap();
        let mut c = database::open(&db).unwrap();
        for _ in 0..2 {
            let tx = c.transaction().unwrap();
            append_generated_tx(&tx, &prepared).unwrap();
            tx.commit().unwrap();
        }
        let before: (i64, i64, i64) = c
            .query_row(
                "SELECT (SELECT count(*) FROM task_work_log_entries),(SELECT count(*) FROM task_decisions),(SELECT count(*) FROM task_distillation_revisions)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let tx = c.transaction().unwrap();
        let restored = restore_as_new_tx(&tx, "task", 1, 2).unwrap();
        tx.commit().unwrap();
        assert_eq!(restored["draftRevision"], 3);
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM knowledge_evidence_snapshots",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row(
                "SELECT published_revision FROM knowledge_pointers WHERE task_id='task'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .unwrap(),
            None
        );
        let after: (i64, i64, i64) = c
            .query_row(
                "SELECT (SELECT count(*) FROM task_work_log_entries),(SELECT count(*) FROM task_decisions),(SELECT count(*) FROM task_distillation_revisions)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn edit_appends_and_rejects_removing_grounded_content() {
        let (_root, db, snapshot) = fixture();
        let reference = SourceRef {
            source_type: "task_revision".into(),
            id: "task".into(),
            revision: "1".into(),
            locator: "definition".into(),
            quote: "Approval flow".into(),
        };
        let prepared = prepared(snapshot, article(reference), "enhanced", "").unwrap();
        let mut c = database::open(&db).unwrap();
        let tx = c.transaction().unwrap();
        let first = append_generated_tx(&tx, &prepared).unwrap();
        tx.commit().unwrap();
        let edited_body = format!(
            "{}\n\n## Note\n\nEditorial note.",
            first["bodyMarkdown"].as_str().unwrap()
        );
        let tx = c.transaction().unwrap();
        let edited = edit_as_new_tx(
            &tx,
            "task",
            1,
            first["contentHash"].as_str().unwrap(),
            first["sourceHash"].as_str().unwrap(),
            &edited_body,
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(edited["draftRevision"], 2);
        assert_ne!(edited["contentHash"], first["contentHash"]);
        assert_eq!(c.query_row("SELECT body_markdown FROM knowledge_draft_versions WHERE task_id='task' AND revision=1",[],|row|row.get::<_,String>(0)).unwrap(),first["bodyMarkdown"]);
        let tx = c.transaction().unwrap();
        let error = edit_as_new_tx(
            &tx,
            "task",
            2,
            edited["contentHash"].as_str().unwrap(),
            edited["sourceHash"].as_str().unwrap(),
            "# Removed evidence",
        )
        .unwrap_err();
        assert!(
            error.contains("applicability_overreach") || error.contains("bound claim"),
            "{error}"
        );
        drop(tx);
        assert_eq!(
            c.query_row("SELECT count(*) FROM knowledge_draft_versions", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            2
        );
    }
}
