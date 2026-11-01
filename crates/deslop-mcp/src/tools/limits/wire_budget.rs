//! [MCP-RESULT-SIZE-CAP] Request identifiers consume the complete response budget.

use super::{
    cap_tool_result_for_request, tests::near_budget_cluster_page, MAX_TOOL_RESULT_BYTES,
    REQUEST_ID_BUDGET_MESSAGE,
};
use crate::{
    protocol::{jsonrpc_response_ok, ErrorCode, RequestId},
    tools::wrap_tool_result,
};
use anyhow::{Context, Result};
use serde_json::Value;

const TOOL: &str = "duplicates";
const NUMERIC_ID: RequestId = RequestId::Number(1);
const ESCAPED_ID_FRAGMENT: &str = "\"\\\n";
const ESCAPED_ID_REPEATS: usize = 8_000;
const FRAME_NEWLINE_BYTES: usize = 1;

#[test]
fn escaped_identifier_uses_its_actual_serialized_response_budget() -> Result<()> {
    let id = RequestId::String(ESCAPED_ID_FRAGMENT.repeat(ESCAPED_ID_REPEATS));
    let numeric = cap_tool_result_for_request(TOOL, near_budget_cluster_page()?, &NUMERIC_ID)
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let escaped = cap_tool_result_for_request(TOOL, near_budget_cluster_page()?, &id)
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let numeric_rows = returned_rows(&numeric)?;
    let escaped_rows = returned_rows(&escaped)?;
    assert!(escaped_rows > 0 && escaped_rows < numeric_rows);
    let response = jsonrpc_response_ok(id.clone(), wrap_tool_result(&escaped));
    let bytes = serde_json::to_vec(&response)?;
    assert!(bytes.len() + FRAME_NEWLINE_BYTES <= MAX_TOOL_RESULT_BYTES);
    assert_eq!(
        serde_json::to_value(response.id)?,
        serde_json::to_value(id)?
    );
    assert_cursor_and_text(&escaped, &response.result)?;
    Ok(())
}

#[test]
fn identifier_that_leaves_no_room_for_a_tool_result_returns_an_error() -> Result<()> {
    const IDENTIFIER_CHARACTER: &str = "a";
    let id = RequestId::String(IDENTIFIER_CHARACTER.repeat(MAX_TOOL_RESULT_BYTES));
    let error = cap_tool_result_for_request(TOOL, near_budget_cluster_page()?, &id)
        .err()
        .context("an oversized identifier cannot yield a capped success frame")?;
    assert_eq!(error.code, ErrorCode::InvalidParams as i32);
    assert_eq!(error.message, REQUEST_ID_BUDGET_MESSAGE);
    Ok(())
}

fn returned_rows(payload: &Value) -> Result<usize> {
    Ok(payload
        .get("clusters")
        .and_then(Value::as_array)
        .context("cluster page")?
        .len())
}

fn assert_cursor_and_text(payload: &Value, result: &Value) -> Result<()> {
    let returned = serde_json::to_value(returned_rows(payload)?)?;
    assert_eq!(payload.pointer("/page/returned"), Some(&returned));
    assert_eq!(payload.pointer("/page/next_offset"), Some(&returned));
    assert_eq!(payload.pointer("/page/has_more"), Some(&Value::Bool(true)));
    assert_eq!(result.get("structuredContent"), Some(payload));
    let text = result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .context("text fallback")?;
    assert_eq!(serde_json::from_str::<Value>(text)?, *payload);
    Ok(())
}
