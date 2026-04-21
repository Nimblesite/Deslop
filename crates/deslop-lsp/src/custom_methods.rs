//! Custom LSP methods in the `deslop/*` namespace
//! ([LSP-CUSTOM-METHODS]). Each method is a thin async forwarder onto
//! the [`deslop_core::live::LiveApi`] surface.

use std::path::PathBuf;

use deslop_core::{
    live::{FindSimilarRequest, LiveApi},
    report::{Report, ReportCluster, LIVE_WIRE_OCCURRENCE_CAP},
};
use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result as LspResult;

use crate::backend::LspBackend;

/// Method name for `deslop/reportSchemaDoc`. Serves the markdown
/// `schema_doc` that used to ride every `deslop/reportGet` response;
/// hoisting it behind its own method is what lets the live wire stay
/// small ([LSP-WIRE-BUDGET]).
pub const REPORT_SCHEMA_DOC: &str = "deslop/reportSchemaDoc";

/// Truncates one cluster to [`LIVE_WIRE_OCCURRENCE_CAP`] occurrences,
/// blanks the derivable summary/interpretation strings, and records
/// the pre-cap total so clients can page via [`CLUSTER_BY_ID`].
/// Mirrors [`Report::truncate_for_wire`] but for a single cluster
/// (used by `report/forFile` + `report/forRange`).
fn truncate_cluster_for_wire(cluster: &mut ReportCluster) {
    let total = cluster.occurrences.len().max(cluster.occurrences_total);
    cluster.occurrences_total = total;
    if cluster.occurrences.len() > LIVE_WIRE_OCCURRENCE_CAP {
        cluster.occurrences.truncate(LIVE_WIRE_OCCURRENCE_CAP);
        cluster.occurrences_truncated = true;
    }
    cluster.summary.clear();
    cluster.interpretation.clear();
}

/// Method name for `deslop/reportGet`.
pub const REPORT_GET: &str = "deslop/reportGet";
/// Method name for `deslop/reportForFile`.
pub const REPORT_FOR_FILE: &str = "deslop/reportForFile";
/// Method name for `deslop/reportForRange`.
pub const REPORT_FOR_RANGE: &str = "deslop/reportForRange";
/// Method name for `deslop/clusterById`.
pub const CLUSTER_BY_ID: &str = "deslop/clusterById";
/// Method name for `deslop/duplicatesFindSimilar`.
pub const FIND_SIMILAR: &str = "deslop/duplicatesFindSimilar";
/// Method name for `deslop/embeddingListModels`.
pub const LIST_MODELS: &str = "deslop/embeddingListModels";
/// Method name for `deslop/embeddingSetModel`.
pub const SET_MODEL: &str = "deslop/embeddingSetModel";
/// Method name for `deslop/sessionConfig`.
pub const SESSION_CONFIG: &str = "deslop/sessionConfig";

/// Parameters for the file/range/cluster lookups.
#[derive(Debug, Deserialize, Serialize)]
pub struct PathParams {
    /// Workspace-relative or absolute path.
    pub path: PathBuf,
}

/// Parameters for `report/forRange`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RangeParams {
    /// Path scoping the range.
    pub path: PathBuf,
    /// Inclusive start byte.
    pub start_byte: usize,
    /// Exclusive end byte.
    pub end_byte: usize,
}

/// Parameters for `cluster/byId`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterIdParams {
    /// Stable cluster id.
    pub id: String,
}

/// Parameters for `embedding/setModel`.
#[derive(Debug, Deserialize, Serialize)]
pub struct SetModelParams {
    /// Provider registry key.
    pub provider_id: String,
    /// Model identifier.
    pub model_id: String,
    /// Optional endpoint override.
    pub endpoint: Option<String>,
}

/// Forwards `report/get`. Accepts and ignores any params the client
/// happens to send (tower-lsp rejects `params: {}` unless the handler
/// declares a param type, and the VSIX sends `{}`).
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn report_get(
    backend: &LspBackend,
    _params: IgnoredParams,
) -> LspResult<serde_json::Value> {
    let report = backend.service().report_get().await;
    let slim: Report = (*report).clone().truncate_for_wire(LIVE_WIRE_OCCURRENCE_CAP);
    Ok(serde_json::to_value(&slim).unwrap_or(serde_json::Value::Null))
}

/// Forwards `report/schemaDoc`. Returns the markdown that used to ride
/// every [`REPORT_GET`] response ([LSP-WIRE-BUDGET]). Clients fetch this
/// lazily (e.g. VSIX `openSchemaDoc` command) so the live wire stays
/// small.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn report_schema_doc(
    backend: &LspBackend,
    _params: IgnoredParams,
) -> LspResult<serde_json::Value> {
    let report = backend.service().report_get().await;
    Ok(serde_json::Value::String(report.schema_doc.clone()))
}

