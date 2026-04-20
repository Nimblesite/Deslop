//! Custom LSP methods in the `codededup/*` namespace
//! ([LSP-CUSTOM-METHODS]). Each method is a thin async forwarder onto
//! the [`codededup_core::live::LiveApi`] surface.

use std::path::PathBuf;

use codededup_core::live::{FindSimilarRequest, LiveApi};
use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result as LspResult;

use crate::backend::LspBackend;

/// Method name for `codededup/reportGet`.
pub const REPORT_GET: &str = "codededup/reportGet";
/// Method name for `codededup/reportForFile`.
pub const REPORT_FOR_FILE: &str = "codededup/reportForFile";
/// Method name for `codededup/reportForRange`.
pub const REPORT_FOR_RANGE: &str = "codededup/reportForRange";
/// Method name for `codededup/clusterById`.
pub const CLUSTER_BY_ID: &str = "codededup/clusterById";
/// Method name for `codededup/duplicatesFindSimilar`.
pub const FIND_SIMILAR: &str = "codededup/duplicatesFindSimilar";
/// Method name for `codededup/embeddingListModels`.
pub const LIST_MODELS: &str = "codededup/embeddingListModels";
/// Method name for `codededup/embeddingSetModel`.
pub const SET_MODEL: &str = "codededup/embeddingSetModel";
/// Method name for `codededup/sessionConfig`.
pub const SESSION_CONFIG: &str = "codededup/sessionConfig";

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
    Ok(serde_json::to_value(report.as_ref()).unwrap_or(serde_json::Value::Null))
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
    let result = backend.service().report_for_file(&params.path).await;
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
    let clusters = backend
        .service()
        .report_for_range(&params.path, params.start_byte, params.end_byte)
        .await;
    Ok(serde_json::to_value(clusters).unwrap_or(serde_json::Value::Null))
}

/// Forwards `cluster/byId`.
///
/// # Errors
///
/// Returns a JSON-RPC error when the underlying
/// [`codededup_core::live::LiveError::UnknownCluster`] fires.
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
/// # Errors
///
/// Returns a JSON-RPC error when the requested provider cannot be
/// reached or when the post-swap pipeline pass fails.
pub async fn embedding_set_model(
    backend: &LspBackend,
    params: SetModelParams,
) -> LspResult<serde_json::Value> {
    match backend
        .service()
        .embedding_set_model(
            &params.provider_id,
            &params.model_id,
            params.endpoint.as_deref(),
        )
        .await
    {
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

/// Lifts a [`codededup_core::live::LiveError`] into the JSON-RPC fault
/// shape `tower-lsp` exposes.
fn into_jsonrpc(error: &codededup_core::live::LiveError) -> tower_lsp::jsonrpc::Error {
    let wire = error.to_wire();
    let message = wire.message.clone();
    let mut out = tower_lsp::jsonrpc::Error::internal_error();
    out.message = message.into();
    out.data = serde_json::to_value(wire).ok();
    out
}
