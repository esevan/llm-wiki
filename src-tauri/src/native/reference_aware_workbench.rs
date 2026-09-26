mod runtime;
pub(crate) use runtime::*;
use crate::workflow_foundation::{validate_retrieval_intent, RetrievalIntent};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn hash(value: &Value) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}
fn required_fields(value: &Value) -> Result<(), String> {
    for key in [
        "description",
        "background",
        "goal",
        "scope",
        "nonGoals",
        "constraints",
        "completionCriteria",
        "initialApproach",
    ] {
        if value.get(key).is_none() {
            return Err(format!("invalid_preview: missing {key}"));
        }
    }
    if value["description"]
        .as_str()
        .unwrap_or("")
        .trim()
        .is_empty()
        || value["goal"].as_str().unwrap_or("").trim().is_empty()
    {
        return Err("invalid_preview: description and goal are required".into());
    }
    if !value["constraints"].is_array() || !value["completionCriteria"].is_array() {
        return Err("invalid_preview: list fields must be arrays".into());
    }
    Ok(())
}
pub(crate) fn validate_planner(value: &Value) -> Result<RetrievalIntent, String> {
    if value["retrieval"].as_object().is_none_or(|o|o.keys().any(|k| !["needed","reason","queries","aspects","filters","requery"].contains(&k.as_str()))) {return Err("invalid_retrieval_intent".into());}
    let intent: RetrievalIntent = serde_json::from_value(value["retrieval"].clone())
        .map_err(|e| format!("invalid_retrieval_intent: {e}"))?;
    validate_retrieval_intent(&intent).map_err(|e| e.to_string())?;
    if intent.queries.len()>4 || intent.aspects.iter().any(|a| !matches!(a.as_str(),"content"|"applicability"|"decision"|"exploration")) || intent.filters.keys().any(|k| !matches!(k.as_str(),"informationTypes"|"statuses")) { return Err("invalid_retrieval_intent".into()); }
    if intent.filters.values().any(|v|v.as_array().is_none_or(|a|a.len()>16||a.iter().any(|v|!v.is_string()))) {return Err("invalid_retrieval_intent: filters".into());}
    if intent.requery.as_ref().is_some_and(|r|r.queries.len()>2||r.queries.iter().any(|q|q.len()>500)) {return Err("invalid_retrieval_intent: requery budget".into());}
    Ok(intent)
}
pub(crate) fn finalize(preview: &Value, references: &[Value]) -> Result<Value, String> {
    let fields = &preview["preview"];
    required_fields(fields)?;
    let allowed = references
        .iter()
        .filter_map(|r| {
            Some((
                r["documentId"].as_str()?,
                r["documentVersion"].as_str()?,
                r["section"].as_str().unwrap_or(""),
            ))
        })
        .collect::<std::collections::HashSet<_>>();
    for claim in preview["claimSources"]
        .as_array()
        .ok_or("invalid_preview: claimSources must be an array")?
    {
        let binding = (
            claim["documentId"].as_str().ok_or("invalid reference")?,
            claim["documentVersion"]
                .as_str()
                .ok_or("invalid reference")?,
            claim["section"].as_str().unwrap_or(""),
        );
        if !allowed.contains(&binding) {
            return Err("invalid_reference_binding".into());
        }
    }
    Ok(fields.clone())
}
pub(crate) fn append_generated(
    db: &Path,
    session_id: &str,
    expected_context: i64,
    task_revision: Option<i64>,
    locale: &str,
    fields: &Value,
    assumptions: &[Value],
    references: &[Value],
    job_id: &str,
) -> Result<Value, String> {
    required_fields(fields)?;
    let mut c = super::database::open(db)?;
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let actual: i64 = tx
        .query_row(
            "SELECT current_draft_revision FROM refinement_sessions WHERE id=?",
            [session_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if actual != expected_context {
        return Err("context_revision_conflict".into());
    }
    let existing: Option<(String, i64)> = tx
        .query_row(
            "SELECT id,current_version FROM work_previews WHERE session_id=?",
            [session_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let (preview_id, current) = existing.unwrap_or_else(|| (id(), 0));
    let version = current + 1;
    let content_hash = hash(fields);
    tx.execute("INSERT OR IGNORE INTO work_previews(id,session_id,subject_type,subject_id,current_version,context_revision,task_revision) SELECT ?,id,CASE WHEN task_id IS NULL THEN 'capture' ELSE 'task' END,COALESCE(task_id,capture_id),0,?,? FROM refinement_sessions WHERE id=?",params![preview_id,expected_context,task_revision,session_id]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO work_preview_versions(preview_id,version,derivation_kind,generation_job_id,task_revision,context_revision,locale,fields_json,content_hash,prompt_id,prompt_version) VALUES(?,?,'generated',?,?,?,?,?,?,'refinement_preview',1)",params![preview_id,version,job_id,task_revision,expected_context,locale,fields.to_string(),content_hash]).map_err(|e|e.to_string())?;
    for assumption in assumptions {
        tx.execute("INSERT INTO work_preview_assumptions(preview_id,version,id,text,status,basis,source_claim_ids_json) VALUES(?,?,?,?,'open',?,?)",params![preview_id,version,assumption["id"].as_str().unwrap_or(""),assumption["text"].as_str().unwrap_or(""),assumption["basis"].as_str().unwrap_or("model_inference"),assumption["sourceClaimIds"].to_string()]).map_err(|e|e.to_string())?;
    }
    for reference in references {
        tx.execute("INSERT INTO work_preview_references(id,preview_id,version,retrieval_request_hash,source_bundle_hash,document_id,document_version,section,role,claim_ids_json) VALUES(?,?,?,?,?,?,?,?,?,?)",params![id(),preview_id,version,reference["retrievalRequestHash"].as_str().unwrap_or("not_needed"),hash(&json!(references)),reference["documentId"].as_str().unwrap_or(""),reference["documentVersion"].as_str().unwrap_or(""),reference["section"].as_str().unwrap_or(""),reference["role"].as_str().unwrap_or("context"),reference["claimIds"].to_string()]).map_err(|e|e.to_string())?;
    }
    tx.execute("UPDATE work_previews SET current_version=?,context_revision=?,task_revision=?,updated_at=CURRENT_TIMESTAMP WHERE id=? AND current_version=?",params![version,expected_context,task_revision,preview_id,current]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(
        json!({"previewId":preview_id,"version":version,"contentHash":content_hash,"applicationDisposition":"review_needed"}),
    )
}
pub(crate) fn compare(db: &Path, preview_id: &str, left: i64, right: i64) -> Result<Value, String> {
    let c = super::database::open(db)?;
    let load = |v| {
        c.query_row(
            "SELECT fields_json FROM work_preview_versions WHERE preview_id=? AND version=?",
            params![preview_id, v],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()))
    };
    let a = load(left)?;
    let b = load(right)?;
    let fields = [
        "description",
        "background",
        "goal",
        "scope",
        "nonGoals",
        "constraints",
        "completionCriteria",
        "initialApproach",
        "assumptions",
    ]
    .map(|k| json!({"key":k,"state":if a[k]==b[k]{"unchanged"}else{"changed"}}));
    Ok(json!({"left":a,"right":b,"fields":fields}))
}
pub(crate) fn restore(
    db: &Path,
    preview_id: &str,
    source: i64,
    expected: i64,
) -> Result<Value, String> {
    let mut c = super::database::open(db)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let current: i64 = tx
        .query_row(
            "SELECT current_version FROM work_previews WHERE id=?",
            [preview_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if current != expected {
        return Err("preview_head_conflict".into());
    }
    let next = current + 1;
    tx.execute("INSERT INTO work_preview_versions(preview_id,version,derivation_kind,derived_from_version,task_revision,context_revision,locale,fields_json,content_hash,prompt_id,prompt_version) SELECT preview_id,?,'restored',version,task_revision,context_revision,locale,fields_json,content_hash,prompt_id,prompt_version FROM work_preview_versions WHERE preview_id=? AND version=?",params![next,preview_id,source]).map_err(|e|e.to_string())?;
    tx.execute(
        "UPDATE work_previews SET current_version=? WHERE id=? AND current_version=?",
        params![next, preview_id, expected],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(json!({"version":next,"derivedFromVersion":source}))
}
pub(crate) fn start_investigation(
    db: &Path,
    preview_id: &str,
    context: i64,
    kind: &str,
    request: &Value,
) -> Result<String, String> {
    let c = super::database::open(db)?;
    c.execute("UPDATE work_preview_investigations SET state='changed_context',finished_at=CURRENT_TIMESTAMP WHERE preview_id=? AND kind=? AND state IN ('queued','running')",params![preview_id,kind]).map_err(|e|e.to_string())?;
    let investigation = id();
    c.execute("INSERT INTO work_preview_investigations(id,preview_id,context_revision,kind,state,request_json) VALUES(?,?,?,?,'queued',?)",params![investigation,preview_id,context,kind,request.to_string()]).map_err(|e|e.to_string())?;
    Ok(investigation)
}
pub(crate) fn record_use(
    db: &Path,
    preview_id: &str,
    context: i64,
    reference: &Value,
    kind: &str,
    reason: &str,
) -> Result<(), String> {
    if !matches!(kind, "viewed" | "mentioned" | "used" | "excluded") {
        return Err("invalid reference interaction".into());
    }
    super::database::open(db)?.execute("INSERT INTO reference_interactions(id,preview_id,context_revision,document_id,document_version,section,kind,reason) VALUES(?,?,?,?,?,?,?,?)",params![id(),preview_id,context,reference["documentId"].as_str().unwrap_or(""),reference["documentVersion"].as_str().unwrap_or(""),reference["section"].as_str().unwrap_or(""),kind,reason]).map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("db");
        super::super::database::initialize(&db).unwrap();
        let c = super::super::database::open(&db).unwrap();
        c.execute("INSERT INTO captures(id,text) VALUES('c','raw')", [])
            .unwrap();
        c.execute("INSERT INTO refinement_sessions(id,capture_id,current_draft_revision) VALUES('s','c',1)",[]).unwrap();
        (root, db)
    }
    fn fields(goal: &str) -> Value {
        json!({"description":"Change contract","background":"Known","goal":goal,"scope":"API","nonGoals":"Deploy","constraints":["approval"],"completionCriteria":["tests pass"],"initialApproach":["inspect"]})
    }
    #[test]
    fn planner_distinguishes_not_needed_and_rejects_empty_needed() {
        assert!(!validate_planner(&json!({"retrieval":{"needed":false,"reason":"Enough context","queries":[],"aspects":[],"filters":{},"requery":null}})).unwrap().needed);
        assert!(validate_planner(&json!({"retrieval":{"needed":true,"reason":"check","queries":[],"aspects":[],"filters":{},"requery":null}})).is_err());
    }
    #[test]
    fn grounded_append_compare_restore_and_use_do_not_mutate_canonical_work() {
        let (_root, db) = fixture();
        let refs = vec![
            json!({"documentId":"d","documentVersion":"v1","section":"Approval","role":"constraint","claimIds":["background:approval"]}),
        ];
        let output = json!({"preview":fields("Ship safely"),"claimSources":[{"claimId":"background:approval","documentId":"d","documentVersion":"v1","section":"Approval"}]});
        let final_fields = finalize(&output, &refs).unwrap();
        let first =
            append_generated(&db, "s", 1, None, "en", &final_fields, &[], &refs, "j1").unwrap();
        let preview = first["previewId"].as_str().unwrap();
        let second = append_generated(
            &db,
            "s",
            1,
            None,
            "en",
            &fields("Ship carefully"),
            &[],
            &refs,
            "j2",
        )
        .unwrap();
        assert_eq!(second["version"], 2);
        assert_eq!(
            compare(&db, preview, 1, 2).unwrap()["fields"][2]["state"],
            "changed"
        );
        assert_eq!(restore(&db, preview, 1, 2).unwrap()["version"], 3);
        record_use(&db, preview, 1, &refs[0], "used", "supports constraint").unwrap();
        let c = super::super::database::open(&db).unwrap();
        assert_eq!(
            c.query_row("SELECT text FROM captures WHERE id='c'", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "raw"
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM reference_interactions WHERE kind='used'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    #[test]
    fn newer_investigation_marks_prior_context_changed() {
        let (_root, db) = fixture();
        let p = append_generated(&db, "s", 1, None, "en", &fields("Goal"), &[], &[], "j").unwrap();
        let preview = p["previewId"].as_str().unwrap();
        let old = start_investigation(&db, preview, 1, "speculative_search", &json!({})).unwrap();
        start_investigation(&db, preview, 2, "speculative_search", &json!({})).unwrap();
        let c = super::super::database::open(&db).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT state FROM work_preview_investigations WHERE id=?",
                [old],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "changed_context"
        );
    }
}

#[cfg(test)]
mod runtime_tests;
