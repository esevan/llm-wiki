use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopE2eResult {
    status: String,
    steps: Vec<String>,
    error: Option<String>,
    #[serde(default)]
    capture: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopE2eState {
    provider_url: String,
    restore_capture: Option<String>,
    restore_steps: Vec<String>,
}

#[tauri::command]
pub(crate) fn desktop_e2e_mode() -> Option<DesktopE2eState> {
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
pub(crate) fn desktop_e2e_complete(
    app: tauri::AppHandle,
    result: DesktopE2eResult,
) -> Result<(), String> {
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
