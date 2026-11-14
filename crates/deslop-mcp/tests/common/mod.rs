//! Shared MCP+LSP integration test helpers.
//!
//! Each integration test file is a separate binary, so cross-file reuse
//! is wired via `mod common;` declarations rather than `pub use`. Keeps
//! the per-test files small and prevents duplicated handshake / spawn
//! plumbing across LSP-backed E2E suites.
//!
//! Platform-neutral on purpose: the TCP transport suite
//! (`tcp_transport.rs`, [LIVE-IPC-TCP]) runs on every platform,
//! including the Windows CI check leg.

#![allow(dead_code)]

/// The per-cluster `language` label contract over `duplicates`,
/// shared by every language whose label regressed.
pub mod language_label;
/// Spawning a real `deslop-lsp` and waiting for its IPC endpoint.
pub mod lsp_child;
/// Request/response and error-envelope helpers for the CLI suites.
pub mod mcp_calls;
/// The `deslop-mcp` child-process harness for the CLI suites.
pub mod mcp_child;
/// `duplicates`-page shape helpers for the CLI suites.
pub mod mcp_pages;
/// Wire-level names and knobs shared by the CLI suites.
pub mod mcp_wire;
/// Pid plumbing for the orphan-exit suite.
#[cfg(unix)]
pub mod pids;
/// Newline-delimited JSON-RPC over a child's stdio.
pub mod rpc;
/// A scripted stand-in for an LSP from another Deslop build.
#[cfg(unix)]
pub mod stub_lsp;

use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use anyhow::{anyhow, ensure, Context, Result};
use lsp_child::drain_in_background;
pub use lsp_child::{
    spawn_lsp_and_wait_for_socket, spawn_lsp_with_args, wait_for_path, ChildKillOnDrop,
};
#[cfg(unix)]
pub use pids::{
    pid_exists, read_mcp_pid, terminate_pid, wait_for_pid_exit, KILLABLE_PARENT_SCRIPT,
};
use rpc::StdioRpc;
use serde_json::{json, Value};
use tempfile::TempDir;

/// Maximum wait for the LSP to create its IPC socket / state file.
pub const SOCKET_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Copies the C# MCP fixture into a writable temp dir so the LSP can
/// write to `.deslop/cache/`.
pub fn copied_fixture() -> Result<TempDir> {
    copied_fixture_named("csharp-mcp")
}

/// Copies the named fixture directory into a writable temp dir so the
/// LSP can write to `.deslop/cache/`. Looks in this crate's
/// `tests/fixtures/<name>` first, then falls back to the workspace-shared
/// root under `crates/deslop/tests/fixtures/<name>` so MCP wire tests
/// reuse the refactor fixtures instead of duplicating them.
pub fn copied_fixture_named(name: &str) -> Result<TempDir> {
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let src = if local.exists() {
        local
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../deslop/tests/fixtures")
            .join(name)
    };
    let dst = TempDir::new()?;
    deslop_test_support::copy_tree(&src, dst.path())?;
    let cache = dst.path().join(".deslop/cache");
    if cache.exists() {
        fs::remove_dir_all(&cache).context("clear cache dir")?;
    }
    Ok(dst)
}

/// Spawned MCP child + an id-tracked JSON-RPC request loop.
pub struct McpHandle {
    child: Child,
    rpc: StdioRpc,
}

impl McpHandle {
    /// Spawns the `deslop-mcp` binary against `root` over stdio.
    pub fn spawn(root: &Path) -> Result<Self> {
        let bin = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut command = Command::new(bin);
        let _configured = command.arg("--root").arg(root);
        Self::from_command(command)
    }

