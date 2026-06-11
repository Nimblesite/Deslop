//! Cross-platform IPC endpoint shared by the LSP server and the MCP
//! client ([LIVE-IPC-SOCKET], [LIVE-IPC-TCP], [MCP-IPC-DISCOVERY]).
//!
//! Two transports carry the same line-delimited JSON-RPC protocol:
//!
//! - **Unix domain socket** `.deslop-cache/deslop.sock` — the default
//!   wherever Unix sockets exist. Behaviour is unchanged from the
//!   original [LIVE-IPC-SOCKET] design.
//! - **TCP loopback** `127.0.0.1:<os-assigned port>` — the default on
//!   Windows (no Unix sockets) and opt-in elsewhere via
//!   `deslop-lsp --ipc-transport tcp`. The bound port and a
//!   per-session secret are published in `.deslop-cache/deslop.port`
//!   ([`IpcEndpointFile`]). Clients present the secret as the first
//!   line of every connection, so the analysis server stays closed to
//!   other local processes and a stale discovery record colliding with
//!   a foreign listener fails closed instead of exchanging garbage.
//!
//! Everything here is plain blocking `std::net` / `std::os` IO. The
//! TCP path compiles on every platform and is exercised by Unix E2E
//! tests, so the code Windows runs is the code CI already tested.

use std::{
    io::{BufRead, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::{Path, PathBuf},
};

pub use crate::wire_generated::IpcEndpointFile;

/// File name of the Unix-domain IPC socket under `.deslop-cache`.
pub const IPC_SOCKET_FILE_NAME: &str = "deslop.sock";

/// File name of the TCP endpoint discovery record under `.deslop-cache`.
pub const IPC_PORT_FILE_NAME: &str = "deslop.port";

/// Absolute path of the Unix-domain socket for `root`.
#[must_use]
pub fn socket_path(root: &Path) -> PathBuf {
    cache_dir(root).join(IPC_SOCKET_FILE_NAME)
}

/// Absolute path of the TCP endpoint discovery record for `root`.
#[must_use]
pub fn port_file_path(root: &Path) -> PathBuf {
    cache_dir(root).join(IPC_PORT_FILE_NAME)
}

/// Workspace cache directory hosting both IPC endpoint artifacts.
fn cache_dir(root: &Path) -> PathBuf {
    root.join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME)
}

/// Which transport the IPC server binds ([LIVE-IPC-TCP]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMode {
    /// Unix-domain socket at [`socket_path`]. Unavailable on Windows.
    Unix,
    /// TCP loopback with the discovery record at [`port_file_path`].
    Tcp,
}

impl IpcMode {
    /// Platform default: Unix sockets where they exist, TCP loopback
    /// on Windows.
    #[must_use]
    pub const fn platform_default() -> Self {
        if cfg!(unix) {
            Self::Unix
        } else {
            Self::Tcp
        }
    }
}

/// Listening IPC endpoint bound by the LSP ([LSP-IPC]).
#[derive(Debug)]
pub enum IpcServerListener {
    /// Unix-domain socket listener.
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixListener),
    /// TCP loopback listener plus the per-session secret published in
    /// the discovery record.
    Tcp {
        /// Bound loopback listener.
        listener: TcpListener,
        /// Secret every client must present as its first line.
        token: String,
    },
}

/// One accepted client connection plus the secret it must present
/// before the JSON-RPC exchange starts. `None` on Unix sockets, which
/// the filesystem already permission-guards.
#[derive(Debug)]
pub struct IpcConnection {
    /// Bidirectional stream carrying line-delimited JSON-RPC.
    pub stream: IpcStream,
    /// Expected token line, when the transport requires authentication.
    pub expected_token: Option<String>,
}

impl IpcServerListener {
    /// Binds `transport` under `root`, creating `.deslop-cache`,
    /// clearing stale endpoint artifacts of both transports, and
    /// publishing the TCP discovery record when applicable.
    ///
    /// # Errors
    ///
    /// Returns the underlying IO failure when the cache directory
    /// cannot be created or the endpoint cannot be bound. Requesting
    /// [`IpcMode::Unix`] on a platform without Unix sockets fails
    /// with [`std::io::ErrorKind::Unsupported`].
    pub fn bind(root: &Path, transport: IpcMode) -> std::io::Result<Self> {
        std::fs::create_dir_all(cache_dir(root))?;
        match transport {
            IpcMode::Unix => bind_unix(root),
            IpcMode::Tcp => bind_tcp(root),
        }
    }

