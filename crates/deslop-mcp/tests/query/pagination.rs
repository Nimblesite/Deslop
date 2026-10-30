//! [MCP-PAGE-CONTINUATION] An occurrence budget must disclose omitted results.

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::common::{
    call_tool, copied_fixture, spawn_lsp_and_wait_for_socket, wait_for_state_then_init_mcp,
    McpHandle,
};

const PAGE_LIMIT: usize = 2;
const OCCURRENCE_BUDGET: usize = 1;
const FIRST_OFFSET: usize = 0;
const NEXT_OFFSET: usize = 1;
const TOTAL_OCCURRENCES: usize = 4;
const FIRST_ID: &str = "54a6b248b64f425d";
const SECOND_ID: &str = "77b94fb5d8ae7d7c";
const FULL: &str = "full";
const CSHARP: &str = "csharp";
const FILTER_PATH: &str = "Gamma.cs";
const FIRST_SOURCE: &str = include_str!("../fixtures/csharp-mcp/Alpha.cs");
const SECOND_SOURCE: &str = include_str!("../fixtures/csharp-mcp/Gamma.cs");
const SIMILARITY_TOOL: &str = "find-similar";
const REPORT_TOOL: &str = "duplicates";
const PAGE_CONTROLS: [&str; 5] = [
    "offset",
    "limit",
    "detail",
    "max_occurrences",
    "include_per_file",
];
const SUMMARY: &str = "summary";
const NO_FILTER: &str = "";
const FIRST_PATH: &str = "Alpha.cs";
const SECOND_PATH: &str = "Delta.cs";
const CLONE_KIND: &str = "nearly_identical";
const FIXTURE_FILE_COUNT: usize = 4;
const NO_ROWS: usize = 0;
const INVALID_PARAMS: i64 = -32_602;
const RANGE_INPUT_FIELDS: [&str; 3] = ["path", "start_byte", "end_byte"];
const SNIPPET_INPUT_FIELDS: [&str; 2] = ["snippet", "language"];

/// The caller deliberately asks for more clusters than the occurrence budget fits.
fn read_page(mcp: &mut McpHandle, offset: usize) -> Result<Value> {
    call_tool(
        mcp,
        "duplicates",
        &json!({
            "offset": offset, "limit": PAGE_LIMIT, "detail": FULL,
            "max_occurrences": OCCURRENCE_BUDGET,
        }),
    )
}

/// Pins the returned member and the totals that budget trimming must not change.
fn assert_page(page: &Value, id: &str, rank: usize) -> Result<()> {
    let clusters = page
        .get("clusters")
        .and_then(Value::as_array)
        .context("cluster rows")?;
    assert_eq!(clusters.len(), OCCURRENCE_BUDGET);
    let cluster = clusters.first().context("one ranked cluster")?;
    assert_eq!(cluster.get("id"), Some(&json!(id)));
    assert_eq!(cluster.get("rank"), Some(&json!(rank)));
    assert_eq!(cluster.get("occurrences_truncated"), Some(&json!(true)));
    assert_eq!(page.get("total_clusters"), Some(&json!(PAGE_LIMIT)));
    assert_eq!(
        page.get("total_occurrences"),
        Some(&json!(TOTAL_OCCURRENCES))
    );
    assert_eq!(page.get("truncated"), Some(&json!(true)));
    assert!(page
        .get("next_action")
        .and_then(Value::as_str)
        .is_some_and(|hint| !hint.is_empty()));
    Ok(())
}

/// The second page completes cluster navigation; omitted occurrences remain explicit.
fn assert_continuation(mcp: &mut McpHandle) -> Result<()> {
    let first = read_page(mcp, FIRST_OFFSET)?;
    assert_page(&first, FIRST_ID, NEXT_OFFSET)?;
    assert_eq!(
        first.pointer("/page/next_offset"),
        Some(&json!(NEXT_OFFSET))
    );
    assert_eq!(first.pointer("/page/has_more"), Some(&json!(true)));
    let next = read_page(mcp, NEXT_OFFSET)?;
    assert_page(&next, SECOND_ID, PAGE_LIMIT)?;
    assert_eq!(next.pointer("/page/next_offset"), Some(&Value::Null));
    assert_eq!(next.pointer("/page/has_more"), Some(&json!(false)));
    let complete = call_tool(mcp, "cluster-by-id", &json!({"id": FIRST_ID}))?;
    assert_eq!(complete.get("occurrences_total"), Some(&json!(PAGE_LIMIT)));
    assert_eq!(complete.get("occurrences_truncated"), Some(&json!(false)));
    Ok(())
}

