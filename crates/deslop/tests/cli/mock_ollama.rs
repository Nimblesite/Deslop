//! In-process mock Ollama HTTP server for black-box tests.
//!
//! [REMOVE-STUB] Replaces the deterministic BLAKE3 stub provider in
//! black-box coverage. Tests that used `--embedding-provider stub` now
//! point `--embedding-endpoint` at one of these mocks so the production
//! code paths (Ollama provider, registry lookup, `EmbeddingMode::Required`)
//! get exercised end-to-end.
//!
//! This is a shared test module: the happy-path callers
//! (`MockOllama::spawn`) and the failure-injection caller
//! (`MockOllama::spawn_with`, issue #5) each use only the subset they need,
//! so the unused-symbol lint is silenced for this module — matching the
//! `common` test modules.

#![allow(dead_code)]

use std::{
    collections::BTreeSet,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// How the mock answers `/api/embed` requests. The dimension probe always
/// succeeds regardless, so the provider can still learn the vector width.
#[derive(Clone, Copy, Debug)]
pub(crate) enum MockBehavior {
    /// Every real embed request returns deterministic vectors.
    Happy,
    /// Every real embed request is rejected with a context-length error.
    RejectAllEmbeds,
    /// Only aggregate (multi-input) embed requests are rejected; single-input
    /// retries succeed — exercises the bisect-and-retry path (#5).
    RejectMultiInputEmbeds,
    /// Every real embed request returns finite-JSON values that overflow
    /// `f32`. Exercises provider-output validation before cache/index use.
    OverflowingEmbeddings,
}

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
    /// Largest `input` array length seen on an `/api/embed` call.
    max_embed_batch_len: Arc<AtomicUsize>,
    /// Largest individual input, in Unicode scalar values, observed on a
    /// real `/api/embed` request (the dimension probe is excluded).
    max_embed_input_chars: Arc<AtomicUsize>,
    /// Whether any real `/api/embed` request enabled provider-side
    /// truncation. Accuracy tests require this to remain false.
    embed_truncation_enabled: Arc<AtomicBool>,
    /// Background acceptor thread handle.
    handle: Option<JoinHandle<()>>,
}

impl MockOllama {
    /// Spawns a happy-path Ollama mock exposing one embedding model.
    /// The model is reported as `nomic-embed-text` with a 4-lane
    /// deterministic vector per input.
    pub(crate) fn spawn() -> Result<Self> {
        Self::spawn_with(MockBehavior::Happy)
    }

    /// Spawns a mock that answers `/api/embed` according to `behavior`.
    pub(crate) fn spawn_with(behavior: MockBehavior) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let max_embed_batch_len = Arc::new(AtomicUsize::new(0));
        let max_embed_input_chars = Arc::new(AtomicUsize::new(0));
        let embed_truncation_enabled = Arc::new(AtomicBool::new(false));
        let server_stop = Arc::clone(&stop);
        let server_max = Arc::clone(&max_embed_batch_len);
        let server_max_input = Arc::clone(&max_embed_input_chars);
        let server_truncation = Arc::clone(&embed_truncation_enabled);
        let handle = thread::spawn(move || {
            serve(
                &listener,
                server_stop.as_ref(),
                server_max.as_ref(),
                server_max_input.as_ref(),
                server_truncation.as_ref(),
                behavior,
            );
        });
        Ok(Self {
            endpoint: format!("http://{addr}"),
            addr,
            stop,
            max_embed_batch_len,
            max_embed_input_chars,
            embed_truncation_enabled,
            handle: Some(handle),
        })
    }

    /// Returns the loopback endpoint string suitable for
    /// `--embedding-endpoint`.
    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Largest `input` batch length the mock has served so far.
    pub(crate) fn max_embed_batch_len(&self) -> usize {
        self.max_embed_batch_len.load(Ordering::SeqCst)
    }

    /// Largest real embedding input observed by the mock.
    pub(crate) fn max_embed_input_chars(&self) -> usize {
        self.max_embed_input_chars.load(Ordering::SeqCst)
    }

    /// Whether production asked Ollama to truncate any real input.
    pub(crate) fn embed_truncation_enabled(&self) -> bool {
        self.embed_truncation_enabled.load(Ordering::SeqCst)
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

fn serve(
    listener: &TcpListener,
    stop: &AtomicBool,
    max_embed_batch_len: &AtomicUsize,
    max_embed_input_chars: &AtomicUsize,
    embed_truncation_enabled: &AtomicBool,
    behavior: MockBehavior,
) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                // Switch accepted stream to blocking so read_request never gets
                // WouldBlock on large (> 1 024 B) request bodies — issue #57.
                let _ = stream.set_nonblocking(false);
                handle_stream(
                    stream,
                    max_embed_batch_len,
                    max_embed_input_chars,
                    embed_truncation_enabled,
                    behavior,
                );
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_other) => break,
        }
    }
}

fn handle_stream(
    mut stream: TcpStream,
    max_embed_batch_len: &AtomicUsize,
    max_embed_input_chars: &AtomicUsize,
    embed_truncation_enabled: &AtomicBool,
    behavior: MockBehavior,
) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let response = response_for(
        &request,
        max_embed_batch_len,
        max_embed_input_chars,
        embed_truncation_enabled,
        behavior,
    );
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

