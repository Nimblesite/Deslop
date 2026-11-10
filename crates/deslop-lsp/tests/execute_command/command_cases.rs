//! One assertion block per [LSP-COMMANDS] command: what the server
//! advertises, what each command returns, and the report shapes
//! ([OUTPUT-HUMAN-HTML]) the open-report command renders.

use std::path::Path;

use anyhow::{anyhow, Result};
use deslop_lsp::{
    commands::{
        OPEN_CLUSTER, OPEN_REPORT, PICK_EMBEDDING_MODEL, REFRESH_REPORT, TOGGLE_INCREMENTAL,
    },
    notifications::REPORT_CHANGED,
    LspBackend,
};
use serde_json::{json, Value};
use tower_lsp::{jsonrpc::ErrorCode, ClientSocket, LspService};

use super::{
    harness::{
        advertised_commands, assert_json_bool, assert_json_str, drain_client_frames,
        execute_in_process, execute_raw_in_process, only_show_request, response_error,
        show_external, show_take_focus, show_uri,
    },
    BETA_FILE, CLUSTERS_REMOVED_POINTER, COMMAND_FIELD, NOTIFIED_GENERATION_POINTER,
    NOTIFIED_REMOVED_POINTER, PROVIDER_ID_FIELD,
};

pub(super) fn assert_advertised_commands(init: &Value) -> Result<()> {
    let commands = advertised_commands(init)?;
    assert_eq!(commands.len(), 7, "unexpected command list: {commands:?}");
    assert!(commands.contains(&REFRESH_REPORT.to_owned()));
    assert!(commands.contains(&OPEN_CLUSTER.to_owned()));
    assert!(commands.contains(&OPEN_REPORT.to_owned()));
    assert!(commands.contains(&"deslop.lsp.renderHtmlReport".to_owned()));
    assert!(commands.contains(&"deslop.lsp.reportJson".to_owned()));
    assert!(commands.contains(&PICK_EMBEDDING_MODEL.to_owned()));
    assert!(commands.contains(&TOGGLE_INCREMENTAL.to_owned()));
    Ok(())
}

pub(super) async fn assert_open_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) =
        execute_in_process(service, socket, json!({ (COMMAND_FIELD): OPEN_REPORT })).await?;
    assert_eq!(shows.len(), 1, "expected one showDocument");
    let show = only_show_request(&shows)?;
    assert_eq!(show_uri(show)?, "deslop://report");
    assert_eq!(show_take_focus(show), Some(true));
    assert_eq!(show_external(show), Some(false));
    assert_json_str(&response, "/command", OPEN_REPORT);
    Ok(())
}

pub(super) async fn assert_render_html_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.lsp.renderHtmlReport" }),
    )
    .await?;
    assert!(shows.is_empty(), "render must not open documents");
    let html = response
        .as_str()
        .ok_or_else(|| anyhow!("renderHtmlReport must return an HTML string: {response}"))?;
    assert_well_formed_report_html(html);
    assert_report_shows_real_clusters(html);
    Ok(())
}

/// Exercises `deslop.lsp.reportJson`: the structured report the `JetBrains`
/// tool window groups natively when the IDE has no embedded browser. Locks
/// the contract that native surface depends on — a JSON string carrying a
/// worst-first `clusters` array, each cluster with a clone-type `bucket` and
/// located `occurrences` — and the lean wire shape (no `schema_doc`).
pub(super) async fn assert_report_json_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.lsp.reportJson" }),
    )
    .await?;
    assert!(shows.is_empty(), "reportJson must not open documents");
    let json = response
        .as_str()
        .ok_or_else(|| anyhow!("reportJson must return a JSON string: {response}"))?;
    let report: Value = serde_json::from_str(json)?;
    assert_report_json_shape(&report)
}

