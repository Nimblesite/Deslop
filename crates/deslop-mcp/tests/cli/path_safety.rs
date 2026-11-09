//! Workspace path-resolution safety tests: outside-root rejection,
//! relative paths inside the workspace, and phantom-leaf handling
//! ([MCP-SAFETY]).

use std::fs;

use anyhow::Result;
use common::{
    mcp_calls::{assert_error_code, init_and_tool_payload, init_and_tool_response},
    mcp_wire::{CLUSTERS_POINTER, DUPLICATES_TOOL, END_BYTE_FIELD, PATH_FIELD, START_BYTE_FIELD},
    value_get,
};

use crate::common;

use serde_json::json;
use tempfile::TempDir;

/// JSON-RPC internal/resource-error code the outside-root rejection
/// surfaces.
const OUTSIDE_ROOT_ERROR_CODE: i64 = -32_003;
/// Byte span that resolves without matching anything.
const NOMINAL_RANGE_END: u64 = 1;
/// Source planted outside the workspace root.
const OUTSIDE_FILE_BODY: &str = "namespace E { class X {} }";

#[test]
fn path_outside_root_is_rejected() -> Result<()> {
    let outside = TempDir::new()?;
    let outside_file = outside.path().join("Evil.cs");
    fs::write(&outside_file, OUTSIDE_FILE_BODY)?;
    let mut child = common::mcp_child::spawn_and_init()?;
    let response = child.request(
        "tools/call",
        &json!({
            "name": DUPLICATES_TOOL,
            "arguments": { (PATH_FIELD): outside_file }
        }),
    )?;
    assert_error_code(&response, OUTSIDE_ROOT_ERROR_CODE)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn relative_path_insideworkspace_is_accepted() -> Result<()> {
    let (child, payload) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (PATH_FIELD): "./Alpha.cs",
            (START_BYTE_FIELD): 0,
            (END_BYTE_FIELD): NOMINAL_RANGE_END,
        }),
    )?;
    assert!(
        value_get(&payload, "/total_clusters")?.is_u64(),
        "a workspace-relative scope path must resolve, not error: {payload}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn report_for_file_accepts_nonexistent_leaf_but_resolves_parent() -> Result<()> {
    // Query a file that doesn't exist but whose *parent* (the scan
    // root) does. This exercises safety::canonicalise_best_effort's
    // nonexistent-leaf branch, returning an empty cluster set.
    let (child, payload) =
        init_and_tool_payload(DUPLICATES_TOOL, &json!({ (PATH_FIELD): "NeverCreated.cs" }))?;
    let clusters = value_get(&payload, CLUSTERS_POINTER)?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "phantom leaf must resolve under root and return no clusters"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn path_in_nonexistent_subdirectory_is_rejected_as_io_failure() -> Result<()> {
    let (child, response) = init_and_tool_response(
        DUPLICATES_TOOL,
        &json!({ (PATH_FIELD): "no/such/dir/Phantom.cs" }),
    )?;
    assert!(
        response.get("error").is_some(),
        "nonexistent parent directory must surface an error: {response}"
    );
    let _ = child.finish();
    Ok(())
}