#[test]
fn occurrence_budget_exposes_continuation_and_preserves_the_complete_pairs() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    assert_continuation(&mut mcp)
}

/// [MCP-TOOL-FILTERS] Filtering precedes the result limit on similarity queries too.
#[test]
fn snippet_filter_selects_a_later_pair_before_applying_the_limit() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    let snippet = format!("{FIRST_SOURCE}\n{SECOND_SOURCE}");
    let all = call_tool(
        &mut mcp,
        "find-similar",
        &json!({
            "snippet": snippet, "language": CSHARP, "limit": PAGE_LIMIT,
        }),
    )?;
    assert_eq!(all.get("total_clusters"), Some(&json!(PAGE_LIMIT)));
    let scoped = call_tool(
        &mut mcp,
        "find-similar",
        &json!({
            "snippet": snippet, "language": CSHARP, "limit": OCCURRENCE_BUDGET,
            "path_contains": FILTER_PATH,
        }),
    )?;
    assert_eq!(
        scoped.get("total_clusters"),
        Some(&json!(OCCURRENCE_BUDGET))
    );
    assert_eq!(scoped.pointer("/clusters/0/id"), Some(&json!(SECOND_ID)));
    assert_eq!(scoped.pointer("/clusters/0/rank"), Some(&json!(PAGE_LIMIT)));
    assert_eq!(
        scoped.pointer("/clusters/0/occurrence_count"),
        Some(&json!(PAGE_LIMIT))
    );
    Ok(())
}

/// Finds a tool in the public discovery response without calling internal schema code.
fn discovered_tool<'a>(response: &'a Value, name: &str) -> Result<&'a Value> {
    let tools = response
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .context("tool list")?;
    tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
        .context("advertised tool")
}

/// [MCP-PAGE-CONTINUATION] Strict clients must be able to express every page control.
fn assert_similarity_page_controls(mcp: &mut McpHandle) -> Result<()> {
    let response = mcp.request("tools/list", &json!({}))?;
    let similar = discovered_tool(&response, SIMILARITY_TOOL)?;
    let report = discovered_tool(&response, REPORT_TOOL)?;
    assert_eq!(
        similar.pointer("/inputSchema/additionalProperties"),
        Some(&json!(false))
    );
    for control in PAGE_CONTROLS {
        assert_page_control(similar, report, control)?;
    }
    Ok(())
}

/// Page tools share one schema so recovery advice remains expressible by strict clients.
fn assert_page_control(similar: &Value, report: &Value, control: &str) -> Result<()> {
    let pointer = format!("/inputSchema/properties/{control}");
    let expected = report
        .pointer(&pointer)
        .context("canonical report page control")?;
    assert_eq!(
        similar.pointer(&pointer),
        Some(expected),
        "find-similar must advertise {control}"
    );
    Ok(())
}

/// Sends only advertised controls; the cursor comes directly from the first response.
fn read_similarity_page(mcp: &mut McpHandle, offset: usize) -> Result<Value> {
    call_tool(
        mcp,
        SIMILARITY_TOOL,
        &json!({
            "snippet": format!("{FIRST_SOURCE}\n{SECOND_SOURCE}"), "language": CSHARP,
            "offset": offset, "limit": PAGE_LIMIT, "detail": FULL,
            "max_occurrences": OCCURRENCE_BUDGET, "include_per_file": false,
        }),
    )
}

/// [MCP-PAGE-CONTINUATION] Discovery and continuation use the same public contract.
#[test]
fn similarity_schema_supports_the_returned_cursor_and_recovery_controls() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    assert_similarity_page_controls(&mut mcp)?;
    let first = read_similarity_page(&mut mcp, FIRST_OFFSET)?;
    assert_page(&first, FIRST_ID, NEXT_OFFSET)?;
    assert_eq!(first.pointer("/page/has_more"), Some(&json!(true)));
    let offset = first
        .pointer("/page/next_offset")
        .and_then(Value::as_u64)
        .context("continuation cursor")?;
    assert_eq!(offset, u64::try_from(NEXT_OFFSET)?);
    let next = read_similarity_page(&mut mcp, usize::try_from(offset)?)?;
    assert_page(&next, SECOND_ID, PAGE_LIMIT)?;
    assert_eq!(next.pointer("/page/has_more"), Some(&json!(false)));
    assert_eq!(next.pointer("/page/next_offset"), Some(&Value::Null));
    Ok(())
}