/// Asserts the native-tree wire contract on a parsed `reportJson` payload.
pub(super) fn assert_report_json_shape(report: &Value) -> Result<()> {
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("reportJson needs a clusters array: {report}"))?;
    let first = clusters
        .first()
        .ok_or_else(|| anyhow!("csharp-small must yield clusters: {report}"))?;
    // Mass-only cutover: clusters carry rank and mass, never a retired
    // clone-type bucket ([MCP-TOOLS]). Pin rank presence and bucket
    // absence so the fat surface cannot leak back into the wire.
    assert!(
        first.get("rank").is_some() && first.get("severity").is_some(),
        "cluster needs its mass-ranked surface: {first}"
    );
    assert!(
        first.get("bucket").is_none(),
        "retired clone-type bucket must not leak into the wire: {first}"
    );
    let path = first.pointer("/occurrences/0/path").and_then(Value::as_str);
    assert!(path.is_some(), "cluster occurrence needs a path: {first}");
    let schema_doc = report.get("schema_doc").and_then(Value::as_str);
    assert!(
        schema_doc.map_or(true, str::is_empty),
        "reportJson must strip schema_doc for the lean wire shape",
    );
    Ok(())
}

/// Asserts the rendered report is not an empty styled shell but carries the
/// fixture's real worst-offender: the `csharp-small` clone is two renamed copies
/// of one method, so a populated `cluster-card` element — with a title, the
/// two-occurrence count, and a real source path — must appear. This is the exact
/// content every editor panel (the VS Code webview, the IDE tool window) displays,
/// so it guards against the "panel shows nothing" regression at the data source
/// ([OUTPUT-HUMAN-HTML]).
pub(super) fn assert_report_shows_real_clusters(html: &str) {
    assert!(
        html.contains("<article class=\"cluster-card"),
        "report must render a populated cluster-card element, not just the stylesheet rule"
    );
    assert!(
        html.contains("cluster-card__title"),
        "the populated cluster card must carry its title heading"
    );
    assert!(
        html.contains("2 occurrences"),
        "the csharp-small clone spans exactly two occurrences: {}",
        &html[..html.len().min(160)]
    );
    assert!(
        html.contains("Alpha.cs") || html.contains("Beta.cs"),
        "the card must reference a real fixture occurrence (Alpha.cs / Beta.cs)"
    );
}

/// Asserts the rendered string is the self-contained, CSS-bearing HTML
/// document the editor clients open in a browser tab ([OUTPUT-HUMAN-HTML]).
pub(super) fn assert_well_formed_report_html(html: &str) {
    assert!(
        html.starts_with("<!doctype html"),
        "report must be a full HTML document: {}",
        &html[..html.len().min(48)]
    );
    assert!(
        html.contains("data-theme=\"dark\""),
        "report must carry the dark-theme attribute the design system targets"
    );
    assert!(
        html.contains("class=\"report-shell\""),
        "report body must use the styled report-shell container"
    );
    assert!(
        html.contains(".report-shell{max-width:"),
        "report must inline its stylesheet so it renders offline with CSS"
    );
    assert!(
        html.contains(".cluster-card{"),
        "report must inline the cluster-card styling"
    );
    assert!(
        html.trim_end().ends_with("</html>"),
        "report must be a well-formed, fully-closed document"
    );
}

pub(super) async fn assert_open_cluster_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["abc123"] }),
    )
    .await?;
    assert_eq!(shows.len(), 1, "expected one cluster open");
    assert_eq!(
        show_uri(only_show_request(&shows)?)?,
        "deslop://cluster/abc123"
    );
    assert_json_bool(&response, "/shown", true);
    Ok(())
}

pub(super) async fn assert_toggle_incremental_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): TOGGLE_INCREMENTAL }),
    )
    .await?;
    assert!(shows.is_empty(), "toggle must not open documents");
    assert_json_bool(&response, "/incremental", false);
    Ok(())
}

