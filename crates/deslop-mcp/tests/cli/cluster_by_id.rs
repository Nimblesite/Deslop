//! `cluster-by-id` deep-dive tests: the round-trip from a page row to
//! the full cluster, and its argument validation.

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{
        assert_tool_rejects_invalid_params, call_tool, init_and_tool_payload,
        structured_tool_result,
    },
    mcp_wire::{
        CLUSTER_BY_ID_TOOL, DUPLICATES_TOOL, ID_FIELD, ID_POINTER, LIMIT_PARAM, OFFSET_PARAM,
    },
    value_get,
};

use crate::common;

use serde_json::json;

#[test]
fn cluster_by_id_round_trips() -> Result<()> {
    let (mut child, report_value) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): 1 }),
    )?;
    let first_id = value_get(&report_value, "/clusters/0/id")?
        .as_str()
        .ok_or_else(|| anyhow!("first cluster id missing"))?
        .to_owned();
    let cluster = structured_tool_result(&call_tool(
        &mut child,
        CLUSTER_BY_ID_TOOL,
        &json!({ (ID_FIELD): &first_id }),
    )?)?;
    assert_eq!(value_get(&cluster, ID_POINTER)?, json!(first_id));
    assert!(
        cluster.get("occurrences").is_some(),
        "cluster-by-id is the deep-dive — must surface occurrences[]"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn cluster_by_id_unknown_returns_error() -> Result<()> {
    assert_tool_rejects_invalid_params(CLUSTER_BY_ID_TOOL, &json!({ (ID_FIELD): "not-a-real-id" }))
}

#[test]
fn cluster_by_id_missing_id_returns_invalid_params() -> Result<()> {
    assert_tool_rejects_invalid_params(CLUSTER_BY_ID_TOOL, &json!({}))
}
