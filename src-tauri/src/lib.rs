use futures_util::StreamExt;
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::Manager;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const ALLOWED_ROOTS: &[&str] = &[
    "ai",
    "board",
    "captures",
    "checklist",
    "conflict-reviews",
    "dashboard",
    "events",
    "features",
    "goals",
    "health",
    "i18n",
    "index",
    "items",
    "jobs",
    "knowledge",
    "notifications",
    "patches",
    "problems",
    "progress",
    "provider",
    "search",
    "settings",
    "transitions",
    "workbench",
];

#[derive(Clone)]
pub struct ApplicationGateway {
    origin: String,
    client: Client,
}

#[derive(Clone, Default)]
pub struct RequestRegistry {
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

pub struct BackendProcess {
    child: StdMutex<Option<Child>>,
}

impl BackendProcess {
    fn terminate(&self) {
        if let Ok(mut child) = self.child.lock() {
            if let Some(mut child) = child.take() {
                #[cfg(unix)]
                {
                    let process_group = child.id() as i32;
                    unsafe {
                        libc::killpg(process_group, libc::SIGTERM);
                    }
                    for _ in 0..40 {
                        let leader_finished = child.try_wait().ok().flatten().is_some();
                        let group_finished = unsafe { libc::killpg(process_group, 0) } != 0;
                        if leader_finished && group_finished {
                            return;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    unsafe {
                        libc::killpg(process_group, libc::SIGKILL);
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for BackendProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

struct PreparedBackend {
    origin: String,
    process: Option<BackendProcess>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRequest {
    pub path: String,
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

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

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamEvent {
    Chunk { data: Vec<u8> },
    Complete,
    Cancelled,
    Error { message: String },
}

impl ApplicationGateway {
    pub fn new(origin: impl Into<String>) -> Result<Self, String> {
        let origin = origin.into();
        if !origin.starts_with("http://127.0.0.1:") && !origin.starts_with("http://[::1]:") {
            return Err("The application backend must use a loopback HTTP origin".into());
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { origin, client })
    }

    pub async fn request(
        &self,
        request: ApplicationRequest,
    ) -> Result<ApplicationResponse, String> {
        let (status, content_type, response) = self.start(request).await?;
        let body = response.text().await.map_err(|error| error.to_string())?;
        Ok(ApplicationResponse {
            status,
            content_type,
            body,
        })
    }

    async fn start(&self, request: ApplicationRequest) -> Result<(u16, String, Response), String> {
        validate_request(&request)?;
        let method =
            Method::from_bytes(request.method.as_bytes()).map_err(|_| "Unsupported method")?;
        let mut outgoing = self
            .client
            .request(method, format!("{}/api{}", self.origin, request.path));
        for (name, value) in &request.headers {
            if matches!(
                name.to_ascii_lowercase().as_str(),
                "content-type" | "x-llm-wiki-locale" | "x-llm-wiki-surface"
            ) {
                outgoing = outgoing.header(name, value);
            }
        }
        if let Some(body) = request.body {
            outgoing = outgoing.body(body);
        }
        let response = outgoing
            .send()
            .await
            .map_err(|error| format!("Application backend unavailable: {error}"))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("text/plain")
            .to_owned();
        Ok((status, content_type, response))
    }
}

impl RequestRegistry {
    async fn register(&self, request_id: String) -> Result<CancellationToken, String> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("Invalid request identifier".into());
        }
        let mut active = self.active.lock().await;
        if active.contains_key(&request_id) {
            return Err("Request identifier is already active".into());
        }
        let token = CancellationToken::new();
        active.insert(request_id, token.clone());
        Ok(token)
    }

    async fn finish(&self, request_id: &str) {
        self.active.lock().await.remove(request_id);
    }

    pub async fn cancel(&self, request_id: &str) -> bool {
        if let Some(token) = self.active.lock().await.remove(request_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
}

fn validate_request(request: &ApplicationRequest) -> Result<(), String> {
    if request.path.len() > 4096
        || !request.path.starts_with('/')
        || request.path.contains("..")
        || request.path.contains("://")
        || request.path.contains('\0')
    {
        return Err("Invalid application path".into());
    }
    if !matches!(
        request.method.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE"
    ) {
        return Err("Unsupported application method".into());
    }
    let root = request.path[1..]
        .split(['/', '?'])
        .next()
        .unwrap_or_default();
    if !ALLOWED_ROOTS.contains(&root) {
        return Err("Application capability is not exposed to the desktop UI".into());
    }
    Ok(())
}

fn available_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| error.to_string())
}

fn sidecar_executable() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("LLM_WIKI_SIDECAR") {
        return Ok(PathBuf::from(path));
    }
    let current = std::env::current_exe().map_err(|error| error.to_string())?;
    let packaged = current
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(if cfg!(windows) {
            "llm-wiki-sidecar.exe"
        } else {
            "llm-wiki-sidecar"
        });
    if packaged.is_file() {
        return Ok(packaged);
    }
    let development = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(if cfg!(windows) {
            ".venv/Scripts/llm-wiki.exe"
        } else {
            ".venv/bin/llm-wiki"
        });
    if development.is_file() {
        return Ok(development);
    }
    Err("The packaged LLM Wiki application runtime was not found".into())
}

fn prepare_backend() -> Result<PreparedBackend, String> {
    if let Ok(origin) = std::env::var("LLM_WIKI_BACKEND_ORIGIN") {
        ApplicationGateway::new(origin.clone())?;
        return Ok(PreparedBackend {
            origin,
            process: None,
        });
    }
    let vault = std::env::var_os("LLM_WIKI_VAULT")
        .map(PathBuf::from)
        .or_else(|| dirs::document_dir().map(|path| path.join("LLM Wiki Vault")))
        .ok_or("A local vault path is required")?;
    std::fs::create_dir_all(&vault)
        .map_err(|error| format!("Could not create the vault: {error}"))?;
    let data_dir = dirs::data_local_dir()
        .ok_or("The local application data directory is unavailable")?
        .join("LLM Wiki");
    std::fs::create_dir_all(&data_dir)
        .map_err(|error| format!("Could not create application data: {error}"))?;
    let db = std::env::var_os("LLM_WIKI_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("llm-wiki.sqlite3"));
    let port = available_port()?;
    let executable = sidecar_executable()?;
    let diagnostic_output = std::env::var_os("LLM_WIKI_E2E_RESULT").is_some();
    let mut command = Command::new(executable);
    command
        .args(["desktop-backend", "--vault"])
        .arg(&vault)
        .arg("--db")
        .arg(&db)
        .args(["--port", &port.to_string()])
        .stdin(Stdio::null())
        .stdout(if diagnostic_output {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(if diagnostic_output {
            Stdio::inherit()
        } else {
            Stdio::null()
        });
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start the application runtime: {error}"))?;
    let address = format!("127.0.0.1:{port}");
    for _ in 0..800 {
        if TcpStream::connect(&address).is_ok() {
            return Ok(PreparedBackend {
                origin: format!("http://{address}"),
                process: Some(BackendProcess {
                    child: StdMutex::new(Some(child)),
                }),
            });
        }
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err("The application runtime stopped during startup".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err("The application runtime did not become ready".into())
}

#[tauri::command]
async fn application_request(
    gateway: tauri::State<'_, ApplicationGateway>,
    request: ApplicationRequest,
) -> Result<ApplicationResponse, String> {
    gateway.request(request).await
}

#[tauri::command]
async fn application_stream(
    gateway: tauri::State<'_, ApplicationGateway>,
    registry: tauri::State<'_, RequestRegistry>,
    request_id: String,
    request: ApplicationRequest,
    on_event: Channel<StreamEvent>,
) -> Result<ApplicationResponse, String> {
    let token = registry.register(request_id.clone()).await?;
    let (status, content_type, response) = match gateway.start(request).await {
        Ok(started) => started,
        Err(error) => {
            registry.finish(&request_id).await;
            return Err(error);
        }
    };
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn(async move {
        let mut chunks = response.bytes_stream();
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    let _ = on_event.send(StreamEvent::Cancelled);
                    break;
                }
                next = chunks.next() => match next {
                    Some(Ok(bytes)) => {
                        if on_event.send(StreamEvent::Chunk { data: bytes.to_vec() }).is_err() {
                            break;
                        }
                    }
                    Some(Err(error)) => {
                        let _ = on_event.send(StreamEvent::Error { message: error.to_string() });
                        break;
                    }
                    None => {
                        let _ = on_event.send(StreamEvent::Complete);
                        break;
                    }
                }
            }
        }
        registry.finish(&request_id).await;
    });
    Ok(ApplicationResponse {
        status,
        content_type,
        body: String::new(),
    })
}

#[tauri::command]
async fn cancel_application_request(
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
    let prepared = prepare_backend().expect("a supervised local application runtime");
    let gateway =
        ApplicationGateway::new(prepared.origin).expect("a loopback application backend origin");
    let mut builder = tauri::Builder::default();
    if let Some(process) = prepared.process {
        builder = builder.manage(process);
    }
    let app = builder
        .manage(gateway)
        .manage(RequestRegistry::default())
        .invoke_handler(tauri::generate_handler![
            application_request,
            application_stream,
            cancel_application_request,
            desktop_e2e_mode,
            desktop_e2e_complete
        ])
        .build(tauri::generate_context!())
        .expect("error while building LLM Wiki desktop");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(process) = app.try_state::<BackendProcess>() {
                process.terminate();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(path: &str, method: &str) -> ApplicationRequest {
        ApplicationRequest {
            path: path.into(),
            method: method.into(),
            headers: Default::default(),
            body: None,
        }
    }

    #[test]
    fn given_an_external_origin_when_configured_then_it_is_rejected() {
        assert!(ApplicationGateway::new("https://example.com").is_err());
    }

    #[test]
    fn given_a_path_escape_when_invoked_then_it_is_rejected() {
        assert_eq!(
            validate_request(&request("/knowledge/../secret", "GET")),
            Err("Invalid application path".into())
        );
    }

    #[test]
    fn given_an_unexposed_capability_when_invoked_then_it_is_rejected() {
        assert_eq!(
            validate_request(&request("/filesystem/read", "POST")),
            Err("Application capability is not exposed to the desktop UI".into())
        );
    }

    #[test]
    fn given_a_shell_method_when_invoked_then_it_is_rejected() {
        assert_eq!(
            validate_request(&request("/board", "TRACE")),
            Err("Unsupported application method".into())
        );
    }

    #[tokio::test]
    async fn given_an_active_request_when_cancelled_then_its_token_is_signalled() {
        let registry = RequestRegistry::default();
        let token = registry.register("request-1".into()).await.unwrap();

        assert!(registry.cancel("request-1").await);
        assert!(token.is_cancelled());
        assert!(!registry.cancel("request-1").await);
    }

    #[tokio::test]
    async fn given_a_duplicate_request_identifier_when_registered_then_it_is_rejected() {
        let registry = RequestRegistry::default();
        registry.register("request-1".into()).await.unwrap();

        assert_eq!(
            registry.register("request-1".into()).await.unwrap_err(),
            "Request identifier is already active"
        );
    }
}
