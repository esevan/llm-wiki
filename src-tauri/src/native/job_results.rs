use rusqlite::{params, OptionalExtension};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}

pub(super) struct JobContext<'a> {
    pub(super) connection: &'a rusqlite::Connection,
    pub(super) db_path: &'a Path,
    pub(super) task: &'a str,
    pub(super) entity_id: &'a str,
    pub(super) input: &'a Value,
    pub(super) vault: &'a Path,
    pub(super) model: &'a str,
    pub(super) source_hash: &'a str,
}

pub(super) fn prepare_result(context: JobContext<'_>, result: Value) -> Result<Value, String> {
    let JobContext {
        connection,
        db_path,
        task,
        entity_id,
        input,
        vault,
        model,
        source_hash: expected_source_hash,
    } = context;
    let locale = crate::native::localization::normalize_locale(
        input.get("locale").and_then(Value::as_str).unwrap_or("en"),
    );
    if matches!(task, "workflow_draft" | "workflow_refinement") {
        let versions = result
            .as_object()
            .filter(|value| value.contains_key("ko") && value.contains_key("en"));
        let mut reviewed = versions
            .and_then(|value| value.get(locale))
            .cloned()
            .unwrap_or_else(|| result.clone());
        if let Some(object) = reviewed.as_object_mut() {
            object.insert(
                "localized_versions".into(),
                versions
                    .map(|value| Value::Object(value.clone()))
                    .unwrap_or_else(|| json!({})),
            );
            object.insert("missing_locales".into(), json!([]));
            if task == "workflow_draft" {
                object.insert("source_locale".into(), Value::String(locale.into()));
            }
        }
        return Ok(reviewed);
    }
    if task == "image_summary" {
        let versions = result
            .as_object()
            .ok_or("Image Summary response must be an object")?;
        if !versions.contains_key("ko") || !versions.contains_key("en") {
            return Err("Image Summary requires Korean and English versions".into());
        }
        let localized = ["ko", "en"]
            .into_iter()
            .map(|language| {
                let summary = versions
                    .get(language)
                    .and_then(|value| value.get("summary"))
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or("Image Summary version is empty")?;
                Ok((language.to_owned(), json!({"image_summary":summary})))
            })
            .collect::<Result<serde_json::Map<_, _>, String>>()?;
        crate::native::localization::save_versions(
            connection,
            "solution_progress_entries",
            entity_id,
            &Value::Object(localized.clone()),
        )?;
        let summary = localized[locale]["image_summary"]
            .as_str()
            .ok_or("Image Summary is empty")?;
        let (feature_id, current_image): (String, String) = connection
            .query_row(
                "SELECT feature_id,image_data FROM solution_progress_entries WHERE id=? AND image_data<>''",
                [entity_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| "Progress image is no longer available".to_string())?;
        if format!("{:x}", Sha256::digest(current_image.as_bytes())) != expected_source_hash {
            return Err("Progress image changed while its summary was running".into());
        }
        connection
            .execute(
                "UPDATE solution_progress_entries SET image_summary=? WHERE id=?",
                params![summary, entity_id],
            )
            .map_err(|error| error.to_string())?;
        return Ok(
            json!({"summary":summary,"localized_versions":localized,"missing_locales":[],"entry_id":entity_id,"feature_id":feature_id}),
        );
    }
    if task == "completion_review" {
        let (problem_id, exists): (String, i64) = connection
            .query_row(
                "SELECT problem_id,1 FROM features WHERE id=?",
                [entity_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| "Solution not found".to_string())?;
        debug_assert_eq!(exists, 1);
        let review_id = id();
        connection
            .execute(
                "INSERT INTO completion_reviews(id,feature_id,report_json) VALUES (?,?,?)",
                params![review_id, entity_id, result.to_string()],
            )
            .map_err(|error| error.to_string())?;
        return Ok(
            json!({"review_id":review_id,"problem_id":problem_id,"feature_id":entity_id,"report":result}),
        );
    }
    if task == "workbench_organization" {
        let locale = input.get("locale").and_then(Value::as_str).unwrap_or("en");
        let current = crate::native::workbench::organization_items(db_path, locale)?;
        if crate::native::workbench::source_hash(&current) != expected_source_hash {
            return Err("Workbench changed during organization".into());
        }
        let organized = crate::native::workbench::apply_ai_organization(
            db_path,
            result.get("entries").unwrap_or(&Value::Null),
        )?;
        return Ok(json!({"organized":organized}));
    }
    if task == "lineage_inference" {
        return crate::native::lineage::add_inferences(db_path, entity_id, &result);
    }
    if task == "derived_translation" {
        let entity_type = input
            .get("entity_type")
            .or_else(|| input.get("entityType"))
            .and_then(Value::as_str)
            .ok_or("entity_type is required")?;
        let field = input.get("field").and_then(Value::as_str).unwrap_or("body");
        let source = input
            .get("source")
            .and_then(Value::as_str)
            .ok_or("source is required")?;
        if format!("{:x}", Sha256::digest(source.as_bytes())) != expected_source_hash {
            return Err("Derived translation source did not match its queued snapshot".into());
        }
        let table_field = match entity_type {
            "captures" => "text",
            "solution_progress_entries"
            | "solution_progress_comments"
            | "solution_checklist_items" => "body",
            _ => return Err("Unsupported derived translation target".into()),
        };
        let current: String = connection
            .query_row(
                &format!("SELECT {table_field} FROM {entity_type} WHERE id=?"),
                [entity_id],
                |row| row.get(0),
            )
            .map_err(|_| "Derived translation target not found".to_string())?;
        if format!("{:x}", Sha256::digest(current.as_bytes())) != expected_source_hash {
            return Err("Authored source changed during derived translation".into());
        }
        let mut versions = Map::new();
        for language in ["ko", "en"] {
            let value = result
                .get(language)
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or("Derived translation requires Korean and English versions")?;
            versions.insert(language.into(), json!({field:value.trim()}));
        }
        let source_locale = crate::native::localization::normalize_locale(
            input
                .get("source_locale")
                .or_else(|| input.get("sourceLocale"))
                .and_then(Value::as_str)
                .unwrap_or("en"),
        );
        versions.insert(source_locale.into(), json!({field:source}));
        crate::native::localization::save_versions(
            connection,
            entity_type,
            entity_id,
            &Value::Object(versions),
        )?;
        return Ok(
            json!({"entity_type":entity_type,"entity_id":entity_id,"field":field,"available_locales":["ko","en"]}),
        );
    }
    if task == "completion_report" {
        let reason: String = connection
            .query_row(
                "SELECT reason FROM problem_completion_decisions WHERE problem_id=? ORDER BY created_at DESC,rowid DESC LIMIT 1",
                [entity_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        return crate::native::completion::complete(
            db_path,
            vault,
            entity_id,
            &json!({
                "reason":reason,"regenerate":true,
                "executive_summary_markdown":result.get("executive_summary_markdown").and_then(Value::as_str).unwrap_or(""),
                "report_body_markdown":result.get("report_body_markdown").and_then(Value::as_str).unwrap_or("")
            }),
        );
    }
    if task == "conflict_review" {
        let run_id = id();
        let query = crate::native::workflow::conflict_query(connection, entity_id)?;
        let mut report = result.as_object().cloned().unwrap_or_default();
        let conflicts = report
            .get("conflicts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, mut conflict)| {
                if conflict.get("id").and_then(Value::as_str).is_none() {
                    conflict["id"] = json!(format!("conflict-{}", index + 1));
                }
                conflict
            })
            .collect::<Vec<_>>();
        let recommended = if conflicts.is_empty() {
            "insufficient_evidence"
        } else {
            "potential_conflict"
        };
        report.insert("run_id".into(), json!(run_id));
        report.insert("feature_id".into(), json!(entity_id));
        report.insert(
            "status".into(),
            json!(if conflicts.is_empty() {
                recommended
            } else {
                "conflicts_found"
            }),
        );
        report.insert("phase".into(), json!("complete"));
        report.insert("recommended_state".into(), json!(recommended));
        report.insert("progress".into(), json!(1.0));
        report.insert("conflicts".into(), Value::Array(conflicts.clone()));
        report
            .entry("findings")
            .or_insert_with(|| Value::Array(conflicts.clone()));
        report.entry("candidates").or_insert_with(|| json!([]));
        report.entry("summary").or_insert_with(|| {
            json!(if conflicts.is_empty() {
                "Available evidence cannot support a clear decision."
            } else {
                "Evidence-backed potential conflicts were found."
            })
        });
        let report = Value::Object(report);
        connection.execute(
            "INSERT INTO conflict_review_runs(id,feature_id,status,query,report_json) VALUES (?,?, 'ready', ?, ?)",
            params![run_id,entity_id,query,report.to_string()],
        ).map_err(|error| error.to_string())?;
        for conflict in conflicts {
            let conflict_id = conflict["id"].as_str().unwrap_or("");
            connection.execute(
                "INSERT INTO conflict_review_conflicts(storage_id,run_id,feature_id,conflict_id,target_id,target_title,severity,category,summary,current_claim,existing_claim,impact,recommendation,evidence_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                params![id(),run_id,entity_id,conflict_id,conflict["target_id"].as_str().unwrap_or(""),conflict["target_title"].as_str().unwrap_or(""),conflict["severity"].as_str().unwrap_or("medium"),conflict["category"].as_str().unwrap_or("Conflicting requirement"),conflict["summary"].as_str().unwrap_or(""),conflict["current_claim"].as_str().unwrap_or(""),conflict["existing_claim"].as_str().unwrap_or(""),conflict["impact"].as_str().unwrap_or(""),conflict["recommendation"].as_str().unwrap_or(""),conflict["evidence"].to_string()],
            ).map_err(|error| error.to_string())?;
        }
        return Ok(report);
    }
    if task == "knowledge_translation" {
        let path = input
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(entity_id);
        let canonical = crate::native::vault::read(vault, path, "en")?;
        let canonical_markdown = canonical["markdown"].as_str().unwrap_or("");
        let current_source_hash = format!("{:x}", Sha256::digest(canonical_markdown.as_bytes()));
        if current_source_hash != expected_source_hash {
            return Err("Knowledge document changed while translation was running".into());
        }
        let translated = result
            .get("markdown")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or("Knowledge translation was empty")?;
        let canonical_link = path.strip_suffix(".md").unwrap_or(path);
        let derived = format!(
            "---\nllm_wiki_derived: true\nlocale: ko\ncanonical: \"[[{canonical_link}]]\"\nsource_path: {}\nsource_hash: \"{current_source_hash}\"\nmodel: {}\n---\n{}",
            serde_json::to_string(path).map_err(|error| error.to_string())?,
            serde_json::to_string(model).map_err(|error| error.to_string())?,
            translated.trim_start()
        );
        let target = Path::new("Translations").join("ko").join(path);
        crate::native::vault::atomic_write(
            vault,
            &target.to_string_lossy().replace('\\', "/"),
            &derived,
        )?;
        return Ok(
            json!({"path":path,"locale":"ko","source_hash":current_source_hash,"translated":true}),
        );
    }
    Ok(result)
}
