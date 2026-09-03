use crate::native::{database, workflow};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn organization_items(db_path: &Path, locale: &str) -> Result<Value, String> {
    let board = workflow::board_for_locale(db_path, locale)?;
    let mut items = Vec::new();
    for entity_type in ["captures", "problems", "features"] {
        for item in board[entity_type].as_array().into_iter().flatten() {
            items.push(json!({
                "entity_type":entity_type,
                "entity_id":item["id"],
                "title":item.get("statement").or_else(|| item.get("title")).or_else(|| item.get("outcome")).or_else(|| item.get("text")).cloned().unwrap_or(Value::Null),
                "state":item.get("state").cloned().unwrap_or_else(|| json!("inbox")),
                "current_category":item.get("category").cloned().unwrap_or_else(|| json!("")),
            }));
        }
    }
    Ok(Value::Array(items))
}

pub fn source_hash(items: &Value) -> String {
    format!("{:x}", Sha256::digest(items.to_string().as_bytes()))
}

pub fn apply_ai_organization(db_path: &Path, entries: &Value) -> Result<usize, String> {
    let entries = entries
        .as_array()
        .ok_or("AI organization response must contain an entries list")?;
    let valid = organization_items(db_path, "en")?;
    let valid = valid.as_array().expect("organization items are an array");
    let connection = database::open(db_path)?;
    let mut applied = 0;
    for entry in entries {
        let entity_type = entry
            .get("entity_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let entity_id = entry.get("entity_id").and_then(Value::as_str).unwrap_or("");
        if !valid
            .iter()
            .any(|item| item["entity_type"] == entity_type && item["entity_id"] == entity_id)
        {
            continue;
        }
        let mut category = entry
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("General")
            .trim()
            .chars()
            .take(80)
            .collect::<String>();
        if category.is_empty() {
            category = "General".into();
        }
        if let Ok(override_category) = connection.query_row(
            "SELECT category FROM workbench_category_overrides WHERE entity_type=? AND entity_id=?",
            params![entity_type, entity_id],
            |row| row.get::<_, String>(0),
        ) {
            category = override_category;
        }
        let mut rank = entry
            .get("attention_rank")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(0, 100);
        let mut rationale = entry
            .get("rationale")
            .and_then(Value::as_str)
            .unwrap_or("AI-organized attention priority")
            .trim()
            .chars()
            .take(400)
            .collect::<String>();
        if let Ok(manual) = connection.query_row("SELECT manual_priority FROM workbench_priority_overrides WHERE entity_type=? AND entity_id=?", params![entity_type,entity_id], |row| row.get::<_,i64>(0)) {
            if manual == 1 { rank = rank.max(90); rationale = "Manually marked important".into(); }
            if manual == -1 { rank = rank.min(89); rationale = "Manually marked not important".into(); }
        }
        connection.execute(
            "INSERT INTO workbench_priorities(entity_type,entity_id,category,attention_rank,rationale) VALUES (?,?,?,?,?) ON CONFLICT(entity_type,entity_id) DO UPDATE SET category=excluded.category,attention_rank=excluded.attention_rank,rationale=excluded.rationale,updated_at=CURRENT_TIMESTAMP",
            params![entity_type,entity_id,category,rank,rationale],
        ).map_err(|error| error.to_string())?;
        applied += 1;
    }
    if applied == 0 {
        return Err("AI organization did not return usable workbench items".into());
    }
    Ok(applied)
}