    /// Spawns the MCP binary from `working_directory` with `root_argument`.
    ///
    /// The explicit textual root is required by the wrong-root regression:
    /// `.` must resolve against the process working directory before the
    /// server binds its live LSP session ([MCP-IPC-DISCOVERY]).
    pub fn spawn_with_root_argument(working_directory: &Path, root_argument: &str) -> Result<Self> {
        let bin = env!("CARGO_BIN_EXE_deslop-mcp");
        let mut command = Command::new(bin);
        let _configured = command
            .arg("--root")
            .arg(root_argument)
            .current_dir(working_directory);
        Self::from_command(command)
    }

    fn from_command(mut command: Command) -> Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn deslop-mcp")?;
        let rpc = StdioRpc::take(&mut child)?;
        // The MCP logs to stderr and nothing here reads it; drain it so
        // the pipe can never fill and stall a response
        // ([`drain_in_background`]).
        let stderr = child.stderr.take().context("mcp stderr")?;
        drain_in_background(stderr);
        Ok(Self { child, rpc })
    }

    /// Sends a JSON-RPC request and reads the matching response,
    /// skipping any pushed notifications in between.
    pub fn request(&mut self, method: &str, params: &Value) -> Result<Value> {
        self.rpc.request(method, params)
    }
}

impl Drop for McpHandle {
    fn drop(&mut self) {
        let _kill = self.child.kill();
        let _wait = self.child.wait();
    }
}

/// Workspace-relative path of the live-report state file the LSP publishes.
pub const LIVE_REPORT_STATE_FILE: &str = ".deslop/cache/live-report.json";

/// Waits for the LSP to publish its live-report state file under `workspace`,
/// then returns an initialized MCP handle — the standard preamble shared by
/// every MCP-over-live-LSP test.
pub fn wait_for_state_then_init_mcp(workspace: &Path) -> Result<McpHandle> {
    let state_file = workspace.join(LIVE_REPORT_STATE_FILE);
    wait_for_path(&state_file, SOCKET_TIMEOUT).context("wait for state file")?;
    initialized_mcp(workspace)
}

/// Spawns + initializes an MCP child against `root`.
pub fn initialized_mcp(root: &Path) -> Result<McpHandle> {
    let mut mcp = McpHandle::spawn(root)?;
    initialize_mcp(&mut mcp)?;
    Ok(mcp)
}

/// Performs the MCP `initialize` request for an already-spawned client.
pub fn initialize_mcp(mcp: &mut McpHandle) -> Result<()> {
    let response = mcp.rpc.initialize("phase5-e2e")?;
    ensure!(
        response.get("error").is_none(),
        "MCP initialize failed: {response}"
    );
    Ok(())
}

/// Drives `tools/call` for `tool` with `arguments` and returns its
/// `structuredContent` envelope, erroring if the call failed.
pub fn call_tool(mcp: &mut McpHandle, tool: &str, arguments: &Value) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({ "name": tool, "arguments": arguments }),
    )?;
    structured_content(&response, tool)
}

/// Requests one `duplicates` summary page of `limit` clusters, returning
/// the raw JSON-RPC response so an error frame stays inspectable.
pub fn request_duplicates_summary(mcp: &mut McpHandle, limit: u64) -> Result<Value> {
    mcp.request(
        "tools/call",
        &json!({
            "name": "duplicates",
            "arguments": { "offset": 0, "limit": limit, "detail": "summary" }
        }),
    )
}

/// Extracts the `structuredContent` envelope from a successful tools/call.
pub fn structured_content(response: &Value, tool: &str) -> Result<Value> {
    ensure!(
        response.get("error").is_none(),
        "{tool} must succeed when the LSP is live, got: {response}"
    );
    response
        .pointer("/result/structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("response missing structuredContent: {response}"))
}

/// Returns the cluster ids on a `duplicates` page in stable order.
pub fn cluster_ids(page: &Value) -> Vec<String> {
    page.get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| cluster.get("id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Calls the `rescan` MCP tool and returns the structured payload.
pub fn rescan_call(mcp: &mut McpHandle, paths: &[String]) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": {
                "paths": paths,
                "limit": 8,
            }
        }),
    )?;
    structured_content(&response, "rescan")
}

