use crate::application::work_tracking_service::WorkTrackingApplicationService;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(windows)]
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::{Arc, Mutex};
#[cfg(unix)]
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub struct McpListenerShutdown {
    #[cfg(unix)]
    registration: Arc<Mutex<Option<OwnedSocketRegistration>>>,
}

#[cfg(unix)]
#[derive(Clone, PartialEq)]
struct SocketIdentity {
    path: PathBuf,
    device: u64,
    inode: u64,
}

#[cfg(unix)]
#[derive(Clone)]
struct OwnedSocketRegistration {
    identity: SocketIdentity,
    shutdown: CancellationToken,
}

impl McpListenerShutdown {
    #[cfg(unix)]
    fn register(&self, identity: SocketIdentity) -> CancellationToken {
        let shutdown = CancellationToken::new();
        *self
            .registration
            .lock()
            .expect("MCP socket registry is available") = Some(OwnedSocketRegistration {
            identity,
            shutdown: shutdown.clone(),
        });
        shutdown
    }

    #[cfg(unix)]
    fn unregister(&self, identity: &SocketIdentity) {
        let mut registration = self
            .registration
            .lock()
            .expect("MCP socket registry is available");
        if registration
            .as_ref()
            .is_some_and(|registered| registered.identity == *identity)
        {
            registration.take();
        }
    }

    pub fn shutdown(&self) {
        #[cfg(unix)]
        if let Some(registration) = self
            .registration
            .lock()
            .expect("MCP socket registry is available")
            .take()
        {
            registration.shutdown.cancel();
            remove_owned_socket(&registration.identity);
        }
    }
}

#[cfg(unix)]
fn remove_owned_socket(identity: &SocketIdentity) {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let Ok(metadata) = std::fs::symlink_metadata(&identity.path) else {
        return;
    };
    // Only unlink the exact socket this listener bound. This keeps a file,
    // symlink, foreign endpoint, or a replacement socket intact.
    // SAFETY: geteuid has no preconditions and does not mutate process state.
    if metadata.file_type().is_socket()
        && metadata.uid() == unsafe { libc::geteuid() }
        && metadata.dev() == identity.device
        && metadata.ino() == identity.inode
    {
        let _ = std::fs::remove_file(&identity.path);
    }
}

#[cfg(unix)]
struct OwnedUnixListener {
    listener: Option<tokio::net::UnixListener>,
    identity: SocketIdentity,
    lifecycle: McpListenerShutdown,
    shutdown: CancellationToken,
}

#[cfg(unix)]
impl OwnedUnixListener {
    fn bind(path: PathBuf, lifecycle: McpListenerShutdown) -> Result<Self, String> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

        let listener = tokio::net::UnixListener::bind(&path).map_err(|error| error.to_string())?;
        let status = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if !status.file_type().is_socket() {
            return Err("MCP endpoint changed while creating the listener".into());
        }
        let identity = SocketIdentity {
            path,
            device: status.dev(),
            inode: status.ino(),
        };
        let owner = Self {
            listener: Some(listener),
            shutdown: lifecycle.register(identity.clone()),
            identity,
            lifecycle,
        };
        // If setting the socket's private mode fails, `owner` drops and
        // removes the exact socket it just created.
        std::fs::set_permissions(&owner.identity.path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
        Ok(owner)
    }

    async fn accept(
        &self,
    ) -> Result<(tokio::net::UnixStream, tokio::net::unix::SocketAddr), String> {
        self.listener
            .as_ref()
            .expect("owned Unix listener is present until drop")
            .accept()
            .await
            .map_err(|error| error.to_string())
    }

    async fn cancelled(&self) {
        self.shutdown.cancelled().await;
    }
}

#[cfg(unix)]
impl Drop for OwnedUnixListener {
    fn drop(&mut self) {
        // Close the listener before releasing its pathname so a normal
        // shutdown cannot leave a reachable listener without an owner.
        drop(self.listener.take());
        self.lifecycle.unregister(&self.identity);
        remove_owned_socket(&self.identity);
    }
}

const PREFACE_LIMIT: usize = 256;
const MAX_CONNECTIONS: usize = 64;

// The native principal is available only through the GUI command adapter, never
// through a transport whose identity is supplied by an external process.
fn validate_external_principal(
    service: &WorkTrackingApplicationService,
    connection_id: &str,
) -> Result<(), String> {
    if connection_id == "native-in-app-chat" || service.scopes(connection_id).is_err() {
        return Err("MCP connection is unavailable".into());
    }
    Ok(())
}

