use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{mpsc, oneshot, Mutex},
};

const EVENT_CAPACITY: usize = 256;
// Bound in-memory protocol events. The reader waits at this limit so the single durable
// consumer catches up without losing provider order or terminal state.
const MAX_PROTOCOL_LINE_BYTES: usize = 8 * 1024 * 1024;
const RPC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
type PendingRequests = HashMap<(i64, u64), PendingRequest>;

struct PendingRequest {
    response: oneshot::Sender<RoutedResponse>,
    hold_following_events: bool,
}

struct RoutedResponse {
    result: Result<Value, String>,
    release: Option<oneshot::Sender<()>>,
}

pub(crate) struct EventBarrierResponse {
    pub(crate) result: Result<Value, String>,
    release: Option<oneshot::Sender<()>>,
}

impl EventBarrierResponse {
    pub(crate) fn release(mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

impl Drop for EventBarrierResponse {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

struct Outbound {
    value: Value,
    written: oneshot::Sender<Result<(), String>>,
}

#[derive(Clone, Debug)]
pub(crate) enum AppServerEvent {
    Notification {
        generation: i64,
        method: String,
        params: Value,
    },
    Request {
        generation: i64,
        id: Value,
        method: String,
        params: Value,
    },
    Exited {
        generation: i64,
        detail: String,
    },
}

#[derive(Clone)]
pub(crate) struct CodexAppServer {
    inner: Arc<Inner>,
}

struct Inner {
    executable: Option<PathBuf>,
    codex_home: Option<PathBuf>,
    startup: Mutex<()>,
    connection: Mutex<Option<Connection>>,
    pending: Arc<Mutex<PendingRequests>>,
    next_id: AtomicU64,
    experimental_initialized: AtomicBool,
    events: mpsc::Sender<AppServerEvent>,
    event_receiver: std::sync::Mutex<Option<mpsc::Receiver<AppServerEvent>>>,
}

struct Connection {
    generation: i64,
    writer: mpsc::Sender<Outbound>,
    child: Arc<Mutex<Child>>,
    alive: Arc<AtomicBool>,
}

impl CodexAppServer {
    fn from_optional_executable(executable: Option<PathBuf>) -> Self {
        Self::from_optional_executable_and_home(executable, None)
    }

    fn from_optional_executable_and_home(executable: Option<PathBuf>, codex_home: Option<PathBuf>) -> Self {
        let (events, event_receiver) = mpsc::channel(EVENT_CAPACITY);
        Self {
            inner: Arc::new(Inner {
                executable,
                codex_home,
                startup: Mutex::new(()),
                connection: Mutex::new(None),
                pending: Arc::new(Mutex::new(HashMap::new())),
                next_id: AtomicU64::new(1),
                experimental_initialized: AtomicBool::new(false),
                events,
                event_receiver: std::sync::Mutex::new(Some(event_receiver)),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_test_executable(executable: PathBuf) -> Self {
        Self::from_optional_executable(Some(executable))
    }

    pub(crate) fn with_codex_home(codex_home: Option<PathBuf>) -> Self {
        Self::from_optional_executable_and_home(None, codex_home)
    }

    pub(crate) fn subscribe(&self) -> Result<mpsc::Receiver<AppServerEvent>, String> {
        self.inner
            .event_receiver
            .lock()
            .map_err(|_| "Codex event receiver lock is unavailable".to_string())?
            .take()
            .ok_or("Codex event receiver is already attached".to_string())
    }

    pub(crate) async fn start(&self, generation: i64) -> Result<(), String> {
        let _startup = self.inner.startup.lock().await;
        if let Some(connection) = self.inner.connection.lock().await.as_ref() {
            if connection.alive.load(Ordering::SeqCst) {
                return Ok(());
            }
        }

        let executable = self
            .inner
            .executable
            .clone()
            .map(Ok)
            .unwrap_or_else(resolve_codex_executable)?;
        let mut command = Command::new(&executable);
        command
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        // Do not set CODEX_HOME for the normal path. Codex itself then uses the
        // same standard local home as its CLI and IDE integration.
        if let Some(home) = &self.inner.codex_home {
            command.env("CODEX_HOME", home);
        }
        let mut child = command.spawn()
            .map_err(|error| classify_spawn_error(&executable, error))?;
        let stdin = child
            .stdin
            .take()
            .ok_or("Codex app-server did not provide stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Codex app-server did not provide stdout")?;
        let child = Arc::new(Mutex::new(child));
        let alive = Arc::new(AtomicBool::new(true));
        let (writer, mut writes) = mpsc::channel::<Outbound>(64);

        let writer_alive = alive.clone();
        let writer_events = self.inner.events.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(message) = writes.recv().await {
                let mut encoded = match serde_json::to_vec(&message.value) {
                    Ok(encoded) => encoded,
                    Err(error) => {
                        let _ = message.written.send(Err(error.to_string()));
                        continue;
                    }
                };
                encoded.push(b'\n');
                let write_result = async {
                    stdin
                        .write_all(&encoded)
                        .await
                        .map_err(|error| error.to_string())?;
                    stdin.flush().await.map_err(|error| error.to_string())
                }
                .await;
                let failed = write_result.is_err();
                let _ = message.written.send(write_result);
                if failed {
                    writer_alive.store(false, Ordering::SeqCst);
                    let _ = writer_events
                        .send(AppServerEvent::Exited {
                            generation,
                            detail: "Codex app-server input closed".into(),
                        })
                        .await;
                    break;
                }
            }
        });

        let reader_pending = self.inner.pending.clone();
        let reader_events = self.inner.events.clone();
        let reader_alive = alive.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_bounded_line(&mut reader).await {
                    Ok(Some(line)) => match serde_json::from_str::<Value>(&line) {
                        Ok(message) => {
                            route_message(generation, message, &reader_pending, &reader_events)
                                .await;
                        }
                        Err(_) => {
                            // Ignore non-protocol output. It is never exposed or persisted.
                        }
                    },
                    Ok(None) => break,
                    Err(error) => {
                        let _ = reader_events
                            .send(AppServerEvent::Exited {
                                generation,
                                detail: error,
                            })
                            .await;
                        break;
                    }
                }
            }
            reader_alive.store(false, Ordering::SeqCst);
            fail_pending_generation(
                &reader_pending,
                generation,
                "Codex app-server connection closed",
            )
            .await;
            let _ = reader_events
                .send(AppServerEvent::Exited {
                    generation,
                    detail: "Codex app-server connection closed".into(),
                })
                .await;
        });

        *self.inner.connection.lock().await = Some(Connection {
            generation,
            writer,
            child,
            alive,
        });

        let initialized = self.request_for_generation(
            generation,
            "initialize",
            json!({
                "clientInfo": {"name": "llm-wiki", "title": "LLM Wiki", "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"experimentalApi": true}
            }),
            false,
        )
        .await;
        if let Err(error) = initialized.and_then(|response| response.result) {
            self.shutdown().await;
            return Err(format!("Codex app-server initialization failed: {error}"));
        }
        self.inner
            .experimental_initialized
            .store(true, Ordering::SeqCst);
        if let Err(error) = self
            .notify_for_generation(generation, "initialized", json!({}))
            .await
        {
            self.shutdown().await;
            return Err(format!("Codex app-server initialization failed: {error}"));
        }
        Ok(())
    }

    pub(crate) async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> Result<(i64, Value), String> {
        let generation = self.generation().await?;
        let result = self
            .request_for_generation(generation, method, params, false)
            .await?;
        Ok((generation, result.result?))
    }

    pub(crate) async fn request_with_event_barrier(
        &self,
        method: &str,
        params: Value,
    ) -> Result<(i64, EventBarrierResponse), String> {
        let generation = self.generation().await?;
        let response = self
            .request_for_generation(generation, method, params, true)
            .await?;
        Ok((
            generation,
            EventBarrierResponse {
                result: response.result,
                release: response.release,
            },
        ))
    }

    pub(crate) async fn respond(
        &self,
        generation: i64,
        id: Value,
        result: Value,
    ) -> Result<(), String> {
        self.send_for_generation(generation, json!({"id": id, "result": result}))
            .await
    }

    pub(crate) async fn generation(&self) -> Result<i64, String> {
        let connection = self.inner.connection.lock().await;
        let connection = connection
            .as_ref()
            .filter(|connection| connection.alive.load(Ordering::SeqCst))
            .ok_or("Codex app-server is not running")?;
        Ok(connection.generation)
    }

    pub(crate) async fn current_generation(&self) -> Option<i64> {
        self.inner
            .connection
            .lock()
            .await
            .as_ref()
            .map(|connection| connection.generation)
    }

    pub(crate) fn structured_input_supported(&self) -> bool {
        self.inner.experimental_initialized.load(Ordering::SeqCst)
    }

    async fn request_for_generation(
        &self,
        generation: i64,
        method: &str,
        params: Value,
        hold_following_events: bool,
    ) -> Result<RoutedResponse, String> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.inner
            .pending
            .lock()
            .await
            .insert(
                (generation, id),
                PendingRequest {
                    response: sender,
                    hold_following_events,
                },
            );
        if let Err(error) = self
            .send_for_generation(
                generation,
                json!({"id": id, "method": method, "params": params}),
            )
            .await
        {
            self.inner.pending.lock().await.remove(&(generation, id));
            return Err(error);
        }
        match tokio::time::timeout(RPC_TIMEOUT, receiver).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err("Codex app-server response channel closed".into()),
            Err(_) => {
                self.inner.pending.lock().await.remove(&(generation, id));
                Err(format!("Codex app-server request timed out: {method}"))
            }
        }
    }

    async fn notify_for_generation(
        &self,
        generation: i64,
        method: &str,
        params: Value,
    ) -> Result<(), String> {
        self.send_for_generation(generation, json!({"method": method, "params": params}))
            .await
    }

    async fn send_for_generation(&self, generation: i64, value: Value) -> Result<(), String> {
        let writer = {
            let connection = self.inner.connection.lock().await;
            let connection = connection
                .as_ref()
                .filter(|connection| connection.alive.load(Ordering::SeqCst))
                .ok_or("Codex app-server is not running")?;
            if connection.generation != generation {
                return Err("Codex app-server connection generation is stale".into());
            }
            connection.writer.clone()
        };
        let (written, acknowledgement) = oneshot::channel();
        writer
            .send(Outbound { value, written })
            .await
            .map_err(|_| "Codex app-server input closed".to_string())?;
        match tokio::time::timeout(RPC_TIMEOUT, acknowledgement).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Codex app-server writer stopped".into()),
            Err(_) => Err("Codex app-server write timed out".into()),
        }
    }