/// Summary detail must honor the same cursor and path filter as full detail.
fn read_similarity_summary(mcp: &mut McpHandle, offset: usize, path: &str) -> Result<Value> {
    call_tool(
        mcp,
        SIMILARITY_TOOL,
        &json!({
            "snippet": format!("{FIRST_SOURCE}\n{SECOND_SOURCE}"), "language": CSHARP,
            "offset": offset, "limit": OCCURRENCE_BUDGET, "detail": SUMMARY,
            "max_occurrences": OCCURRENCE_BUDGET, "include_per_file": true,
            "path_contains": path,
        }),
    )
}

/// Pin the pair, its canonical navigation path, and the requested metrics detail.
fn assert_summary_pair(page: &Value, id: &str, rank: usize, path: &str) -> Result<()> {
    let clusters = page
        .get("clusters")
        .and_then(Value::as_array)
        .context("summary rows")?;
    assert_eq!(clusters.len(), OCCURRENCE_BUDGET);
    let cluster = clusters.first().context("ranked summary")?;
    assert_summary_row(cluster, id, rank, path);
    let metrics = page
        .pointer("/metrics/per_file")
        .and_then(Value::as_array)
        .context("per-file metrics")?;
    assert_eq!(metrics.len(), FIXTURE_FILE_COUNT);
    assert_eq!(page.get("truncated"), Some(&json!(false)));
    Ok(())
}

/// Summary projection keeps the pair evidence needed for navigation.
fn assert_summary_row(cluster: &Value, id: &str, rank: usize, path: &str) {
    assert_eq!(cluster.get("id"), Some(&json!(id)));
    assert_eq!(cluster.get("rank"), Some(&json!(rank)));
    assert_eq!(cluster.get("kind"), Some(&json!(CLONE_KIND)));
    assert_eq!(cluster.get("occurrence_count"), Some(&json!(PAGE_LIMIT)));
    assert_eq!(
        cluster.pointer("/first_occurrence/path"),
        Some(&json!(path))
    );
    assert_eq!(
        cluster.get("occurrences"),
        None,
        "summary omits the full occurrence list"
    );
}

/// An exhausted cursor is an ordinary empty page with preserved matched totals.
fn assert_similarity_boundary(
    mcp: &mut McpHandle,
    offset: usize,
    path: &str,
    total: usize,
) -> Result<()> {
    let page = read_similarity_summary(mcp, offset, path)?;
    assert_eq!(page.get("clusters"), Some(&json!([])));
    assert_eq!(page.get("total_clusters"), Some(&json!(total)));
    assert_eq!(page.pointer("/page/offset"), Some(&json!(offset)));
    assert_eq!(page.pointer("/page/returned"), Some(&json!(NO_ROWS)));
    assert_eq!(page.pointer("/page/next_offset"), Some(&Value::Null));
    assert_eq!(page.pointer("/page/has_more"), Some(&json!(false)));
    assert_eq!(page.get("truncated"), Some(&json!(false)));
    Ok(())
}

/// A summary cursor reaches both ranked pairs without an occurrence-budget omission.
fn assert_summary_continuation(mcp: &mut McpHandle) -> Result<()> {
    let first = read_similarity_summary(mcp, FIRST_OFFSET, NO_FILTER)?;
    assert_summary_pair(&first, FIRST_ID, NEXT_OFFSET, FIRST_PATH)?;
    assert_eq!(
        first.pointer("/page/next_offset"),
        Some(&json!(NEXT_OFFSET))
    );
    let next = read_similarity_summary(mcp, NEXT_OFFSET, NO_FILTER)?;
    assert_summary_pair(&next, SECOND_ID, PAGE_LIMIT, SECOND_PATH)?;
    assert_eq!(next.pointer("/page/next_offset"), Some(&Value::Null));
    assert_similarity_boundary(mcp, PAGE_LIMIT, NO_FILTER, PAGE_LIMIT)
}