    /// Accepts the next client connection.
    ///
    /// # Errors
    ///
    /// Propagates the OS accept failure.
    pub fn accept(&self) -> std::io::Result<IpcConnection> {
        match self {
            #[cfg(unix)]
            Self::Unix(listener) => {
                let (stream, _addr) = listener.accept()?;
                Ok(IpcConnection {
                    stream: IpcStream::Unix(stream),
                    expected_token: None,
                })
            }
            Self::Tcp { listener, token } => {
                let (stream, _addr) = listener.accept()?;
                Ok(IpcConnection {
                    stream: IpcStream::Tcp(stream),
                    expected_token: Some(token.clone()),
                })
            }
        }
    }

    /// Endpoint artifacts to delete when the server shuts down.
    #[must_use]
    pub fn cleanup_paths(&self, root: &Path) -> Vec<PathBuf> {
        match self {
            #[cfg(unix)]
            Self::Unix(_) => vec![socket_path(root)],
            Self::Tcp { .. } => vec![port_file_path(root)],
        }
    }
}

/// Unix leg of [`IpcServerListener::bind`].
#[cfg(unix)]
fn bind_unix(root: &Path) -> std::io::Result<IpcServerListener> {
    let path = socket_path(root);
    let _removed = std::fs::remove_file(&path);
    let _stale_record = std::fs::remove_file(port_file_path(root));
    let listener = std::os::unix::net::UnixListener::bind(&path)?;
    tracing::info!(path = %path.display(), "ipc_socket_bound");
    Ok(IpcServerListener::Unix(listener))
}

/// Unix sockets do not exist on this platform; callers should bind
/// [`IpcMode::Tcp`] instead.
#[cfg(not(unix))]
fn bind_unix(_root: &Path) -> std::io::Result<IpcServerListener> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unix-domain IPC transport is unavailable on this platform",
    ))
}

/// TCP leg of [`IpcServerListener::bind`]: bind an OS-assigned
/// loopback port and publish the discovery record.
fn bind_tcp(root: &Path) -> std::io::Result<IpcServerListener> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    let token = generate_token()?;
    write_port_file(root, port, &token)?;
    remove_stale_socket(root);
    tracing::info!(port, "ipc_tcp_bound");
    Ok(IpcServerListener::Tcp { listener, token })
}

/// Clears a stale Unix socket left by a previous Unix-transport run.
#[cfg(unix)]
fn remove_stale_socket(root: &Path) {
    let _removed = std::fs::remove_file(socket_path(root));
}

/// No socket artifact can exist on platforms without Unix sockets.
#[cfg(not(unix))]
fn remove_stale_socket(_root: &Path) {}

/// Publishes the TCP discovery record ([`IpcEndpointFile`]) for `root`.
fn write_port_file(root: &Path, port: u16, token: &str) -> std::io::Result<()> {
    let endpoint = IpcEndpointFile {
        port,
        token: token.to_owned(),
    };
    let payload = serde_json::to_vec(&endpoint)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let path = port_file_path(root);
    std::fs::write(&path, payload)?;
    restrict_to_owner(&path)
}

/// Tightens the discovery record to owner-only where Unix permissions
/// exist; Windows workspace ACLs already scope the file to the user.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

/// Windows ACLs on the workspace already scope the record to the user.
#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Per-session hex token derived from OS entropy. The token only
/// keeps foreign local processes out of the analysis server, so a
/// plain equality check at the server suffices.
fn generate_token() -> std::io::Result<String> {
    let mut seed = [0_u8; 16];
    getrandom::getrandom(&mut seed).map_err(std::io::Error::other)?;
    Ok(blake3::hash(&seed).to_hex().to_string())
}

/// Reads the client's token line on authenticated transports and
/// compares it against `expected`. Unix-socket connections pass `None`
/// and are accepted unconditionally. Returns `false` when the client
/// must be dropped; logs no payload content either way.
pub fn authenticate<R: BufRead>(reader: &mut R, expected: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return false;
    }
    let accepted = line.trim_end_matches(['\r', '\n']) == expected;
    if !accepted {
        tracing::warn!("ipc_tcp_token_rejected");
    }
    accepted
}

