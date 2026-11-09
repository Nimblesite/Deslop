//! MCP resource tests: `resources/list` / `resources/read` over the
//! `deslop://report` and `deslop://schema` uris, and the schema-doc
//! tool that serves the same markdown ([MCP-RESOURCES]).

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{call_tool, structured_tool_result},
    mcp_child::{spawn_and_init, McpChild},
    mcp_wire::{CLUSTERS_POINTER, REPORT_URI, SCHEMA_DOC_FIELD, SCHEMA_DOC_TOOL},
    value_get,
};

use crate::common;

use serde_json::{json, Value};

/// MCP resource uri: the report-schema markdown.
const SCHEMA_URI: &str = "deslop://schema";
/// JSON pointer: the text body a `resources/read` serves.
const CONTENT_TEXT_POINTER: &str = "/result/contents/0/text";
/// JSON-RPC method: read one resource.
const RESOURCES_READ_METHOD: &str = "resources/read";
/// Small page limit for the schema-inlining checks.
const SCHEMA_TEST_PAGE_LIMIT: u64 = 2;
/// Floor the schema markdown's length must clear (it is the full
/// report schema document).
const SCHEMA_DOC_MIN_CHARS: usize = 1_000;

/// The text body a `resources/read` of `uri` serves.
fn resource_text(child: &mut McpChild, uri: &str) -> Result<String> {
    let response = child.request(RESOURCES_READ_METHOD, &json!({ "uri": uri }))?;
    value_get(&response, CONTENT_TEXT_POINTER)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{uri} resource must serve a text body: {response}"))
}

#[test]
fn resources_list_returns_report_and_schema_uris() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request("resources/list", &json!({}))?;
    let resources_value = value_get(&response, "/result/resources")?;
    let resources = resources_value
        .as_array()
        .ok_or_else(|| anyhow!("resources not array"))?;
    let uris: Vec<String> = resources
        .iter()
        .filter_map(|resource| {
            resource
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    assert!(
        uris.iter().any(|uri| uri == REPORT_URI),
        "report uri missing: {uris:?}"
    );
    assert!(
        uris.iter().any(|uri| uri == SCHEMA_URI),
        "schema uri missing: {uris:?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_report_returns_parseable_json() -> Result<()> {
    let mut child = spawn_and_init()?;
    let text = resource_text(&mut child, REPORT_URI)?;
    let parsed: Value = serde_json::from_str(&text)?;
    assert!(value_get(&parsed, CLUSTERS_POINTER)?.is_array());
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_schema_returns_markdown_body() -> Result<()> {
    let mut child = spawn_and_init()?;
    let text = resource_text(&mut child, SCHEMA_URI)?;
    assert!(!text.is_empty(), "schema_doc must not be empty");
    let _ = child.finish();
    Ok(())
}

#[test]
fn resources_read_unknown_uri_errors() -> Result<()> {
    common::mcp_calls::assert_method_rejects_invalid_params(
        RESOURCES_READ_METHOD,
        &json!({ "uri": "deslop://invalid" }),
    )
}

#[test]
fn issue_110_duplicates_pages_omit_schema_doc_and_schema_doc_tool_serves_it() -> Result<()> {
    let mut child = spawn_and_init()?;
    let duplicates = structured_tool_result(&call_tool(
        &mut child,
        "duplicates",
        &json!({ "offset": 0, "limit": SCHEMA_TEST_PAGE_LIMIT }),
    )?)?;
    assert!(
        duplicates.get(SCHEMA_DOC_FIELD).is_none(),
        "issue #110/#111: duplicates must not inline repeated schema_doc; got {} chars",
        duplicates
            .get(SCHEMA_DOC_FIELD)
            .and_then(Value::as_str)
            .map_or(0, str::len)
    );
    let filtered = structured_tool_result(&call_tool(
        &mut child,
        "duplicates",
        &json!({
            "offset": 0,
            "limit": SCHEMA_TEST_PAGE_LIMIT,
            "severities": ["warning"],
        }),
    )?)?;
    assert!(
        filtered.get(SCHEMA_DOC_FIELD).is_none(),
        "issue #110/#111: filtered duplicates pages must not inline repeated schema_doc; got {} chars",
        filtered
            .get(SCHEMA_DOC_FIELD)
            .and_then(Value::as_str)
            .map_or(0, str::len)
    );
    let tools_response = child.request("tools/list", &json!({}))?;
    let tools_value = value_get(&tools_response, "/result/tools")?;
    let tools = tools_value
        .as_array()
        .ok_or_else(|| anyhow!("tools/list must return an array"))?;
    let schema_tool = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some(SCHEMA_DOC_TOOL))
        .ok_or_else(|| anyhow!("schema-doc must be listed as the one-shot schema tool"))?;
    assert_eq!(
        schema_tool.pointer("/inputSchema/properties"),
        Some(&json!({})),
        "schema-doc must take no arguments: {schema_tool}"
    );
    let schema_payload =
        structured_tool_result(&call_tool(&mut child, SCHEMA_DOC_TOOL, &json!({}))?)?;
    let schema_doc_value = value_get(&schema_payload, "/schema_doc")?;
    let schema_doc = schema_doc_value
        .as_str()
        .ok_or_else(|| anyhow!("schema-doc payload must include schema_doc text"))?;
    assert!(
        schema_doc.len() > SCHEMA_DOC_MIN_CHARS,
        "schema-doc must return the full report schema markdown, got {} chars",
        schema_doc.len()
    );
    let resource_doc = resource_text(&mut child, SCHEMA_URI)?;
    assert_eq!(
        schema_doc, resource_doc,
        "schema-doc tool and deslop://schema resource must serve the same markdown"
    );
    let _ = child.finish();
    Ok(())
}
