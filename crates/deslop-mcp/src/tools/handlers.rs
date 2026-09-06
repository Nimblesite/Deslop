//! Handler functions for each MCP tool call.

use std::path::{Path, PathBuf};

use deslop_core::{
    report::occurrence_count,
    wire_generated::{
        EmbeddingModelList, FindSimilarResult, McpSessionConfig, RangeReport, ReportCluster,
        ReportPageFilters, RescanPayload, SchemaDocPayload, SetEmbeddingModelResponse,
        TopOffendersPayload,
    },
    Report,
};
use serde_json::Value;

use crate::{
    backend::{FindSimilarInput, McpBackend, RescanProgress},
    page::{build_page, Pagination},
    protocol::{jsonrpc_error, ErrorCode, JsonRpcError},
};

use super::backend_to_rpc;

/// Default total-occurrence budget for tools that ship full clusters
/// over the wire ([MCP-OCCURRENCE-BUDGET]). Worst-first: clusters fill
/// the budget in order; the cluster that overruns it is included with
/// its occurrences truncated and `occurrences_truncated = true`, and
/// any following clusters are dropped. The unfiltered count is always
/// reported in `total_occurrences` so the agent knows how much was
/// filtered. Default chosen so a `top-offenders` n=5 response on a
/// monorepo stays well under any plausible MCP client buffer limit.
pub(super) const DEFAULT_MAX_OCCURRENCES: usize = 15;

