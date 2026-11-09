//! Request/response helpers shared by the `deslop-mcp` CLI suites:
//! the success-path `tools/call` wrapper, the spawn-and-call combos
//! every argument-validation test repeats, and the JSON-RPC error
//! envelope assertions.

use std::process::Command;

use super::{fixture_root, value_get};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::mcp_child::{spawn_and_init, McpChild};
use super::mcp_wire::{
    ARGUMENTS_FIELD, DUPLICATES_TOOL, ERROR_CODE_POINTER, ERROR_FIELD, LIMIT_PARAM, OFFSET_PARAM,
    ROOT_FLAG, TOOLS_CALL_METHOD, TOOLS_LIST_METHOD, TOOLS_LIST_POINTER, TOTAL_CLUSTERS_POINTER,
};
use super::NAME_FIELD;

/// Magnitude of the JSON-RPC invalid-params error code (the wire code
/// is negative).
const INVALID_PARAMS_CODE_MAGNITUDE: i64 = 32_602;
/// JSON-RPC method-not-found error code.
const METHOD_NOT_FOUND_CODE: i64 = -32_601;
/// Limit for the broadest `duplicates` page the harness requests.
const BROAD_RESULT_LIMIT: u64 = 100;

/// The `CARGO_PKG_VERSION` the version-contract tests expect.
pub fn expected_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Delegates to the shared version-manifest assertion
/// (`deslop-test-support`), binding this crate's expected version.
pub fn assert_version_manifest(value: &Value, name: &str, kind: &str) {
    deslop_test_support::assert_version_manifest(value, name, kind, expected_version());
}

/// Drives `tools/call` for `tool` with `arguments` through the
/// success path, returning the raw JSON-RPC `result` object.
pub fn call_tool(child: &mut McpChild, name: &str, arguments: &Value) -> Result<Value> {
    let response = child.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): name, (ARGUMENTS_FIELD): arguments }),
    )?;
    if response.get(ERROR_FIELD).is_some() {
        return Err(anyhow!("tools/call {name} failed: {response}"));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("missing result in response: {response}"))
}

/// Extracts the `structuredContent` envelope from a successful
/// tools/call result.
pub fn structured_tool_result(result: &Value) -> Result<Value> {
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| anyhow!("missing structuredContent in {result}"))
}

/// Arguments for the broadest `duplicates` page the harness requests.
pub fn broad_page_arguments() -> Value {
    json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): BROAD_RESULT_LIMIT })
}

/// The `total_clusters` of a broad `duplicates` page: the figure every
/// mutation scenario compares before and after an edit.
pub fn broad_cluster_count(child: &mut McpChild) -> Result<u64> {
    let page =
        structured_tool_result(&call_tool(child, DUPLICATES_TOOL, &broad_page_arguments())?)?;
    Ok(value_get(&page, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0))
}

/// Asserts the server still answers `tools/list`, then finishes the
/// child: the liveness check every malformed-input scenario ends with.
pub fn finish_after_tools_list(mut child: McpChild) -> Result<()> {
    let response = child.request(TOOLS_LIST_METHOD, &json!({}))?;
    assert!(value_get(&response, TOOLS_LIST_POINTER)?.is_array());
    let _ = child.finish();
    Ok(())
}

/// Spawns + initializes an MCP child against the read-only fixture, invokes
/// `tool` with `arguments` through the success-path [`call_tool`], and returns
/// the live child alongside the call's `structuredContent` payload. The child
/// is returned so the test can issue follow-up calls and `finish()` it; the
/// payload is the value the test asserts on.
pub fn init_and_tool_payload(tool: &str, arguments: &Value) -> Result<(McpChild, Value)> {
    let mut child = spawn_and_init()?;
    let payload = structured_tool_result(&call_tool(&mut child, tool, arguments)?)?;
    Ok((child, payload))
}

/// Spawns + initializes an MCP child against the read-only fixture and sends a
/// raw `tools/call` request for `tool` with `arguments`, returning the live
/// child and the unwrapped JSON-RPC response frame (no success assertion). Used
/// by tests that assert on the error envelope or on `structuredContent`
/// directly rather than through [`structured_tool_result`].
pub fn init_and_tool_response(tool: &str, arguments: &Value) -> Result<(McpChild, Value)> {
    let mut child = spawn_and_init()?;
    let response = child.request(
        TOOLS_CALL_METHOD,
        &json!({ (NAME_FIELD): tool, (ARGUMENTS_FIELD): arguments }),
    )?;
    Ok((child, response))
}

/// Asserts `response` is the JSON-RPC error frame carrying `code`.
pub fn assert_error_code(response: &Value, code: i64) -> Result<()> {
    assert_eq!(
        value_get(response, ERROR_CODE_POINTER)?.as_i64(),
        Some(code),
        "expected JSON-RPC error code {code}: {response}"
    );
    Ok(())
}

/// Asserts `response` is the JSON-RPC method-not-found error frame.
pub fn assert_method_not_found(response: &Value) -> Result<()> {
    assert_error_code(response, METHOD_NOT_FOUND_CODE)
}

/// Asserts `response` carries the JSON-RPC invalid-params error code. The
/// wire code is negative; [`INVALID_PARAMS_CODE_MAGNITUDE`] holds its
/// magnitude, so the expected value is negated here once for every caller.
pub fn assert_invalid_params(response: &Value) -> Result<()> {
    assert_eq!(
        value_get(response, ERROR_CODE_POINTER)?.as_i64(),
        Some(-INVALID_PARAMS_CODE_MAGNITUDE),
        "expected an invalid-params error envelope: {response}"
    );
    Ok(())
}

/// Asserts the MCP tool `tool` rejects `arguments` with invalid-params.
/// Spawns one child, calls the tool, checks the envelope, and shuts the
/// child down — the whole shape every argument-validation test repeats.
pub fn assert_tool_rejects_invalid_params(tool: &str, arguments: &Value) -> Result<()> {
    let (child, response) = init_and_tool_response(tool, arguments)?;
    assert_invalid_params(&response)?;
    let _ = child.finish();
    Ok(())
}

/// Asserts the raw JSON-RPC `method` rejects `params` with invalid-params.
/// The sibling of [`assert_tool_rejects_invalid_params`] for the methods
/// that are not `tools/call`.
pub fn assert_method_rejects_invalid_params(method: &str, params: &Value) -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request(method, params)?;
    assert_invalid_params(&response)?;
    let _ = child.finish();
    Ok(())
}

/// Runs the binary with `args` alone, asserting a clean exit and a
/// silent stderr, and returns its stdout.
pub fn binary_stdout(args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new(env!("CARGO_BIN_EXE_deslop-mcp"))
        .args(args)
        .output()?;
    assert!(output.status.success(), "status was {}", output.status);
    assert!(output.stderr.is_empty(), "stderr must stay empty");
    Ok(output.stdout)
}

/// Runs the binary over the fixture root with `args` and asserts it
/// refuses to start, naming both streams when it wrongly succeeds.
pub fn assert_binary_refuses_to_start(args: &[&str], reason: &str) -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_deslop-mcp"))
        .arg(ROOT_FLAG)
        .arg(fixture_root())
        .args(args)
        .output()?;
    assert!(
        !output.status.success(),
        "{reason}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