    pub(crate) async fn shutdown(&self) {
        let connection = self.inner.connection.lock().await.take();
        if let Some(connection) = connection {
            connection.alive.store(false, Ordering::SeqCst);
            let _ = connection.child.lock().await.kill().await;
        }
        self.inner
            .experimental_initialized
            .store(false, Ordering::SeqCst);
        fail_all_pending(&self.inner.pending, "Codex app-server stopped").await;
    }
}

async fn route_message(
    generation: i64,
    message: Value,
    pending: &Mutex<PendingRequests>,
    events: &mpsc::Sender<AppServerEvent>,
) {
    if let Some(id) = message.get("id").and_then(Value::as_u64) {
        if message.get("method").is_none() {
            let request = {
                let mut pending = pending.lock().await;
                pending.remove(&(generation, id))
            };
            if let Some(request) = request {
                let result = if let Some(error) = message.get("error") {
                    Err(safe_rpc_error(error))
                } else {
                    Ok(message.get("result").cloned().unwrap_or(Value::Null))
                };
                let (release, released) = oneshot::channel();
                let response = RoutedResponse {
                    result,
                    release: request.hold_following_events.then_some(release),
                };
                if request.response.send(response).is_ok() && request.hold_following_events {
                    // The caller releases this only after it has persisted any identity needed
                    // to route the protocol messages that follow this response. Dropping the
                    // caller future also drops or signals the release, so stdout cannot stall.
                    let _ = released.await;
                }
            }
            return;
        }
    }
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return;
    };
    let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
    let event = if let Some(id) = message.get("id") {
        AppServerEvent::Request {
            generation,
            id: id.clone(),
            method: method.to_owned(),
            params,
        }
    } else {
        AppServerEvent::Notification {
            generation,
            method: method.to_owned(),
            params,
        }
    };
    let _ = events.send(event).await;
}

