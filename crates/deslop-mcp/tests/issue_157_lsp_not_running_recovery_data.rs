//! [Deslop#157] When the LSP is not running, the `-32004` error must
//! carry structured recovery guidance in its `data` field so an agent
//! can decide whether to retry and where the on-disk fallback lives.
//!
//! Before this fix the error carried only a human message (`data: null`),
//! leaving agents to discover the `.deslop/cache/live-report.json`
//! fallback by accident. The numeric code and the existing human message
//! (socket path + `--root`, per [Deslop#151]) must be preserved verbatim.

#![cfg(unix)]

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};
use tempfile::TempDir;

mod common;
use common::{error_and_message, expected_socket_fragment, initialized_mcp};

#[test]
fn issue_157_lsp_not_running_carries_structured_recovery_data() -> Result<()> {
    let workspace = TempDir::new()?;
    // Intentionally do NOT spawn an LSP. The socket file is absent, so
    // every tool call returns BackendError::LspNotRunning (-32004).
    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 5 } }),
    )?;
    let (error, message) = error_and_message(&response)?;

    // Wire back-compat: numeric code and the [Deslop#151] message are intact.
    ensure!(
        error.get("code").and_then(Value::as_i64) == Some(-32_004),
        "numeric error code must stay -32004 for wire back-compat: {error}"
    );
    let socket_fragment = expected_socket_fragment(workspace.path())?;
    ensure!(
        message.contains(&socket_fragment) && message.contains("--root"),
        "message must still name the socket path and --root ([Deslop#151]): {message}"
    );

    // New: structured recovery payload.
    let data = error
        .get("data")
        .ok_or_else(|| anyhow!("LspNotRunning error must carry a data payload: {error}"))?;
    ensure!(
        data.get("reason").and_then(Value::as_str) == Some("lsp_not_running"),
        "data.reason must be the stable machine-readable id: {data}"
    );
    let retry = data
        .get("retry_after_ms")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("data.retry_after_ms must be a positive integer: {data}"))?;
    ensure!(retry > 0, "retry_after_ms must be positive: {data}");
    ensure!(
        data.get("socket_path")
            .and_then(Value::as_str)
            .is_some_and(under_cache_dir),
        "data.socket_path must name the IPC socket: {data}"
    );
    let fallback = data
        .get("cache_fallback")
        .ok_or_else(|| anyhow!("data.cache_fallback must document the on-disk fallback: {data}"))?;
    ensure!(
        fallback.is_object(),
        "cache_fallback must be a structured object agents can dispatch on: {fallback}"
    );
    ensure!(
        fallback
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.ends_with("live-report.json") && under_cache_dir(path)),
        "cache_fallback.path must point at .deslop/cache/live-report.json: {fallback}"
    );
    Ok(())
}

/// True when `path` names an artefact inside a workspace's
/// `.deslop/cache` directory ([OUTPUT-DIR]). Normalises separators so
/// the assertion holds on the Windows backslash convention too.
fn under_cache_dir(path: &str) -> bool {
    path.replace('\\', "/").contains("/.deslop/cache/")
}