/// Catch-all params for no-arg methods. Accepts any JSON value
/// (including `{}`, `null`, or missing) because the JSON-RPC clients we
/// talk to are inconsistent about sending `params` for no-arg methods.
#[derive(Debug, Default)]
pub struct IgnoredParams;

impl<'de> Deserialize<'de> for IgnoredParams {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde::de::IgnoredAny::deserialize(deserializer).map(|_| Self)
    }
}

impl Serialize for IgnoredParams {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}

/// Forwards `report/forFile`.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn report_for_file(
    backend: &LspBackend,
    params: PathParams,
) -> LspResult<serde_json::Value> {
    let mut result = backend.service().report_for_file(&params.path).await;
    for cluster in &mut result.clusters {
        truncate_cluster_for_wire(cluster);
    }
    Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null))
}

/// Forwards `report/forRange`.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn report_for_range(
    backend: &LspBackend,
    params: RangeParams,
) -> LspResult<serde_json::Value> {
    let mut clusters = backend
        .service()
        .report_for_range(&params.path, params.start_byte, params.end_byte)
        .await;
    for cluster in &mut clusters {
        truncate_cluster_for_wire(cluster);
    }
    Ok(serde_json::to_value(clusters).unwrap_or(serde_json::Value::Null))
}

/// Forwards `cluster/byId`.
///
/// # Errors
///
/// Returns a JSON-RPC error when the underlying
/// [`deslop_core::live::LiveError::UnknownCluster`] fires.
pub async fn cluster_by_id(
    backend: &LspBackend,
    params: ClusterIdParams,
) -> LspResult<serde_json::Value> {
    match backend.service().cluster_by_id(&params.id).await {
        Ok(cluster) => Ok(serde_json::to_value(cluster).unwrap_or(serde_json::Value::Null)),
        Err(error) => Err(into_jsonrpc(&error)),
    }
}

/// Forwards `duplicates/findSimilar`.
///
/// # Errors
///
/// Returns a JSON-RPC error for parse failures, unsupported languages,
/// and out-of-workspace paths.
pub async fn find_similar(
    backend: &LspBackend,
    params: FindSimilarRequest,
) -> LspResult<serde_json::Value> {
    match backend.service().find_similar(&params).await {
        Ok(result) => Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
        Err(error) => Err(into_jsonrpc(&error)),
    }
}

/// Forwards `embedding/listModels`.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn embedding_list_models(
    backend: &LspBackend,
    _params: IgnoredParams,
) -> LspResult<serde_json::Value> {
    let models = backend.service().embedding_list_models().await;
    Ok(serde_json::to_value(models).unwrap_or(serde_json::Value::Null))
}

/// Forwards `embedding/setModel`.
///
/// Installs a per-request embedding-progress reporter on the session so
/// the client sees `deslop/embeddingProgress` notifications around the
/// swap ([VSIX-SESSION-PROGRESS]). The reporter is cleared before the
/// response is returned.
///
/// # Errors
///
/// Returns a JSON-RPC error when the requested provider cannot be
/// reached or when the post-swap pipeline pass fails.
pub async fn embedding_set_model(
    backend: &LspBackend,
    params: SetModelParams,
) -> LspResult<serde_json::Value> {
    let (reporter, mut receiver) = crate::backend::embedding_progress_channel();
    {
        let session = backend.service().session();
        let mut guard = session.lock().await;
        guard.set_embedding_progress_reporter(Some(reporter));
    }
    let outcome = backend
        .service()
        .embedding_set_model(
            &params.provider_id,
            &params.model_id,
            params.endpoint.as_deref(),
        )
        .await;
    {
        let session = backend.service().session();
        let mut guard = session.lock().await;
        guard.set_embedding_progress_reporter(None);
    }
    while let Ok(event) = receiver.try_recv() {
        backend
            .client()
            .send_notification::<crate::backend::EmbeddingProgressNotification>(event)
            .await;
    }
    match outcome {
        Ok(result) => Ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
        Err(error) => Err(into_jsonrpc(&error)),
    }
}

/// Forwards `session/config`.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn session_config(
    backend: &LspBackend,
    _params: IgnoredParams,
) -> LspResult<serde_json::Value> {
    let config = backend.service().session_config().await;
    Ok(serde_json::to_value(config).unwrap_or(serde_json::Value::Null))
}

/// Lifts a [`deslop_core::live::LiveError`] into the JSON-RPC fault
/// shape `tower-lsp` exposes.
fn into_jsonrpc(error: &deslop_core::live::LiveError) -> tower_lsp::jsonrpc::Error {
    let wire = error.to_wire();
    let message = wire.message.clone();
    let mut out = tower_lsp::jsonrpc::Error::internal_error();
    out.message = message.into();
    out.data = serde_json::to_value(wire).ok();
    out
}
