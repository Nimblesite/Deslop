//! Real-binary MCP regression for GH #164.
//!
//! Drives the actual `deslop-lsp` + `deslop-mcp` binaries (never a fake
//! server) against the `dart-mcp` fixture and asserts that a hand-written
//! Dart cluster reports `language: "dart"` over the `report-query` wire.
//!
//! The MCP page summary derived each cluster's language from a hand-maintained
//! extension → id map in `deslop-mcp` that omitted `.dart` (a drifted copy of
//! the renderer's mapping). Every Dart cluster therefore surfaced as
//! `language: "unknown"`, breaking the language label and the `report-query`
//! language filter on Dart repos even after the enum gained `dart` (#170/#198).

#![cfg(unix)]

use anyhow::{ensure, Result};
use serde_json::{json, Value};

mod common;
use common::{call_tool, copied_fixture_named, initialized_mcp, spawn_lsp_and_wait_for_socket};

/// The `language` label of every returned cluster whose representative
/// occurrence is a `.dart` file.
fn dart_cluster_languages(page: &Value) -> Vec<String> {
    page.get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter(|cluster| first_occurrence_is_dart(cluster))
                .filter_map(|cluster| cluster.get("language").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// True when a cluster's `first_occurrence.path` is a `.dart` file.
fn first_occurrence_is_dart(cluster: &Value) -> bool {
    cluster
        .pointer("/first_occurrence/path")
        .and_then(Value::as_str)
        .and_then(|path| std::path::Path::new(path).extension()?.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dart"))
}

#[test]
fn dart_clusters_report_dart_language_over_mcp() -> Result<()> {
    let workspace = copied_fixture_named("dart-mcp")?;
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let page = call_tool(
        &mut mcp,
        "report-query",
        &json!({ "offset": 0, "limit": 50 }),
    )?;
    let languages = dart_cluster_languages(&page);

    ensure!(
        !languages.is_empty(),
        "fixture must surface at least one hand-written Dart cluster so the \
         language label is exercised: {page}"
    );
    for language in &languages {
        ensure!(
            language == "dart",
            "issue #164: a cluster whose representative occurrence is a `.dart` \
             file must report language=\"dart\", not {language:?}; full page: {page}"
        );
    }
    Ok(())
}
