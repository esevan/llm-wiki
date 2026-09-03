use crate::native::{database, workflow};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;

pub fn context(
    db_path: &Path,
    entity_type: &str,
    entity_id: &str,
    locale: &str,
) -> Result<Value, String> {
    let current = workflow::item_for_locale(db_path, entity_type, entity_id, locale)?;
    let title = current
        .get("title")
        .or_else(|| current.get("statement"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let detail = current
        .get("detail")
        .or_else(|| current.get("outcome"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let mut entries = Vec::new();
    if !title.is_empty() {
        entries.push(json!({"label":"Current item","text":bounded_text(title)}));
    }
    if useful_detail(detail, title) {
        entries.push(json!({"label":"Current context","text":bounded_text(detail)}));
    }

    let connection = database::open(db_path)?;
    let mut statement = connection
        .prepare("SELECT kind,input_text,output_text FROM ai_runs WHERE entity_type=? AND entity_id=? AND kind IN ('workflow_chat','workflow_refinement') ORDER BY created_at DESC,rowid DESC LIMIT 3")
        .map_err(|error| error.to_string())?;
    let runs = statement
        .query_map(params![entity_type, entity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for run in runs {
        let (kind, input, output) = run.map_err(|error| error.to_string())?;
        let combined = if kind == "workflow_chat" {
            format!("{} — {}", input.trim(), output.trim())
        } else {
            output
        };
        let text = bounded_text(&combined);
        if !text.is_empty() && entries.len() < 5 {
            entries.push(json!({
                "label": if kind == "workflow_chat" { "Recent discussion" } else { "Previous preview" },
                "text": text
            }));
        }
    }
    drop(statement);

    Ok(json!({
        "has_context":!entries.is_empty(),
        "entries":entries,
        "refinement_draft":latest_job(&connection, "workflow_refinement", entity_type, entity_id)?,
        "next_draft":latest_job(&connection, "workflow_draft", entity_type, entity_id)?,
        "current_detail":current
    }))
}

fn latest_job(
    connection: &rusqlite::Connection,
    task_kind: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Value, String> {
    let result: Option<String> = connection
        .query_row(
            "SELECT result_json FROM ai_jobs_v2 WHERE task_kind=? AND entity_type=? AND entity_id=? AND status IN ('completed','awaiting_review') ORDER BY created_at DESC,rowid DESC LIMIT 1",
            params![task_kind, entity_type, entity_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(result
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or(Value::Null))
}

fn useful_detail(detail: &str, title: &str) -> bool {
    !detail.is_empty()
        && !detail.eq_ignore_ascii_case(title)
        && !matches!(
            detail.to_ascii_lowercase().as_str(),
            "n/a" | "none" | "unknown" | "tbd"
        )
}

fn bounded_text(value: &str) -> String {
    let normalized = value
        .replace("**", "")
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.chars().count() <= 160 {
        return normalized;
    }
    let mut result = normalized.chars().take(159).collect::<String>();
    result.push('…');
    result
}
