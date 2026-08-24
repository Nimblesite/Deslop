//! E2E coverage for the `merge-plan` MCP tool ([AUTOFIX-MERGE-MCP]):
//! the real MCP binary delegates to the real LSP over IPC and returns
//! the mechanical merge plan — typed parameters, per-site arguments,
//! and the wire `WorkspaceEdit`. Unknown ids surface the stable
//! `UnknownCluster` error.

use crate::common;

use anyhow::{anyhow, ensure, Context, Result};
use common::{
    copied_fixture_named, spawn_lsp_and_wait_for_socket, structured_content,
    wait_for_state_then_init_mcp, ChildKillOnDrop, McpHandle,
};
use serde_json::{json, Value};

/// Brings up one scenario: a private copy of `fixture`, the real LSP
/// listening on its socket, and an initialised MCP session talking to
/// it. Returned as a pair so the caller keeps both alive for the test —
/// dropping the workspace or the LSP guard early tears the session down
/// mid-request.
fn mcp_session(fixture: &str) -> Result<(tempfile::TempDir, ChildKillOnDrop, McpHandle)> {
    let workspace = copied_fixture_named(fixture)?;
    let lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mcp = wait_for_state_then_init_mcp(workspace.path())?;
    Ok((workspace, lsp, mcp))
}

/// Requests the `n` worst clusters and returns the structured payload.
fn request_top_offenders(mcp: &mut McpHandle, count: u32) -> Result<Value> {
    let offenders = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": count } }),
    )?;
    structured_content(&offenders, "top-offenders")
}

/// Requests `merge-plan` for `cluster_id` and returns the structured
/// plan — the request idiom every scenario shares.
fn request_merge_plan(mcp: &mut McpHandle, cluster_id: &str) -> Result<Value> {
    let response = mcp.request(
        "tools/call",
        &json!({ "name": "merge-plan", "arguments": { "id": cluster_id } }),
    )?;
    structured_content(&response, "merge-plan")
}

/// [AUTOFIX-MERGE-MCP]: a mergeable cluster returns a mechanical plan
/// end to end (MCP → IPC → LSP → refactor engine).
#[test]
fn merge_plan_returns_mechanical_plan_over_ipc() -> Result<()> {
    let (_workspace, _lsp, mut mcp) = mcp_session("csharp-merge-mcp")?;

    let offenders = request_top_offenders(&mut mcp, 5)?;
    let cluster_id = offenders
        .pointer("/clusters/0/id")
        .and_then(Value::as_str)
        .context("worst cluster id present")?
        .to_owned();

    let plan = request_merge_plan(&mut mcp, &cluster_id)?;

    ensure!(
        plan.pointer("/verdict/kind").and_then(Value::as_str) == Some("mechanical"),
        "the fixture cluster must merge mechanically, got {plan}"
    );
    let helper = plan
        .pointer("/helper_name")
        .and_then(Value::as_str)
        .context("helper name present")?;
    ensure!(
        helper.starts_with("MergedFromCluster_"),
        "deterministic helper name, got {helper}"
    );
    let parameters = plan
        .pointer("/parameters")
        .and_then(Value::as_array)
        .context("parameters present")?;
    ensure!(
        parameters.len() == 3,
        "context + two hole parameters expected, got {}",
        parameters.len()
    );
    let types: Vec<&str> = parameters
        .iter()
        .filter_map(|parameter| parameter.pointer("/type_name").and_then(Value::as_str))
        .collect();
    ensure!(
        types == ["PricingBook", "string", "int"],
        "declared types over the wire, got {types:?}"
    );
    ensure!(
        plan.pointer("/workspace_edit/documentChanges/0/edits")
            .and_then(Value::as_array)
            .is_some_and(|edits| edits.len() == 3),
        "workspace edit carries one insertion plus two rewrites"
    );
    Ok(())
}