async fn fail_pending_generation(pending: &Mutex<PendingRequests>, generation: i64, reason: &str) {
    let mut pending = pending.lock().await;
    let ids = pending
        .keys()
        .filter(|(candidate, _)| *candidate == generation)
        .copied()
        .collect::<Vec<_>>();
    for id in ids {
        let Some(request) = pending.remove(&id) else {
            continue;
        };
        let _ = request.response.send(RoutedResponse {
            result: Err(reason.to_owned()),
            release: None,
        });
    }
}

async fn fail_all_pending(pending: &Mutex<PendingRequests>, reason: &str) {
    for (_, request) in std::mem::take(&mut *pending.lock().await) {
        let _ = request.response.send(RoutedResponse {
            result: Err(reason.to_owned()),
            release: None,
        });
    }
}

async fn read_bounded_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(|error| error.to_string())?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .unwrap_or(available.len());
        if bytes.len() + consumed > MAX_PROTOCOL_LINE_BYTES {
            return Err("Codex app-server emitted an oversized protocol message".into());
        }
        bytes.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            break;
        }
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "Codex app-server emitted invalid UTF-8".into())
}

fn safe_rpc_error(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Codex app-server request failed")
        .chars()
        .take(500)
        .collect()
}

fn classify_spawn_error(executable: &Path, error: std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        "Codex CLI is not installed or could not be found".into()
    } else {
        format!(
            "Could not start Codex app-server from {}: {error}",
            executable.display()
        )
    }
}

