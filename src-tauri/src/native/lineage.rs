use crate::native::database;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub(crate) fn create(db_path: &Path, feature_id: &str, force: bool) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (problem_id, title, outcome, solution_created): (String, String, String, String) =
        connection
            .query_row(
                "SELECT problem_id,title,outcome,created_at FROM features WHERE id=?",
                [feature_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or("Solution not found")?;
    let (capture_id, statement, detail, problem_created): (Option<String>, String, String, String) =
        connection
            .query_row(
                "SELECT capture_id,statement,detail,created_at FROM problems WHERE id=?",
                [&problem_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| error.to_string())?;
    let capture = capture_id.as_ref().and_then(|capture_id| {
        connection
            .query_row(
                "SELECT text,created_at FROM captures WHERE id=?",
                [capture_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .ok()
    });
    let completion: Option<(String, String, String)> = connection
        .query_row("SELECT reason,created_at,id FROM problem_completion_decisions WHERE problem_id=? ORDER BY created_at DESC,rowid DESC LIMIT 1", [&problem_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .optional().map_err(|error| error.to_string())?;
    let source_hash = digest(&json!({"capture":capture,"problem":[statement,detail],"solution":[title,outcome],"completion":completion}).to_string());
    if !force {
        let existing = connection.query_row("SELECT id FROM lineage_snapshots WHERE feature_id=? AND source_hash=? AND schema_version=? ORDER BY version DESC LIMIT 1", params![feature_id, source_hash, SCHEMA_VERSION], |row| row.get::<_, String>(0)).optional().map_err(|error| error.to_string())?;
        if let Some(snapshot_id) = existing {
            return load(&connection, feature_id, &snapshot_id);
        }
    }
    let version: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version),0)+1 FROM lineage_snapshots WHERE feature_id=?",
            [feature_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let snapshot_id = id();
    connection.execute("INSERT INTO lineage_snapshots(id,feature_id,version,schema_version,source_hash,status) VALUES (?,?,?,?,?,'ready_without_inference')", params![snapshot_id, feature_id, version, SCHEMA_VERSION, source_hash]).map_err(|error| error.to_string())?;
    let mut stages = Vec::new();
    let mut claims = Map::new();
    let mut evidence = Map::new();
    if let (Some(capture_id), Some((text, created))) = (capture_id.as_deref(), capture.as_ref()) {
        add_stage(
            &connection,
            &snapshot_id,
            &mut stages,
            &mut claims,
            &mut evidence,
            Stage {
                kind: "capture",
                record_type: "captures",
                record_id: capture_id,
                title: text,
                text,
                occurred_at: created,
                classification: "observed",
            },
        )?;
    }
    add_stage(
        &connection,
        &snapshot_id,
        &mut stages,
        &mut claims,
        &mut evidence,
        Stage {
            kind: "problem",
            record_type: "problems",
            record_id: &problem_id,
            title: &statement,
            text: &format!("{statement}\n{detail}"),
            occurred_at: &problem_created,
            classification: "observed",
        },
    )?;
    add_stage(
        &connection,
        &snapshot_id,
        &mut stages,
        &mut claims,
        &mut evidence,
        Stage {
            kind: "solution",
            record_type: "features",
            record_id: feature_id,
            title: &title,
            text: &format!("{title}\n{outcome}"),
            occurred_at: &solution_created,
            classification: "decided",
        },
    )?;
    if let Some((reason, created, decision_id)) = completion {
        add_stage(
            &connection,
            &snapshot_id,
            &mut stages,
            &mut claims,
            &mut evidence,
            Stage {
                kind: "complete",
                record_type: "problem_completion_decisions",
                record_id: &decision_id,
                title: "Completed",
                text: &reason,
                occurred_at: &created,
                classification: "decided",
            },
        )?;
    }
    let transitions = stages.windows(2).enumerate().map(|(index, pair)| json!({"from":pair[0]["kind"],"to":pair[1]["kind"],"context_kind":if index == 1 {"recorded_change"} else {"recorded_transition"},"claim_id":pair[1]["claim_id"]})).collect::<Vec<_>>();
    let document = json!({"snapshot_id":snapshot_id,"feature_id":feature_id,"version":version,"status":"ready_without_inference","source_hash":source_hash,"lineage":{"stages":stages,"transitions":transitions},"claims":claims,"evidence":evidence,"decision_changes":[],"conflicts":[],"completion_evidence":[],"generation":{"schema_version":SCHEMA_VERSION,"inference_error":""}});
    connection
        .execute(
            "UPDATE lineage_snapshots SET document_json=? WHERE id=?",
            params![document.to_string(), snapshot_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(document)
}

struct Stage<'a> {
    kind: &'a str,
    record_type: &'a str,
    record_id: &'a str,
    title: &'a str,
    text: &'a str,
    occurred_at: &'a str,
    classification: &'a str,
}

fn add_stage(
    connection: &rusqlite::Connection,
    snapshot_id: &str,
    stages: &mut Vec<Value>,
    claims: &mut Map<String, Value>,
    evidence: &mut Map<String, Value>,
    stage: Stage<'_>,
) -> Result<(), String> {
    let claim_id = id();
    let evidence_id = id();
    let revision_id = id();
    connection.execute("INSERT INTO lineage_claims(id,snapshot_id,claim_key,section,subject_type,subject_id,classification,material) VALUES (?,?,?,?,?,?,?,1)", params![claim_id, snapshot_id, format!("stage:{}", stage.kind), "lineage", stage.record_type, stage.record_id, stage.classification]).map_err(|error| error.to_string())?;
    connection.execute("INSERT INTO lineage_revisions(id,claim_id,author_type,text) VALUES (?,?,'deterministic',?)", params![revision_id, claim_id, stage.text]).map_err(|error| error.to_string())?;
    connection.execute("INSERT INTO lineage_evidence(id,claim_id,source_type,source_id,field_name,excerpt,source_hash,live_entity_type) VALUES (?,?,?,?,?,?,?,?)", params![evidence_id, claim_id, stage.record_type, stage.record_id, "record", stage.text, digest(stage.text), stage.record_type]).map_err(|error| error.to_string())?;
    stages.push(json!({"kind":stage.kind,"record_type":stage.record_type,"record_id":stage.record_id,"id":stage.record_id,"title":stage.title,"occurred_at":stage.occurred_at,"claim_id":claim_id}));
    claims.insert(claim_id.clone(), json!({"id":claim_id,"claim_key":format!("stage:{}",stage.kind),"section":"lineage","classification":stage.classification,"confidence":null,"material":true,"text":stage.text,"evidence_ids":[evidence_id],"current_revision_id":revision_id,"current_author_type":"deterministic","revisions":[{"id":revision_id,"author_type":"deterministic","text":stage.text,"is_current":true}]}));
    evidence.insert(evidence_id.clone(), json!({"id":evidence_id,"claim_id":claim_id,"source_type":stage.record_type,"source_id":stage.record_id,"field_name":"record","excerpt":stage.text,"source_hash":digest(stage.text),"live_record":{"available":true,"entity_type":stage.record_type,"entity_id":stage.record_id}}));
    Ok(())
}

pub(crate) fn get(db_path: &Path, feature_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let snapshot = connection
        .query_row(
            "SELECT id FROM lineage_snapshots WHERE feature_id=? ORDER BY version DESC LIMIT 1",
            [feature_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(snapshot_id) = snapshot {
        load(&connection, feature_id, &snapshot_id)
    } else {
        drop(connection);
        create(db_path, feature_id, false)
    }
}

fn load(
    connection: &rusqlite::Connection,
    feature_id: &str,
    snapshot_id: &str,
) -> Result<Value, String> {
    let raw: String = connection
        .query_row(
            "SELECT document_json FROM lineage_snapshots WHERE id=? AND feature_id=?",
            params![snapshot_id, feature_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Lineage snapshot not found")?;
    let mut document = serde_json::from_str::<Value>(&raw).map_err(|error| error.to_string())?;
    for (claim_id, claim) in document["claims"]
        .as_object_mut()
        .ok_or("Lineage claims are invalid")?
    {
        let revisions = connection.prepare("SELECT id,supersedes_id,author_type,text,reason,is_current,created_at FROM lineage_revisions WHERE claim_id=? ORDER BY created_at,rowid").and_then(|mut statement| statement.query_map([claim_id], |row| Ok(json!({"id":row.get::<_,String>(0)?,"supersedes_id":row.get::<_,Option<String>>(1)?,"author_type":row.get::<_,String>(2)?,"text":row.get::<_,String>(3)?,"reason":row.get::<_,String>(4)?,"is_current":row.get::<_,bool>(5)?,"created_at":row.get::<_,String>(6)?})))?.collect::<Result<Vec<_>,_>>()).map_err(|error| error.to_string())?;
        if let Some(current) = revisions
            .iter()
            .find(|revision| revision["is_current"] == true)
        {
            claim["text"] = current["text"].clone();
            claim["current_revision_id"] = current["id"].clone();
            claim["current_author_type"] = current["author_type"].clone();
        }
        claim["revisions"] = Value::Array(revisions);
    }
    Ok(document)
}

pub(crate) fn evidence(
    db_path: &Path,
    feature_id: &str,
    evidence_id: &str,
) -> Result<Value, String> {
    get(db_path, feature_id)?["evidence"]
        .get(evidence_id)
        .cloned()
        .ok_or_else(|| "Lineage evidence not found".into())
}

pub(crate) fn correct(
    db_path: &Path,
    feature_id: &str,
    claim_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let text = input
        .get("text")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or("Lineage correction cannot be empty")?;
    let reason = input.get("reason").and_then(Value::as_str).unwrap_or("");
    let expected = input
        .get("current_revision_id")
        .or_else(|| input.get("currentRevisionId"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let connection = database::open(db_path)?;
    let classification: String = connection.query_row("SELECT lc.classification FROM lineage_claims lc JOIN lineage_snapshots ls ON ls.id=lc.snapshot_id WHERE lc.id=? AND ls.feature_id=?", params![claim_id, feature_id], |row| row.get(0)).optional().map_err(|error| error.to_string())?.ok_or("Lineage claim not found")?;
    if classification != "inferred" {
        return Err(
            "Only AI interpretations can be corrected; source-backed records are immutable".into(),
        );
    }
    let current: String = connection
        .query_row(
            "SELECT id FROM lineage_revisions WHERE claim_id=? AND is_current=1",
            [claim_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !expected.is_empty() && expected != current {
        return Err("Lineage claim changed; reload before correcting".into());
    }
    connection
        .execute(
            "UPDATE lineage_revisions SET is_current=0 WHERE claim_id=? AND is_current=1",
            [claim_id],
        )
        .map_err(|error| error.to_string())?;
    let revision_id = id();
    connection.execute("INSERT INTO lineage_revisions(id,claim_id,supersedes_id,author_type,text,reason,is_current) VALUES (?,?,?,'user',?,?,1)", params![revision_id, claim_id, current, text.trim(), reason.trim()]).map_err(|error| error.to_string())?;
    Ok(
        json!({"id":revision_id,"claim_id":claim_id,"supersedes_id":current,"author_type":"user","text":text.trim(),"reason":reason.trim(),"is_current":true}),
    )
}

pub(crate) fn add_inferences(
    db_path: &Path,
    feature_id: &str,
    result: &Value,
) -> Result<Value, String> {
    let mut lineage = get(db_path, feature_id)?;
    let snapshot_id = lineage["snapshot_id"]
        .as_str()
        .ok_or("Lineage snapshot is invalid")?
        .to_owned();
    let valid_evidence = lineage["evidence"].as_object().cloned().unwrap_or_default();
    let connection = database::open(db_path)?;
    for (index, inferred) in result
        .get("claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let text = inferred
            .get("text")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or("Inferred claim text is required")?;
        let evidence_ids = inferred
            .get("evidence_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if evidence_ids.is_empty()
            || evidence_ids.iter().any(|value| {
                value
                    .as_str()
                    .is_none_or(|evidence_id| !valid_evidence.contains_key(evidence_id))
            })
        {
            return Err("Inferred claims require valid evidence_ids".into());
        }
        let claim_id = id();
        let revision_id = id();
        let base_key = inferred
            .get("claim_key")
            .and_then(Value::as_str)
            .unwrap_or("inferred:interpretation");
        let claim_key = format!("{base_key}:{index}");
        connection.execute("INSERT INTO lineage_claims(id,snapshot_id,claim_key,section,subject_type,subject_id,classification,confidence,material) VALUES (?,?,?,'interpretation','features',?,'inferred',?,0)", params![claim_id, snapshot_id, claim_key, feature_id, inferred.get("confidence").and_then(Value::as_str)]).map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO lineage_revisions(id,claim_id,author_type,text) VALUES (?,?,'ai',?)",
                params![revision_id, claim_id, text],
            )
            .map_err(|error| error.to_string())?;
        lineage["claims"][&claim_id] = json!({"id":claim_id,"claim_key":claim_key,"section":"interpretation","classification":"inferred","confidence":inferred.get("confidence"),"material":false,"text":text,"evidence_ids":evidence_ids,"current_revision_id":revision_id,"current_author_type":"ai","revisions":[{"id":revision_id,"author_type":"ai","text":text,"is_current":true}]});
    }
    lineage["status"] = json!("ready");
    connection
        .execute(
            "UPDATE lineage_snapshots SET status='ready',document_json=? WHERE id=?",
            params![lineage.to_string(), snapshot_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(lineage)
}
