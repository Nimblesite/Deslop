//! Handler functions for the canonical MCP tools.

use std::path::{Path, PathBuf};

use deslop_core::{
    report::ReportCluster,
    wire_generated::{
        DuplicatesFilters, EmbeddingModelList, McpSessionConfig, PairComparisonParams,
        RescanPayload, SchemaDocPayload, SetEmbeddingModelResponse,
    },
    Report,
};
use serde_json::{json, Value};

use crate::{
    backend::{FindSimilarInput, McpBackend},
    page::{build_page, Detail, PageShape, Pagination},
    protocol::{jsonrpc_error, ErrorCode, JsonRpcError},
};

use super::backend_to_rpc;

/// Default total-occurrence budget for full cluster responses.
const DEFAULT_MAX_OCCURRENCES: usize = 15;
/// Default page size.
const DEFAULT_LIMIT: usize = 5;
/// Maximum occurrences returned by one cluster drill-in.
const CLUSTER_OCCURRENCE_PAGE: usize = 100;
/// Tool-argument field naming a source file.
const PATH_FIELD: &str = "path";
/// Tool-argument field carrying an inclusive byte start.
const START_BYTE_FIELD: &str = "start_byte";
/// Tool-argument field carrying an exclusive byte end.
const END_BYTE_FIELD: &str = "end_byte";

/// Serialises one generated wire payload.
fn to_value<T: serde::Serialize>(value: &T) -> Result<Value, JsonRpcError> {
    serde_json::to_value(value).map_err(|error| {
        jsonrpc_error(
            ErrorCode::InternalError,
            format!("failed to serialize tool result: {error}"),
        )
    })
}

/// Returns the one mass-ranked duplicate page.
pub(super) fn call_duplicates(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let candidates = scoped_clusters(backend, args, &report)?;
    duplicates_page_value(backend, args, &report, &candidates)
}

/// Forces a refresh and returns a page from the same fresh generation.
pub(super) fn call_rescan(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let paths = extract_optional_paths(args, "paths")?;
    let progress = backend.mark_changed(&paths).map_err(backend_to_rpc)?;
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let page = build_page(
        &report,
        progress.generation,
        &report.clusters,
        extract_page_shape(args)?,
        &extract_filters(args)?,
    );
    to_value(&RescanPayload {
        generation: progress.generation,
        summary: progress.summary,
        page,
    })
}

/// Returns pair evidence for the two exact endpoints supplied by the caller.
pub(super) fn call_compare_pair(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let params: PairComparisonParams = serde_json::from_value(args.clone()).map_err(|error| {
        jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("compare-pair requires exact left and right endpoints: {error}"),
        )
    })?;
    if params.left == params.right {
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            "compare-pair endpoints must be distinct",
        ));
    }
    let comparison = backend.compare_pair(&params).map_err(backend_to_rpc)?;
    to_value(&comparison)
}

/// Returns the large schema markdown outside report pages.
pub(super) fn call_schema_doc(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let report = backend.report_get().map_err(backend_to_rpc)?;
    to_value(&SchemaDocPayload {
        schema_doc: report.schema_doc.clone(),
    })
}

/// Finds similar clusters for an explicit range or in-memory snippet.
pub(super) fn call_find_similar(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let has_range = has_complete_range(args);
    let has_snippet = args.get("snippet").is_some() && args.get("language").is_some();
    if has_range == has_snippet {
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            "find-similar requires exactly one range or snippet input",
        ));
    }
    let limit = extract_limit(args);
    let output = if has_range {
        find_similar_range(backend, args, limit)?
    } else {
        find_similar_snippet(backend, args, limit)?
    };
    let report = backend.report_get().map_err(backend_to_rpc)?;
    let mut value = duplicates_page_value(backend, args, &report, &output.clusters)?;
    if let Some(object) = value.as_object_mut() {
        let _old = object.insert("below_min_nodes".to_owned(), json!(output.below_min_nodes));
    }
    Ok(value)
}

/// Returns one stable cluster, paging only its occurrence list.
pub(super) fn call_cluster_by_id(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let id = extract_string(args, "id")?;
    let offset = extract_optional_usize(args, "offset").unwrap_or(0);
    let mut cluster = backend.cluster_by_id(&id).map_err(backend_to_rpc)?;
    let total = cluster.occurrences.len();
    let start = offset.min(total);
    let end = start.saturating_add(CLUSTER_OCCURRENCE_PAGE).min(total);
    cluster.occurrences = cluster
        .occurrences
        .get(start..end)
        .unwrap_or_default()
        .to_vec();
    cluster.occurrences_total = total;
    cluster.occurrences_truncated = start > 0 || end < total;
    to_value(&cluster)
}

