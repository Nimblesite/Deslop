//! Real-binary MCP regression for GH #270.
//!
//! Drives the actual `deslop-lsp` + `deslop-mcp` binaries (never a fake
//! server) against the `fsharp-mcp` fixture and asserts that a hand-written
//! F# cluster reports `language: "fsharp"` over the `report-query` wire.
//!
//! On a real F# repo (fantomas) every `report-query` cluster surfaced as
//! `language: "unknown"` — a recurrence of the #164/#170/#198 drift for a
//! language the analyzer actually ran (`session-config` lists `fsharp`). The
//! per-cluster language label must be driven by the same parser registry as
//! `session-config`, so every analyzed language classifies, not just C#/Rust/Py.

#![cfg(unix)]

use anyhow::{ensure, Result};
use serde_json::{json, Value};

mod common;
use common::{call_tool, copied_fixture_named, initialized_mcp, spawn_lsp_and_wait_for_socket};

/// The `language` label of every returned cluster whose representative
/// occurrence is a `.fs` file.
fn fsharp_cluster_languages(page: &Value) -> Vec<String> {
    page.get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter(|cluster| first_occurrence_is_fsharp(cluster))
                .filter_map(|cluster| cluster.get("language").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// True when a cluster's `first_occurrence.path` is a `.fs` file.
fn first_occurrence_is_fsharp(cluster: &Value) -> bool {
    cluster
        .pointer("/first_occurrence/path")
        .and_then(Value::as_str)
        .and_then(|path| std::path::Path::new(path).extension()?.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("fs"))
}

#[test]
fn fsharp_clusters_report_fsharp_language_over_mcp() -> Result<()> {
    let workspace = copied_fixture_named("fsharp-mcp")?;
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let page = call_tool(
        &mut mcp,
        "report-query",
        &json!({ "offset": 0, "limit": 50 }),
    )?;
    let languages = fsharp_cluster_languages(&page);

    ensure!(
        !languages.is_empty(),
        "fixture must surface at least one hand-written F# cluster so the \
         language label is exercised: {page}"
    );
    for language in &languages {
        ensure!(
            language == "fsharp",
            "issue #270: a cluster whose representative occurrence is a `.fs` \
             file must report language=\"fsharp\", not {language:?}; full page: {page}"
        );
    }
    Ok(())
}
