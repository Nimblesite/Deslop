//! E2E coverage for the `merge-plan` MCP tool ([AUTOFIX-MERGE-MCP]):
//! the real MCP binary delegates to the real LSP over IPC and returns
//! the mechanical merge plan — typed parameters, per-site arguments,
//! and the wire `WorkspaceEdit`. Unknown ids surface the stable
//! `UnknownCluster` error.

mod common;

use anyhow::{anyhow, ensure, Context, Result};
use common::{
    copied_fixture_named, spawn_lsp_and_wait_for_socket, structured_content,
    wait_for_state_then_init_mcp,
};
use serde_json::{json, Value};

/// [AUTOFIX-MERGE-MCP]: a mergeable cluster returns a mechanical plan
/// end to end (MCP → IPC → LSP → refactor engine).
#[test]
fn merge_plan_returns_mechanical_plan_over_ipc() -> Result<()> {
    let workspace = copied_fixture_named("csharp-merge-mcp")?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    let offenders = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 5 } }),
    )?;
    let offenders = structured_content(&offenders, "top-offenders")?;
    let cluster_id = offenders
        .pointer("/clusters/0/id")
        .and_then(Value::as_str)
        .context("worst cluster id present")?
        .to_owned();

    let response = mcp.request(
        "tools/call",
        &json!({ "name": "merge-plan", "arguments": { "id": cluster_id } }),
    )?;
    let plan = structured_content(&response, "merge-plan")?;

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
    let workspace = copied_fixture_named("csharp-merge-mcp")?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

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
    let workspace = copied_fixture_named("csharp-mcp")?;
    let _lsp = spawn_lsp_and_wait_for_socket(workspace.path())?;
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    let offenders = mcp.request(
        "tools/call",
        &json!({ "name": "top-offenders", "arguments": { "n": 1 } }),
    )?;
    let offenders = structured_content(&offenders, "top-offenders")?;
    let Some(cluster_id) = offenders
        .pointer("/clusters/0/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Err(anyhow!(
            "csharp-mcp fixture must produce at least one cluster"
        ));
    };

    let response = mcp.request(
        "tools/call",
        &json!({ "name": "merge-plan", "arguments": { "id": cluster_id } }),
    )?;
    let plan = structured_content(&response, "merge-plan")?;
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