async fn identify_external_connection(
    service: &WorkTrackingApplicationService,
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<String, String> {
    let connection_id =
        tokio::time::timeout(std::time::Duration::from_secs(5), read_preface(stream))
            .await
            .map_err(|_| "MCP identification timed out")??;
    validate_external_principal(service, &connection_id)?;
    Ok(connection_id)
}

fn endpoint_override() -> Option<String> {
    std::env::var("LLM_WIKI_MCP_ENDPOINT")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(unix)]
pub fn default_endpoint() -> Result<String, String> {
    if let Some(value) = endpoint_override() {
        return Ok(value);
    }
    let directory = dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .ok_or("The local runtime directory is unavailable")?
        .join("LLM Wiki");
    Ok(directory
        .join("mcp")
        .join("mcp.sock")
        .to_string_lossy()
        .into_owned())
}

#[cfg(windows)]
pub fn default_endpoint() -> Result<String, String> {
    if let Some(value) = endpoint_override() {
        return Ok(value);
    }
    let data = dirs::data_local_dir().ok_or("The local data directory is unavailable")?;
    let digest = Sha256::digest(data.to_string_lossy().as_bytes());
    Ok(format!(r"\\.\pipe\llm-wiki-mcp-{:x}", digest)[..49].to_owned())
}

async fn write_preface(
    stream: &mut (impl AsyncWrite + Unpin),
    connection_id: &str,
) -> Result<(), String> {
    if connection_id.trim().is_empty()
        || connection_id.len() > PREFACE_LIMIT
        || connection_id.contains('\n')
    {
        return Err("Invalid MCP connection ID".into());
    }
    stream
        .write_all(format!("{connection_id}\n").as_bytes())
        .await
        .map_err(|error| format!("Could not identify the MCP connection: {error}"))?;
    stream.flush().await.map_err(|error| error.to_string())
}

async fn read_preface(stream: &mut (impl AsyncRead + Unpin)) -> Result<String, String> {
    let mut bytes = Vec::new();
    loop {
        let byte = stream
            .read_u8()
            .await
            .map_err(|error| format!("Could not read the MCP connection identity: {error}"))?;
        if byte == b'\n' {
            break;
        }
        if bytes.len() >= PREFACE_LIMIT {
            return Err("MCP connection identity is too long".into());
        }
        bytes.push(byte);
    }
    let value = String::from_utf8(bytes).map_err(|_| "Invalid MCP connection identity")?;
    if value.trim().is_empty() {
        return Err("Missing MCP connection identity".into());
    }
    Ok(value)
}

#[cfg(unix)]
pub async fn run_stdio_bridge(connection_id: String) -> Result<(), String> {
    let endpoint = default_endpoint()?;
    let mut socket = tokio::net::UnixStream::connect(&endpoint)
        .await
        .map_err(|error| format!("Open LLM Wiki before using MCP ({endpoint}): {error}"))?;
    write_preface(&mut socket, &connection_id).await?;
    let mut stdio = tokio::io::join(tokio::io::stdin(), tokio::io::stdout());
    tokio::io::copy_bidirectional(&mut stdio, &mut socket)
        .await
        .map_err(|error| format!("MCP bridge failed: {error}"))?;
    Ok(())
}

#[cfg(unix)]
pub async fn run_gui_listener(
    service: WorkTrackingApplicationService,
    lifecycle: McpListenerShutdown,
) -> Result<(), String> {
    run_gui_listener_at_with_shutdown(service, default_endpoint()?, lifecycle).await
}

#[cfg(unix)]
pub async fn run_gui_listener_at(
    service: WorkTrackingApplicationService,
    endpoint: String,
) -> Result<(), String> {
    run_gui_listener_at_with_shutdown(service, endpoint, McpListenerShutdown::default()).await
}

#[cfg(unix)]
pub async fn run_gui_listener_at_with_shutdown(
    service: WorkTrackingApplicationService,
    endpoint: String,
    lifecycle: McpListenerShutdown,
) -> Result<(), String> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    let path = PathBuf::from(&endpoint);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                .map_err(|error| error.to_string())?;
        }
        let metadata = std::fs::symlink_metadata(parent).map_err(|error| error.to_string())?;
        // Never chmod a caller-supplied existing directory (for example /tmp).
        // SAFETY: geteuid has no preconditions and does not mutate process state.
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(
                "MCP endpoint requires a private directory owned by the current user".into(),
            );
        }
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if !metadata.file_type().is_socket() {
            return Err("MCP endpoint is not a socket; refusing to replace it".into());
        }
        // SAFETY: geteuid has no preconditions and does not mutate process state.
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err("MCP endpoint belongs to another user".into());
        }
        match tokio::net::UnixStream::connect(&path).await {
            Ok(_) => return Err("Another LLM Wiki process owns the MCP endpoint".into()),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                std::fs::remove_file(&path).map_err(|error| error.to_string())?
            }
            Err(_) => return Err("MCP endpoint is unavailable; refusing to replace it".into()),
        }
    }
    let listener = OwnedUnixListener::bind(path, lifecycle)?;
    let mut sessions = tokio::task::JoinSet::new();
    loop {
        while sessions.try_join_next().is_some() {}
        if sessions.len() >= MAX_CONNECTIONS {
            tokio::select! {
                _ = listener.cancelled() => return Ok(()),
                _ = sessions.join_next() => {}
            }
            continue;
        }
        let (mut stream, _) = tokio::select! {
            _ = listener.cancelled() => return Ok(()),
            accepted = listener.accept() => accepted?,
        };
        let service = service.clone();
        sessions.spawn(async move {
            let result = async {
                let connection_id = identify_external_connection(&service, &mut stream).await?;
                crate::mcp::serve_transport(service, connection_id, stream).await
            }
            .await;
            if result.is_err() {
                eprintln!("MCP local session ended unsuccessfully");
            }
        });
    }
}