/// Returns one mechanical merge plan.
pub(super) fn call_merge_plan(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let id = extract_string(args, "id")?;
    let plan = backend.merge_plan(&id).map_err(backend_to_rpc)?;
    to_value(&plan)
}

/// Handles session metadata and embedding-model management.
pub(super) fn call_session(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    match args.get("action").and_then(Value::as_str).unwrap_or("get") {
        "get" => session_config(backend),
        "list-embedding-models" => list_embedding_models(backend),
        "set-embedding-model" => set_embedding_model(backend, args),
        action => Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("unknown session action {action:?}"),
        )),
    }
}

/// Builds a duplicates page value from scoped candidates.
fn duplicates_page_value(
    backend: &dyn McpBackend,
    args: &Value,
    report: &Report,
    candidates: &[ReportCluster],
) -> Result<Value, JsonRpcError> {
    let page = build_page(
        report,
        backend.generation(),
        candidates,
        extract_page_shape(args)?,
        &extract_filters(args)?,
    );
    to_value(&page)
}

/// Resolves the optional duplicates file/range scope.
fn scoped_clusters(
    backend: &dyn McpBackend,
    args: &Value,
    report: &Report,
) -> Result<Vec<ReportCluster>, JsonRpcError> {
    let has_path = args.get(PATH_FIELD).is_some();
    let has_start = args.get(START_BYTE_FIELD).is_some();
    let has_end = args.get(END_BYTE_FIELD).is_some();
    match (has_path, has_start, has_end) {
        (false, false, false) => Ok(report.clusters.clone()),
        (true, false, false) => {
            let path = extract_string(args, PATH_FIELD)?;
            backend
                .report_for_file(Path::new(&path))
                .map_err(backend_to_rpc)
        }
        (true, true, true) => scoped_range(backend, args),
        _ => Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            "range scope requires path, start_byte, and end_byte together",
        )),
    }
}

