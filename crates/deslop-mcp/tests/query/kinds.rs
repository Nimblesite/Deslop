//! [MCP-TOOL-FILTERS-KINDS] Configured clone kinds and per-call overrides.

use std::{fs, path::PathBuf};

use anyhow::Result;
use deslop_core::{buckets::ClusterKind, report::ReportCluster};
use serde_json::{json, Value};

use crate::common::{
    call_tool, copied_fixture, spawn_lsp_with_args, wait_for_state_then_init_mcp,
    ChildKillOnDrop, McpHandle,
};

/// A persisted selection whose fixture has no matching cluster.
const IDENTICAL_CONFIG: &str = "[total]\ninclude = [\"identical\"]\n";
const CONFIG_NAME: &str = ".deslop.toml";
const IDENTICAL: &str = "identical";
const NEARLY_IDENTICAL: &str = "nearly_identical";
const DUPLICATES: &str = "duplicates";
const FULL: &str = "full";
const PAGE_LIMIT: usize = 10;
const OCCURRENCE_BUDGET: usize = 100;
const EXPECTED_CLUSTERS: usize = 2;
const EXPECTED_COPIES: usize = 2;
const NO_MATCHES: usize = 0;
const FIRST_RANK: usize = 1;
const SECOND_RANK: usize = 2;
const FIRST_PATHS: [&str; EXPECTED_COPIES] = ["Alpha.cs", "Beta.cs"];
const SECOND_PATHS: [&str; EXPECTED_COPIES] = ["Delta.cs", "Gamma.cs"];

/// The same complete page shape for inherited and explicit selections.
fn arguments(kinds: Option<&[&str]>) -> Value {
    let mut args = json!({
        "detail": FULL, "limit": PAGE_LIMIT, "max_occurrences": OCCURRENCE_BUDGET,
    });
    if let Some(kinds) = kinds {
        args["kinds"] = json!(kinds);
    }
    args
}

/// Pins the actual ranked pairs so an empty detector cannot pass the test.
fn assert_nearly_identical_page(page: &Value) -> Result<()> {
    let clusters: Vec<ReportCluster> = serde_json::from_value(page["clusters"].clone())?;
    assert_eq!(clusters.len(), EXPECTED_CLUSTERS);
    let expected = [(FIRST_RANK, FIRST_PATHS), (SECOND_RANK, SECOND_PATHS)];
    for (cluster, (rank, paths)) in clusters.iter().zip(expected) {
        assert_eq!(cluster.kind, ClusterKind::NearlyIdentical);
        assert_eq!(cluster.rank, rank);
        assert_eq!(cluster.occurrence_count, EXPECTED_COPIES);
        assert_eq!(cluster.occurrences.len(), EXPECTED_COPIES);
        assert!(!cluster.id.is_empty(), "the pair has a stable identity");
        let actual: Vec<_> = cluster.occurrences.iter().map(|item| &item.path).collect();
        assert_eq!(actual, paths.map(PathBuf::from).iter().collect::<Vec<_>>());
    }
    assert_eq!(page["total_clusters"], json!(EXPECTED_CLUSTERS));
    Ok(())
}

/// A configured identical-only query must agree with its counted kinds.
fn assert_identical_only_page(page: &Value) {
    assert_eq!(page["metrics"]["total_includes"], json!([IDENTICAL]));
    assert_eq!(
        page["clusters"],
        json!([]),
        "identical-only configuration must exclude the fixture's nearly identical pairs"
    );
    assert_eq!(page["total_clusters"], json!(NO_MATCHES));
    assert_eq!(page["total_occurrences"], json!(NO_MATCHES));
    assert_eq!(page["page"]["returned"], json!(NO_MATCHES));
    assert_eq!(page["filters"]["kinds"], json!([IDENTICAL]));
}

/// One explicit selection must not change what the next omitted selection inherits.
fn check_queries(mcp: &mut McpHandle) -> Result<()> {
    let override_nearly = arguments(Some(&[NEARLY_IDENTICAL]));
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?);
    let override_identical = arguments(Some(&[IDENTICAL]));
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &override_identical)?);
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?);
    Ok(())
}

/// [MCP-TOOL-FILTERS-KINDS] Real LSP and MCP processes share one configured policy.
#[test]
fn configured_clone_kinds_are_inherited_and_explicit_arguments_override_one_call() -> Result<()> {
    let workspace = copied_fixture()?;
    fs::write(workspace.path().join(CONFIG_NAME), IDENTICAL_CONFIG)?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp = ChildKillOnDrop(lsp);
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    check_queries(&mut mcp)
}
