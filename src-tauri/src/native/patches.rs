use crate::native::{database, vault};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn proposed_text(
    current: &str,
    operation: &str,
    heading: &str,
    content: &str,
) -> Result<String, String> {
    if !matches!(
        operation,
        "append_section" | "replace_section" | "insert_after_heading"
    ) {
        return Err("Unsupported structured patch operation".into());
    }
    if heading.trim().is_empty() {
        return Err("heading is required".into());
    }
    if operation == "append_section" {
        return Ok(format!(
            "{}\n\n# {}\n\n{}\n",
            current.trim_end(),
            heading,
            content.trim_end()
        ));
    }
    let marker = format!("# {heading}");
    let start = current
        .find(&marker)
        .ok_or_else(|| format!("Heading not found: {heading}"))?;
    let next = current[start + marker.len()..]
        .find("\n# ")
        .map(|offset| start + marker.len() + offset)
        .unwrap_or(current.len());
    if operation == "replace_section" {
        Ok(format!(
            "{}# {}\n\n{}\n{}",
            &current[..start],
            heading,
            content.trim_end(),
            &current[next..]
        ))
    } else {
        let line_end = current[start..]
            .find('\n')
            .map(|offset| start + offset + 1)
            .unwrap_or(current.len());
        Ok(format!(
            "{}\n{}\n{}",
            &current[..line_end],
            content.trim_end(),
            &current[line_end..]
        ))
    }
}

pub fn propose(
    db_path: &Path,
    vault_root: &Path,
    feature_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let text = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{key} is required"))
    };
    database::open(db_path)?
        .query_row("SELECT id FROM features WHERE id=?", [feature_id], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Solution not found")?;
    let path = text("path")?;
    let operation = text("operation")?;
    let heading = text("heading")?;
    let content = text("content")?;
    let target = vault::resolve_markdown(vault_root, path, true)?;
    let before = fs::read_to_string(target).map_err(|error| error.to_string())?;
    let proposed = proposed_text(&before, operation, heading, content)?;
    let patch_id = Uuid::new_v4().to_string();
    let base_hash = digest(&before);
    database::open(db_path)?.execute(
        "INSERT INTO patch_proposals(id,feature_id,path,operation,heading,content,base_hash,before_text,proposed_text) VALUES (?,?,?,?,?,?,?,?,?)",
        params![patch_id, feature_id, path, operation, heading, content, base_hash, before, proposed],
    ).map_err(|error| error.to_string())?;
    Ok(
        json!({"id":patch_id,"feature_id":feature_id,"path":path,"operation":operation,"heading":heading,"content":content,"base_hash":base_hash,"before_text":before,"proposed_text":proposed,"status":"proposed"}),
    )
}

fn stored(
    db_path: &Path,
    patch_id: &str,
) -> Result<(String, String, String, String, String), String> {
    database::open(db_path)?.query_row(
        "SELECT feature_id,path,base_hash,before_text,proposed_text FROM patch_proposals WHERE id=?",
        [patch_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|error| error.to_string())?.ok_or_else(|| "Patch proposal not found".into())
}

pub fn apply(db_path: &Path, vault_root: &Path, patch_id: &str) -> Result<Value, String> {
    let (feature_id, path, base_hash, before, proposed) = stored(db_path, patch_id)?;
    let current = fs::read_to_string(vault::resolve_markdown(vault_root, &path, true)?)
        .map_err(|error| error.to_string())?;
    if digest(&current) != base_hash {
        return Err("The source document changed after review; create a new patch".into());
    }
    vault::atomic_write(vault_root, &path, &proposed)?;
    let connection = database::open(db_path)?;
    connection
        .execute(
            "UPDATE patch_proposals SET status='applied',reverse_text=? WHERE id=?",
            params![before, patch_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE completions SET knowledge_status='integrated' WHERE feature_id=?",
            [feature_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":patch_id,"status":"applied"}))
}

pub fn undo(db_path: &Path, vault_root: &Path, patch_id: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (path, reverse, proposed, status): (String, String, String, String) = connection
        .query_row(
            "SELECT path,reverse_text,proposed_text,status FROM patch_proposals WHERE id=?",
            [patch_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Patch proposal not found")?;
    if status != "applied" {
        return Err("Only an applied patch can be undone".into());
    }
    let current = fs::read_to_string(vault::resolve_markdown(vault_root, &path, true)?)
        .map_err(|error| error.to_string())?;
    if digest(&current) != digest(&proposed) {
        return Err("The patched document changed after apply; undo was blocked".into());
    }
    vault::atomic_write(vault_root, &path, &reverse)?;
    connection
        .execute(
            "UPDATE patch_proposals SET status='undone' WHERE id=?",
            [patch_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(json!({"id":patch_id,"status":"undone"}))
}