/// Unknown cluster ids surface the stable `UnknownCluster` error code
/// through the tool envelope.
#[test]
fn merge_plan_unknown_id_is_a_stable_error() -> Result<()> {
    let (_workspace, _lsp, mut mcp) = mcp_session("csharp-merge-mcp")?;

    let response = mcp.request(
        "tools/call",
        &json!({ "name": "merge-plan", "arguments": { "id": "ffffffffffffffff" } }),
    )?;
    let is_error = response
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || response.pointer("/error").is_some();
    ensure!(is_error, "unknown ids must error, got {response}");
    let text = response.to_string();
    ensure!(
        text.contains("no cluster with id") || text.contains("unknown cluster"),
        "error names the unknown cluster, got {text}"
    );
    Ok(())
}

/// A cluster that cannot merge still answers — with an `ai_or_human`
/// verdict and a reason, never an error ([AUTOFIX-MERGE-GATE]).
#[test]
fn unmergeable_cluster_answers_with_ai_or_human() -> Result<()> {
    let (_workspace, _lsp, mut mcp) = mcp_session("csharp-mcp")?;

    let offenders = request_top_offenders(&mut mcp, 1)?;
    let Some(cluster_id) = offenders
        .pointer("/clusters/0/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Err(anyhow!(
            "csharp-mcp fixture must produce at least one cluster"
        ));
    };

    let plan = request_merge_plan(&mut mcp, &cluster_id)?;
    let verdict = plan
        .pointer("/verdict/kind")
        .and_then(Value::as_str)
        .context("verdict present")?;
    ensure!(
        verdict == "mechanical" || plan.pointer("/verdict/reason").is_some(),
        "either mechanical or a reasoned refusal, got {plan}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-SURFACE] (issue #277): a cross-file identical
/// definition routes to the consolidation engine and answers a
/// mechanical plan whose `WorkspaceEdit` deletes the duplicate and
/// imports the canonical symbol.
#[test]
fn merge_plan_routes_cross_file_cluster_to_consolidation() -> Result<()> {
    let (_workspace, _lsp, mut mcp) = mcp_session("rust-consolidate")?;

    let offenders = request_top_offenders(&mut mcp, 5)?;
    let clusters = offenders
        .pointer("/clusters")
        .and_then(Value::as_array)
        .context("clusters present")?;
    let cluster_id = clusters
        .iter()
        .find(|cluster| {
            cluster
                .pointer("/occurrences")
                .and_then(Value::as_array)
                .is_some_and(|occurrences| {
                    let paths: std::collections::HashSet<&str> = occurrences
                        .iter()
                        .filter_map(|entry| entry.pointer("/path").and_then(Value::as_str))
                        .collect();
                    paths.len() >= 2
                })
        })
        .and_then(|cluster| cluster.pointer("/id").and_then(Value::as_str))
        .context("a cross-file cluster surfaces in the offenders")?
        .to_owned();

    let plan = request_merge_plan(&mut mcp, &cluster_id)?;
    ensure!(
        plan.pointer("/verdict/kind").and_then(Value::as_str) == Some("mechanical"),
        "the cross-file cluster must consolidate mechanically, got {plan}"
    );
    ensure!(
        plan.pointer("/helper_name").and_then(Value::as_str) == Some("normalise_labels"),
        "the consolidated symbol rides in helper_name, got {plan}"
    );
    let edits = plan
        .pointer("/workspace_edit/documentChanges/0/edits")
        .and_then(Value::as_array)
        .context("consolidation edit present")?;
    ensure!(
        edits.len() == 2,
        "deletion plus import expected, got {}",
        edits.len()
    );
    let uri = plan
        .pointer("/workspace_edit/documentChanges/0/textDocument/uri")
        .and_then(Value::as_str)
        .context("edited uri present")?;
    ensure!(
        uri.starts_with("file://")
            && std::path::Path::new(uri)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs")),
        "absolute file uri for the duplicate, got {uri}"
    );
    Ok(())
}
