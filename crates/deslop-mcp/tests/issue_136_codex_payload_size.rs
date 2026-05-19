//! Wire-payload size regression for issue #136 ([MCP-RESULT-SIZE-CAP]).
//!
//! Codex's `rmcp_client` crashes its `app-server` when handed a
//! multi-hundred-KB tool response. The mitigation lives in two
//! layers:
//!
//! 1. **`tools/list` is slim.** Every description is ≤200 chars and
//!    the total `tools/list` payload stays under 16 KB. Long-form
//!    rationale belongs in the `deslop://schema` resource — agents
//!    fetch it on demand via `schema-doc`.
//! 2. **`tools/call` results are capped at 200 KB.** The dispatcher
//!    walks any oversize payload, drops clusters from the tail of
//!    the inner array, and stamps the response with `truncated:
//!    true` plus a pointer to the paginated `report-get` tool.
//!
//! Both layers are exercised here against the live LSP→MCP IPC
//! chain — the same wire contract Codex sees in production.

#![cfg(unix)]

use std::{fs, path::Path};

use anyhow::{anyhow, ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture, initialized_mcp, spawn_lsp_and_initialize, structured_content, wait_for_path,
    ChildKillOnDrop, SOCKET_TIMEOUT,
};

/// Sanity guard for the full `tools/list` payload. Picked an order
/// of magnitude under what most JSON-RPC clients tolerate so a
/// future regression that re-inflates a description blows up here
/// instead of silently re-introducing the Codex crash.
const TOOLS_LIST_MAX_BYTES: usize = 16 * 1024;

/// Per-tool description budget. Enforced against the slimmed
/// descriptions in `crates/deslop-mcp/src/tools/mod.rs`.
const TOOL_DESCRIPTION_MAX_CHARS: usize = 200;

/// `tools/list` ships every description ≤200 chars and the full
/// payload ≤16 KB ([MCP-RESULT-SIZE-CAP]).
#[test]
fn tools_list_payload_stays_under_codex_wire_budget() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request("tools/list", &json!({}))?;
    assert_tools_list_within_budget(&response)?;
    assert_every_description_within_budget(&response)?;
    Ok(())
}

/// Asserts the full `tools/list` response serialises to at most
/// `TOOLS_LIST_MAX_BYTES`.
fn assert_tools_list_within_budget(response: &Value) -> Result<()> {
    let serialised = serde_json::to_vec(response)?;
    let size = serialised.len();
    ensure!(
        size <= TOOLS_LIST_MAX_BYTES,
        "issue #136: tools/list must stay within Codex's wire budget; got {size} bytes (cap {TOOLS_LIST_MAX_BYTES})"
    );
    Ok(())
}

/// Asserts each `tools[].description` is ≤200 chars and non-empty.
fn assert_every_description_within_budget(response: &Value) -> Result<()> {
    let tools = response
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("tools/list must return result.tools array"))?;
    ensure!(
        tools.len() == 12,
        "issue #136: deslop ships exactly 12 tools today; tools/list returned {}",
        tools.len()
    );
    for tool in tools {
        check_one_description(tool)?;
    }
    Ok(())
}

/// Per-tool description budget check. Extracted so the assertion
/// failure pins which tool exceeded the cap.
fn check_one_description(tool: &Value) -> Result<()> {
    let name = tool
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unnamed>");
    let description = tool
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tool {name} missing description"))?;
    ensure!(
        !description.is_empty(),
        "tool {name} must have a non-empty description"
    );
    let length = description.chars().count();
    ensure!(
        length <= TOOL_DESCRIPTION_MAX_CHARS,
        "issue #136: tool {name} description must be ≤{TOOL_DESCRIPTION_MAX_CHARS} chars; got {length}: {description}"
    );
    Ok(())
}

/// `find-similar`'s slimmed description still satisfies the issue
/// #113 prevention-first contract (`starts_with`, `PREVENT`,
/// `reuse`, `avoid introducing new clones`).
#[test]
fn find_similar_description_still_leads_with_prevention() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request("tools/list", &json!({}))?;
    let description = find_description(&response, "find-similar")?;
    ensure!(
        description.starts_with("Call BEFORE writing new code"),
        "issue #136: slim find-similar description must still lead with prevention: {description}"
    );
    ensure!(
        description.contains("PREVENT"),
        "issue #136: slim find-similar description must still name PREVENT: {description}"
    );
    ensure!(
        description.contains("reuse"),
        "issue #136: slim find-similar description must still point to reuse: {description}"
    );
    ensure!(
        description.contains("avoid introducing new clones"),
        "issue #136: slim find-similar description must keep the duplication-risk clause: {description}"
    );
    Ok(())
}

/// Pulls one tool's description out of a `tools/list` response.
fn find_description(response: &Value, tool_name: &str) -> Result<String> {
    response
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("tools/list must return result.tools array"))?
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some(tool_name))
        .and_then(|tool| tool.get("description").and_then(Value::as_str))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("tool {tool_name} missing description in tools/list"))
}