/// Bidirectional IPC stream over either transport.
#[derive(Debug)]
pub enum IpcStream {
    /// Unix-domain socket stream.
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    /// TCP loopback stream.
    Tcp(TcpStream),
}

impl IpcStream {
    /// Clones the underlying OS handle so reads and writes can run on
    /// separate halves of the same connection.
    ///
    /// # Errors
    ///
    /// Propagates the OS handle-duplication failure.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            #[cfg(unix)]
            Self::Unix(stream) => stream.try_clone().map(Self::Unix),
            Self::Tcp(stream) => stream.try_clone().map(Self::Tcp),
        }
    }
}

impl Read for IpcStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(stream) => stream.read(buf),
            Self::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for IpcStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(stream) => stream.write(buf),
            Self::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(stream) => stream.flush(),
            Self::Tcp(stream) => stream.flush(),
        }
    }
}

/// Why an IPC connection could not be established ([MCP-IPC-CLIENT]).
#[derive(Debug, thiserror::Error)]
pub enum IpcConnectError {
    /// Neither endpoint is live — the LSP is not running, or it left
    /// stale discovery artifacts behind.
    #[error("no live IPC endpoint (socket {socket_path:?}, discovery record {port_file:?})")]
    NotRunning {
        /// Unix socket path that was tried (or would be, on Unix).
        socket_path: PathBuf,
        /// TCP discovery record path that was tried.
        port_file: PathBuf,
    },
    /// The discovery record exists but cannot be parsed.
    #[error("ipc discovery record corrupt: {0}")]
    DiscoveryCorrupt(String),
    /// An endpoint exists but transport-level IO failed.
    #[error("ipc connect failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Connects to the live server for `root`: the Unix socket first where
/// it exists, then the TCP discovery record. On TCP the token line is
/// written before returning, so callers start straight at JSON-RPC.
///
/// # Errors
///
/// [`IpcConnectError::NotRunning`] when no endpoint answers — the LSP
/// is down. Other variants indicate a live-but-broken endpoint.
pub fn connect(root: &Path) -> Result<IpcStream, IpcConnectError> {
    #[cfg(unix)]
    match connect_unix(root) {
        Ok(stream) => return Ok(stream),
        Err(error) if endpoint_absent(&error) => {}
        Err(error) => return Err(IpcConnectError::Io(error)),
    }
    connect_tcp(root)
}

/// Dials the Unix socket for `root`.
#[cfg(unix)]
fn connect_unix(root: &Path) -> std::io::Result<IpcStream> {
    std::os::unix::net::UnixStream::connect(socket_path(root)).map(IpcStream::Unix)
}

/// True for IO failures meaning "nothing is listening here", which
/// permit falling through to the next transport.
fn endpoint_absent(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
    )
}

/// TCP leg of [`connect`]: read the discovery record, dial loopback,
/// present the token line.
fn connect_tcp(root: &Path) -> Result<IpcStream, IpcConnectError> {
    let endpoint = read_port_file(root)?;
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, endpoint.port))
        .map_err(|error| classify_tcp_dial_error(root, error))?;
    stream.write_all(endpoint.token.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(IpcStream::Tcp(stream))
}

/// Loads and parses the discovery record, mapping a missing file to
/// [`IpcConnectError::NotRunning`].
fn read_port_file(root: &Path) -> Result<IpcEndpointFile, IpcConnectError> {
    let path = port_file_path(root);
    let bytes = std::fs::read(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            not_running(root)
        } else {
            IpcConnectError::Io(error)
        }
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| IpcConnectError::DiscoveryCorrupt(error.to_string()))
}

/// A refused loopback dial means the record is stale (the LSP died) —
/// report "not running"; anything else is a real IO failure.
fn classify_tcp_dial_error(root: &Path, error: std::io::Error) -> IpcConnectError {
    if endpoint_absent(&error) {
        not_running(root)
    } else {
        IpcConnectError::Io(error)
    }
}

/// Builds the both-paths [`IpcConnectError::NotRunning`] for `root`.
fn not_running(root: &Path) -> IpcConnectError {
    IpcConnectError::NotRunning {
        socket_path: socket_path(root),
        port_file: port_file_path(root),
    }
}
