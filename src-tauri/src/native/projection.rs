use crate::native::database;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

fn safe_projection_title(title: &str, fallback: &str) -> String {
    let value = title
        .chars()
        .filter(|character| character.is_alphanumeric() || matches!(character, ' ' | '-' | '_'))
        .take(80)
        .collect::<String>()
        .trim()
        .to_owned();
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}

pub fn project(
    db_path: &Path,
    vault: &Path,
    entity_type: &str,
    entity_id: &str,
) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (singular, folder, title, state, fields) = match entity_type {
        "problems" => {
            let row: (String, String, String) = connection
                .query_row(
                    "SELECT statement,detail,state FROM problems WHERE id=?",
                    [entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or("Projection entity not found")?;
            (
                "problem",
                "20. Problems",
                row.0.clone(),
                row.2,
                vec![("Detail", row.1)],
            )
        }
        "features" => {
            let row: (String, String, String, String, String) = connection
                .query_row(
                    "SELECT title,outcome,non_goals,validation_criteria,state FROM features WHERE id=?",
                    [entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or("Projection entity not found")?;
            (
                "feature",
                "30. Features",
                row.0.clone(),
                row.4,
                vec![
                    ("Outcome", row.1),
                    ("Non Goals", row.2),
                    ("Validation Criteria", row.3),
                ],
            )
        }
        _ => return Err("Unsupported projection type".into()),
    };
    let relative = format!(
        "{}/{}/{}.md",
        chrono::Utc::now().format("%Y"),
        folder,
        safe_projection_title(&title, entity_id)
    );
    let mut content = format!(
        "---\nllm_wiki_id: {entity_id}\nllm_wiki_managed: true\ncanonical_locale: en\ntype: {singular}\nstate: {state}\n---\n\n# {title}\n"
    );
    for (label, value) in fields {
        if !value.is_empty() {
            content.push_str(&format!("\n- **{label}**: {value}"));
        }
    }
    content.push('\n');
    let target = vault.join(&relative);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Ok(existing) = std::fs::read_to_string(&target) {
        let previous: Option<String> = connection
            .query_row(
                "SELECT source_hash FROM mirror_files WHERE entity_type=? AND entity_id=?",
                params![entity_type, entity_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let current_hash = format!("{:x}", Sha256::digest(existing.as_bytes()));
        if previous.as_deref() != Some(&current_hash) {
            return Err(
                "Generated file was modified externally; import or regenerate after review".into(),
            );
        }
    }
    crate::native::vault::atomic_write(vault, &relative, &content)?;
    let source_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    connection.execute(
        "INSERT INTO mirror_files(entity_type,entity_id,path,source_hash) VALUES (?,?,?,?) ON CONFLICT(entity_type,entity_id) DO UPDATE SET path=excluded.path,source_hash=excluded.source_hash",
        params![entity_type,entity_id,relative,source_hash],
    ).map_err(|error| error.to_string())?;
    Ok(json!({"path":relative}))
}

pub fn archive(
    db_path: &Path,
    vault: &Path,
    entity_type: &str,
    entity_id: &str,
) -> Result<Value, String> {
    if !matches!(entity_type, "problems" | "features") {
        return Err("Unsupported archive type".into());
    }
    let connection = database::open(db_path)?;
    if entity_type == "features" {
        let state: Option<String> = connection
            .query_row(
                "SELECT state FROM completions WHERE feature_id=?",
                [entity_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if state.as_deref() != Some("verified") {
            return Err("A Solution can be archived only after verified completion".into());
        }
    }
    let (source, expected_hash): (String, String) = connection
        .query_row(
            "SELECT path,source_hash FROM mirror_files WHERE entity_type=? AND entity_id=?",
            params![entity_type, entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Create a generated projection before archiving")?;
    let source_path = vault.join(&source);
    let content = std::fs::read(&source_path).map_err(|error| error.to_string())?;
    let current_hash = format!("{:x}", Sha256::digest(&content));
    if current_hash != expected_hash {
        return Err("Generated file was modified externally; archive is blocked".into());
    }
    let filename = source_path
        .file_name()
        .ok_or("Generated projection path is invalid")?;
    let destination = Path::new(&chrono::Utc::now().format("%Y").to_string())
        .join("90. Archive")
        .join(if entity_type == "features" {
            "Features"
        } else {
            "Problems"
        })
        .join(filename);
    let destination_path = vault.join(&destination);
    if let Some(parent) = destination_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&source_path, &destination_path).map_err(|error| error.to_string())?;
    let destination = destination.to_string_lossy().replace('\\', "/");
    connection
        .execute(
            &format!("UPDATE {entity_type} SET state='archived' WHERE id=?"),
            [entity_id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE mirror_files SET path=? WHERE entity_type=? AND entity_id=?",
            params![destination, entity_type, entity_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}