/// `top-offenders` on a fabricated wide workspace MUST stay under
/// the 200 KB cap. Inflates the fixture with enough clusters that
/// a naive serialiser would breach the cap, then asserts the wire
/// frame is bounded and carries the truncation marker if it tripped.
#[test]
fn top_offenders_result_capped_at_two_hundred_kilobytes() -> Result<()> {
    let workspace = copied_fixture()?;
    inflate_workspace_with_clones(workspace.path())?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let mut mcp = initialized_mcp(workspace.path())?;
    let _rescan = mcp.request(
        "tools/call",
        &json!({"name": "rescan", "arguments": {"n": 100, "max_occurrences": 5000}}),
    )?;

    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "top-offenders",
            "arguments": {"n": 100, "max_occurrences": 5000}
        }),
    )?;
    let serialised = serde_json::to_vec(&response)?;
    let size = serialised.len();
    let cap = 200 * 1024;
    ensure!(
        size <= cap + 4096,
        "issue #136: top-offenders wire frame must stay within ~200 KB cap; got {size} bytes"
    );
    let structured = structured_content(&response, "top-offenders")?;
    let truncated = structured
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if truncated {
        assert_truncation_marker(&structured)?;
    }
    Ok(())
}

/// Asserts the truncated payload carries the documented marker
/// fields (issue #136 contract — agents reading `truncated: true`
/// must see a human-readable reason plus a pointer to the paginated
/// tool).
fn assert_truncation_marker(structured: &Value) -> Result<()> {
    let reason = structured
        .get("truncated_reason")
        .and_then(Value::as_str)
        .unwrap_or("");
    ensure!(
        reason.contains("MCP wire cap"),
        "truncated_reason must explain the wire cap: {reason}"
    );
    let next_action = structured
        .get("next_action")
        .and_then(Value::as_str)
        .unwrap_or("");
    ensure!(
        next_action.contains("report-get"),
        "next_action must point to the paginated report-get tool: {next_action}"
    );
    let at_bytes = structured
        .get("truncated_at_bytes")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    ensure!(
        at_bytes == 200 * 1024,
        "truncated_at_bytes must match the documented 200 KB cap; got {at_bytes}"
    );
    Ok(())
}

/// Writes ~120 nearly-identical C# files into the workspace so the
/// LSP produces enough clusters for the cap path to engage. Each
/// file is a deep-enough subtree to clear the default `min-nodes`
/// threshold without bloating the test runtime.
fn inflate_workspace_with_clones(root: &Path) -> Result<()> {
    for index in 0..120_u32 {
        let body = format!(
            "namespace Codex{index} {{ public class Probe{index} {{ public int Run(int x, int y, int z) {{ var a = x + y; var b = a * z; var c = b - x; return c + a - b; }} public int Mirror(int x, int y, int z) {{ var a = x + y; var b = a * z; var c = b - x; return c + a - b; }} }} }}\n"
        );
        fs::write(root.join(format!("Codex{index}.cs")), body)?;
    }
    Ok(())
}

/// `resources/templates/list` returns a strict MCP-spec error
/// envelope — `code`, `message`, no extra fields — so Codex's
/// `rmcp_client` doesn't trip on a shape it didn't expect
/// ([MCP-WIRE-FRAMING]).
#[test]
fn resources_templates_list_returns_well_formed_method_not_found() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;
    let mut mcp = initialized_mcp(workspace.path())?;

    let response = mcp.request("resources/templates/list", &json!({}))?;
    assert_method_not_found_envelope(&response)?;
    Ok(())
}

/// Asserts a JSON-RPC error frame matches the MCP 2024-11-05 spec
/// for `MethodNotFound`: `{"jsonrpc":"2.0","id":<n>,"error":{"code":-32601,"message":"..."}}`
/// with no extra top-level fields and no extra error fields beyond
/// the spec-permitted `data`.
fn assert_method_not_found_envelope(response: &Value) -> Result<()> {
    let object = response
        .as_object()
        .ok_or_else(|| anyhow!("response must be a JSON object: {response}"))?;
    ensure!(
        object.get("jsonrpc").and_then(Value::as_str) == Some("2.0"),
        "jsonrpc field must be exactly \"2.0\": {response}"
    );
    ensure!(
        object.get("id").is_some(),
        "id field must be present on error response: {response}"
    );
    let error = object
        .get("error")
        .ok_or_else(|| anyhow!("error field must be present: {response}"))?;
    ensure!(
        error.get("code").and_then(Value::as_i64) == Some(-32_601),
        "code must be -32601 MethodNotFound: {response}"
    );
    let message = error.get("message").and_then(Value::as_str).unwrap_or("");
    ensure!(
        !message.is_empty(),
        "message must be present and non-empty: {response}"
    );
    assert_only_permitted_keys(object, &["jsonrpc", "id", "error"])?;
    let error_object = error
        .as_object()
        .ok_or_else(|| anyhow!("error must be a JSON object: {response}"))?;
    assert_only_permitted_keys(error_object, &["code", "message", "data"])?;
    Ok(())
}

/// Rejects any top-level key that is not in the allow list.
fn assert_only_permitted_keys(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
) -> Result<()> {
    for key in object.keys() {
        ensure!(
            allowed.iter().any(|candidate| candidate == key),
            "unexpected key {key:?} on error envelope; allowed={allowed:?}"
        );
    }
    Ok(())
}