/// Resolves one complete range scope.
fn scoped_range(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Vec<ReportCluster>, JsonRpcError> {
    let path = extract_string(args, PATH_FIELD)?;
    let start = extract_usize(args, START_BYTE_FIELD)?;
    let end = extract_usize(args, END_BYTE_FIELD)?;
    reject_inverted_range(start, end)?;
    backend
        .report_for_range(Path::new(&path), start, end)
        .map_err(backend_to_rpc)
}

/// Whether all three range fields are present.
fn has_complete_range(args: &Value) -> bool {
    args.get(PATH_FIELD).is_some()
        && args.get(START_BYTE_FIELD).is_some()
        && args.get(END_BYTE_FIELD).is_some()
}

/// Calls the range variant of find-similar.
fn find_similar_range(
    backend: &dyn McpBackend,
    args: &Value,
    limit: usize,
) -> Result<crate::backend::FindSimilarOutput, JsonRpcError> {
    let path = PathBuf::from(extract_string(args, PATH_FIELD)?);
    let start_byte = extract_usize(args, START_BYTE_FIELD)?;
    let end_byte = extract_usize(args, END_BYTE_FIELD)?;
    reject_inverted_range(start_byte, end_byte)?;
    backend
        .find_similar(
            FindSimilarInput::Range {
                path: &path,
                start_byte,
                end_byte,
            },
            limit,
        )
        .map_err(backend_to_rpc)
}

/// Calls the snippet variant of find-similar.
fn find_similar_snippet(
    backend: &dyn McpBackend,
    args: &Value,
    limit: usize,
) -> Result<crate::backend::FindSimilarOutput, JsonRpcError> {
    let snippet = extract_string(args, "snippet")?;
    let language = extract_string(args, "language")?;
    backend
        .find_similar(
            FindSimilarInput::Snippet {
                snippet: &snippet,
                language: &language,
            },
            limit,
        )
        .map_err(backend_to_rpc)
}

/// Returns current session metadata.
fn session_config(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let snapshot = backend.session_config().map_err(backend_to_rpc)?;
    to_value(&McpSessionConfig {
        root: snapshot.root,
        min_nodes: snapshot.min_nodes,
        languages: snapshot.languages,
        incremental: snapshot.incremental,
        embedding_provenance: snapshot.embedding_provenance,
        cache_stats: snapshot.cumulative_cache_stats,
        generation: backend.generation(),
    })
}

/// Lists available embedding models.
fn list_embedding_models(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let models = backend.list_embedding_models().map_err(backend_to_rpc)?;
    to_value(&EmbeddingModelList { models })
}

/// Sets the explicitly user-selected embedding model.
fn set_embedding_model(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    require_user_initiated(args)?;
    let provider_id = extract_string(args, "provider_id")?;
    let model_id = extract_string(args, "model_id")?;
    let endpoint = args.get("endpoint").and_then(Value::as_str);
    let spec = backend
        .set_embedding_model(&provider_id, &model_id, endpoint)
        .map_err(backend_to_rpc)?;
    to_value(&SetEmbeddingModelResponse {
        provider_id: spec.provider_id,
        model_id: spec.model_id,
        model_version: spec.model_version,
        dimensions: spec.dimensions,
    })
}

/// Requires explicit human consent for model mutation.
fn require_user_initiated(args: &Value) -> Result<(), JsonRpcError> {
    if args.get("user_initiated").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    Err(jsonrpc_error(
        ErrorCode::InvalidParams,
        "session set-embedding-model requires user_initiated=true",
    ))
}

/// Extracts page shape with canonical defaults.
fn extract_page_shape(args: &Value) -> Result<PageShape, JsonRpcError> {
    let detail = match args.get("detail").and_then(Value::as_str).unwrap_or("full") {
        "full" => Detail::Full,
        "summary" => Detail::Summary,
        value => {
            return Err(jsonrpc_error(
                ErrorCode::InvalidParams,
                format!("invalid detail {value:?}"),
            ))
        }
    };
    Ok(PageShape {
        pagination: Pagination {
            offset: extract_optional_usize(args, "offset").unwrap_or(0),
            limit: extract_limit(args),
            include_per_file: args
                .get("include_per_file")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        detail,
        max_occurrences: extract_optional_usize(args, "max_occurrences")
            .unwrap_or(DEFAULT_MAX_OCCURRENCES)
            .max(1),
    })
}

/// Extracts the shared cluster-owned filters.
fn extract_filters(args: &Value) -> Result<DuplicatesFilters, JsonRpcError> {
    Ok(DuplicatesFilters {
        languages: extract_string_array(args, "languages")?,
        path_contains: args
            .get("path_contains")
            .and_then(Value::as_str)
            .map(str::to_owned),
        severities: extract_string_array(args, "severities")?,
        min_size: extract_optional_usize(args, "min_size"),
    })
}

/// Extracts the positive result limit.
fn extract_limit(args: &Value) -> usize {
    extract_optional_usize(args, "limit")
        .unwrap_or(DEFAULT_LIMIT)
        .max(1)
}

/// Extracts a required string field.
fn extract_string(args: &Value, field: &str) -> Result<String, JsonRpcError> {
    args.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid_parameter(field, "string"))
}

/// Extracts one optional string array.
fn extract_string_array(args: &Value, field: &str) -> Result<Option<Vec<String>>, JsonRpcError> {
    let Some(value) = args.get(field) else {
        return Ok(None);
    };
    let Some(items) = value.as_array() else {
        return Err(invalid_parameter(field, "array of strings"));
    };
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid_parameter(field, "array of strings"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// Extracts an optional string path array.
fn extract_optional_paths(args: &Value, field: &str) -> Result<Vec<PathBuf>, JsonRpcError> {
    extract_string_array(args, field).map(|paths| {
        paths
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect()
    })
}

/// Extracts an optional non-negative integer.
fn extract_optional_usize(args: &Value, field: &str) -> Option<usize> {
    args.get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

/// Extracts a required non-negative integer.
fn extract_usize(args: &Value, field: &str) -> Result<usize, JsonRpcError> {
    extract_optional_usize(args, field).ok_or_else(|| invalid_parameter(field, "integer"))
}

/// Rejects inverted byte ranges.
fn reject_inverted_range(start: usize, end: usize) -> Result<(), JsonRpcError> {
    if end >= start {
        return Ok(());
    }
    Err(jsonrpc_error(
        ErrorCode::InvalidParams,
        format!("end_byte ({end}) must be >= start_byte ({start})"),
    ))
}

/// Builds a consistent invalid-parameter error.
fn invalid_parameter(field: &str, expected: &str) -> JsonRpcError {
    jsonrpc_error(
        ErrorCode::InvalidParams,
        format!("parameter {field:?} must be a {expected}"),
    )
}
