//! [MCP-TOOL-FILTERS-KINDS] Configured clone kinds and per-call overrides.

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use deslop_core::{buckets::ClusterKind, report::ReportCluster};
use serde_json::{json, Value};

use crate::common::{
    call_tool, copied_fixture, spawn_lsp_with_args, wait_for_state_then_init_mcp, ChildKillOnDrop,
    McpHandle,
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
const INVALID_KIND: &str = "not_a_clone_kind";
const INVALID_PARAMS: i64 = -32_602;
const RESCAN: &str = "rescan";
const FIND_SIMILAR: &str = "find-similar";
const CSHARP: &str = "csharp";
const SNIPPET: &str = include_str!("../fixtures/csharp-mcp/Alpha.cs");

/// The same complete page shape for inherited and explicit selections.
fn arguments(kinds: Option<&[&str]>) -> Value {
    let mut args = serde_json::Map::from_iter([
        ("detail".to_owned(), json!(FULL)),
        ("limit".to_owned(), json!(PAGE_LIMIT)),
        ("max_occurrences".to_owned(), json!(OCCURRENCE_BUDGET)),
    ]);
    args.extend(kinds.map(|kinds| ("kinds".to_owned(), json!(kinds))));
    Value::Object(args)
}

/// Changes only the kind selection on a known object-shaped request.
fn set_kinds(args: &mut Value, kinds: Value) -> Result<()> {
    let _previous = args
        .as_object_mut()
        .context("request must be an object")?
        .insert("kinds".to_owned(), kinds);
    Ok(())
}

/// A missing wire field is an assertion failure, never an implicit null.
fn field<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| anyhow!("response is missing asserted field {pointer}: {value}"))
}

/// Pins the actual ranked pairs so an empty detector cannot pass the test.
fn assert_nearly_identical_page(page: &Value) -> Result<()> {
    let clusters: Vec<ReportCluster> = serde_json::from_value(field(page, "/clusters")?.clone())?;
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
    assert_eq!(field(page, "/total_clusters")?, &json!(EXPECTED_CLUSTERS));
    Ok(())
}

/// A configured identical-only query must agree with its counted kinds.
fn assert_identical_only_page(page: &Value) -> Result<()> {
    assert_eq!(field(page, "/metrics/total_includes")?, &json!([IDENTICAL]));
    assert_eq!(
        field(page, "/clusters")?,
        &json!([]),
        "identical-only configuration must exclude the fixture's nearly identical pairs"
    );
    assert_eq!(field(page, "/total_clusters")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/total_occurrences")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/page/returned")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/filters/kinds")?, &json!([IDENTICAL]));
    Ok(())
}

/// One explicit selection must not change what the next omitted selection inherits.
fn check_queries(mcp: &mut McpHandle) -> Result<()> {
    let override_nearly = arguments(Some(&[NEARLY_IDENTICAL]));
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?)?;
    let override_identical = arguments(Some(&[IDENTICAL]));
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &override_identical)?)?;
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?)?;
    Ok(())
}

/// Every cluster-returning tool inherits the identical-only policy.
fn check_shared_filters(mcp: &mut McpHandle) -> Result<()> {
    let refreshed = call_tool(mcp, RESCAN, &arguments(None))?;
    assert_identical_only_page(field(&refreshed, "/page")?)?;
    let inherited = json!({"snippet": SNIPPET, "language": CSHARP});
    assert_identical_only_page(&call_tool(mcp, FIND_SIMILAR, &inherited)?)?;
    let mut explicit = inherited;
    set_kinds(&mut explicit, json!([NEARLY_IDENTICAL]))?;
    let found = call_tool(mcp, FIND_SIMILAR, &explicit)?;
    assert!(field(&found, "/clusters")?
        .as_array()
        .is_some_and(|rows| !rows.is_empty()));
    assert_eq!(field(&found, "/filters/kinds")?, &json!([NEARLY_IDENTICAL]));
    Ok(())
}

/// Invalid overrides must be rejected rather than ignored as a broader selection.
fn check_invalid_kinds(mcp: &mut McpHandle) -> Result<()> {
    for invalid in [json!([INVALID_KIND]), json!(IDENTICAL), Value::Null] {
        let mut args = arguments(None);
        set_kinds(&mut args, invalid)?;
        let reply = mcp.request(
            "tools/call",
            &json!({"name": DUPLICATES, "arguments": args}),
        )?;
        assert_eq!(field(&reply, "/error/code")?, &json!(INVALID_PARAMS));
    }
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
    check_queries(&mut mcp)?;
    check_shared_filters(&mut mcp)?;
    check_invalid_kinds(&mut mcp)
}

/// [MCP-TOOL-FILTERS-KINDS] The default selection and explicit empty set differ.
#[test]
fn default_clone_kinds_and_explicit_empty_selection_are_reported_truthfully() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp = ChildKillOnDrop(lsp);
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    let inherited = call_tool(&mut mcp, DUPLICATES, &arguments(None))?;
    assert_nearly_identical_page(&inherited)?;
    assert_eq!(
        field(&inherited, "/filters/kinds")?,
        &json!([IDENTICAL, NEARLY_IDENTICAL])
    );
    let empty = call_tool(&mut mcp, DUPLICATES, &arguments(Some(&[])))?;
    assert_eq!(field(&empty, "/clusters")?, &json!([]));
    assert_eq!(field(&empty, "/filters/kinds")?, &json!([]));
    assert_eq!(field(&empty, "/total_clusters")?, &json!(NO_MATCHES));
    assert_nearly_identical_page(&call_tool(&mut mcp, DUPLICATES, &arguments(None))?)
}
