//! Custom LSP methods in the `deslop/*` namespace
//! ([LSP-CUSTOM-METHODS]). Each method is a thin async forwarder onto
//! the [`deslop_core::live::LiveApi`] surface.

use std::time::Instant;

use deslop_core::{
    live::{FindSimilarRequest, LiveApi},
    render::{render_cluster_markdown, render_text},
    report::{occurrence_count, Report, ReportCluster, LIVE_WIRE_OCCURRENCE_CAP},
    wire_generated::{
        ClusterIdParams, PairComparisonParams, PathParams, RangeParams, ReportDeltaParams,
        SetModelParams, VirtualDocumentParams,
    },
};
use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result as LspResult;

use crate::backend::LspBackend;
use crate::observability::CpuPhase;

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
    // `evidence_verdict` deliberately survives the blanking: it is a
    // measured reading of signals the client can no longer recompute
    // once the occurrence list is capped, and re-deriving it client-side
    // is what the one-calculation rule forbids.
    let count = occurrence_count(cluster);
    cluster.occurrences_total = count;
    cluster.occurrence_count = count;
    if cluster.occurrences.len() > LIVE_WIRE_OCCURRENCE_CAP {
        cluster.occurrences.truncate(LIVE_WIRE_OCCURRENCE_CAP);
        cluster.occurrences_truncated = true;
    }
}

/// Method name for `deslop/reportGet`.
pub const REPORT_GET: &str = "deslop/reportGet";
/// Method name for `deslop/reportDelta`.
pub const REPORT_DELTA: &str = "deslop/reportDelta";
/// Method name for `deslop/reportForFile`.
pub const REPORT_FOR_FILE: &str = "deslop/reportForFile";
/// Method name for `deslop/reportForRange`.
pub const REPORT_FOR_RANGE: &str = "deslop/reportForRange";
/// Method name for `deslop/clusterById`.
pub const CLUSTER_BY_ID: &str = "deslop/clusterById";
/// Method name for `deslop/pairCompare`.
pub const PAIR_COMPARE: &str = "deslop/pairCompare";
/// Method name for `deslop/duplicatesFindSimilar`.
pub const FIND_SIMILAR: &str = "deslop/duplicatesFindSimilar";
/// Method name for `deslop/embeddingListModels`.
pub const LIST_MODELS: &str = "deslop/embeddingListModels";
/// Method name for `deslop/embeddingSetModel`.
pub const SET_MODEL: &str = "deslop/embeddingSetModel";
/// Method name for `deslop/sessionConfig`.
pub const SESSION_CONFIG: &str = "deslop/sessionConfig";
/// Method name for `deslop/cpuReport`.
pub const CPU_REPORT: &str = "deslop/cpuReport";

/// Method name for `deslop/virtualDocument`. Resolves `deslop://schema`,
/// `deslop://report`, and `deslop://cluster/<id>` URIs into markdown so
/// every LSP client renders the same editor-neutral readonly view
/// ([LSP-EDITOR-SURFACES]).
pub const VIRTUAL_DOCUMENT: &str = "deslop/virtualDocument";

// `PathParams`, `ReportDeltaParams`, `RangeParams`, `ClusterIdParams`,
// `VirtualDocumentParams`, and `SetModelParams` live in
// `deslop_core::wire_generated`, generated from
// `docs/models/live-ipc.td` so the VSIX and the LSP share one wire
// definition. They're re-exported into the module's `use` block at the
// top of the file.

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
    backend.observability().record_handler(REPORT_GET);
    let started = Instant::now();
    let _phase = backend
        .observability()
        .start_phase(CpuPhase::ReportRendering, Vec::new());
    let report = backend.service().report_get().await;
    let slim: Report = (*report)
        .clone()
        .truncate_for_wire(LIVE_WIRE_OCCURRENCE_CAP);
    let clusters = slim.clusters.len();
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        handler = REPORT_GET,
        clusters,
        elapsed_ms,
        "deslop/reportGet handler complete"
    );
    Ok(serde_json::to_value(&slim).unwrap_or(serde_json::Value::Null))
}

/// Returns the current CPU/work observability snapshot.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn cpu_report(
    backend: &LspBackend,
    _params: IgnoredParams,
) -> LspResult<serde_json::Value> {
    backend.observability().record_handler(CPU_REPORT);
    Ok(serde_json::to_value(backend.observability().snapshot()).unwrap_or(serde_json::Value::Null))
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
    schema_doc_value(backend).await
}

/// Returns the live schema markdown as a JSON string value.
async fn schema_doc_value(backend: &LspBackend) -> LspResult<serde_json::Value> {
    let report = backend.service().report_get().await;
    Ok(serde_json::Value::String(report.schema_doc.clone()))
}

/// Forwards `report/delta`.
///
/// # Errors
///
/// Never errors today — kept fallible to match the JSON-RPC method
/// signature.
pub async fn report_delta(
    backend: &LspBackend,
    params: ReportDeltaParams,
) -> LspResult<serde_json::Value> {
    let since = match params.since_generation {
        Some(generation) => generation,
        None => previous_generation(backend).await,
    };
    let delta = backend.service().report_delta(since).await;
    Ok(serde_json::to_value(delta).unwrap_or(serde_json::Value::Null))
}

