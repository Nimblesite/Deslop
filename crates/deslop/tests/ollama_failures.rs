//! End-to-end coverage for rejected Ollama subtree embeddings.
//!
//! Issue #5: provider failures must not be represented as zero vectors.

use std::{
    fs,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::{json, Value};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn mock_provider_rejected_subtrees_are_reported() -> Result<()> {
    let server = MockOllama::spawn()?;
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&scan_root)
        .arg("--min-nodes")
        .arg("8")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .arg("--embeddings")
        .arg("required")
        .arg("--embedding-provider")
        .arg("ollama")
        .arg("--embedding-model")
        .arg("nomic-embed-text")
        .arg("--embedding-endpoint")
        .arg(server.endpoint())
        .assert()
        .success();
    let provenance = embedding_provenance(tmp.path())?;
    let attempted = metric(&provenance, "attempted_subtrees");
    let failed = metric(&provenance, "failed_subtrees");
    assert!(
        attempted > 0,
        "embedding attempts must be surfaced: {provenance}"
    );
    assert!(
        failed > 0,
        "provider rejections must be counted: {provenance}"
    );
    assert!(
        failed <= attempted,
        "failed_subtrees cannot exceed attempted_subtrees: {provenance}"
    );
    assert!(
        server.max_embed_batch_len() > 1,
        "Ollama embeddings must be requested in batches; max batch was {}",
        server.max_embed_batch_len()
    );
    Ok(())
}

fn seed_scan_root(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let _bytes = fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn embedding_provenance(tmp: &Path) -> Result<Value> {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    let report: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    report
        .get("embedding_provenance")
        .cloned()
        .ok_or_else(|| anyhow!("embedding_provenance missing: {report}"))
}

fn metric(provenance: &Value, field: &str) -> u64 {
    provenance
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

struct MockOllama {
    endpoint: String,
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    max_embed_batch_len: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl MockOllama {
    fn spawn() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let max_embed_batch_len = Arc::new(AtomicUsize::new(0));
        let server_stop = Arc::clone(&stop);
        let server_max_embed_batch_len = Arc::clone(&max_embed_batch_len);
        let handle = thread::spawn(move || {
            server_loop(
                &listener,
                server_stop.as_ref(),
                server_max_embed_batch_len.as_ref(),
            );
        });
        Ok(Self {
            endpoint: format!("http://{addr}"),
            addr,
            stop,
            max_embed_batch_len,
            handle: Some(handle),
        })
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn max_embed_batch_len(&self) -> usize {
        self.max_embed_batch_len.load(Ordering::SeqCst)
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

fn server_loop(listener: &TcpListener, stop: &AtomicBool, max_embed_batch_len: &AtomicUsize) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => handle_stream(stream, max_embed_batch_len),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
}

fn handle_stream(mut stream: TcpStream, max_embed_batch_len: &AtomicUsize) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let response = response_for(&request, max_embed_batch_len);
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

fn response_for(request: &HttpRequest, max_embed_batch_len: &AtomicUsize) -> String {
    match request.path.as_str() {
        "/api/tags" => json_response("200 OK", &tags_body()),
        "/api/embed" if is_dimension_probe(&request.body) => {
            json_response("200 OK", &json!({ "embeddings": [[1.0, 0.0, 0.0, 0.0]] }))
        }
        "/api/embed" => {
            record_embed_batch_len(&request.body, max_embed_batch_len);
            json_response(
                "500 Internal Server Error",
                &json!({ "error": "input length exceeds the context length" }),
            )
        }
        _ => json_response("404 Not Found", &json!({ "error": "not found" })),
    }
}

fn tags_body() -> Value {
    json!({
        "models": [{
            "name": "nomic-embed-text:latest",
            "digest": "0123456789abcdef",
            "size": 42
        }]
    })
}

fn is_dimension_probe(body: &str) -> bool {
    request_inputs(body).is_some_and(|inputs| inputs == ["deslop"])
}

fn record_embed_batch_len(body: &str, max_embed_batch_len: &AtomicUsize) {
    let len = request_inputs(body).map_or(0, |inputs| inputs.len());
    let _previous = max_embed_batch_len.fetch_max(len, Ordering::SeqCst);
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

fn json_response(status: &str, body: &Value) -> String {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_owned());
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
        text.len()
    )
}
