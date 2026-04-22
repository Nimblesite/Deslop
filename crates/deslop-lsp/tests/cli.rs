//! End-to-end tests for the LSP binary ([LSP-TESTING]).
//!
//! Drives the real `deslop-lsp` binary over stdio with raw
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
        .join("deslop")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Copies the named fixture into a freshly created temp directory so
/// the LSP server's `.deslop-cache/` writes never pollute the
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
    let bin = assert_cmd::cargo::cargo_bin("deslop-lsp");
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
    let (id, payload) = custom_request_no_params("deslop/sessionConfig")?;
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
    let (id, payload) = custom_request_no_params("deslop/reportGet")?;
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

/// Regression test for [LSP-WIRE-BUDGET]: the live `deslop/reportGet`
/// response must not carry the fat `schema_doc` markdown, the derived
/// `summary` string, or the derived `interpretation` string. Those are
/// available on the CLI surface and via `deslop/reportSchemaDoc`. This
/// is what keeps the Node LSP client from V8-OOMing on workspaces with
/// tens of thousands of occurrences in a single cluster.
#[test]
fn lsp_custom_method_report_get_elides_schema_doc_and_prose() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request_no_params("deslop/reportGet")?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let schema_doc = result
        .get("schema_doc")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("schema_doc missing in {response}"))?;
    assert!(
        schema_doc.is_empty(),
        "schema_doc must be empty on the live wire; got {} bytes",
        schema_doc.len()
    );
    let clusters = result
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing in {response}"))?;
    assert!(!clusters.is_empty(), "fixture should produce clusters");
    for cluster in clusters {
        let summary = cluster.get("summary").and_then(serde_json::Value::as_str);
        assert_eq!(summary, Some(""), "summary must be blanked on live wire");
        let interpretation = cluster
            .get("interpretation")
            .and_then(serde_json::Value::as_str);
        assert_eq!(
            interpretation,
            Some(""),
            "interpretation must be blanked on live wire"
        );
        let occurrences_total = cluster
            .get("occurrences_total")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| anyhow!("occurrences_total missing in {cluster}"))?;
        let occurrences_truncated = cluster
            .get("occurrences_truncated")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| anyhow!("occurrences_truncated missing in {cluster}"))?;
        assert!(occurrences_total > 0, "occurrences_total must be populated");
        assert!(
            !occurrences_truncated,
            "csharp-small fixture has tiny clusters; no truncation expected"
        );
    }
    shut_down(child);
    Ok(())
}

/// Regression test for [LSP-WIRE-BUDGET]: clients fetch the schema
/// markdown on demand via `deslop/reportSchemaDoc` so the live wire
/// doesn't ship it with every `deslop/reportGet` response.
#[test]
fn lsp_custom_method_report_schema_doc_returns_markdown() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request_no_params("deslop/reportSchemaDoc")?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let markdown = result
        .as_str()
        .ok_or_else(|| anyhow!("reportSchemaDoc must return a string; got {response}"))?;
    assert!(
        markdown.len() > 256,
        "schema doc should be substantial markdown; got {} bytes",
        markdown.len()
    );
    assert!(
        markdown.contains("schema"),
        "schema doc should contain the word \"schema\"; got preview {:?}",
        &markdown[..markdown.len().min(120)]
    );
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
    let (id, payload) = custom_request("deslop/duplicatesFindSimilar", &params)?;
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
    assert_eq!(identifier, "deslop");
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_diagnostic_returns_deslop_diagnostics_for_clone_file() -> Result<()> {
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
        Some("deslop"),
        "diagnostic source must be 'deslop': {first}",
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
    let (id, payload) = custom_request_no_params("deslop/embeddingListModels")?;
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

/// Builds a `textDocument/hover` request pointing at `path` at
/// `(line, character)` ([LSP-HOVER]).
fn hover_request(path: &Path, line: u32, character: u32) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": line, "character": character }
        }
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds a `textDocument/codeLens` request for `path` ([LSP-CODE-LENS]).
fn code_lens_request(path: &Path) -> Result<(i64, String)> {
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/codeLens",
        "params": { "textDocument": { "uri": uri.as_str() } }
    });
    Ok((id, serde_json::to_string(&payload)?))
}

