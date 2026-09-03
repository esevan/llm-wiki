mod native;

pub use native::{NativeApplication, NativeOperation, NativeResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::path::BaseDirectory;
use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopE2eResult {
    status: String,
    steps: Vec<String>,
    error: Option<String>,
    #[serde(default)]
    capture: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopE2eState {
    provider_url: String,
    restore_capture: Option<String>,
    restore_steps: Vec<String>,
}

#[derive(Clone, Default)]
struct RequestRegistry {
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum StreamEvent {
    Chunk { data: Vec<u8> },
    Complete,
    Cancelled,
    Error { message: String },
}

impl RequestRegistry {
    async fn register(&self, request_id: &str) -> Result<CancellationToken, String> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("Invalid request identifier".into());
        }
        let mut active = self.active.lock().await;
        if active.contains_key(request_id) {
            return Err("Request identifier is already active".into());
        }
        let token = CancellationToken::new();
        active.insert(request_id.to_owned(), token.clone());
        Ok(token)
    }
    async fn finish(&self, request_id: &str) {
        self.active.lock().await.remove(request_id);
    }
    async fn cancel(&self, request_id: &str) -> bool {
        if let Some(token) = self.active.lock().await.remove(request_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
}

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
fn workflow_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "workflow", operation)
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
    let text = |key: &str| {
        operation
            .input
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    match native::jobs::enqueue(
        application.db_path(),
        text("taskKind"),
        text("entityType"),
        text("entityId"),
    )
    .await
    {
        Ok(body) => Ok(NativeResponse { status: 202, body }),
        Err(error) => Ok(NativeResponse {
            status: 400,
            body: serde_json::json!({"detail":error}),
        }),
    }
}

#[tauri::command]
async fn conversation_stream(
    application: tauri::State<'_, NativeApplication>,
    registry: tauri::State<'_, RequestRegistry>,
    request_id: String,
    entity_type: String,
    entity_id: String,
    message: String,
    on_event: Channel<StreamEvent>,
) -> Result<NativeResponse, String> {
    if message.trim().is_empty() {
        return Err("message is required".into());
    }
    native::workflow::item(&application.db_path(), &entity_type, &entity_id)?;
    let (base_url, model, api_key) =
        native::settings::provider_credentials(&application.db_path())?;
    if model.trim().is_empty() {
        return Err("Provider model is required".into());
    }
    let token = registry.register(&request_id).await?;
    let registry = registry.inner().clone();
    let db_path = application.db_path();
    tauri::async_runtime::spawn(async move {
        let task = async {
            let response = reqwest::Client::new().post(format!("{}/chat/completions", base_url.trim_end_matches('/')))
                .bearer_auth(api_key)
                .json(&json!({"model":model,"messages":[{"role":"user","content":message}],"stream":true}))
                .send().await.map_err(|error| error.to_string())?;
            if !response.status().is_success() { return Err(format!("Provider request failed ({})", response.status())); }
            let source = response.text().await.map_err(|error| error.to_string())?;
            let mut output = String::new();
            for line in source.lines().filter_map(|line| line.strip_prefix("data: ")) {
                if token.is_cancelled() { let _ = on_event.send(StreamEvent::Cancelled); return Ok(()); }
                if line == "[DONE]" { break; }
                let value: serde_json::Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
                if let Some(text) = value.pointer("/choices/0/delta/content").and_then(serde_json::Value::as_str) {
                    output.push_str(text);
                    let data = format!("data: {}\n\n", text.replace('\n', " ")).into_bytes();
                    let _ = on_event.send(StreamEvent::Chunk { data });
                }
            }
            native::workflow::record_ai_run(&db_path, &entity_type, &entity_id, &message, &output)?;
            let _ = on_event.send(StreamEvent::Chunk { data: b"event: done\ndata: done\n\n".to_vec() });
            let _ = on_event.send(StreamEvent::Complete);
            Ok::<(), String>(())
        }.await;
        if let Err(message) = task {
            let _ = on_event.send(StreamEvent::Error { message });
        }
        registry.finish(&request_id).await;
    });
    Ok(NativeResponse {
        status: 200,
        body: serde_json::Value::Null,
    })
}

#[tauri::command]
async fn cancel_conversation(
    registry: tauri::State<'_, RequestRegistry>,
    request_id: String,
) -> Result<bool, String> {
    Ok(registry.cancel(&request_id).await)
}

#[tauri::command]
fn desktop_e2e_mode() -> Option<DesktopE2eState> {
    if let Some(path) = std::env::var_os("LLM_WIKI_E2E_RESULT").map(PathBuf::from) {
        let _ = std::fs::write(path.with_extension("started"), b"webview started");
        let provider_url = std::env::var("LLM_WIKI_E2E_PROVIDER_URL").ok()?;
        let restore_capture = std::env::var("LLM_WIKI_E2E_RESTORE_CAPTURE").ok();
        let restore_steps = std::env::var("LLM_WIKI_E2E_RESTORE_STEPS")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_default();
        Some(DesktopE2eState {
            provider_url,
            restore_capture,
            restore_steps,
        })
    } else {
        None
    }
}

#[tauri::command]
fn desktop_e2e_complete(app: tauri::AppHandle, result: DesktopE2eResult) -> Result<(), String> {
    let path = std::env::var_os("LLM_WIKI_E2E_RESULT")
        .map(PathBuf::from)
        .ok_or("Desktop E2E mode is disabled")?;
    if result.status == "progress" {
        let payload =
            serde_json::to_vec_pretty(&result.steps).map_err(|error| error.to_string())?;
        return std::fs::write(path.with_extension("progress"), payload)
            .map_err(|error| error.to_string());
    }
    let payload = serde_json::to_vec_pretty(&result).map_err(|error| error.to_string())?;
    std::fs::write(path, payload).map_err(|error| error.to_string())?;
    app.exit(if matches!(result.status.as_str(), "passed" | "relaunch") {
        0
    } else {
        1
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RequestRegistry::default())
        .setup(|app| {
            let (vault, db) = application_paths()?;
            let model_dir = app
                .path()
                .resolve("resources/embedding-model", BaseDirectory::Resource)
                .map_err(|error| error.to_string())?;
            let application = NativeApplication::new(vault, db, Some(model_dir))?;
            let indexed = application.execute_domain(
                "vault",
                NativeOperation {
                    name: "vault.index".into(),
                    input: serde_json::Value::Null,
                },
            );
            if indexed.status != 200 {
                return Err(format!("Initial Vault index failed: {}", indexed.body).into());
            }
            app.manage(application);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system_command,
            vault_command,
            settings_command,
            workflow_command,
            jobs_command,
            enqueue_ai_job,
            conversation_stream,
            cancel_conversation,
            desktop_e2e_mode,
            desktop_e2e_complete
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