fn response_for(
    request: &HttpRequest,
    max_embed_batch_len: &AtomicUsize,
    max_embed_input_chars: &AtomicUsize,
    embed_truncation_enabled: &AtomicBool,
    behavior: MockBehavior,
) -> String {
    match request.path.as_str() {
        "/api/tags" => json_response("200 OK", &tags_body()),
        "/api/show" => json_response("200 OK", &show_body()),
        // The dimension probe always succeeds so the provider can learn the
        // vector width even while real embeds are being rejected.
        "/api/embed" if is_dimension_probe(&request.body) => {
            let inputs = request_inputs(&request.body).unwrap_or_default();
            let embeddings: Vec<Vec<f32>> = inputs.iter().map(|text| embed_vector(text)).collect();
            json_response("200 OK", &json!({ "embeddings": embeddings }))
        }
        "/api/embed" => {
            record_embed_request(
                &request.body,
                max_embed_batch_len,
                max_embed_input_chars,
                embed_truncation_enabled,
            );
            embed_response(&request.body, behavior)
        }
        _ => json_response("404 Not Found", &json!({ "error": "not found" })),
    }
}

fn embed_response(body: &str, behavior: MockBehavior) -> String {
    let inputs = request_inputs(body).unwrap_or_default();
    match behavior {
        MockBehavior::RejectAllEmbeds => context_length_error(),
        MockBehavior::RejectMultiInputEmbeds if inputs.len() > 1 => context_length_error(),
        MockBehavior::OverflowingEmbeddings => overflowing_response(inputs.len()),
        MockBehavior::Happy | MockBehavior::RejectMultiInputEmbeds => {
            let embeddings: Vec<Vec<f32>> = inputs.iter().map(|text| embed_vector(text)).collect();
            json_response("200 OK", &json!({ "embeddings": embeddings }))
        }
    }
}

/// Returns valid JSON numbers that cannot be represented by `f32`.
fn overflowing_response(input_count: usize) -> String {
    let embeddings = vec![vec![3.5e38_f64, 0.0, 0.0, 0.0]; input_count];
    json_response("200 OK", &json!({ "embeddings": embeddings }))
}

fn context_length_error() -> String {
    json_response(
        "500 Internal Server Error",
        &json!({ "error": "input length exceeds the context length" }),
    )
}

/// Width of the deterministic content-sensitive test embedding.
pub(crate) const MOCK_EMBEDDING_DIMENSIONS: usize = 128;

/// Byte width of one content shingle.
const MOCK_SHINGLE_WIDTH: usize = 5;
/// Stable FNV-1a offset basis for shingle hashing.
const FNV_OFFSET_BASIS: u64 = 14_695_981_039_346_656_037;
/// Stable FNV-1a prime for shingle hashing.
const FNV_PRIME: u64 = 1_099_511_628_211;

/// Returns a deterministic signed feature hash of the snippet's distinct
/// five-byte shingles. Content overlap now drives cosine: renamed clones
/// stay close while unrelated snippets of coincidentally similar length do
/// not inherit the near-unit floor of the deleted four-lane vector (#369).
fn embed_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; MOCK_EMBEDDING_DIMENSIONS];
    for shingle in distinct_shingles(text) {
        let hash = shingle_hash(shingle);
        let lane = usize::from(u8::try_from(hash & 0x7F).unwrap_or_default());
        let sign = if hash & 0x80 == 0 { 1.0_f32 } else { -1.0_f32 };
        if let Some(slot) = vector.get_mut(lane) {
            *slot += sign;
        }
    }
    vector
}

/// Distinct byte shingles, with one whole-text feature for short inputs.
fn distinct_shingles(text: &str) -> BTreeSet<&[u8]> {
    let bytes = text.as_bytes();
    if bytes.len() < MOCK_SHINGLE_WIDTH {
        return std::iter::once(bytes).collect();
    }
    bytes.windows(MOCK_SHINGLE_WIDTH).collect()
}

/// Stable 64-bit FNV-1a hash of one shingle.
fn shingle_hash(shingle: &[u8]) -> u64 {
    shingle.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn is_dimension_probe(body: &str) -> bool {
    request_inputs(body).is_some_and(|inputs| inputs == ["deslop"])
}

fn record_embed_request(
    body: &str,
    max_embed_batch_len: &AtomicUsize,
    max_embed_input_chars: &AtomicUsize,
    embed_truncation_enabled: &AtomicBool,
) {
    let inputs = request_inputs(body).unwrap_or_default();
    let _previous = max_embed_batch_len.fetch_max(inputs.len(), Ordering::SeqCst);
    for input in inputs {
        let _previous = max_embed_input_chars.fetch_max(input.chars().count(), Ordering::SeqCst);
    }
    let truncates = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("truncate").and_then(Value::as_bool))
        .unwrap_or(false);
    if truncates {
        embed_truncation_enabled.store(true, Ordering::SeqCst);
    }
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

/// `POST /api/show` metadata. The context length is architecture-keyed
/// exactly as Ollama reports it, and is deliberately far larger than
/// the conservative default so a test can prove the provider's own
/// budget is what the pipeline honours (#286).
pub(crate) fn show_body() -> Value {
    json!({
        "model_info": {
            "general.architecture": MOCK_ARCHITECTURE,
            "mock-bert.context_length": MOCK_CONTEXT_TOKENS,
        }
    })
}

/// Architecture name reported by [`show_body`].
const MOCK_ARCHITECTURE: &str = "mock-bert";

/// Token context length reported by [`show_body`].
pub(crate) const MOCK_CONTEXT_TOKENS: u64 = 32_768;

fn json_response(status: &str, body: &Value) -> String {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_owned());
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    )
}
