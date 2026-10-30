//! Immutable local image references shared by drafts, revisions and the Vault reader.
use super::database;
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) const PREFIX: &str = "llm-wiki-image:";
fn supported(media: &str) -> bool {
    matches!(
        media,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}
pub(crate) fn valid_ref(reference: &str) -> bool {
    reference.strip_prefix(PREFIX).is_some_and(|key| {
        key.len() == 64
            && key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
/// References include the original identity and bytes. No new copy or filesystem path.
pub(crate) fn annotate(value: &mut Value, owner: &str) {
    match value {
        Value::Object(object) => {
            if let (Some(data), Some(media)) = (
                object.get("data").and_then(Value::as_str),
                object.get("mediaType").and_then(Value::as_str),
            ) {
                if !data.is_empty() && supported(media) {
                    let identity = object
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(owner)
                        .to_owned();
                    let key = format!("{identity}\0{media}\0{data}");
                    object.insert("assetIdentity".into(), json!(identity));
                    object.insert(
                        "assetRef".into(),
                        json!(format!("{PREFIX}{:x}", Sha256::digest(key.as_bytes()))),
                    );
                }
            }
            for (key, child) in object {
                if key != "data" {
                    annotate(child, &format!("{owner}/{key}"));
                }
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter_mut().enumerate() {
                annotate(child, &format!("{owner}/{i}"));
            }
        }
        _ => {}
    }
}
pub(crate) fn catalog(snapshot: &Value) -> Vec<Value> {
    fn visit(value: &Value, source: &Value, out: &mut Vec<Value>) {
        match value {
            Value::Object(o) => {
                if let Some(reference) = o
                    .get("assetRef")
                    .and_then(Value::as_str)
                    .filter(|r| valid_ref(r))
                {
                    if !out.iter().any(|v| v["imageRef"] == reference) {
                        out.push(json!({"imageRef":reference,"attachmentId":o.get("id"),"name":o.get("name"),"mediaType":o.get("mediaType"),"source":{"type":source["type"],"id":source["id"],"revision":source["revision"],"locator":source["locator"]}}));
                    }
                }
                for (key, child) in o {
                    if key != "data" {
                        visit(child, source, out);
                    }
                }
            }
            Value::Array(a) => {
                for child in a {
                    visit(child, source, out);
                }
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    for source in snapshot["sourceManifest"].as_array().into_iter().flatten() {
        visit(&source["content"], source, &mut result);
    }
    result
}
fn find(value: &Value, reference: &str) -> Option<Value> {
    match value {
        Value::Object(o) => {
            if o.get("assetRef").and_then(Value::as_str) == Some(reference) {
                return Some(value.clone());
            }
            o.iter()
                .filter(|(key, _)| key.as_str() != "data")
                .find_map(|(_, v)| find(v, reference))
        }
        Value::Array(a) => a.iter().find_map(|v| find(v, reference)),
        _ => None,
    }
}
/// Keep previously embedded visuals available when their mutable source is removed.
pub(crate) fn inherit_retained(
    c: &rusqlite::Connection,
    task: &str,
    manifest: &mut Vec<Value>,
) -> Result<(), String> {
    let mut current = catalog(&json!({"sourceManifest":manifest}));
    let mut stmt=c.prepare("SELECT s.manifest_json,v.body_markdown,v.result_json FROM knowledge_draft_versions v JOIN knowledge_evidence_snapshots s ON s.id=v.generation_snapshot_id WHERE v.task_id=? AND (v.revision=(SELECT MAX(revision) FROM knowledge_draft_versions WHERE task_id=?) OR v.revision=(SELECT published_revision FROM knowledge_pointers WHERE task_id=?)) ORDER BY v.revision DESC").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![task, task, task], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (raw, body, result) = row.map_err(|e| e.to_string())?;
        let result: Value = serde_json::from_str(&result).map_err(|e| e.to_string())?;
        let snapshot: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        for entry in catalog(&snapshot) {
            let reference = entry["imageRef"].as_str().unwrap();
            let assessed = result["article"]["imageDecisions"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|decision| {
                    decision["imageRef"] == reference
                        && decision["disposition"] == "text_integrated"
                });
            if (body.contains(&format!("]({reference})")) || assessed)
                && !current.iter().any(|i| i["imageRef"] == reference)
            {
                if let Some(image) = find(&snapshot["sourceManifest"], reference) {
                    let content = json!({"image":image,"originalSource":entry["source"]});
                    manifest.push(json!({"type":"knowledge_image_archive","id":reference,"revision":reference,"locator":"image","role":"historical_image","contentHash":format!("sha256:{:x}",Sha256::digest(content.to_string().as_bytes())),"content":content}));
                    current.push(entry);
                }
            }
        }
    }
    Ok(())
}

/// Read only frozen Knowledge inputs, never the current mutable attachment or arbitrary files.
pub(crate) fn read(db: &Path, reference: &str) -> Result<Value, String> {
    if !valid_ref(reference) {
        return Err("knowledge_image_invalid: invalid local image reference".into());
    }
    let c = database::open(db)?;
    let mut stmt=c.prepare("SELECT manifest_json FROM knowledge_evidence_snapshots WHERE instr(manifest_json,?)>0 UNION ALL SELECT source_snapshot_json FROM knowledge_revision_turns WHERE instr(source_snapshot_json,?)>0").map_err(|e|e.to_string())?;
    let rows = stmt
        .query_map(params![reference, reference], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let snapshot: Value =
            serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        if let Some(image) = find(&snapshot["sourceManifest"], reference) {
            let media = image["mediaType"].as_str().unwrap_or("");
            let data = image["data"].as_str().unwrap_or("");
            let bytes = STANDARD
                .decode(data)
                .map_err(|_| "knowledge_image_invalid: invalid image bytes")?;
            if !supported(media) || bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
                return Err("knowledge_image_invalid: unsupported or oversized image".into());
            }
            // Recompute the identity-bound reference, rejecting altered frozen metadata/bytes.
            let identity = image["assetIdentity"]
                .as_str()
                .ok_or("knowledge_image_invalid: identity missing")?;
            let expected = format!(
                "{PREFIX}{:x}",
                Sha256::digest(format!("{identity}\0{media}\0{data}").as_bytes())
            );
            if expected != reference {
                return Err("knowledge_image_invalid: image content changed".into());
            }
            return Ok(json!({"dataUrl":format!("data:{media};base64,{data}")}));
        }
    }
    Err("knowledge_image_missing: original image is unavailable".into())
}