#[cfg(windows)]
pub async fn run_stdio_bridge(connection_id: String) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ClientOptions;
    let endpoint = default_endpoint()?;
    let mut pipe = ClientOptions::new()
        .open(&endpoint)
        .map_err(|error| format!("Open LLM Wiki before using MCP ({endpoint}): {error}"))?;
    write_preface(&mut pipe, &connection_id).await?;
    let mut stdio = tokio::io::join(tokio::io::stdin(), tokio::io::stdout());
    tokio::io::copy_bidirectional(&mut stdio, &mut pipe)
        .await
        .map_err(|error| format!("MCP bridge failed: {error}"))?;
    Ok(())
}

#[cfg(windows)]
pub async fn run_gui_listener(
    service: WorkTrackingApplicationService,
    _lifecycle: McpListenerShutdown,
) -> Result<(), String> {
    run_gui_listener_at(service, default_endpoint()?).await
}

#[cfg(windows)]
fn private_pipe(
    endpoint: &str,
    first: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer, String> {
    use tokio::net::windows::named_pipe::ServerOptions;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            },
            GetTokenInformation, TokenUser, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    // SAFETY: token buffers are aligned, lengths come from Windows, and all
    // allocated descriptors/handles are released before returning. No pointer
    // escapes this synchronous pipe-construction function.
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err("Cannot identify the current Windows user".into());
        }
        let mut length = 0;
        GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut length);
        if length == 0 {
            CloseHandle(token);
            return Err("Windows token information is unavailable".into());
        }
        let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
        let result = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        );
        CloseHandle(token);
        if result == 0 {
            return Err("Windows user information is unavailable".into());
        }
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err("Cannot encode Windows user identity".into());
        }
        let mut count = 0;
        while count < 256 && *sid.add(count) != 0 {
            count += 1;
        }
        let identity = String::from_utf16_lossy(std::slice::from_raw_parts(sid, count));
        LocalFree(sid.cast());
        if count == 256 {
            return Err("Invalid Windows user identity".into());
        }
        let sddl: Vec<u16> = format!("D:P(A;;GA;;;{identity})")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = std::ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err("Cannot protect the local MCP pipe".into());
        }
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let result = ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                endpoint,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            );
        LocalFree(descriptor);
        result.map_err(|error| format!("Could not create protected MCP pipe: {error}"))
    }
}

#[cfg(windows)]
pub async fn run_gui_listener_at(
    service: WorkTrackingApplicationService,
    endpoint: String,
) -> Result<(), String> {
    let mut first = true;
    let mut sessions = tokio::task::JoinSet::new();
    loop {
        while sessions.try_join_next().is_some() {}
        if sessions.len() >= MAX_CONNECTIONS {
            sessions.join_next().await;
            continue;
        }
        let mut pipe = private_pipe(&endpoint, first)?;
        first = false;
        pipe.connect().await.map_err(|error| error.to_string())?;
        let service = service.clone();
        sessions.spawn(async move {
            let result = async {
                let connection_id = identify_external_connection(&service, &mut pipe).await?;
                crate::mcp::serve_transport(service, connection_id, pipe).await
            }
            .await;
            if result.is_err() {
                eprintln!("MCP local session ended unsuccessfully");
            }
        });
    }
}
