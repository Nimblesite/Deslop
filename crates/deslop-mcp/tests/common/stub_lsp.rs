//! A stand-in for a `deslop-lsp` from another Deslop build: it binds the
//! IPC socket the MCP discovers and answers each request with whatever
//! the scripted responder returns. A real LSP cannot serve a wire it was
//! not built with, and an LSP from another release cannot be linked into
//! this test binary, so the two mismatch suites — a method the older
//! release does not know (gh #148) and a report written by an older wire
//! ([MCP-IPC-WIRE-MISMATCH]) — script the foreign reply instead. Each
//! stub stamps its reply envelopes as the build it stands in for would:
//! with this build's version, another release's, or — for a release
//! older than the stamp — not at all. The `report/subscribe` ack is
//! always honoured so the MCP's startup handshake never blocks.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
    sync::Arc,
    thread,
};

use anyhow::Result;
use serde_json::{json, Value};

/// Workspace-relative directory the LSP publishes its endpoint in.
const IPC_CACHE_DIR: &str = ".deslop/cache";
/// The socket name the MCP discovers under that directory.
const IPC_SOCKET_NAME: &str = "deslop.sock";
/// The one method every stub answers itself.
const SUBSCRIBE_METHOD: &str = "report/subscribe";
/// JSON-RPC's code for a method the server does not know.
pub const METHOD_NOT_FOUND_CODE: i64 = -32_601;
/// JSON-RPC's message for that code.
const METHOD_NOT_FOUND_MESSAGE: &str = "method not found";

/// The envelope field a `deslop-lsp` stamps its build's version in.
const PRODUCER_FIELD: &str = "tool_version";

/// How the stub stamps its reply envelopes.
#[derive(Clone, Copy)]
pub enum Stamp {
    /// As this build's own `deslop-lsp` does.
    ThisBuild,
    /// As another release that stamps its envelopes does.
    Release(&'static str),
    /// As a release older than the envelope stamp does: not at all.
    Unstamped,
}

impl Stamp {
    /// The version the envelope carries, if any.
    fn version(self) -> Option<&'static str> {
        match self {
            Self::ThisBuild => Some(env!("CARGO_PKG_VERSION")),
            Self::Release(version) => Some(version),
            Self::Unstamped => None,
        }
    }
}

/// One scripted reply to a request.
pub enum Reply {
    /// A `result` payload.
    Result(Value),
    /// An `error` envelope.
    Error {
        /// JSON-RPC error code.
        code: i64,
        /// JSON-RPC error message.
        message: &'static str,
    },
}

/// Maps a request method to its scripted reply.
pub type Responder = Arc<dyn Fn(&str) -> Reply + Send + Sync>;

/// The reply of an LSP that does not know the method — an older release.
pub fn method_not_found(_method: &str) -> Reply {
    Reply::Error {
        code: METHOD_NOT_FOUND_CODE,
        message: METHOD_NOT_FOUND_MESSAGE,
    }
}

/// Creates the IPC directory under `workspace` and binds the stub there,
/// stamping every reply envelope as `stamp` says.
pub fn bind_stub_lsp(workspace: &Path, stamp: Stamp, respond: Responder) -> Result<()> {
    let cache_dir = workspace.join(IPC_CACHE_DIR);
    fs::create_dir_all(&cache_dir)?;
    let listener = UnixListener::bind(cache_dir.join(IPC_SOCKET_NAME))?;
    let _accept_loop = thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { continue };
            let respond = Arc::clone(&respond);
            let _connection = thread::spawn(move || serve_one_connection(stream, stamp, &respond));
        }
    });
    Ok(())
}

/// Answers the single request one MCP connection carries.
fn serve_one_connection(stream: UnixStream, stamp: Stamp, respond: &Responder) {
    let Ok(writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let request: Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(_) => return,
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let _written = write_frame(&writer, &stamped(frame_for(&id, method, respond), stamp));
}

/// The reply frame for one request: the subscribe ack, or the scripted reply.
fn frame_for(id: &Value, method: &str, respond: &Responder) -> Value {
    if method == SUBSCRIBE_METHOD {
        return json!({ "jsonrpc": "2.0", "id": id, "result": { "subscribed": true, "generation": 0 } });
    }
    match respond(method) {
        Reply::Result(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Reply::Error { code, message } => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    }
}

/// `frame` with its envelope stamped as `stamp` says.
fn stamped(mut frame: Value, stamp: Stamp) -> Value {
    if let (Some(version), Some(fields)) = (stamp.version(), frame.as_object_mut()) {
        let _previous = fields.insert(PRODUCER_FIELD.to_owned(), json!(version));
    }
    frame
}

/// Writes one newline-delimited JSON-RPC frame.
fn write_frame(mut stream: &UnixStream, value: &Value) -> std::io::Result<()> {
    let mut payload = serde_json::to_vec(value).unwrap_or_default();
    payload.push(b'\n');
    stream.write_all(&payload)
}