pub(super) async fn assert_pick_embedding_model_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) = execute_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): PICK_EMBEDDING_MODEL }),
    )
    .await?;
    assert!(shows.is_empty(), "model picker must not open documents");
    // [REMOVE-STUB] When Ollama is unreachable in CI the models list
    // is empty; when Ollama is reachable every row reports `provider_id
    // == "ollama"`. The deterministic stub must never appear.
    let models = response
        .pointer("/models")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("/models field missing: {response}"))?;
    for entry in models {
        assert_ne!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("stub"),
            "pickEmbeddingModel response must not include the stub: {entry}",
        );
        assert_eq!(
            entry.get(PROVIDER_ID_FIELD).and_then(Value::as_str),
            Some("ollama"),
            "pickEmbeddingModel response must only expose ollama rows: {entry}",
        );
    }
    Ok(())
}

pub(super) async fn assert_open_cluster_invalid_id(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, requests) = execute_raw_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): OPEN_CLUSTER, "arguments": ["bad/id"] }),
    )
    .await?;
    assert!(
        requests.is_empty(),
        "invalid cluster command must not contact the client"
    );
    let error = response_error(response)?;
    assert_eq!(error.code, ErrorCode::InvalidParams);
    assert_eq!(
        error.data,
        Some(Value::String(
            "deslop.lsp.openCluster requires a cluster id".to_owned()
        ))
    );
    Ok(())
}

pub(super) async fn assert_unknown_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, requests) = execute_raw_in_process(
        service,
        socket,
        json!({ (COMMAND_FIELD): "deslop.unknown" }),
    )
    .await?;
    assert!(
        requests.is_empty(),
        "unknown command must not contact the client"
    );
    assert_eq!(response_error(response)?.code, ErrorCode::MethodNotFound);
    Ok(())
}

pub(super) async fn assert_refresh_report_command(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
) -> Result<()> {
    let (response, shows) =
        execute_in_process(service, socket, json!({ (COMMAND_FIELD): REFRESH_REPORT })).await?;
    assert!(shows.is_empty(), "refresh must not open documents");
    assert_json_str(&response, "/command", REFRESH_REPORT);
    assert!(response.pointer("/generation").is_some());
    assert!(response.pointer("/clustersAdded").is_some());
    Ok(())
}

/// [LSP-COMMANDS] [LIVE-NOTIFICATIONS] [PRINCIPLES-LIVE-IS-REACTIVE]
/// `assert_refresh_report_command` above proves a no-change refresh stays
/// silent. This proves the other half: a refresh whose delta actually retires
/// a cluster must push `deslop/reportChanged`, naming the generation it
/// advanced to and reporting the same removal count the command result does.
/// Without this the notification could be unwired entirely and every existing
/// assertion would still pass, because an empty delta returns before it.
pub(super) async fn assert_refresh_after_edit_notifies_report_changed(
    service: &mut LspService<LspBackend>,
    socket: &mut ClientSocket,
    workspace_root: &Path,
) -> Result<()> {
    std::fs::remove_file(workspace_root.join(BETA_FILE))?;
    let (response, during_call) =
        execute_in_process(service, socket, json!({ (COMMAND_FIELD): REFRESH_REPORT })).await?;
    let mut client_frames = during_call;
    client_frames.extend(drain_client_frames(socket));
    let removed = response
        .pointer(CLUSTERS_REMOVED_POINTER)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("refresh reported no removal count: {response}"))?;
    assert!(
        removed > 0,
        "deleting {BETA_FILE} must retire the fixture's cluster: {response}"
    );
    let changed = client_frames
        .iter()
        .find(|frame| frame.get("method").and_then(Value::as_str) == Some(REPORT_CHANGED))
        .ok_or_else(|| {
            anyhow!("a refresh that removed {removed} cluster(s) pushed no {REPORT_CHANGED}: {client_frames:?}")
        })?;
    assert!(
        changed
            .pointer(NOTIFIED_GENERATION_POINTER)
            .and_then(Value::as_u64)
            .is_some_and(|generation| generation > 0),
        "{REPORT_CHANGED} must name the generation it advanced to: {changed}"
    );
    assert_eq!(
        changed
            .pointer(NOTIFIED_REMOVED_POINTER)
            .and_then(Value::as_u64),
        Some(removed),
        "{REPORT_CHANGED} summary must agree with the command result: {changed}"
    );
    Ok(())
}