/// Returns the generation immediately before the current live snapshot.
async fn previous_generation(backend: &LspBackend) -> u64 {
    let session = backend.service().session();
    let guard = session.lock().await;
    guard.generation().saturating_sub(1)
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

/// Recomputes evidence for exactly the two requested occurrence endpoints.
///
/// # Errors
///
/// Returns a JSON-RPC error when either endpoint is absent from the current
/// analysis generation or active embedding evidence cannot be measured.
pub async fn pair_compare(
    backend: &LspBackend,
    params: PairComparisonParams,
) -> LspResult<serde_json::Value> {
    match backend.service().pair_compare(&params).await {
        Ok(comparison) => Ok(serde_json::to_value(comparison).unwrap_or(serde_json::Value::Null)),
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
/// Queues the selected-model embedding refresh. The backend installs
/// a persistent progress reporter at startup, so this request returns
/// after the job is queued while progress notifications stream
/// independently ([VSIX-SESSION-PROGRESS]).
///
/// # Errors
///
/// Returns a JSON-RPC error when the requested provider cannot be
/// reached or when the post-swap pipeline pass fails.
pub async fn embedding_set_model(
    backend: &LspBackend,
    params: SetModelParams,
) -> LspResult<serde_json::Value> {
    let outcome = backend
        .service()
        .embedding_set_model(
            &params.provider_id,
            &params.model_id,
            params.endpoint.as_deref(),
        )
        .await;
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

/// Serves markdown for a `deslop://` virtual-document URI
/// ([LSP-EDITOR-SURFACES], [LSP-VIRTUAL-DOC]).
///
/// # Errors
///
/// Returns `invalid_params` for any URI that is not one of
/// `deslop://schema`, `deslop://report`, or `deslop://cluster/<id>`, and
/// surfaces [`deslop_core::live::LiveError::UnknownCluster`] when the
/// embedded id does not match an active cluster.
pub async fn virtual_document(
    backend: &LspBackend,
    params: VirtualDocumentParams,
) -> LspResult<serde_json::Value> {
    match parse_virtual_uri(&params.uri) {
        Some(VirtualDocument::Schema) => schema_doc_value(backend).await,
        Some(VirtualDocument::Report) => {
            let report = backend.service().report_get().await;
            Ok(serde_json::Value::String(render_text(&report)))
        }
        Some(VirtualDocument::Cluster(id)) => match backend.service().cluster_by_id(&id).await {
            Ok(cluster) => Ok(serde_json::Value::String(render_cluster_markdown(
                &cluster,
                |path| std::fs::read_to_string(path).ok(),
            ))),
            Err(error) => Err(into_jsonrpc(&error)),
        },
        None => Err(invalid_params(format!(
            "unsupported deslop:// uri: {}",
            params.uri
        ))),
    }
}

/// The three virtual-document URI shapes served by [`virtual_document`].
#[derive(Debug)]
enum VirtualDocument {
    /// Static JSON schema documentation.
    Schema,
    /// Current duplicate report rendered as text.
    Report,
    /// One cluster detail document addressed by stable cluster id.
    Cluster(String),
}

/// Parses a `deslop://` URI into a routing tag. Returns `None` for any
/// other scheme or malformed path.
fn parse_virtual_uri(uri: &str) -> Option<VirtualDocument> {
    let rest = uri.strip_prefix("deslop://")?;
    match rest {
        "schema" => Some(VirtualDocument::Schema),
        "report" => Some(VirtualDocument::Report),
        _ => {
            let id = rest.strip_prefix("cluster/")?;
            if id.is_empty() || id.contains('/') {
                return None;
            }
            Some(VirtualDocument::Cluster(id.to_owned()))
        }
    }
}

/// Builds a JSON-RPC `-32602 Invalid params` error with `message`.
fn invalid_params(message: String) -> tower_lsp::jsonrpc::Error {
    let mut error = tower_lsp::jsonrpc::Error::invalid_params(message.clone());
    error.data = Some(serde_json::Value::String(message));
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_schema_uri() {
        assert!(matches!(
            parse_virtual_uri("deslop://schema"),
            Some(VirtualDocument::Schema)
        ));
    }

    #[test]
    fn parses_report_uri() {
        assert!(matches!(
            parse_virtual_uri("deslop://report"),
            Some(VirtualDocument::Report)
        ));
    }

    #[test]
    fn parses_cluster_uri_with_id() {
        let parsed = parse_virtual_uri("deslop://cluster/abc-123");
        assert!(
            matches!(parsed, Some(VirtualDocument::Cluster(ref id)) if id == "abc-123"),
            "expected Cluster variant, got {parsed:?}"
        );
    }

    #[test]
    fn rejects_empty_cluster_id() {
        assert!(parse_virtual_uri("deslop://cluster/").is_none());
    }

    #[test]
    fn rejects_nested_cluster_path() {
        assert!(parse_virtual_uri("deslop://cluster/abc/extra").is_none());
    }

    #[test]
    fn rejects_non_deslop_scheme() {
        assert!(parse_virtual_uri("http://example").is_none());
    }

    #[test]
    fn rejects_unknown_authority() {
        assert!(parse_virtual_uri("deslop://whatever").is_none());
    }
}
