//! End-to-end tests for the LSP binary ([LSP-TESTING]).
//!
//! Drives the real `codededup-lsp` binary over stdio with raw
//! JSON-RPC frames per the LSP base protocol — `Content-Length: N\r\n
//! \r\n{json}`. No mocking; the binary links the live feature and
//! runs against fixture workspaces under `tests/fixtures/`.

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::atomic::{AtomicI64, Ordering},
};

use anyhow::{anyhow, Result};

/// Atomic counter for JSON-RPC ids so concurrent tests in the same
/// process never collide.
static NEXT_ID: AtomicI64 = AtomicI64::new(1);

/// Returns a workspace-relative fixture path. Mirrors the helper used
/// by the CLI tests so the same C# corpora power both layers.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("codededup")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies the named fixture into a freshly created temp directory so
/// the LSP server's `.codededup-cache/` writes never pollute the
/// committed fixtures.
fn copy_fixture(name: &str) -> Result<tempfile::TempDir> {
    let src = fixture(name);
    let dst = tempfile::tempdir()?;
    for entry in fs::read_dir(&src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let _bytes = fs::copy(entry.path(), dst.path().join(entry.file_name()))?;
        }
    }
    Ok(dst)
}

/// Spawns the LSP binary against `workspace_root`.
fn spawn_lsp(workspace_root: &Path, min_nodes: u32) -> Result<Child> {
    let bin = assert_cmd::cargo::cargo_bin("codededup-lsp");
    let child = Command::new(bin)
        .arg(workspace_root)
        .arg("--min-nodes")
        .arg(min_nodes.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}

/// Writes one LSP framed message.
fn write_frame(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(payload.as_bytes())?;
    stdin.flush()?;
    Ok(())
}

/// Reads one framed JSON-RPC message from `stdout`.
fn read_frame(reader: &mut BufReader<ChildStdout>) -> Result<serde_json::Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let _read = reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }
    let length = content_length.ok_or_else(|| anyhow!("missing Content-Length"))?;
    let mut buf = vec![0_u8; length];
    reader.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

/// Builds an `initialize` request with a unique id.
fn initialize_request() -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds a custom-method request with a unique id.
fn custom_request(method: &str, params: &serde_json::Value) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds a parameter-less custom-method request.
fn custom_request_no_params(method: &str) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Sends a request and reads exactly one response, discarding any
/// notifications that come first.
fn send_and_recv(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    payload: &str,
) -> Result<serde_json::Value> {
    write_frame(stdin, payload)?;
    loop {
        let frame = read_frame(reader)?;
        if frame.get("id").and_then(serde_json::Value::as_i64) == Some(id) {
            return Ok(frame);
        }
    }
}

/// Acquires `child.stdin` + `child.stdout` after a successful spawn.
fn take_io(child: &mut Child) -> Result<(ChildStdin, BufReader<ChildStdout>)> {
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("child stdin missing"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("child stdout missing"))?;
    Ok((stdin, BufReader::new(stdout)))
}

/// Convenience: best-effort kill + wait. Test cleanup; ignore errors.
fn shut_down(mut child: Child) {
    let _kill = child.kill();
    let _wait = child.wait();
}

/// Returns the JSON `result` field or an error containing the response.
fn result_value(response: &serde_json::Value) -> Result<&serde_json::Value> {
    response
        .get("result")
        .ok_or_else(|| anyhow!("missing result field: {response}"))
}

#[test]
fn lsp_binary_responds_to_initialize() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (id, payload) = initialize_request()?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    assert_eq!(
        response.get("id").and_then(serde_json::Value::as_i64),
        Some(id)
    );
    let result = result_value(&response)?;
    assert!(result.get("capabilities").is_some(), "{response}");
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_session_config_returns_workspace_root() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request_no_params("codededup/sessionConfig")?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let workspace_root = result
        .get("workspace_root")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("workspace_root missing in {response}"))?;
    assert!(
        !workspace_root.is_empty(),
        "workspace_root should be non-empty: {workspace_root}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_report_get_returns_clusters() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request_no_params("codededup/reportGet")?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let clusters = result
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing in {response}"))?;
    assert!(!clusters.is_empty(), "fixture should produce clusters");
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_find_similar_returns_below_min_nodes_for_tiny_snippet() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 1_000)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let params = serde_json::json!({
        "input": {
            "kind": "snippet",
            "snippet": "class A { void M() {} }",
            "language": "csharp"
        },
        "max_results": null
    });
    let (id, payload) = custom_request("codededup/duplicatesFindSimilar", &params)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let below = result
        .get("below_min_nodes")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| anyhow!("below_min_nodes missing in {response}"))?;
    assert!(below, "tiny snippet should set below_min_nodes");
    shut_down(child);
    Ok(())
}

/// Builds a `textDocument/diagnostic` pull request for `path` ([LSP-DIAGNOSTICS]).
fn diagnostic_request(path: &Path) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/diagnostic",
        "params": { "textDocument": { "uri": uri.as_str() } }
    });
    Ok((id, serde_json::to_string(&payload)?))
}

#[test]
fn lsp_initialize_advertises_pull_diagnostic_provider() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (id, payload) = initialize_request()?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let capabilities = result_value(&response)?
        .get("capabilities")
        .ok_or_else(|| anyhow!("missing capabilities: {response}"))?;
    let diagnostic = capabilities
        .get("diagnosticProvider")
        .ok_or_else(|| anyhow!("diagnosticProvider not advertised: {capabilities}"))?;
    let inter_file = diagnostic
        .get("interFileDependencies")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| anyhow!("interFileDependencies missing: {diagnostic}"))?;
    assert!(
        inter_file,
        "interFileDependencies must be true so global percentile recalcs propagate: {diagnostic}",
    );
    let identifier = diagnostic
        .get("identifier")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("identifier missing: {diagnostic}"))?;
    assert_eq!(identifier, "codededup");
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_diagnostic_returns_codededup_diagnostics_for_clone_file() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = diagnostic_request(&alpha)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let items = result
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("items missing: {response}"))?;
    assert!(
        !items.is_empty(),
        "csharp-small Alpha.cs/Beta.cs are clones; expected diagnostics: {response}",
    );
    let first = items
        .first()
        .ok_or_else(|| anyhow!("no diagnostic items: {items:?}"))?;
    assert_eq!(
        first.get("source").and_then(serde_json::Value::as_str),
        Some("codededup"),
        "diagnostic source must be 'codededup': {first}",
    );
    let code = first
        .get("code")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("code missing or not a string: {first}"))?;
    assert!(
        !code.is_empty(),
        "code must carry the cluster id per [LSP-DIAGNOSTICS]: {first}",
    );
    let related = first
        .get("relatedInformation")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("relatedInformation must list other occurrences: {first}"))?;
    assert!(
        !related.is_empty(),
        "Alpha.cs has at least one sibling occurrence in Beta.cs: {first}",
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_embedding_list_models_returns_stub_when_ollama_unreachable() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request_no_params("codededup/embeddingListModels")?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let models = result
        .as_array()
        .ok_or_else(|| anyhow!("models is not an array: {response}"))?;
    assert!(
        models.iter().any(
            |model| model.get("provider_id").and_then(serde_json::Value::as_str) == Some("stub")
        ),
        "stub must always appear: {response}"
    );
    shut_down(child);
    Ok(())
}
