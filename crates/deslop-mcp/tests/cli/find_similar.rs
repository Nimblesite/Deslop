//! `find-similar` tests: range and snippet inputs, the below-min-nodes
//! floor, schema validation, and the `top_n` fallback.

use std::fs;

use anyhow::Result;
use common::{
    mcp_calls::init_and_tool_response,
    mcp_wire::{
        ALPHA_FILE_NAME, ARGUMENTS_FIELD, CSHARP_LANGUAGE, END_BYTE_FIELD, ERROR_CODE_POINTER,
        FIND_SIMILAR_TOOL, LANGUAGE_FIELD, PATH_FIELD, REQUESTED_TOP_OFFENDERS_COUNT,
        SNIPPET_FIELD, START_BYTE_FIELD, TOOLS_CALL_METHOD,
    },
    value_array, value_get, NAME_FIELD,
};

use crate::common;

use serde_json::json;

/// JSON pointer: the clusters array inside structuredContent.
const STRUCTURED_CLUSTERS_POINTER: &str = "/result/structuredContent/clusters";
/// JSON-RPC LSP-not-running error code.
const LSP_NOT_RUNNING_CODE: i64 = -32_004;

#[test]
fn find_similar_snippet_returns_below_min_nodes_for_tiny_input() -> Result<()> {
    // [MCP-IPC-CLIENT] A snippet smaller than `min_nodes` parses
    // cleanly but produces no fingerprint — the response surfaces
    // an empty `clusters` list with `below_min_nodes: true` per
    // [MCP-TOOL-FINDSIMILAR].
    let (child, response) = init_and_tool_response(
        FIND_SIMILAR_TOOL,
        &json!({ (SNIPPET_FIELD): "int x = 0;", (LANGUAGE_FIELD): CSHARP_LANGUAGE }),
    )?;
    let payload = value_get(&response, "/result/structuredContent")?;
    assert_eq!(
        payload.get("below_min_nodes"),
        Some(&json!(true)),
        "tiny snippet must surface below_min_nodes: {response}",
    );
    assert!(
        payload
            .get("clusters")
            .and_then(serde_json::Value::as_array)
            .is_some_and(Vec::is_empty),
        "below-min-nodes input must return no clusters: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_unsupported_language_yields_error() -> Result<()> {
    // StateFileBackend returns LspNotRunning (-32004) before language validation.
    let (child, response) = init_and_tool_response(
        FIND_SIMILAR_TOOL,
        &json!({ (SNIPPET_FIELD): "fn main() {}", (LANGUAGE_FIELD): "cobol" }),
    )?;
    assert_eq!(
        value_get(&response, ERROR_CODE_POINTER)?.as_i64(),
        Some(LSP_NOT_RUNNING_CODE)
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_requires_exactly_one_input_variant() -> Result<()> {
    common::mcp_calls::assert_tool_rejects_invalid_params(FIND_SIMILAR_TOOL, &json!({}))
}

#[test]
fn find_similar_range_finds_clone_on_alpha() -> Result<()> {
    // [MCP-IPC-CLIENT] find-similar delegates to the live LSP via
    // IPC. Range input on a file already in the corpus must surface
    // its sibling cluster on Beta.cs.
    let mut child = common::mcp_child::spawn_and_init()?;
    // Use a workspace-relative path so the MCP's `resolve_within_root`
    // canonical form lines up with the LSP's pinned workspace root —
    // macOS exposes `/private/var/...` vs `/var/...` for the same
    // tempdir, and only the LSP's view is authoritative.
    let source = fs::read_to_string(child.workspace_root().join(ALPHA_FILE_NAME))?;
    let response = child.request(
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): FIND_SIMILAR_TOOL,
            (ARGUMENTS_FIELD): {
                (PATH_FIELD): ALPHA_FILE_NAME,
                (START_BYTE_FIELD): 0,
                (END_BYTE_FIELD): source.len(),
                "top_n": REQUESTED_TOP_OFFENDERS_COUNT,
            }
        }),
    )?;
    let clusters = value_array(&response, STRUCTURED_CLUSTERS_POINTER)?;
    assert!(
        !clusters.is_empty(),
        "find-similar on Alpha.cs must return at least the Beta sibling cluster: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_with_top_n_zero_falls_back_to_default() -> Result<()> {
    // [MCP-IPC-CLIENT] `top_n: 0` must use the default cap rather
    // than returning an empty list. The find-similar IPC roundtrip
    // surfaces at least the Beta sibling.
    let mut child = common::mcp_child::spawn_and_init()?;
    // Workspace-relative path keeps the MCP↔LSP canonical view
    // aligned (see find_similar_range_finds_clone_on_alpha).
    let source = fs::read_to_string(child.workspace_root().join(ALPHA_FILE_NAME))?;
    let response = child.request(
        TOOLS_CALL_METHOD,
        &json!({
            (NAME_FIELD): FIND_SIMILAR_TOOL,
            (ARGUMENTS_FIELD): { (PATH_FIELD): ALPHA_FILE_NAME, (START_BYTE_FIELD): 0, (END_BYTE_FIELD): source.len(), "top_n": 0 }
        }),
    )?;
    let clusters = value_array(&response, STRUCTURED_CLUSTERS_POINTER)?;
    assert!(
        !clusters.is_empty(),
        "find-similar with top_n=0 must fall back to default and return clusters: {response}",
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn find_similar_snippet_with_empty_source_returns_empty_result() -> Result<()> {
    // [MCP-IPC-CLIENT] find-similar against the live LSP must
    // tolerate an empty snippet — it is below the parser's minimum
    // node floor, so the success-path reply marks `below_min_nodes`
    // and returns no clusters (no error envelope).
    let (child, response) = init_and_tool_response(
        FIND_SIMILAR_TOOL,
        &json!({ (SNIPPET_FIELD): "", (LANGUAGE_FIELD): CSHARP_LANGUAGE }),
    )?;
    assert!(
        response.pointer("/error").is_none(),
        "find-similar with the live LSP must succeed (no JSON-RPC error envelope): {response}"
    );
    assert_eq!(
        value_get(&response, "/result/structuredContent/below_min_nodes")?.as_bool(),
        Some(true),
        "empty snippet must report below_min_nodes=true: {response}"
    );
    let clusters = value_array(&response, STRUCTURED_CLUSTERS_POINTER)?;
    assert!(
        clusters.is_empty(),
        "empty snippet must return no clusters: {response}"
    );
    let _ = child.finish();
    Ok(())
}
