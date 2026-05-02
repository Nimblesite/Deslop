//! Handler functions for each MCP tool call.

use std::path::{Path, PathBuf};

use deslop_core::{
    wire_generated::{
        EmbeddingModelList, FindSimilarResult, McpSessionConfig, RangeReport, ReportPageFilters,
        SchemaDocPayload, SetEmbeddingModelResponse, TopOffendersPayload,
    },
    Report,
};
use serde_json::Value;

use crate::{
    backend::{FindSimilarInput, McpBackend},
    page::{build_page, Pagination},
    protocol::{ErrorCode, JsonRpcError},
};

use super::backend_to_rpc;

/// Serialises a typed wire payload, falling back to JSON `null` only if
/// serde fails (which it cannot for the wire types here — they all derive
/// `Serialize`). Centralised so the handlers stay terse.
fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// `top-offenders` forwarder. Returns up to `n` full [`ReportCluster`]
/// records — occurrences, interpretation, signals, bucket — everything
/// the agent needs to act without a follow-up `cluster-by-id` call.
pub(super) fn call_top_offenders(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let n = extract_top_n(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(top_offenders_payload(&report, n))
}

/// `rescan` reloads the LSP-written state file and returns fresh top offenders.
pub(super) fn call_rescan(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let paths = extract_optional_paths(args, "paths")?;
    backend.mark_changed(&paths).map_err(backend_to_rpc)?;
    let n = extract_top_n(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(top_offenders_payload(&report, n))
}

/// Builds the shared top-offenders JSON payload for `top-offenders` and `rescan`.
fn top_offenders_payload(report: &Report, n: usize) -> Value {
    let payload = TopOffendersPayload {
        total_clusters: report.clusters.len(),
        n,
        clusters: report.clusters.iter().take(n).cloned().collect(),
    };
    to_value(&payload)
}

/// Extracts a positive `n` value from MCP tool arguments, defaulting to five.
fn extract_top_n(args: &Value) -> usize {
    args.get("n")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(5)
        .max(1)
}

/// `report-get` forwarder. Renders a slim paginated `ReportPage`
/// ([MCP-TOOL-REPORT-PAGINATION]).
pub(super) fn call_report_get(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let pagination = extract_pagination(args)?;
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let page = build_page(
        &report,
        backend.generation(),
        pagination,
        &ReportPageFilters::default(),
    );
    Ok(to_value(&page))
}

/// `report-query` forwarder. Same `ReportPage` shape as `report-get`
/// plus AND-combined filters ([MCP-TOOL-REPORT-QUERY]).
pub(super) fn call_report_query(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let pagination = extract_pagination(args)?;
    let filters = extract_filters(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let page = build_page(&report, backend.generation(), pagination, &filters);
    Ok(to_value(&page))
}

/// `schema-doc` returns the large markdown schema out-of-band so paged
/// report calls stay lean by default.
pub(super) fn call_schema_doc(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let payload = SchemaDocPayload {
        schema_doc: report.schema_doc.clone(),
    };
    Ok(to_value(&payload))
}

/// `report-for-file` forwarder.
pub(super) fn call_report_for_file(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let path = extract_string(args, "path")?;
    let clusters = backend
        .report_for_file(Path::new(&path))
        .map_err(backend_to_rpc)?;
    let payload = deslop_core::wire_generated::FileReport {
        path: PathBuf::from(path),
        clusters,
    };
    Ok(to_value(&payload))
}

/// `report-for-range` forwarder.
pub(super) fn call_report_for_range(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let path = extract_string(args, "path")?;
    let start_byte = extract_u64(args, "start_byte")?;
    let end_byte = extract_u64(args, "end_byte")?;
    reject_inverted_range(start_byte, end_byte)?;
    let start_byte_usize = usize::try_from(start_byte).unwrap_or(usize::MAX);
    let end_byte_usize = usize::try_from(end_byte).unwrap_or(usize::MAX);
    let clusters = backend
        .report_for_range(Path::new(&path), start_byte_usize, end_byte_usize)
        .map_err(backend_to_rpc)?;
    let payload = RangeReport {
        path: PathBuf::from(path),
        start_byte: start_byte_usize,
        end_byte: end_byte_usize,
        clusters,
    };
    Ok(to_value(&payload))
}

/// `find-similar` forwarder. Selects between the range and snippet
/// variants based on which fields were supplied.
pub(super) fn call_find_similar(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let has_range = args.get("path").is_some()
        && args.get("start_byte").is_some()
        && args.get("end_byte").is_some();
    let has_snippet = args.get("snippet").is_some() && args.get("language").is_some();
    if has_range == has_snippet {
        return Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            "find-similar requires exactly one of (path + start_byte + end_byte) or (snippet + language)",
        ));
    }
    let top_n = args
        .get("top_n")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(5);
    let output = if has_range {
        call_find_similar_range(backend, args, top_n)?
    } else {
        call_find_similar_snippet(backend, args, top_n)?
    };
    let payload = FindSimilarResult {
        clusters: output.clusters,
        below_min_nodes: output.below_min_nodes,
    };
    Ok(to_value(&payload))
}

/// Range variant of `find-similar`.
fn call_find_similar_range(
    backend: &dyn McpBackend,
    args: &Value,
    top_n: usize,
) -> Result<crate::backend::FindSimilarOutput, JsonRpcError> {
    let path = extract_string(args, "path")?;
    let start_byte = extract_u64(args, "start_byte")?;
    let end_byte = extract_u64(args, "end_byte")?;
    reject_inverted_range(start_byte, end_byte)?;
    let path_buf = PathBuf::from(&path);
    backend
        .find_similar(
            FindSimilarInput::Range {
                path: &path_buf,
                start_byte: usize::try_from(start_byte).unwrap_or(usize::MAX),
                end_byte: usize::try_from(end_byte).unwrap_or(usize::MAX),
            },
            top_n,
        )
        .map_err(backend_to_rpc)
}

/// Snippet variant of `find-similar`.
fn call_find_similar_snippet(
    backend: &dyn McpBackend,
    args: &Value,
    top_n: usize,
) -> Result<crate::backend::FindSimilarOutput, JsonRpcError> {
    let snippet = extract_string(args, "snippet")?;
    let language = extract_string(args, "language")?;
    backend
        .find_similar(
            FindSimilarInput::Snippet {
                snippet: &snippet,
                language: &language,
            },
            top_n,
        )
        .map_err(backend_to_rpc)
}

/// `cluster-by-id` forwarder.
pub(super) fn call_cluster_by_id(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let id = extract_string(args, "id")?;
    let cluster = backend.cluster_by_id(&id).map_err(backend_to_rpc)?;
    Ok(to_value(&cluster))
}

/// `list-embedding-models` forwarder.
pub(super) fn call_list_embedding_models(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let models = backend.list_embedding_models().map_err(backend_to_rpc)?;
    let payload = EmbeddingModelList { models };
    Ok(to_value(&payload))
}

/// `set-embedding-model` forwarder.
pub(super) fn call_set_embedding_model(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    require_user_initiated(args)?;
    let provider_id = extract_string(args, "provider_id")?;
    let model_id = extract_string(args, "model_id")?;
    let endpoint = args
        .get("endpoint")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let spec = backend
        .set_embedding_model(&provider_id, &model_id, endpoint.as_deref())
        .map_err(backend_to_rpc)?;
    let payload = SetEmbeddingModelResponse {
        provider_id: spec.provider_id,
        model_id: spec.model_id,
        model_version: spec.model_version,
        dimensions: spec.dimensions,
    };
    Ok(to_value(&payload))
}

/// `session-config` forwarder.
pub(super) fn call_session_config(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let snapshot = backend.session_config().map_err(backend_to_rpc)?;
    let payload = McpSessionConfig {
        root: snapshot.root,
        min_nodes: snapshot.min_nodes,
        languages: snapshot.languages,
        incremental: snapshot.incremental,
        embedding_provenance: snapshot.embedding_provenance,
        cache_stats: snapshot.cumulative_cache_stats,
        generation: backend.generation(),
    };
    Ok(to_value(&payload))
}

/// Requires explicit user consent for model-changing tool calls.
pub(super) fn require_user_initiated(args: &Value) -> Result<(), JsonRpcError> {
    if args
        .get("user_initiated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }
    Err(JsonRpcError::new(
        ErrorCode::InvalidParams,
        "set-embedding-model requires explicit user_initiated=true",
    ))
}

/// Extracts the required `offset` + `limit` pagination knobs.
pub(super) fn extract_pagination(args: &Value) -> Result<Pagination, JsonRpcError> {
    let offset = extract_u64(args, "offset")?;
    let limit = extract_u64(args, "limit")?;
    Ok(Pagination {
        offset: usize::try_from(offset).unwrap_or(usize::MAX),
        limit: usize::try_from(limit).unwrap_or(usize::MAX),
    })
}

/// Extracts the optional `report-query` filter knobs. Unknown / wrong-typed
/// fields are quietly ignored — the JSON schema layer rejects them up
/// front when a strict client is in use.
pub(super) fn extract_filters(args: &Value) -> ReportPageFilters {
    ReportPageFilters {
        language: args
            .get("language")
            .and_then(Value::as_str)
            .map(str::to_owned),
        bucket: args
            .get("bucket")
            .and_then(Value::as_str)
            .map(str::to_owned),
        path_contains: args
            .get("path_contains")
            .and_then(Value::as_str)
            .map(str::to_owned),
        min_score: args.get("min_score").and_then(Value::as_f64),
        min_size: args
            .get("min_size")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok()),
    }
}

/// Extracts a required string field from `args`.
pub(super) fn extract_string(args: &Value, field: &str) -> Result<String, JsonRpcError> {
    args.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            JsonRpcError::new(
                ErrorCode::InvalidParams,
                format!("missing or non-string parameter {field:?}"),
            )
        })
}

/// Extracts an optional string path array from `args`.
pub(super) fn extract_optional_paths(
    args: &Value,
    field: &str,
) -> Result<Vec<PathBuf>, JsonRpcError> {
    let Some(value) = args.get(field) else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("parameter {field:?} must be an array of strings"),
        ));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(PathBuf::from).ok_or_else(|| {
                JsonRpcError::new(
                    ErrorCode::InvalidParams,
                    format!("parameter {field:?} must be an array of strings"),
                )
            })
        })
        .collect()
}

/// Extracts a required non-negative integer field from `args`.
pub(super) fn extract_u64(args: &Value, field: &str) -> Result<u64, JsonRpcError> {
    args.get(field).and_then(Value::as_u64).ok_or_else(|| {
        JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("missing or non-integer parameter {field:?}"),
        )
    })
}

/// Rejects byte ranges with `end < start`.
pub(super) fn reject_inverted_range(start: u64, end: u64) -> Result<(), JsonRpcError> {
    if end < start {
        return Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("end_byte ({end}) must be >= start_byte ({start})"),
        ));
    }
    Ok(())
}
