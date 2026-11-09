//! Cross-tool argument-validation tests: `tools/call` envelope shape
//! and required-argument enforcement.

use anyhow::Result;
use common::{
    mcp_calls::{
        assert_method_not_found, assert_method_rejects_invalid_params,
        assert_tool_rejects_invalid_params, init_and_tool_response,
    },
    mcp_wire::{ARGUMENTS_FIELD, COMPARE_PAIR_TOOL, PATH_FIELD, TOOLS_CALL_METHOD},
};

use crate::common;

use serde_json::json;

/// JSON field: the left endpoint of a `compare-pair` call.
const LEFT_ENDPOINT_FIELD: &str = "left";
/// A tool name no registry will ever carry.
const BOGUS_TOOL_NAME: &str = "bogus-tool";

#[test]
fn tools_call_missing_name_returns_invalid_params() -> Result<()> {
    assert_method_rejects_invalid_params(TOOLS_CALL_METHOD, &json!({ (ARGUMENTS_FIELD): {} }))
}

#[test]
fn tools_call_unknown_tool_returns_method_not_found_error() -> Result<()> {
    let (child, response) = init_and_tool_response(BOGUS_TOOL_NAME, &json!({}))?;
    assert_method_not_found(&response)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_missing_uri_returns_invalid_params() -> Result<()> {
    assert_method_rejects_invalid_params("resources/read", &json!({}))
}

#[test]
fn tool_missing_required_string_arg_returns_invalid_params() -> Result<()> {
    // compare-pair needs both endpoints — omit them.
    assert_tool_rejects_invalid_params(COMPARE_PAIR_TOOL, &json!({}))
}

#[test]
fn tool_missing_required_integer_arg_returns_invalid_params() -> Result<()> {
    // compare-pair needs left AND right — provide only one.
    assert_tool_rejects_invalid_params(
        COMPARE_PAIR_TOOL,
        &json!({ (LEFT_ENDPOINT_FIELD): { (PATH_FIELD): "Alpha.cs" } }),
    )
}
