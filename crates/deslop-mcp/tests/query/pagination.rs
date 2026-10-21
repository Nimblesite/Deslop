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