/// Serialises a typed wire payload, falling back to JSON `null` only if
/// serde fails (which it cannot for the wire types here — they all derive
/// `Serialize`). Centralised so the handlers stay terse.
fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// `top-offenders` forwarder. Returns up to `n` clusters with full
/// data — occurrences, interpretation, signals, bucket — capped by
/// the `max_occurrences` total-occurrence budget so a single response
/// never blows up the agent's context window
/// ([MCP-OCCURRENCE-BUDGET]).
// [MCP-TOOL-DUPLICATES] current top-offenders surface; redesign
// consolidates the report tools into one `duplicates` tool.
pub(super) fn call_top_offenders(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let n = extract_top_n(args);
    let max_occurrences = extract_max_occurrences(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(top_offenders_payload(&report, n, max_occurrences))
}

/// `rescan` reloads the LSP-written state file and returns fresh top offenders.
pub(super) fn call_rescan(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let paths = extract_optional_paths(args, "paths")?;
    let progress = backend.mark_changed(&paths).map_err(backend_to_rpc)?;
    let n = extract_top_n(args);
    let max_occurrences = extract_max_occurrences(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(rescan_payload(&report, n, max_occurrences, progress))
}

/// Builds the top-offenders JSON payload.
fn top_offenders_payload(report: &Report, n: usize, max_occurrences: usize) -> Value {
    let total_occurrences = total_occurrences_in(&report.clusters);
    let clusters = apply_occurrence_budget(report.clusters.iter().take(n), max_occurrences);
    let payload = TopOffendersPayload {
        total_clusters: report.clusters.len(),
        n,
        max_occurrences,
        total_occurrences,
        clusters,
    };
    to_value(&payload)
}

/// Builds the `rescan` JSON payload from the refreshed report and progress.
fn rescan_payload(
    report: &Report,
    n: usize,
    max_occurrences: usize,
    progress: RescanProgress,
) -> Value {
    let total_occurrences = total_occurrences_in(&report.clusters);
    let clusters = apply_occurrence_budget(report.clusters.iter().take(n), max_occurrences);
    let payload = RescanPayload {
        total_clusters: report.clusters.len(),
        n,
        max_occurrences,
        total_occurrences,
        clusters,
        generation: progress.generation,
        summary: progress.summary,
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

/// Extracts the `max_occurrences` total-occurrence budget, defaulting
/// to [`DEFAULT_MAX_OCCURRENCES`]. Floored at 1 so a misbehaving caller
/// cannot zero out every cluster.
pub(super) fn extract_max_occurrences(args: &Value) -> usize {
    args.get("max_occurrences")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(DEFAULT_MAX_OCCURRENCES)
        .max(1)
}

/// Returns the total number of occurrences across all clusters,
/// using the authoritative count from [`occurrence_count`] so wire
/// truncations do not under-count the unfiltered total
/// ([MCP-OCCURRENCE-BUDGET]).
pub(super) fn total_occurrences_in(clusters: &[ReportCluster]) -> usize {
    clusters.iter().map(occurrence_count).sum()
}

/// Walks `clusters` worst-first, including each one until the running
/// total of occurrences would exceed `budget`. The cluster that overruns
/// the budget is included with its occurrences truncated to the
/// remaining headroom and `occurrences_truncated` set; any following
/// cluster is dropped entirely. A budget of zero returns no clusters.
pub(super) fn apply_occurrence_budget<'a, I>(clusters: I, budget: usize) -> Vec<ReportCluster>
where
    I: IntoIterator<Item = &'a ReportCluster>,
{
    let mut used = 0usize;
    let mut emitted = Vec::new();
    for cluster in clusters {
        if used >= budget {
            break;
        }
        let mut copy = cluster.clone();
        let total_for_cluster = occurrence_count(&copy);
        copy.occurrences_total = total_for_cluster;
        let remaining = budget.saturating_sub(used);
        if copy.occurrences.len() <= remaining {
            used = used.saturating_add(copy.occurrences.len());
            emitted.push(copy);
            continue;
        }
        copy.occurrences.truncate(remaining);
        copy.occurrences_truncated = true;
        used = budget;
        emitted.push(copy);
    }
    emitted
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
    let max_occurrences = extract_max_occurrences(args);
    let clusters = backend
        .report_for_file(Path::new(&path))
        .map_err(backend_to_rpc)?;
    let total_occurrences = total_occurrences_in(&clusters);
    let trimmed = apply_occurrence_budget(clusters.iter(), max_occurrences);
    let payload = deslop_core::wire_generated::FileReport {
        path: PathBuf::from(path),
        clusters: trimmed,
        total_occurrences,
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
    let max_occurrences = extract_max_occurrences(args);
    reject_inverted_range(start_byte, end_byte)?;
    let start_byte_usize = usize::try_from(start_byte).unwrap_or(usize::MAX);
    let end_byte_usize = usize::try_from(end_byte).unwrap_or(usize::MAX);
    let clusters = backend
        .report_for_range(Path::new(&path), start_byte_usize, end_byte_usize)
        .map_err(backend_to_rpc)?;
    let total_occurrences = total_occurrences_in(&clusters);
    let trimmed = apply_occurrence_budget(clusters.iter(), max_occurrences);
    let payload = RangeReport {
        path: PathBuf::from(path),
        start_byte: start_byte_usize,
        end_byte: end_byte_usize,
        clusters: trimmed,
        total_occurrences,
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
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            "find-similar requires exactly one of (path + start_byte + end_byte) or (snippet + language)",
        ));
    }
    // Floor at 1 so `top_n: 0` falls back to a meaningful budget
    // rather than truncating the response to zero clusters. The
    // `top-offenders` tool uses the same semantics
    // ([MCP-OCCURRENCE-BUDGET]).
    let top_n = args
        .get("top_n")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(5)
        .max(1);
    let max_occurrences = extract_max_occurrences(args);
    let output = if has_range {
        call_find_similar_range(backend, args, top_n)?
    } else {
        call_find_similar_snippet(backend, args, top_n)?
    };
    let total_occurrences = total_occurrences_in(&output.clusters);
    let trimmed = apply_occurrence_budget(output.clusters.iter(), max_occurrences);
    let payload = FindSimilarResult {
        clusters: trimmed,
        below_min_nodes: output.below_min_nodes,
        total_occurrences,
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

/// `merge-plan` forwarder ([AUTOFIX-MERGE-MCP]).
pub(super) fn call_merge_plan(
    backend: &dyn McpBackend,
    args: &Value,
) -> Result<Value, JsonRpcError> {
    let id = extract_string(args, "id")?;
    let plan = backend.merge_plan(&id).map_err(backend_to_rpc)?;
    Ok(to_value(&plan))
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
// [MCP-TOOL-SESSION] current session-config surface; redesign merges
// the three session tools into one.
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
// [MCP-EMBEDDING-CONSENT] consent gate: set-embedding-model requires
// user_initiated==true before mutating the embedding model.
pub(super) fn require_user_initiated(args: &Value) -> Result<(), JsonRpcError> {
    if args
        .get("user_initiated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }
    Err(jsonrpc_error(
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
        include_per_file: args
            .get("include_per_file")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// Extracts the optional `report-query` filter knobs. Unknown / wrong-typed
/// fields are quietly ignored — the JSON schema layer rejects them up
/// front when a strict client is in use.
// [MCP-TOOL-FILTERS] current scalar filter form; redesign adds array
// buckets/categories.
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
            jsonrpc_error(
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
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("parameter {field:?} must be an array of strings"),
        ));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(PathBuf::from).ok_or_else(|| {
                jsonrpc_error(
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
        jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("missing or non-integer parameter {field:?}"),
        )
    })
}

/// Rejects byte ranges with `end < start`.
pub(super) fn reject_inverted_range(start: u64, end: u64) -> Result<(), JsonRpcError> {
    if end < start {
        return Err(jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("end_byte ({end}) must be >= start_byte ({start})"),
        ));
    }
    Ok(())
}