/// Reads `field` off `value` as an unsigned integer. The error names
/// the payload that lacked it, so a missing field is diagnosable
/// without the caller re-formatting the response.
pub fn u64_field(value: &Value, field: &str) -> Result<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("{field} missing or not a number in {value}"))
}

/// Reads `field` off `value` as a string.
pub fn str_field<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{field} missing or not a string in {value}"))
}

/// Reads `field` off `value` as an array.
pub fn array_field<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("{field} missing or not an array in {value}"))
}

/// `name` key, carried by both tool descriptors and model rows.
pub const NAME_FIELD: &str = "name";
/// `provider_id` row key in an embedding-model listing.
pub const PROVIDER_ID_FIELD: &str = "provider_id";
/// `models[]` key inside a `session` list-embedding-models result.
pub const MODELS_FIELD: &str = "models";
/// The deterministic BLAKE3 test provider ([REMOVE-STUB]). It is test
/// infrastructure and must never appear in a production payload.
pub const STUB_PROVIDER: &str = "stub";
/// Row keys the retired twelve-tool surface leaked out of Ollama's raw
/// manifest. [MCP-IPC-CLIENT] keeps them off the wire for good.
pub const LEGACY_EMBEDDING_MODEL_KEYS: [&str; 5] = [
    NAME_FIELD,
    "bare_id",
    "digest",
    "size_bytes",
    "is_embedding_model",
];

/// Asserts an embedding-model listing is a production payload: it never
/// advertises the deterministic test stub, and no row carries a legacy
/// key ([MCP-IPC-CLIENT], [REMOVE-STUB]).
///
/// # Errors
///
/// Returns an error naming the offending row, and the legacy key it
/// carried when that is the failure.
pub fn assert_production_embedding_models(models: &[Value]) -> Result<()> {
    for model in models {
        ensure!(
            model.get(PROVIDER_ID_FIELD) != Some(&json!(STUB_PROVIDER)),
            "list-embedding-models must never include the stub provider: {model}"
        );
        for legacy_key in LEGACY_EMBEDDING_MODEL_KEYS {
            ensure!(
                model.get(legacy_key).is_none(),
                "model row must not expose legacy key {legacy_key}: {model}"
            );
        }
    }
    Ok(())
}

/// Resolves a JSON pointer in `value`, erroring when the path is absent.
pub fn value_get(value: &Value, pointer: &str) -> Result<Value> {
    value
        .pointer(pointer)
        .cloned()
        .ok_or_else(|| anyhow!("pointer {pointer} not found in {value}"))
}

/// Resolves a JSON pointer in `value` and clones the array it points at,
/// erroring when the pointer is absent or does not name an array.
pub fn value_array(value: &Value, pointer: &str) -> Result<Vec<Value>> {
    value_get(value, pointer)?
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("pointer {pointer} is not an array in {value}"))
}

/// Extracts the `/error` frame and its string `message` from a JSON-RPC
/// error response, erroring if either is absent.
pub fn error_and_message(response: &Value) -> Result<(Value, String)> {
    let error = response
        .pointer("/error")
        .ok_or_else(|| anyhow!("expected error frame, got: {response}"))?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("error missing string message: {error}"))?;
    Ok((error.clone(), message.to_owned()))
}

/// Canonicalises `workspace` and returns the absolute
/// `.deslop/cache/deslop.sock` path string the MCP backend connects to.
pub fn expected_socket_fragment(workspace: &Path) -> Result<String> {
    let canonical = std::fs::canonicalize(workspace)?;
    Ok(canonical
        .join(".deslop/cache")
        .join("deslop.sock")
        .display()
        .to_string())
}

/// Read-only path to the bundled `csharp-mcp` MCP fixture template.
pub fn fixture_root() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/csharp-mcp"
    ))
}
