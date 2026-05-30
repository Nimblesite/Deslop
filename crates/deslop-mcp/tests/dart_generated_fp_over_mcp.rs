//! Real-binary MCP regression for Dart false positives.
//!
//! Drives the actual `deslop-lsp` + `deslop-mcp` binaries (never a fake
//! server) against the `dart-mcp` fixture and asserts over the
//! `top-offenders` wire that generated `*.g.dart` serialisation clones are
//! hidden (#95 carried to Dart) while a genuine hand-written clone still
//! surfaces. The engine is shared with the CLI, so this proves the MCP
//! transport carries the Dart report-hide semantics end to end
//! ([EXCLUSION-CONFIG], [LANG-CAND-DART]).

#![cfg(unix)]

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

mod common;
use common::{
    copied_fixture_named, initialized_mcp, spawn_lsp_and_initialize, structured_content,
    wait_for_path, ChildKillOnDrop, SOCKET_TIMEOUT,
};

/// File names of every occurrence across all returned clusters.
fn occurrence_file_names(payload: &Value) -> Vec<String> {
    payload
        .get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| cluster.get("occurrences").and_then(Value::as_array))
                .flatten()
                .filter_map(|occ| occ.get("path").and_then(Value::as_str))
                .map(|path| path.rsplit('/').next().unwrap_or(path).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Issue #95 over the MCP wire: a generated `.g.dart` self-duplicate must
/// never reach `top-offenders`, but the hand-written `computeCartTotal` /
/// `computeOrderTotal` Type-2 clone must — proving the suppression is
/// targeted, not a blanket hide of every Dart cluster.
#[test]
fn dart_generated_files_never_top_offenders_over_mcp() -> Result<()> {
    let workspace = copied_fixture_named("dart-mcp")?;
    let lsp = spawn_lsp_and_initialize(workspace.path())?;
    let _lsp_guard = ChildKillOnDrop(lsp);
    let socket = workspace.path().join(".deslop-cache/deslop.sock");
    wait_for_path(&socket, SOCKET_TIMEOUT).context("wait for ipc socket")?;

    let mut mcp = initialized_mcp(workspace.path())?;
    let response = mcp.request(
        "tools/call",
        &json!({
            "name": "top-offenders",
            "arguments": { "n": 50, "max_occurrences": 100_000_usize }
        }),
    )?;
    let payload = structured_content(&response, "top-offenders")?;
    let files = occurrence_file_names(&payload);

    ensure!(
        !files
            .iter()
            .any(|name| name == "cart.g.dart" || name == "order.g.dart"),
        "generated `.g.dart` files must never appear in MCP top-offenders; got {files:?}"
    );
    ensure!(
        files.iter().any(|name| name == "cart_totals.dart")
            && files.iter().any(|name| name == "order_totals.dart"),
        "the hand-written Dart clone must still surface over MCP top-offenders; got {files:?}"
    );
    Ok(())
}
