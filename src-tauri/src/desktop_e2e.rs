use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Packaged acceptance only: a real stdio child calls back into this GUI's IPC.
/// No test helper opens the database or substitutes an application service here.
#[tauri::command]
pub(crate) async fn desktop_e2e_mcp_probe(
    connection_id: String,
    revoked: bool,
) -> Result<Value, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_none() {
        return Err("Desktop E2E mode is disabled".into());
    }
    let mut child =
        tokio::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
            .args(["--mcp", "--connection", &connection_id])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().ok_or("Missing child input")?;
    let mut output = BufReader::new(child.stdout.take().ok_or("Missing child output")?);
    async fn exchange(
        input: &mut tokio::process::ChildStdin,
        output: &mut BufReader<tokio::process::ChildStdout>,
        request: Value,
    ) -> Result<Value, String> {
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        input.flush().await.map_err(|e| e.to_string())?;
        let mut line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            output.read_line(&mut line),
        )
        .await
        .map_err(|_| "MCP child timed out")?
        .map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|_| "MCP child returned no valid response".into())
    }
    let meta = json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"packaged-acceptance","version":"1"},"io.modelcontextprotocol/clientCapabilities":{"elicitation":{}}});
    let discovered = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":meta}}),
    )
    .await;
    if revoked {
        if discovered.is_ok() {
            return Err("Revoked connection still received protocol data".into());
        }
        return Ok(json!({"revoked":true}));
    }
    discovered?;
    let tools = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta}}),
    )
    .await?;
    if !tools["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools.iter().any(|t| t["name"] == "inbound_work_open"))
    {
        return Err("Packaged MCP tools missing".into());
    }
    let args = json!({"operationId":"packaged-open","lineageKey":"packaged-external","mode":"create","capture":{"title":"Packaged MCP Capture","summary":"Created through a real stdio child and the GUI owner"}});
    let preview=exchange(&mut input,&mut output,json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"_meta":meta}})).await?;
    if preview["result"]["resultType"] != "input_required" {
        return Err("MCP Capture bypassed review".into());
    }
    let result=exchange(&mut input,&mut output,json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"inbound_work_open","arguments":args,"requestState":preview["result"]["requestState"],"inputResponses":{"decision":{"action":"accept","content":{"decision":"accept"}}},"_meta":meta}})).await?;
    let result = result["result"]["structuredContent"].clone();
    if result["sessionId"].as_str().is_none() {
        return Err("MCP Capture acceptance failed".into());
    }
    child.kill().await.map_err(|e| e.to_string())?;
    Ok(result)
}

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