fn resolve_codex_executable() -> Result<PathBuf, String> {
    if std::env::var_os("LLM_WIKI_E2E_RESULT").is_some()
        && matches!(
            std::env::var("LLM_WIKI_E2E_SCENARIO").ok().as_deref(),
            Some("task-session-execution") | Some("task-codex-execution")
        )
    {
        if let Some(path) = std::env::var_os("LLM_WIKI_CODEX_EXECUTABLE") {
            return Ok(PathBuf::from(path));
        }
    }
    if let Some(path) = executable_on_path("codex") {
        return Ok(path);
    }
    if let Some(path) = dirs::home_dir().map(|home| home.join(".local/bin/codex")) {
        if path.is_file() {
            return Ok(path);
        }
        #[cfg(windows)]
        {
            let path = path.with_extension("exe");
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err("Codex CLI is not installed or could not be found".into())
}

fn executable_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|directory| {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let candidate = directory.join(format!("{name}.exe"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn executable_fixture(script: &str) -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("codex-fixture");
        std::fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
        let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(executable, permissions).unwrap();
        root
    }

    #[tokio::test]
    async fn routes_responses_requests_and_notifications_without_mixing_them() {
        let pending = Mutex::new(HashMap::new());
        let (sender, receiver) = oneshot::channel();
        pending.lock().await.insert(
            (3, 7),
            PendingRequest {
                response: sender,
                hold_following_events: false,
            },
        );
        let (events, mut subscribed) = mpsc::channel(8);

        route_message(
            3,
            json!({"id":7,"result":{"thread":{"id":"t"}}}),
            &pending,
            &events,
        )
        .await;
        assert_eq!(
            receiver.await.unwrap().result.unwrap()["thread"]["id"],
            "t"
        );

        route_message(
            3,
            json!({"method":"turn/completed","params":{"threadId":"t"}}),
            &pending,
            &events,
        )
        .await;
        assert!(
            matches!(subscribed.recv().await.unwrap(), AppServerEvent::Notification { generation: 3, method, .. } if method == "turn/completed")
        );

        route_message(3, json!({"id":"approval-1","method":"item/commandExecution/requestApproval","params":{"threadId":"t"}}), &pending, &events).await;
        assert!(
            matches!(subscribed.recv().await.unwrap(), AppServerEvent::Request { generation: 3, id, method, .. } if id == "approval-1" && method == "item/commandExecution/requestApproval")
        );

        let duplicate = json!({"method":"item/agentMessage/delta","params":{"threadId":"t","turnId":"turn","delta":"same"}});
        route_message(3, duplicate.clone(), &pending, &events).await;
        route_message(3, duplicate, &pending, &events).await;
        for _ in 0..2 {
            assert!(matches!(
                subscribed.recv().await.unwrap(),
                AppServerEvent::Notification { generation: 3, method, params }
                    if method == "item/agentMessage/delta" && params["delta"] == "same"
            ));
        }
    }

    #[tokio::test]
    async fn turn_response_barrier_orders_a_burst_before_request_continuation_without_loss() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = oneshot::channel();
        pending.lock().await.insert(
            (9, 41),
            PendingRequest {
                response: sender,
                hold_following_events: true,
            },
        );
        // A deliberately small channel proves delivery uses backpressure rather than a
        // capacity increase. The producer cannot finish until the consumer drains it.
        let (events, mut received) = mpsc::channel(4);
        let producer_pending = pending.clone();
        let producer_events = events.clone();
        let producer = tokio::spawn(async move {
            route_message(
                9,
                json!({"id":41,"result":{"turn":{"id":"turn-burst","threadId":"thread-burst"}}}),
                &producer_pending,
                &producer_events,
            )
            .await;
            for index in 0..160 {
                route_message(
                    9,
                    json!({"method":"item/agentMessage/delta","params":{"threadId":"thread-burst","turnId":"turn-burst","itemId":"agent","delta":format!("{index} ")}}),
                    &producer_pending,
                    &producer_events,
                )
                .await;
            }
            route_message(
                9,
                json!({"id":"question-burst","method":"item/tool/requestUserInput","params":{"threadId":"thread-burst","turnId":"turn-burst","isBlocking":false}}),
                &producer_pending,
                &producer_events,
            )
            .await;
            route_message(
                9,
                json!({"method":"turn/completed","params":{"threadId":"thread-burst","turnId":"turn-burst","turn":{"id":"turn-burst","status":"completed"}}}),
                &producer_pending,
                &producer_events,
            )
            .await;
        });

        let routed = receiver.await.unwrap();
        assert_eq!(routed.result.as_ref().unwrap()["turn"]["id"], "turn-burst");
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            received.recv()
        )
        .await
        .is_err());
        EventBarrierResponse {
            result: routed.result,
            release: routed.release,
        }
        .release();

        for index in 0..160 {
            assert!(matches!(
                received.recv().await.unwrap(),
                AppServerEvent::Notification { method, params, .. }
                    if method == "item/agentMessage/delta" && params["delta"] == format!("{index} ")
            ));
        }
        assert!(matches!(
            received.recv().await.unwrap(),
            AppServerEvent::Request { id, method, .. }
                if id == "question-burst" && method == "item/tool/requestUserInput"
        ));
        assert!(matches!(
            received.recv().await.unwrap(),
            AppServerEvent::Notification { method, .. } if method == "turn/completed"
        ));
        producer.await.unwrap();
    }

    #[tokio::test]
    async fn dropping_a_turn_response_barrier_releases_the_reader() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (sender, receiver) = oneshot::channel();
        pending.lock().await.insert(
            (12, 8),
            PendingRequest {
                response: sender,
                hold_following_events: true,
            },
        );
        let (events, mut received) = mpsc::channel(1);
        let producer_pending = pending.clone();
        let producer_events = events.clone();
        let producer = tokio::spawn(async move {
            route_message(
                12,
                json!({"id":8,"result":{"turn":{"id":"cancelled-continuation"}}}),
                &producer_pending,
                &producer_events,
            )
            .await;
            route_message(
                12,
                json!({"method":"turn/completed","params":{"turnId":"cancelled-continuation"}}),
                &producer_pending,
                &producer_events,
            )
            .await;
        });

        let routed = receiver.await.unwrap();
        drop(EventBarrierResponse {
            result: routed.result,
            release: routed.release,
        });
        assert!(matches!(
            tokio::time::timeout(std::time::Duration::from_secs(1), received.recv())
                .await
                .unwrap()
                .unwrap(),
            AppServerEvent::Notification { method, .. } if method == "turn/completed"
        ));
        producer.await.unwrap();
    }

    #[test]
    fn rpc_errors_expose_only_the_bounded_message() {
        let error = json!({"code":-32000,"message":"authentication required","data":{"secret":"do not expose"}});
        assert_eq!(safe_rpc_error(&error), "authentication required");
    }

    #[tokio::test]
    async fn missing_executable_is_a_safe_startup_failure() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing-codex");
        let server = CodexAppServer::from_optional_executable(Some(missing));
        let error = server.start(1).await.unwrap_err();
        assert_eq!(error, "Codex CLI is not installed or could not be found");
        assert!(server.current_generation().await.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn authentication_initialization_failure_is_safe_and_not_reused() {
        let fixture = executable_fixture(
            "read _request\necho '{\"id\":1,\"error\":{\"message\":\"authentication required\",\"data\":{\"token\":\"do-not-expose\"}}}'\nsleep 1",
        );
        let server = CodexAppServer::from_optional_executable(Some(
            fixture.path().join("codex-fixture"),
        ));
        let error = server.start(7).await.unwrap_err();
        assert_eq!(
            error,
            "Codex app-server initialization failed: authentication required"
        );
        assert!(!error.contains("do-not-expose"));
        assert!(server.current_generation().await.is_none());
        assert!(!server.structured_input_supported());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn explicit_codex_home_is_passed_to_the_app_server_process() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("alternate-codex-home");
        std::fs::create_dir(&home).unwrap();
        let fixture = executable_fixture(&format!(
            "test \"$CODEX_HOME\" = \"{}\" || exit 1\nread _initialize\necho '{{\"id\":1,\"result\":{{\"capabilities\":{{}}}}}}'\nread _initialized\nsleep 1",
            home.display()
        ));
        let server = CodexAppServer::from_optional_executable_and_home(
            Some(fixture.path().join("codex-fixture")),
            Some(home),
        );
        server.start(9).await.unwrap();
        assert_eq!(server.current_generation().await, Some(9));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn thread_start_rpc_failure_remains_bounded_and_connection_stays_initialized() {
        let fixture = executable_fixture(
            "read _initialize\necho '{\"id\":1,\"result\":{\"capabilities\":{\"experimentalApi\":true}}}'\nread _initialized\nread _thread_start\necho '{\"id\":2,\"error\":{\"message\":\"thread start refused\",\"data\":{\"secret\":\"hidden\"}}}'\nsleep 1",
        );
        let server = CodexAppServer::from_optional_executable(Some(
            fixture.path().join("codex-fixture"),
        ));
        server.start(8).await.unwrap();
        let error = server
            .request("thread/start", json!({"model":"fixture"}))
            .await
            .unwrap_err();
        assert_eq!(error, "thread start refused");
        assert!(!error.contains("hidden"));
        assert_eq!(server.generation().await.unwrap(), 8);
        server.shutdown().await;
    }

    #[tokio::test]
    async fn controlled_process_routes_requests_and_terminal_events() {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fakes/codex_app_server.mjs");
        if !fixture.is_file() {
            return;
        }
        let server = CodexAppServer::from_optional_executable(Some(fixture));
        let mut events = server.subscribe().unwrap();
        server.start(11).await.unwrap();
        let (_, started) = server
            .request(
                "thread/start",
                json!({"model":"gpt-5.6-luna","cwd":env!("CARGO_MANIFEST_DIR"),"approvalsReviewer":"user"}),
            )
            .await
            .unwrap();
        let thread = started["thread"]["id"].as_str().unwrap().to_owned();
        let (_, turn) = server
            .request(
                "turn/start",
                json!({"threadId":thread,"input":[{"type":"text","text":"fixture"}]}),
            )
            .await
            .unwrap();
        let turn_id = turn["turn"]["id"].as_str().unwrap().to_owned();

        let approval_id = loop {
            let event = tokio::time::timeout(RPC_TIMEOUT, events.recv())
                .await
                .unwrap()
                .unwrap();
            if let AppServerEvent::Request { id, method, .. } = event {
                if method == "item/commandExecution/requestApproval" {
                    break id;
                }
            }
        };
        server
            .respond(11, approval_id, json!({"decision":"accept"}))
            .await
            .unwrap();
        let question_id = loop {
            let event = tokio::time::timeout(RPC_TIMEOUT, events.recv())
                .await
                .unwrap()
                .unwrap();
            if let AppServerEvent::Request { id, method, .. } = event {
                if method == "item/tool/requestUserInput" {
                    break id;
                }
            }
        };
        server
            .respond(
                11,
                question_id,
                json!({"answers":{"fixture-choice":{"answers":["Proceed"]},"fixture-text":{"answers":["verified"]}}}),
            )
            .await
            .unwrap();

        let mut completed_item = false;
        let mut completed_turn = false;
        while !completed_turn {
            let event = tokio::time::timeout(RPC_TIMEOUT, events.recv())
                .await
                .unwrap()
                .unwrap();
            if let AppServerEvent::Notification { method, params, .. } = event {
                if method == "item/completed" && params["turnId"] == turn_id {
                    completed_item = true;
                }
                if method == "turn/completed" && params["turnId"] == turn_id {
                    completed_turn = true;
                }
            }
        }
        assert!(completed_item);
        server.shutdown().await;
    }
}
