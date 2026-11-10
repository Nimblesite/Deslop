//! E2E coverage for [LSP-COMMANDS] `workspace/executeCommand`.
//!
//! Drives the real `deslop-lsp` binary over stdio and responds to
//! client-bound `window/showDocument` requests so command handlers run
//! through the same transport a real editor uses.

use anyhow::{anyhow, Result};
use command_cases::{
    assert_advertised_commands, assert_open_cluster_command, assert_open_cluster_invalid_id,
    assert_open_report_command, assert_pick_embedding_model_command,
    assert_refresh_after_edit_notifies_report_changed, assert_refresh_report_command,
    assert_render_html_report_command, assert_report_json_command,
    assert_toggle_incremental_command, assert_unknown_command,
};
use deslop_lsp::commands::{
    OPEN_CLUSTER, OPEN_REPORT, PICK_EMBEDDING_MODEL, REFRESH_REPORT, TOGGLE_INCREMENTAL,
};
use harness::{
    call_with_show_document_response, execute, in_process_lsp, initialize_in_process,
    only_show_request, show_external, show_take_focus, show_uri,
};
use serde_json::{json, Value};

use crate::common::{call, copy_fixture, session::FixtureSession};

#[path = "execute_command/command_cases.rs"]
mod command_cases;
#[path = "execute_command/harness.rs"]
mod harness;

/// The two-file C# workspace these cases are served from.
const CSHARP_FIXTURE: &str = "csharp-small";
const EXECUTE_COMMAND: &str = "workspace/executeCommand";
const COMMAND_FIELD: &str = "command";
const RESULT_COMMAND_POINTER: &str = "/result/command";
const PROVIDER_ID_FIELD: &str = "provider_id";
/// The `csharp-small` file whose deletion retires the fixture's only cluster.
const BETA_FILE: &str = "Beta.cs";
/// Count of clusters the refresh response reports as removed.
const CLUSTERS_REMOVED_POINTER: &str = "/clustersRemoved";
/// Generation the pushed notification advanced to.
const NOTIFIED_GENERATION_POINTER: &str = "/params/generation";
/// Removed-cluster count carried by the pushed notification's summary.
const NOTIFIED_REMOVED_POINTER: &str = "/params/summary/clusters_removed";
#[test]
fn execute_command_provider_advertises_and_opens_virtual_documents() -> Result<()> {
    let mut lsp = FixtureSession::open(CSHARP_FIXTURE)?;
    assert_advertised_commands(&lsp.init)?;

    let (report_response, report_shows) = call_with_show_document_response(
        &mut lsp.stdin,
        &mut lsp.stdout,
        &json!({ (COMMAND_FIELD): OPEN_REPORT }),
    )?;
    assert_eq!(report_shows.len(), 1, "expected one showDocument request");
    let report_show = only_show_request(&report_shows)?;
    assert_eq!(show_uri(report_show)?, "deslop://report");
    assert_eq!(show_take_focus(report_show), Some(true));
    assert_eq!(show_external(report_show), Some(false));
    assert_eq!(
        report_response
            .pointer("/result/uri")
            .and_then(Value::as_str),
        Some("deslop://report")
    );

    let (cluster_response, cluster_shows) = call_with_show_document_response(
        &mut lsp.stdin,
        &mut lsp.stdout,
        &json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["abc123"] }),
    )?;
    assert_eq!(cluster_shows.len(), 1, "expected one cluster document open");
    let cluster_show = only_show_request(&cluster_shows)?;
    assert_eq!(show_uri(cluster_show)?, "deslop://cluster/abc123");
    assert_eq!(
        cluster_response
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(OPEN_CLUSTER)
    );
    assert_eq!(
        cluster_response
            .pointer("/result/shown")
            .and_then(Value::as_bool),
        Some(true)
    );

    drop(lsp);
    Ok(())
}

#[test]
fn execute_command_dispatches_refresh_models_and_incremental_toggle() -> Result<()> {
    let mut lsp = FixtureSession::open(CSHARP_FIXTURE)?;
    let initial_config = lsp.call("deslop/sessionConfig", &json!({}))?;
    assert_eq!(
        initial_config
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(true)
    );

    let toggled = execute(
        &mut lsp.stdin,
        &mut lsp.stdout,
        &json!({
            (COMMAND_FIELD): TOGGLE_INCREMENTAL
        }),
    )?;
    assert_eq!(
        toggled
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(TOGGLE_INCREMENTAL)
    );
    assert_eq!(
        toggled
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(false)
    );
    let updated_config = call(
        &mut lsp.stdin,
        &mut lsp.stdout,
        "deslop/sessionConfig",
        &json!({}),
    )?;
    assert_eq!(
        updated_config
            .pointer("/result/incremental")
            .and_then(Value::as_bool),
        Some(false)
    );

    let refreshed = execute(
        &mut lsp.stdin,
        &mut lsp.stdout,
        &json!({
            (COMMAND_FIELD): REFRESH_REPORT
        }),
    )?;
    assert_eq!(
        refreshed
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(REFRESH_REPORT)
    );
    assert_eq!(
        refreshed
            .pointer("/result/generation")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert!(refreshed.pointer("/result/clustersAdded").is_some());
    assert!(refreshed.pointer("/result/clustersRemoved").is_some());
    assert!(refreshed.pointer("/result/clustersUpdated").is_some());

    let models = execute(
        &mut lsp.stdin,
        &mut lsp.stdout,
        &json!({
            (COMMAND_FIELD): PICK_EMBEDDING_MODEL
        }),
    )?;
    assert_eq!(
        models
            .pointer(RESULT_COMMAND_POINTER)
            .and_then(Value::as_str),
        Some(PICK_EMBEDDING_MODEL)
    );
    // [REMOVE-STUB] Production listing only carries Ollama-provided
    // entries — when Ollama is unreachable the list is empty, when it
    // is running every row reports `provider_id == "ollama"`. Either
    // way no `stub` row may appear.
    let models_array = models
        .pointer("/result/models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("models field missing: {models}"))?;
    for entry in models_array {
        assert_ne!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("stub"),
            "production payload must not include the deterministic stub: {entry}",
        );
        assert_eq!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("ollama"),
            "production listing must only expose ollama-provided models: {entry}",
        );
    }

    drop(lsp);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn execute_command_handlers_run_in_process_for_coverage() -> Result<()> {
    let workspace = copy_fixture("csharp-small")?;
    let (mut service, mut socket) = in_process_lsp(workspace.path());
    let init = initialize_in_process(&mut service).await?;
    assert_advertised_commands(&init)?;
    assert_open_report_command(&mut service, &mut socket).await?;
    assert_render_html_report_command(&mut service, &mut socket).await?;
    assert_report_json_command(&mut service, &mut socket).await?;
    assert_open_cluster_command(&mut service, &mut socket).await?;
    assert_toggle_incremental_command(&mut service, &mut socket).await?;
    assert_pick_embedding_model_command(&mut service, &mut socket).await?;
    assert_open_cluster_invalid_id(&mut service, &mut socket).await?;
    assert_unknown_command(&mut service, &mut socket).await?;
    assert_refresh_report_command(&mut service, &mut socket).await?;
    assert_refresh_after_edit_notifies_report_changed(&mut service, &mut socket, workspace.path())
        .await?;
    Ok(())
}
