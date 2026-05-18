//! In-process mock Ollama HTTP server for CLI black-box tests.
//!
//! [REMOVE-STUB] Replaces the deterministic BLAKE3 stub provider in
//! black-box CLI coverage. Tests that used `--embedding-provider stub`
//! now point `--embedding-endpoint` at one of these mocks so the
//! production code paths (Ollama provider, registry lookup,
//! `EmbeddingMode::Required`) get exercised end-to-end.

use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// In-process mock Ollama HTTP server that returns deterministic
/// embedding vectors. Keep the helper tiny — production tests should
/// only depend on the loopback endpoint, the served model name, and
/// the per-call vectors.
pub(crate) struct MockOllama {
    /// HTTP endpoint formatted as `http://127.0.0.1:<port>`.
    endpoint: String,
    /// Bound socket address used for the join-on-drop poison pill.
    addr: SocketAddr,
    /// Cooperative shutdown flag.
    stop: Arc<AtomicBool>,
    /// Background acceptor thread handle.
    handle: Option<JoinHandle<()>>,
}

impl MockOllama {
    /// Spawns a happy-path Ollama mock exposing one embedding model.
    /// The model is reported as `nomic-embed-text` with a 4-lane
    /// deterministic vector per input.
    pub(crate) fn spawn() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let server_stop = Arc::clone(&stop);
        let handle = thread::spawn(move || serve(&listener, server_stop.as_ref()));
        Ok(Self {
            endpoint: format!("http://{addr}"),
            addr,
            stop,
            handle: Some(handle),
        })
    }

    /// Returns the loopback endpoint string suitable for
    /// `--embedding-endpoint`.
    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl Drop for MockOllama {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn serve(listener: &TcpListener, stop: &AtomicBool) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let _ = stream.set_nonblocking(false);
                handle_stream(stream);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_other) => break,
        }
    }
}

fn handle_stream(mut stream: TcpStream) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let response = response_for(&request);
    let _ = stream.write_all(response.as_bytes());
}

#[derive(Debug)]
struct HttpRequest {
    path: String,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut data = Vec::new();
    let mut buffer = [0_u8; 1024];
    while complete_len(&data).map_or(true, |len| data.len() < len) {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if let Some(chunk) = buffer.get(..read) {
            data.extend_from_slice(chunk);
        }
    }
    parse_request(&data)
}

fn complete_len(data: &[u8]) -> Option<usize> {
    let header_end = header_end(data)?;
    let headers = String::from_utf8_lossy(data.get(..header_end)?);
    Some(
        header_end
            .saturating_add(4)
            .saturating_add(content_length(&headers)),
    )
}

fn header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(parse_content_length)
        .unwrap_or_default()
}

fn parse_content_length(line: &str) -> Option<usize> {
    let (name, value) = line.split_once(':')?;
    name.eq_ignore_ascii_case("content-length")
        .then(|| value.trim().parse().ok())
        .flatten()
}

fn parse_request(data: &[u8]) -> Result<HttpRequest> {
    let header_end = header_end(data).ok_or_else(|| anyhow!("missing header terminator"))?;
    let headers = String::from_utf8_lossy(data.get(..header_end).unwrap_or_default());
    let body_start = header_end.saturating_add(4);
    let body_len = content_length(&headers);
    let body_end = body_start.saturating_add(body_len).min(data.len());
    Ok(HttpRequest {
        path: request_path(&headers),
        body: String::from_utf8_lossy(data.get(body_start..body_end).unwrap_or_default())
            .into_owned(),
    })
}

fn request_path(headers: &str) -> String {
    headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_owned()
}

fn response_for(request: &HttpRequest) -> String {
    match request.path.as_str() {
        "/api/tags" => json_response("200 OK", &tags_body()),
        "/api/embed" => embed_response(&request.body),
        _ => json_response("404 Not Found", &json!({ "error": "not found" })),
    }
}

fn embed_response(body: &str) -> String {
    let inputs = request_inputs(body).unwrap_or_default();
    let embeddings: Vec<Vec<f32>> = inputs.iter().map(|text| embed_vector(text)).collect();
    json_response("200 OK", &json!({ "embeddings": embeddings }))
}

/// Returns a 4-lane deterministic vector seeded by `text` length and
/// first byte. Stable across runs so cache round-trip tests keep
/// converging.
fn embed_vector(text: &str) -> Vec<f32> {
    let len_bits = u16::try_from(text.len() & 0xffff).unwrap_or(0);
    let len = f32::from(len_bits);
    let first = f32::from(text.bytes().next().unwrap_or(0));
    vec![len.sin(), first.cos(), 0.5_f32, -0.5_f32]
}

fn request_inputs(body: &str) -> Option<Vec<String>> {
    serde_json::from_str::<Value>(body).ok().and_then(|value| {
        let input = value.get("input")?;
        if let Some(text) = input.as_str() {
            return Some(vec![text.to_owned()]);
        }
        input.as_array().map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
    })
}

fn tags_body() -> Value {
    json!({
        "models": [{
            "name": "nomic-embed-text:latest",
            "digest": "0123456789abcdef",
            "size": 42_u64
        }]
    })
}

fn json_response(status: &str, body: &Value) -> String {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_owned());
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    )
}
