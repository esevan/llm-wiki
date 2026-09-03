use crate::native::database;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;

const TASKS: &[(&str, bool)] = &[
    ("capture_assistance", true),
    ("problem_drafting", true),
    ("problem_assistance", true),
    ("workbench_organization", false),
    ("solution_drafting", true),
    ("solution_assistance", true),
    ("completed_solution_chat", false),
    ("conflict_review", true),
    ("image_summary", true),
    ("completion_review", true),
    ("completion_report", true),
    ("lineage_inference", true),
    ("problem_enrichment", false),
    ("knowledge_translation", false),
];

fn key_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("llm-wiki", "provider-api-key").map_err(|error| error.to_string())
}

fn api_key() -> String {
    std::env::var("LLM_WIKI_TEST_API_KEY")
        .ok()
        .or_else(|| key_entry().ok().and_then(|entry| entry.get_password().ok()))
        .unwrap_or_default()
}

pub fn resources(locale: &str) -> Result<Value, String> {
    let raw = match locale {
        "en" => include_str!("../../../llm_wiki/static/i18n/en.json"),
        "ko" => include_str!("../../../llm_wiki/static/i18n/ko.json"),
        _ => return Err("Unsupported locale".into()),
    };
    serde_json::from_str(raw).map_err(|error| error.to_string())
}

pub fn locale(db_path: &Path, browser_locale: &str) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let saved: Option<(String, i64)> = connection
        .query_row(
            "SELECT locale,explicit FROM locale_settings WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (locale, explicit) = saved.unwrap_or_else(|| {
        (
            if browser_locale.to_lowercase().starts_with("ko") {
                "ko"
            } else {
                "en"
            }
            .into(),
            0,
        )
    });
    Ok(json!({"locale":locale,"explicit":explicit != 0}))
}

pub fn save_locale(db_path: &Path, input: &Value) -> Result<Value, String> {
    let locale = input.get("locale").and_then(Value::as_str).unwrap_or("");
    if !matches!(locale, "ko" | "en") {
        return Err("Unsupported locale".into());
    }
    let connection = database::open(db_path)?;
    connection.execute("INSERT INTO locale_settings(id,locale,explicit) VALUES (1,?,1) ON CONFLICT(id) DO UPDATE SET locale=excluded.locale,explicit=1", [locale]).map_err(|error| error.to_string())?;
    Ok(json!({"locale":locale,"explicit":true}))
}

pub fn provider(db_path: &Path) -> Result<Value, String> {
    let connection = database::open(db_path)?;
    let row = connection.query_row("SELECT base_url,model,advanced_model,advanced_tasks,report_language,async_worker_count FROM provider_settings WHERE id=1", [], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,i64>(5)?))).map_err(|error| error.to_string())?;
    let saved: Value = serde_json::from_str(&row.3).unwrap_or_else(|_| json!({}));
    let mut tasks = serde_json::Map::new();
    for (name, default) in TASKS {
        tasks.insert(
            (*name).into(),
            saved
                .get(*name)
                .and_then(Value::as_bool)
                .unwrap_or(*default)
                .into(),
        );
    }
    Ok(
        json!({"base_url":row.0,"model":row.1,"advanced_model":row.2,"advanced_tasks":tasks,"report_language":row.4,"async_worker_count":row.5,"api_key_configured":!api_key().is_empty()}),
    )
}

pub fn save_provider(db_path: &Path, input: &Value) -> Result<Value, String> {
    let base_url = input
        .get("base_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_end_matches('/');
    if !(base_url.starts_with("http://127.0.0.1:") || base_url.starts_with("https://")) {
        return Err("Provider URL must use HTTPS or loopback HTTP".into());
    }
    let model = input.get("model").and_then(Value::as_str).unwrap_or("");
    let advanced_model = input
        .get("advanced_model")
        .and_then(Value::as_str)
        .unwrap_or("");
    let advanced_tasks = input
        .get("advanced_tasks")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let report_language = input
        .get("report_language")
        .and_then(Value::as_str)
        .unwrap_or("ko");
    let workers = input
        .get("async_worker_count")
        .and_then(Value::as_i64)
        .unwrap_or(2);
    if !(1..=32).contains(&workers) {
        return Err("Async worker count must be between 1 and 32".into());
    }
    let connection = database::open(db_path)?;
    connection.execute("UPDATE provider_settings SET base_url=?,model=?,advanced_model=?,advanced_tasks=?,report_language=?,async_worker_count=? WHERE id=1", params![base_url,model,advanced_model,advanced_tasks.to_string(),report_language,workers]).map_err(|error| error.to_string())?;
    if let Some(secret) = input
        .get("api_key")
        .and_then(Value::as_str)
        .filter(|secret| !secret.is_empty())
    {
        key_entry()?
            .set_password(secret)
            .map_err(|error| error.to_string())?;
    }
    provider(db_path)
}

pub fn provider_credentials_for(
    db_path: &Path,
    task: &str,
) -> Result<(String, String, String), String> {
    let (base_url, default_model, advanced_model, advanced_tasks): (
        String,
        String,
        String,
        String,
    ) = database::open(db_path)?
        .query_row(
            "SELECT base_url,model,advanced_model,advanced_tasks FROM provider_settings WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|error| error.to_string())?;
    let advanced = serde_json::from_str::<Value>(&advanced_tasks)
        .ok()
        .and_then(|value| value.get(task).and_then(Value::as_bool))
        .unwrap_or_else(|| {
            TASKS
                .iter()
                .find(|(name, _)| *name == task)
                .map(|(_, enabled)| *enabled)
                .unwrap_or(false)
        });
    let model = if advanced && !advanced_model.trim().is_empty() {
        advanced_model
    } else {
        default_model
    };
    Ok((base_url, model, api_key()))
}