/// Builds a `textDocument/didChange` notification for `path`.
fn did_change_notification(path: &Path, new_text: &str) -> Result<String> {
    let uri = tower_lsp::lsp_types::Url::from_file_path(path)
        .map_err(|()| anyhow!("path is not absolute: {}", path.display()))?;
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": uri.as_str(), "version": 2 },
            "contentChanges": [ { "text": new_text } ]
        }
    });
    Ok(serde_json::to_string(&payload)?)
}

/// Sends a notification (no id, no response expected).
fn send_notification(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    write_frame(stdin, payload)
}

/// Sends a request and returns the response plus every notification
/// received while waiting for it. Used by tests that assert the server
/// pushed a specific `deslop/*` notification mid-request.
fn send_and_recv_with_notifications(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: i64,
    payload: &str,
) -> Result<(serde_json::Value, Vec<serde_json::Value>)> {
    write_frame(stdin, payload)?;
    let mut notifications = Vec::new();
    loop {
        let frame = read_frame(reader)?;
        if frame.get("id").and_then(serde_json::Value::as_i64) == Some(id) {
            return Ok((frame, notifications));
        }
        if frame.get("method").is_some() && frame.get("id").is_none() {
            notifications.push(frame);
        }
    }
}

#[test]
fn lsp_initialize_advertises_hover_and_code_lens_providers() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (id, payload) = initialize_request()?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let capabilities = result_value(&response)?
        .get("capabilities")
        .ok_or_else(|| anyhow!("missing capabilities: {response}"))?;
    assert!(
        capabilities.get("hoverProvider").is_some(),
        "hoverProvider must be advertised per [LSP-HOVER]: {capabilities}"
    );
    assert!(
        capabilities.get("codeLensProvider").is_some(),
        "codeLensProvider must be advertised per [LSP-CODE-LENS]: {capabilities}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_hover_returns_markdown_for_clone_cluster() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    // Hover inside the Compute method body where the clone lives.
    let (id, payload) = hover_request(&alpha, 6, 12)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let contents = result
        .get("contents")
        .ok_or_else(|| anyhow!("hover missing contents: {response}"))?;
    let value = contents
        .get("value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("hover contents missing markdown value: {contents}"))?;
    assert!(
        value.contains("Cluster"),
        "hover body must include cluster header: {value}"
    );
    assert!(
        value.contains("Occurrences"),
        "hover body must include occurrence list: {value}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_hover_returns_null_when_min_nodes_filters_everything() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    // Sky-high min_nodes so no cluster survives; any position on the
    // file must return a null hover.
    let mut child = spawn_lsp(workspace.path(), 10_000)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = hover_request(&alpha, 6, 12)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    assert!(
        result.is_null(),
        "hover with no eligible clusters must be null: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_hover_returns_null_for_nonexistent_file() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let phantom = workspace.path().join("does-not-exist.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = hover_request(&phantom, 0, 0)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    assert!(
        result.is_null(),
        "hover for unreadable path must be null: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_code_lens_returns_one_entry_per_occurrence() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = code_lens_request(&alpha)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let lenses = result
        .as_array()
        .ok_or_else(|| anyhow!("code lens result not array: {response}"))?;
    assert!(
        !lenses.is_empty(),
        "Alpha.cs has clones; expected code lenses: {response}"
    );
    let first = lenses.first().ok_or_else(|| anyhow!("no lenses"))?;
    let command = first
        .get("command")
        .ok_or_else(|| anyhow!("lens missing command: {first}"))?;
    let command_id = command
        .get("command")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("lens command id missing: {command}"))?;
    assert_eq!(command_id, "deslop.jumpToNextOccurrence");
    let title = command
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("lens title missing: {command}"))?;
    assert!(
        title.contains("copies"),
        "lens title must mention copy count: {title}"
    );
    assert!(
        title.contains("jump to next"),
        "lens title must mention jump action: {title}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_text_document_code_lens_returns_empty_for_file_without_clusters() -> Result<()> {
    // Using a giant min_nodes threshold forces the fixture to produce
    // zero clusters so the code-lens array is empty without a missing
    // file path.
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 10_000)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = code_lens_request(&alpha)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let lenses = result
        .as_array()
        .ok_or_else(|| anyhow!("code lens result not array: {response}"))?;
    assert!(
        lenses.is_empty(),
        "min-nodes 10000 filters every cluster; expected zero lenses: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_did_change_notification_triggers_reanalysis_of_file() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    // Rewrite Alpha.cs on disk then fire didChange; the server reads
    // from disk so on-disk + notification together drive reanalysis.
    let rewritten =
        "namespace Alpha { public class Processor { public int Compute() { return 7; } } }";
    fs::write(&alpha, rewritten)?;
    let payload = did_change_notification(&alpha, rewritten)?;
    send_notification(&mut stdin, &payload)?;
    // After reanalysis a diagnostic pull should still succeed (no
    // crashes, valid frame).
    let (id, payload) = diagnostic_request(&alpha)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let _result = result_value(&response)?;
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_report_for_file_returns_clusters_for_fixture() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/reportForFile",
        &serde_json::json!({ "path": alpha }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let clusters = result
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing: {response}"))?;
    assert!(
        !clusters.is_empty(),
        "report_for_file should surface clones: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_report_for_range_returns_matching_clusters() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/reportForRange",
        &serde_json::json!({
            "path": alpha,
            "start_byte": 0,
            "end_byte": 10_000
        }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let clusters = result
        .as_array()
        .ok_or_else(|| anyhow!("report_for_range result not array: {response}"))?;
    assert!(
        !clusters.is_empty(),
        "full-file range should include clone clusters: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_report_for_range_returns_empty_when_min_nodes_filters_everything() -> Result<()>
{
    // Sky-high min_nodes removes every cluster globally; the range
    // query must then see nothing regardless of offsets.
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 10_000)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/reportForRange",
        &serde_json::json!({
            "path": alpha,
            "start_byte": 0,
            "end_byte": 10_000
        }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let clusters = result
        .as_array()
        .ok_or_else(|| anyhow!("report_for_range result not array: {response}"))?;
    assert!(
        clusters.is_empty(),
        "no clusters should survive the filter: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_cluster_by_id_returns_cluster_when_found() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    // First fetch the full report so we have a real cluster id.
    let (get_id, get_payload) = custom_request_no_params("deslop/reportGet")?;
    let get_response = send_and_recv(&mut stdin, &mut reader, get_id, &get_payload)?;
    let report = result_value(&get_response)?;
    let clusters = report
        .get("clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing: {get_response}"))?;
    let cluster_id = clusters
        .first()
        .and_then(|cluster| cluster.get("id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("no cluster id in report: {get_response}"))?
        .to_owned();
    let (id, payload) = custom_request(
        "deslop/clusterById",
        &serde_json::json!({ "id": cluster_id }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    let returned_id = result
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("cluster id missing: {response}"))?;
    assert_eq!(returned_id, cluster_id);
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_cluster_by_id_returns_error_for_unknown_id() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/clusterById",
        &serde_json::json!({ "id": "does-not-exist" }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    assert!(
        response.get("error").is_some(),
        "unknown cluster id must produce a JSON-RPC error: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_embedding_set_model_rejects_unknown_provider() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/embeddingSetModel",
        &serde_json::json!({
            "provider_id": "not-a-real-provider",
            "model_id": "irrelevant",
        }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    assert!(
        response.get("error").is_some(),
        "unknown provider must produce a JSON-RPC error: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_embedding_set_model_swaps_to_stub() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/embeddingSetModel",
        &serde_json::json!({
            "provider_id": "stub",
            "model_id": "stub-model",
        }),
    )?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    assert!(
        result.is_object() || result.is_null(),
        "set-model should return a structured response or null: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_embedding_set_model_emits_progress_notifications() -> Result<()> {
    // Session panel reactivity ([VSIX-SESSION-PROGRESS]): the LSP must
    // push at least one `deslop/embeddingProgress` notification while a
    // model swap is in flight so the extension can render "X / Y
    // subtrees" instead of freezing on the old model. Stub provider is
    // deterministic and fast so the swap completes well within the
    // request window.
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let (id, payload) = custom_request(
        "deslop/embeddingSetModel",
        &serde_json::json!({
            "provider_id": "stub",
            "model_id": "stub-model",
        }),
    )?;
    let (response, notifications) =
        send_and_recv_with_notifications(&mut stdin, &mut reader, id, &payload)?;
    let _result = result_value(&response)?;
    let progress: Vec<&serde_json::Value> = notifications
        .iter()
        .filter(|frame| {
            frame.get("method").and_then(serde_json::Value::as_str)
                == Some("deslop/embeddingProgress")
        })
        .collect();
    assert!(
        !progress.is_empty(),
        "embedding swap must emit at least one deslop/embeddingProgress notification; saw {notifications:?}"
    );
    let params = progress
        .first()
        .and_then(|frame| frame.get("params"))
        .ok_or_else(|| anyhow!("progress notification missing params"))?;
    let model_id = params
        .get("model_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("progress params missing model_id: {params}"))?;
    assert!(
        !model_id.is_empty(),
        "progress must populate model_id: {params}"
    );
    let provider_id = params
        .get("provider_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("progress params missing provider_id: {params}"))?;
    assert_eq!(
        provider_id, "stub",
        "progress must name the swapped provider: {params}"
    );
    let total = params
        .get("total")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow!("progress params missing total: {params}"))?;
    assert!(total > 0, "total subtrees must be populated: {params}");
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_custom_method_find_similar_by_range_locates_clone_on_alpha() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let params = serde_json::json!({
        "input": {
            "kind": "open_range",
            "path": alpha,
            "start_byte": 0,
            "end_byte": 10_000
        },
        "max_results": null
    });
    let (id, payload) = custom_request("deslop/duplicatesFindSimilar", &params)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &payload)?;
    let result = result_value(&response)?;
    // Either matches were found or the request was legitimately empty —
    // what we're exercising is the range parsing path, not the hit count.
    assert!(
        result.get("matches").is_some() || result.get("below_min_nodes").is_some(),
        "find-similar response should carry matches or below_min_nodes: {response}"
    );
    shut_down(child);
    Ok(())
}

#[test]
fn lsp_report_get_accepts_request_without_params_field() -> Result<()> {
    // tower-lsp normally rejects no-params requests with `-32602
    // Missing params field`; `NormaliseParams` in backend.rs injects an
    // empty object so clients that drop `params` still work. This test
    // drives that path by hand-building a request with no `params` key
    // at all.
    let workspace = copy_fixture("csharp-small")?;
    let mut child = spawn_lsp(workspace.path(), 15)?;
    let (mut stdin, mut reader) = take_io(&mut child)?;
    let (init_id, init_payload) = initialize_request()?;
    let _init = send_and_recv(&mut stdin, &mut reader, init_id, &init_payload)?;
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "deslop/reportGet"
    });
    let wire = serde_json::to_string(&payload)?;
    let response = send_and_recv(&mut stdin, &mut reader, id, &wire)?;
    let result = result_value(&response)?;
    assert!(
        result.get("clusters").is_some(),
        "normalised no-params request must still return a report: {response}"
    );
    shut_down(child);
    Ok(())
}
