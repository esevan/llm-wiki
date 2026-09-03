use crate::native::{database, lineage, vault};
use base64::Engine;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn slug(value: &str) -> String {
    let result = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let result = result
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if result.is_empty() {
        "completed-work".into()
    } else {
        result.chars().take(80).collect()
    }
}

pub(crate) fn complete(
    db_path: &Path,
    vault_root: &Path,
    problem_id: &str,
    input: &Value,
) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (capture_id, statement, detail): (Option<String>, String, String) = connection
        .query_row(
            "SELECT capture_id,statement,detail FROM problems WHERE id=?",
            [problem_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Problem not found")?;
    ensure_unmodified(&connection, vault_root, problem_id)?;
    let reason = input.get("reason").and_then(Value::as_str).unwrap_or("");
    let review_id = input
        .get("review_id")
        .or_else(|| input.get("reviewId"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut solution_statement = connection.prepare(
        "SELECT id,title,outcome,non_goals,validation_criteria,state FROM features WHERE problem_id=? ORDER BY created_at",
    ).map_err(|error| error.to_string())?;
    let solutions = solution_statement
        .query_map([problem_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let directory = format!(
        "{}/90. Archive/Completed Work",
        chrono::Utc::now().format("%Y")
    );
    let name = slug(&statement);
    let path = format!("{directory}/{name}.md");
    let raw_path = format!("{directory}/assets/{name}.raw.md");
    let mut raw = format!("# Raw work record: {statement}\n\n## Problem\n\n{detail}\n\n## Completion decision\n\n{reason}\n\n## Feedback and workflow history\n");
    if let Some(capture_id) = capture_id {
        if let Ok(capture) = connection.query_row(
            "SELECT text FROM captures WHERE id=?",
            [capture_id],
            |row| row.get::<_, String>(0),
        ) {
            raw.push_str(&format!("\n### Original Capture\n\n{capture}\n"));
        }
    }
    for (feature_id, title, outcome, non_goals, criteria, state) in &solutions {
        raw.push_str(&format!("\n## Solution: {title}\n\nState: {state}\n\n{outcome}\n\n### Non-goals\n\n{non_goals}\n\n### Validation criteria\n\n{criteria}\n"));
        let mut entries = connection.prepare("SELECT id,body,image_data,image_media_type,image_summary,created_at FROM solution_progress_entries WHERE feature_id=? ORDER BY created_at").map_err(|error| error.to_string())?
            .query_map([feature_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?))).map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>,_>>().map_err(|error| error.to_string())?;
        for (index, (entry_id, body, image_data, media_type, image_summary, created_at)) in
            entries.drain(..).enumerate()
        {
            raw.push_str(&format!(
                "\n### Work log {}\n\n{created_at}\n\n{body}\n",
                index + 1
            ));
            if !image_summary.is_empty() {
                raw.push_str(&format!("\nCanonical AI image summary: {image_summary}\n"));
            }
            if !image_data.is_empty() {
                let extension = if media_type.contains("jpeg") {
                    "jpg"
                } else if media_type.contains("gif") {
                    "gif"
                } else if media_type.contains("webp") {
                    "webp"
                } else {
                    "png"
                };
                let image_path = format!("{directory}/assets/{entry_id}.{extension}");
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(image_data)
                    .map_err(|_| "Stored work image is invalid")?;
                write_bytes(vault_root, &image_path, &bytes)?;
                raw.push_str(&format!("\n![[{entry_id}.{extension}]]\n"));
            }
            let comments = connection.prepare("SELECT body FROM solution_progress_comments WHERE entry_id=? ORDER BY created_at").and_then(|mut statement| statement.query_map([&entry_id], |row| row.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()).map_err(|error| error.to_string())?;
            for comment in comments {
                raw.push_str(&format!("\n- Comment: {comment}\n"));
            }
        }
        let checklist = connection.prepare("SELECT body,checked FROM solution_checklist_items WHERE feature_id=? ORDER BY created_at").and_then(|mut statement| statement.query_map([feature_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,bool>(1)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|error| error.to_string())?;
        if !checklist.is_empty() {
            raw.push_str("\n### Completion checklist\n\n");
        }
        for (body, checked) in checklist {
            raw.push_str(&format!("- [{}] {body}\n", if checked { "x" } else { " " }));
        }
    }
    let executive = input
        .get("executive_summary_markdown")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if reason.trim().is_empty() {
                "The recorded work was completed."
            } else {
                reason
            }
        });
    let report = input
        .get("report_body_markdown")
        .and_then(Value::as_str)
        .unwrap_or("");
    let report_section = if report.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n## Completion Report\n\n{}", report.trim())
    };
    if !input
        .get("regenerate")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        connection.execute("INSERT INTO problem_completion_decisions(id,problem_id,review_id,reason) VALUES (?,?,?,?)", params![Uuid::new_v4().to_string(),problem_id,review_id,reason]).map_err(|error| error.to_string())?;
    }
    let lineage = solutions
        .first()
        .map(|solution| lineage::create(db_path, &solution.0, false))
        .transpose()?
        .unwrap_or_else(|| json!({"status":"ready","lineage":{"stages":[],"transitions":[]},"conflicts":[],"completion_evidence":[]}));
    let lineage_sections = render_lineage_sections(&lineage);
    let summary = format!("# {statement}\n\n## Executive Summary\n\n{executive}{report_section}\n\n{lineage_sections}\n\n## Supporting evidence\n\n- [Raw work record](<assets/{name}.raw.md>)\n");
    vault::atomic_write(vault_root, &raw_path, &raw)?;
    vault::atomic_write(vault_root, &path, &summary)?;
    let source_hash = digest(&summary);
    connection.execute("INSERT OR REPLACE INTO completion_playbooks(problem_id,path,source_hash) VALUES (?,?,?)", params![problem_id,path,source_hash]).map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE problems SET state='completed' WHERE id=?",
            [problem_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(
        json!({"path":path,"problem_id":problem_id,"source_hash":source_hash,"lineage":lineage.get("lineage").cloned().unwrap_or(lineage)}),
    )
}

fn render_lineage_sections(document: &Value) -> String {
    let mut sections = vec!["## Lineage".to_owned()];
    let stages = document["lineage"]["stages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if stages.is_empty() {
        sections.push("\n- No workflow stages were recorded.".into());
    } else {
        for stage in stages {
            sections.push(format!(
                "\n- **{}** — {}",
                stage["kind"].as_str().unwrap_or("record"),
                stage["title"].as_str().unwrap_or("Untitled")
            ));
        }
    }
    sections.push("\n\n## Decision Changes".into());
    let transitions = document["lineage"]["transitions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if transitions.is_empty() {
        sections.push("\n\nNo decision changes were recorded.".into());
    } else {
        for transition in transitions {
            sections.push(format!(
                "\n- {} → {}",
                transition["from"].as_str().unwrap_or("unknown"),
                transition["to"].as_str().unwrap_or("unknown")
            ));
        }
    }
    sections.push("\n\n## Conflicts & Addresses".into());
    let conflicts = document["conflicts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if conflicts.is_empty() {
        sections.push("\n\nNo unresolved conflicts were recorded.".into());
    } else {
        for conflict in conflicts {
            sections.push(format!(
                "\n- {}",
                conflict["summary"].as_str().unwrap_or("Recorded conflict")
            ));
        }
    }
    sections.push("\n\n## Completion Evidence".into());
    let evidence = document["completion_evidence"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if evidence.is_empty() {
        sections
            .push("\n\nSee the linked raw work record for Work Log and checklist evidence.".into());
    } else {
        for item in evidence {
            sections.push(format!(
                "\n- {}",
                item["label"]
                    .as_str()
                    .unwrap_or("Recorded completion evidence")
            ));
        }
    }
    sections.concat()
}

fn ensure_unmodified(
    connection: &rusqlite::Connection,
    vault_root: &Path,
    problem_id: &str,
) -> Result<(), String> {
    let tracked: Option<(String, String)> = connection
        .query_row(
            "SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?",
            [problem_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some((path, expected)) = tracked {
        if let Ok(current) = fs::read_to_string(vault::resolve_markdown(vault_root, &path, true)?) {
            if digest(&current) != expected {
                return Err("This completed-work document was modified outside LLM Wiki".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn remove(
    db_path: &Path,
    vault_root: &Path,
    problem_id: &str,
    force: bool,
) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let (path, expected): (String, String) = connection
        .query_row(
            "SELECT path,source_hash FROM completion_playbooks WHERE problem_id=?",
            [problem_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or("Completed-work document is not tracked for this Problem")?;
    if !force {
        if let Ok(content) = fs::read_to_string(vault::resolve_markdown(vault_root, &path, true)?) {
            if digest(&content) != expected {
                return Err("This completed-work document was modified outside LLM Wiki. Delete again with force to remove generated files.".into());
            }
        }
    }
    let directory = Path::new(&path)
        .parent()
        .ok_or("Invalid completed-work path")?;
    let stem = Path::new(&path)
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Invalid completed-work path")?;
    let _ = fs::remove_file(vault_root.join(&path));
    let _ = fs::remove_file(
        vault_root
            .join(directory)
            .join("assets")
            .join(format!("{stem}.raw.md")),
    );
    let feature_ids = connection
        .prepare("SELECT id FROM features WHERE problem_id=?")
        .and_then(|mut statement| {
            statement
                .query_map([problem_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?;
    for feature_id in feature_ids {
        let entry_ids = connection.prepare("SELECT id,image_media_type FROM solution_progress_entries WHERE feature_id=? AND image_data<>''").and_then(|mut statement| statement.query_map([feature_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,_>>()).map_err(|error| error.to_string())?;
        for (entry_id, media) in entry_ids {
            let extension = if media.contains("jpeg") {
                "jpg"
            } else if media.contains("gif") {
                "gif"
            } else if media.contains("webp") {
                "webp"
            } else {
                "png"
            };
            let _ = fs::remove_file(
                vault_root
                    .join(directory)
                    .join("assets")
                    .join(format!("{entry_id}.{extension}")),
            );
        }
    }
    connection
        .execute(
            "DELETE FROM completion_playbooks WHERE problem_id=?",
            [problem_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(Value::Null)
}

fn write_bytes(vault_root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let target = vault_root.join(relative);
    if !target.starts_with(vault_root) {
        return Err("Asset path is outside the Vault".into());
    }
    let parent = target.parent().ok_or("Invalid asset path")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if let Err(error) = vault::replace_file(&temporary, &target) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}
