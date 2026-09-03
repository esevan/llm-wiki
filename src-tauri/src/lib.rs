mod conversation;
mod desktop_e2e;
mod native;
mod provider;

pub use native::{NativeApplication, NativeOperation, NativeResponse};
use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::Manager;

fn application_paths() -> Result<(PathBuf, PathBuf), String> {
    let vault = std::env::var_os("LLM_WIKI_VAULT")
        .map(PathBuf::from)
        .or_else(|| dirs::document_dir().map(|path| path.join("LLM Wiki Vault")))
        .ok_or("A local vault path is required")?;
    let data_dir = dirs::data_local_dir()
        .ok_or("The local application data directory is unavailable")?
        .join("LLM Wiki");
    let db = std::env::var_os("LLM_WIKI_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("llm-wiki.sqlite3"));
    Ok((vault, db))
}

fn execute_domain(
    application: tauri::State<'_, NativeApplication>,
    domain: &str,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    Ok(application.execute_domain(domain, operation))
}

#[tauri::command]
fn system_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "system", operation)
}

#[tauri::command]
fn vault_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "vault", operation)
}

#[tauri::command]
fn settings_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "settings", operation)
}

#[tauri::command]
async fn workflow_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    Ok(application.execute_workflow(operation).await)
}

#[tauri::command]
fn jobs_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "jobs", operation)
}

#[tauri::command]
async fn enqueue_ai_job(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    if operation.name != "jobs.enqueue" {
        return Ok(NativeResponse {
            status: 400,
            body: serde_json::json!({"detail":"Unsupported AI job command"}),
        });
    }
    Ok(application.enqueue_job(operation.input).await)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(conversation::RequestRegistry::default())
        .setup(|app| {
            let (vault, db) = application_paths()?;
            let model_dir = app
                .path()
                .resolve("resources/embedding-model", BaseDirectory::Resource)
                .map_err(|error| error.to_string())?;
            let application = NativeApplication::new(vault, db, Some(model_dir))?;
            let background_index = application.clone();
            app.manage(application);
            tauri::async_runtime::spawn_blocking(move || {
                let _ = background_index.execute_domain(
                    "vault",
                    NativeOperation {
                        name: "vault.index".into(),
                        input: serde_json::json!({"semantic":true}),
                    },
                );
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system_command,
            vault_command,
            settings_command,
            workflow_command,
            jobs_command,
            enqueue_ai_job,
            conversation::conversation_stream,
            conversation::cancel_conversation,
            desktop_e2e::desktop_e2e_mode,
            desktop_e2e::desktop_e2e_complete,
            provider::provider_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running LLM Wiki desktop");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn native_runtime_uses_sqlite_without_a_loopback_origin() {
        let state = tempdir().unwrap();
        let app = NativeApplication::isolated(
            &state.path().join("vault"),
            &state.path().join("db.sqlite"),
        )
        .unwrap();
        let created = app.execute(NativeOperation {
            name: "capture.create".into(),
            input: json!({"text":"Native state"}),
        });
        let board = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({}),
        });
        assert_eq!(created.status, 201);
        assert_eq!(board.status, 200);
        assert!(board.body.to_string().contains("Native state"));
    }

    #[test]
    fn native_board_preserves_bilingual_versions_and_legacy_fallback() {
        let state = tempdir().unwrap();
        let app = NativeApplication::isolated(
            &state.path().join("vault"),
            &state.path().join("db.sqlite"),
        )
        .unwrap();
        let capture = app.execute(NativeOperation {
            name: "capture.create".into(),
            input: json!({"text":"원문 캡처"}),
        });
        let problem = app.execute(NativeOperation {
            name: "capture.promote".into(),
            input: json!({
                "captureId": capture.body["id"],
                "statement": "한글 문제",
                "detail": "한글 맥락",
                "localized_versions": {
                    "ko": {"statement":"한글 문제","detail":"한글 맥락"},
                    "en": {"statement":"English problem","detail":"English context"}
                }
            }),
        });
        assert_eq!(problem.status, 201, "{}", problem.body);

        let english = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({"locale":"en-US"}),
        });
        assert_eq!(english.body["problems"][0]["statement"], "English problem");
        assert_eq!(english.body["problems"][0]["fallback_used"], false);
        assert_eq!(
            english.body["problems"][0]["available_locales"],
            json!(["ko", "en"])
        );

        let supplemented = app.execute(NativeOperation {
            name: "item.localization.save".into(),
            input: json!({
                "entityType":"problems",
                "entityId":problem.body["id"],
                "locale":"en",
                "fields":{"statement":"Updated English"}
            }),
        });
        assert_eq!(supplemented.status, 204, "{}", supplemented.body);
        let english = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({"locale":"en"}),
        });
        assert_eq!(english.body["problems"][0]["statement"], "Updated English");
        assert_eq!(english.body["problems"][0]["detail"], "English context");
    }

    #[test]
    fn bundled_model_drives_native_semantic_search_offline() {
        let state = tempdir().unwrap();
        let vault = state.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(
            vault.join("korean.md"),
            "# 배포 안내\n\n네이티브 앱은 인터넷 연결 없이 문서를 검색합니다.",
        )
        .unwrap();
        std::fs::write(
            vault.join("cooking.md"),
            "# Dinner\n\nRoast vegetables in the oven.",
        )
        .unwrap();
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("embedding-model");
        let app =
            NativeApplication::isolated_with_model(&vault, &state.path().join("db.sqlite"), &model)
                .unwrap();
        let indexed = app.execute(NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        });
        assert_eq!(indexed.status, 200, "{}", indexed.body);
        let health = app.execute(NativeOperation {
            name: "health.get".into(),
            input: json!({}),
        });
        assert_eq!(health.body["semantic_available"], true);
        assert_eq!(health.body["semantic_documents"], 2);
        let search = app.execute(NativeOperation {
            name: "vault.search".into(),
            input: json!({"query":"인터넷", "semantic":true}),
        });
        assert_eq!(search.status, 200, "{}", search.body);
        assert_eq!(search.body["semantic_available"], true);
        assert!(search.body["results"][0]["semantic_score"].is_number());
    }
}
