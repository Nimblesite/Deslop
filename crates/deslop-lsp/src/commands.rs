//! `workspace/executeCommand` support for editor-neutral LSP commands
//! ([LSP-COMMANDS]).

use deslop_core::live::{ChangeSummary, LiveApi, LiveError, ReportChangedNotification};
use serde_json::{json, Value};
use tower_lsp::{
    jsonrpc::{Error, Result as LspResult},
    lsp_types::{
        ExecuteCommandOptions, ExecuteCommandParams, ShowDocumentParams, Url,
        WorkDoneProgressOptions,
    },
};

use crate::backend::{LspBackend, ReportChangedLspNotification};

/// Forces a full report refresh.
pub const REFRESH_REPORT: &str = "deslop.refreshReport";
/// Opens the current report virtual document.
pub const OPEN_REPORT: &str = "deslop.openReport";
/// Opens a cluster virtual document by id.
pub const OPEN_CLUSTER: &str = "deslop.openCluster";
/// Returns available embedding models for a client-side picker.
pub const PICK_EMBEDDING_MODEL: &str = "deslop.pickEmbeddingModel";
/// Toggles live incremental-cache behaviour.
pub const TOGGLE_INCREMENTAL: &str = "deslop.toggleIncremental";

/// Stable command list advertised during `initialize`.
const COMMANDS: &[&str] = &[
    REFRESH_REPORT,
    OPEN_CLUSTER,
    OPEN_REPORT,
    PICK_EMBEDDING_MODEL,
    TOGGLE_INCREMENTAL,
];

/// Builds the advertised command provider capability.
#[must_use]
pub fn provider() -> ExecuteCommandOptions {
    ExecuteCommandOptions {
        commands: COMMANDS
            .iter()
            .map(|command| (*command).to_owned())
            .collect(),
        work_done_progress_options: WorkDoneProgressOptions::default(),
    }
}

/// Dispatches a documented `workspace/executeCommand` verb.
///
/// # Errors
///
/// Returns JSON-RPC errors for unknown commands, invalid command
/// arguments, invalid generated URIs, client-side `showDocument`
/// failures, or live refresh failures.
pub async fn execute(
    backend: &LspBackend,
    params: ExecuteCommandParams,
) -> LspResult<Option<Value>> {
    match params.command.as_str() {
        REFRESH_REPORT => refresh_report(backend).await,
        OPEN_REPORT => show_uri(backend, OPEN_REPORT, "deslop://report").await,
        OPEN_CLUSTER => open_cluster(backend, &params.arguments).await,
        PICK_EMBEDDING_MODEL => pick_embedding_model(backend).await,
        TOGGLE_INCREMENTAL => toggle_incremental(backend).await,
        _ => Err(Error::method_not_found()),
    }
}

/// Re-runs the full live analysis session and returns a compact summary.
async fn refresh_report(backend: &LspBackend) -> LspResult<Option<Value>> {
    let session = backend.service().session();
    let (previous_generation, previous_report, delta) = {
        let mut guard = session.lock().await;
        let previous_generation = guard.generation();
        let previous_report = guard.report();
        let delta = guard.refresh_full().map_err(|error| live_error(&error))?;
        (previous_generation, previous_report, delta)
    };
    backend
        .service()
        .remember_snapshot(previous_generation, previous_report)
        .await;
    notify_if_changed(backend, &delta).await;
    Ok(Some(json!({
        "command": REFRESH_REPORT,
        "generation": delta.to_generation,
        "clustersAdded": delta.clusters_added.len(),
        "clustersRemoved": delta.clusters_removed.len(),
        "clustersUpdated": delta.clusters_updated.len(),
    })))
}

/// Pushes `deslop/reportChanged` only when refresh changed visible clusters.
async fn notify_if_changed(backend: &LspBackend, delta: &deslop_core::ReportDelta) {
    if delta.is_empty() {
        return;
    }
    backend
        .client()
        .send_notification::<ReportChangedLspNotification>(ReportChangedNotification {
            generation: delta.to_generation,
            summary: ChangeSummary::from_delta(delta),
        })
        .await;
}

/// Opens a concrete cluster virtual document from the first command argument.
async fn open_cluster(backend: &LspBackend, arguments: &[Value]) -> LspResult<Option<Value>> {
    let id = arguments
        .first()
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && !id.contains('/'))
        .ok_or_else(|| invalid_params("deslop.openCluster requires a cluster id"))?;
    let uri = format!("deslop://cluster/{id}");
    show_uri(backend, OPEN_CLUSTER, &uri).await
}

/// Sends `window/showDocument` for a Deslop virtual-document URI.
async fn show_uri(backend: &LspBackend, command: &str, uri: &str) -> LspResult<Option<Value>> {
    let shown = backend
        .client()
        .show_document(ShowDocumentParams {
            uri: parse_uri(uri)?,
            external: Some(false),
            take_focus: Some(true),
            selection: None,
        })
        .await?;
    Ok(Some(
        json!({ "command": command, "uri": uri, "shown": shown }),
    ))
}

/// Returns available embedding models so the client can present a picker.
async fn pick_embedding_model(backend: &LspBackend) -> LspResult<Option<Value>> {
    let models = backend.service().embedding_list_models().await;
    Ok(Some(
        json!({ "command": PICK_EMBEDDING_MODEL, "models": models }),
    ))
}

/// Flips the live incremental-cache flag and reports the new value.
async fn toggle_incremental(backend: &LspBackend) -> LspResult<Option<Value>> {
    let config = {
        let session = backend.service().session();
        let mut guard = session.lock().await;
        guard.toggle_incremental()
    };
    Ok(Some(json!({
        "command": TOGGLE_INCREMENTAL,
        "incremental": config.incremental,
    })))
}

/// Parses a Deslop virtual-document URI for `window/showDocument`.
fn parse_uri(uri: &str) -> LspResult<Url> {
    uri.parse()
        .map_err(|_| invalid_params("command produced an invalid deslop URI"))
}

/// Builds an `Invalid params` JSON-RPC error with structured data.
fn invalid_params(message: &str) -> Error {
    let mut error = Error::invalid_params(message.to_owned());
    error.data = Some(Value::String(message.to_owned()));
    error
}

/// Converts live-analysis failures into JSON-RPC internal errors.
fn live_error(error: &LiveError) -> Error {
    let mut out = Error::internal_error();
    out.message = error.to_string().into();
    out.data = serde_json::to_value(error.to_wire()).ok();
    out
}