/// Filtering precedes cursor selection and preserves the engine's original rank.
fn assert_filtered_summary(mcp: &mut McpHandle) -> Result<()> {
    let filtered = read_similarity_summary(mcp, FIRST_OFFSET, FILTER_PATH)?;
    assert_summary_pair(&filtered, SECOND_ID, PAGE_LIMIT, SECOND_PATH)?;
    assert_eq!(
        filtered.get("total_clusters"),
        Some(&json!(OCCURRENCE_BUDGET))
    );
    assert_eq!(
        filtered.pointer("/filters/path_contains"),
        Some(&json!(FILTER_PATH))
    );
    assert_similarity_boundary(mcp, NEXT_OFFSET, FILTER_PATH, OCCURRENCE_BUDGET)
}

/// [MCP-PAGE-CONTINUATION] Summary pages preserve rank and filters across boundaries.
#[test]
fn similarity_summary_pages_preserve_filters_and_exhaust_the_cursor() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    assert_summary_continuation(&mut mcp)?;
    assert_filtered_summary(&mut mcp)
}

/// Missing variants and two complete variants are invalid on the live handler.
fn invalid_similarity_inputs() -> [Value; 6] {
    [
        json!({}),
        json!({"path": FIRST_PATH}),
        json!({"path": FIRST_PATH, "start_byte": FIRST_OFFSET}),
        json!({"snippet": FIRST_SOURCE}),
        json!({"language": CSHARP}),
        json!({"path": FIRST_PATH, "start_byte": FIRST_OFFSET, "end_byte": FIRST_SOURCE.len(),
            "snippet": FIRST_SOURCE, "language": CSHARP}),
    ]
}

/// A snippet may retain a partial range block as optional file scope.
fn valid_similarity_inputs() -> [Value; 3] {
    [
        json!({"path": FIRST_PATH, "start_byte": FIRST_OFFSET, "end_byte": FIRST_SOURCE.len()}),
        json!({"snippet": FIRST_SOURCE, "language": CSHARP}),
        json!({"snippet": FIRST_SOURCE, "language": CSHARP, "path": FIRST_PATH}),
    ]
}

/// A malformed variant receives `InvalidParams` rather than an empty result.
fn assert_rejected_similarity_variants(mcp: &mut McpHandle) -> Result<()> {
    for arguments in invalid_similarity_inputs() {
        let response = mcp.request(
            "tools/call",
            &json!({"name": SIMILARITY_TOOL, "arguments": arguments}),
        )?;
        assert_eq!(
            response.pointer("/error/code"),
            Some(&json!(INVALID_PARAMS))
        );
        assert_eq!(response.get("result"), None);
    }
    Ok(())
}

/// Pin real results for both complete inputs and the supported scoped-snippet form.
fn assert_valid_similarity_variants(mcp: &mut McpHandle) -> Result<()> {
    for arguments in valid_similarity_inputs() {
        let page = call_tool(mcp, SIMILARITY_TOOL, &arguments)?;
        assert_eq!(page.pointer("/clusters/0/id"), Some(&json!(FIRST_ID)));
        assert_eq!(page.pointer("/clusters/0/rank"), Some(&json!(NEXT_OFFSET)));
        assert_eq!(
            page.pointer("/clusters/0/occurrence_count"),
            Some(&json!(PAGE_LIMIT))
        );
    }
    Ok(())
}

/// [MCP-TOOL-FINDSIMILAR] Discovery and the live handler require one complete input.
#[test]
fn similarity_schema_requires_exactly_one_complete_input_variant() -> Result<()> {
    let workspace = copied_fixture()?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    assert_rejected_similarity_variants(&mut mcp)?;
    assert_valid_similarity_variants(&mut mcp)?;
    let response = mcp.request("tools/list", &json!({}))?;
    let similar = discovered_tool(&response, SIMILARITY_TOOL)?;
    let required_variants = json!([
        {"required": RANGE_INPUT_FIELDS}, {"required": SNIPPET_INPUT_FIELDS},
    ]);
    assert_eq!(
        similar.pointer("/inputSchema/oneOf"),
        Some(&required_variants),
        "strict clients must require exactly one complete range or snippet input"
    );
    assert_similarity_page_controls(&mut mcp)
}
