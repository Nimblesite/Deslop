//! MCP `tools/list` + `tools/call` for the eight [MCP-TOOLS] tools.
//!
//! Each entry binds a [`ToolDefinition`] — schema + agent-facing
//! description — to a dispatch function that forwards to the active
//! [`McpBackend`]. The [MCP-AGENT-PROMPT-GUIDANCE] descriptions are
//! authored for an LLM planner, not a human reader.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::{
    backend::{BackendError, FindSimilarInput, McpBackend},
    page::{build_page, Pagination, QueryFilters},
    protocol::{ErrorCode, JsonRpcError},
};

/// Static definition of one MCP tool.
#[derive(Debug)]
pub struct ToolDefinition {
    /// Tool name as exposed to the agent (e.g. `"find-similar"`).
    pub name: &'static str,
    /// Agent-facing description per [MCP-AGENT-PROMPT-GUIDANCE].
    pub description: &'static str,
    /// JSON schema for the tool's input parameters.
    pub input_schema: fn() -> Value,
}

/// Renders the tool registry into the JSON shape MCP's `tools/list`
/// response expects.
#[must_use]
pub fn tools_list_payload() -> Value {
    let items: Vec<Value> = TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": (tool.input_schema)(),
            })
        })
        .collect();
    json!({ "tools": items })
}

/// Wraps a tool result in the MCP `tools/call` response envelope.
///
/// Shape: `{ content: [{ type: "text", text: "..." }], isError: false }`.
/// Content is serialised JSON so agents that can only read text still
/// get structured data.
#[must_use]
pub fn wrap_tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).unwrap_or_else(|_| "null".to_owned());
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false,
        "structuredContent": payload,
    })
}

/// Dispatches a `tools/call` request by name.
///
/// # Errors
///
/// Returns a [`JsonRpcError`] ready to serialise into the response
/// envelope on schema violations, backend failures, and unknown
/// tool names.
pub fn dispatch_tool_call(
    backend: &dyn McpBackend,
    name: &str,
    arguments: &Value,
) -> Result<Value, JsonRpcError> {
    match name {
        "report-get" => call_report_get(backend, arguments),
        "report-query" => call_report_query(backend, arguments),
        "report-for-file" => call_report_for_file(backend, arguments),
        "report-for-range" => call_report_for_range(backend, arguments),
        "find-similar" => call_find_similar(backend, arguments),
        "cluster-by-id" => call_cluster_by_id(backend, arguments),
        "list-embedding-models" => call_list_embedding_models(backend),
        "set-embedding-model" => call_set_embedding_model(backend, arguments),
        "session-config" => call_session_config(backend),
        other => Err(JsonRpcError::new(
            ErrorCode::MethodNotFound,
            format!("no tool named {other:?}"),
        )),
    }
}

/// Static tool registry.
const TOOLS: [ToolDefinition; 9] = [
    ToolDefinition {
        name: "report-get",
        description:
            "Fetch one page of the current duplication report. Worst offenders first. Returns headline metrics + a slim cluster summary slice (no member list, no full occurrences[]). Call this at session start; follow up with cluster-by-id for any cluster you want to drill into. Both `offset` and `limit` are required — the agent must size its own context window.",
        input_schema: schema_report_get,
    },
    ToolDefinition {
        name: "report-query",
        description:
            "Targeted, filterable lookup over the duplication report. Same slim ReportPage shape as report-get, plus optional `language`, `bucket`, `path_contains`, `min_score`, `min_size` filters that combine with logical AND. Use this whenever you can describe what you're looking for instead of dumping the whole report. `offset` + `limit` required.",
        input_schema: schema_report_query,
    },
    ToolDefinition {
        name: "report-for-file",
        description:
            "All clone clusters whose occurrences touch this file. Call before editing to see what's already a duplicate here.",
        input_schema: schema_report_for_file,
    },
    ToolDefinition {
        name: "report-for-range",
        description:
            "Clusters overlapping the byte range you're about to edit. Call before a refactor — tells you if the range is part of a larger clone family.",
        input_schema: schema_report_for_range,
    },
    ToolDefinition {
        name: "find-similar",
        description:
            "Before you write a new block, call this. Give either a byte range on an open file or a snippet + language. Returns existing clusters similar to the input via the full structural + LSH + embedding passes. Prevents you from introducing new clones.",
        input_schema: schema_find_similar,
    },
    ToolDefinition {
        name: "cluster-by-id",
        description:
            "Fetch a cluster by its stable 16-char id (the one shown in report text and LSP diagnostics).",
        input_schema: schema_cluster_by_id,
    },
    ToolDefinition {
        name: "list-embedding-models",
        description:
            "Enumerate Ollama models installed on the host plus the built-in stub provider. Use before switching models.",
        input_schema: schema_empty,
    },
    ToolDefinition {
        name: "set-embedding-model",
        description:
            "Switch the live embedding model only after explicit user initiation. Persists the shared VSIX/LSP embedding settings; structural + LSH caches stay warm.",
        input_schema: schema_set_embedding_model,
    },
    ToolDefinition {
        name: "session-config",
        description:
            "Min-nodes, active languages, embedding provenance, exclusion config path, cache root.",
        input_schema: schema_empty,
    },
];

/// Empty-parameter schema.
fn schema_empty() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
    })
}

/// Schema for `report-get`. Both pagination knobs are required so the
/// agent always states its context budget explicitly
/// ([MCP-TOOL-REPORT-PAGINATION]).
fn schema_report_get() -> Value {
    json!({
        "type": "object",
        "properties": {
            "offset": { "type": "integer", "minimum": 0, "description": "Zero-based cluster index to start at." },
            "limit": { "type": "integer", "minimum": 0, "description": "Max clusters in this page. Pick a sensible value for your context window." }
        },
        "required": ["offset", "limit"],
        "additionalProperties": false,
    })
}

/// Schema for `report-query`. Same pagination contract as `report-get`
/// plus optional filter knobs ([MCP-TOOL-REPORT-QUERY]).
fn schema_report_query() -> Value {
    json!({
        "type": "object",
        "properties": {
            "offset": { "type": "integer", "minimum": 0 },
            "limit": { "type": "integer", "minimum": 0 },
            "language": { "type": "string", "enum": ["csharp", "rust", "python"], "description": "Match clusters whose detected source language equals this id." },
            "bucket": { "type": "string", "enum": ["identical", "nearly_identical", "loosely_similar", "same_behavior"], "description": "Match clusters whose canonical bucket equals this id." },
            "path_contains": { "type": "string", "description": "Case-sensitive substring match against any occurrence path on the cluster." },
            "min_score": { "type": "number", "description": "Inclusive ranking-score floor." },
            "min_size": { "type": "integer", "minimum": 0, "description": "Inclusive subtree-node-count floor (canonical_node_count)." }
        },
        "required": ["offset", "limit"],
        "additionalProperties": false,
    })
}

/// Schema for `report-for-file`.
fn schema_report_for_file() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Absolute or workspace-relative path." }
        },
        "required": ["path"],
        "additionalProperties": false,
    })
}

/// Schema for `report-for-range`.
fn schema_report_for_range() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Absolute or workspace-relative path." },
            "start_byte": { "type": "integer", "minimum": 0 },
            "end_byte": { "type": "integer", "minimum": 0 }
        },
        "required": ["path", "start_byte", "end_byte"],
        "additionalProperties": false,
    })
}

/// Schema for `find-similar`.
fn schema_find_similar() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "start_byte": { "type": "integer", "minimum": 0 },
            "end_byte": { "type": "integer", "minimum": 0 },
            "snippet": { "type": "string" },
            "language": {
                "type": "string",
                "enum": ["csharp", "rust", "python"]
            },
            "top_n": { "type": "integer", "minimum": 0, "default": 5 }
        },
        "additionalProperties": false,
    })
}

/// Schema for `cluster-by-id`.
fn schema_cluster_by_id() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 }
        },
        "required": ["id"],
        "additionalProperties": false,
    })
}

/// Schema for `set-embedding-model`.
fn schema_set_embedding_model() -> Value {
    json!({
        "type": "object",
        "properties": {
            "provider_id": { "type": "string", "enum": ["stub", "ollama"] },
            "model_id": { "type": "string", "minLength": 1 },
            "endpoint": { "type": "string", "description": "Optional override (Ollama only)." },
            "user_initiated": {
                "type": "boolean",
                "const": true,
                "description": "Must be true only when a human explicitly requested this model switch."
            }
        },
        "required": ["provider_id", "model_id", "user_initiated"],
        "additionalProperties": false,
    })
}

/// `report-get` forwarder. Renders a slim paginated `ReportPage`
/// ([MCP-TOOL-REPORT-PAGINATION]).
fn call_report_get(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let pagination = extract_pagination(args)?;
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(build_page(
        &report,
        backend.generation(),
        pagination,
        &QueryFilters::default(),
    ))
}

/// `report-query` forwarder. Same `ReportPage` shape as `report-get`
/// plus AND-combined filters ([MCP-TOOL-REPORT-QUERY]).
fn call_report_query(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let pagination = extract_pagination(args)?;
    let filters = extract_filters(args);
    let report = backend.report_get().map_err(backend_to_rpc)?;
    Ok(build_page(
        &report,
        backend.generation(),
        pagination,
        &filters,
    ))
}

/// Extracts the required `offset` + `limit` pagination knobs.
fn extract_pagination(args: &Value) -> Result<Pagination, JsonRpcError> {
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
fn extract_filters(args: &Value) -> QueryFilters {
    QueryFilters {
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

/// `report-for-file` forwarder.
fn call_report_for_file(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let path = extract_string(args, "path")?;
    let clusters = backend
        .report_for_file(Path::new(&path))
        .map_err(backend_to_rpc)?;
    Ok(json!({ "path": path, "clusters": clusters }))
}

/// `report-for-range` forwarder.
fn call_report_for_range(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let path = extract_string(args, "path")?;
    let start_byte = extract_u64(args, "start_byte")?;
    let end_byte = extract_u64(args, "end_byte")?;
    reject_inverted_range(start_byte, end_byte)?;
    let clusters = backend
        .report_for_range(
            Path::new(&path),
            usize::try_from(start_byte).unwrap_or(usize::MAX),
            usize::try_from(end_byte).unwrap_or(usize::MAX),
        )
        .map_err(backend_to_rpc)?;
    Ok(json!({
        "path": path,
        "start_byte": start_byte,
        "end_byte": end_byte,
        "clusters": clusters,
    }))
}

/// `find-similar` forwarder. Selects between the range and snippet
/// variants based on which fields were supplied.
fn call_find_similar(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
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
            .map_err(backend_to_rpc)?
    } else {
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
            .map_err(backend_to_rpc)?
    };
    Ok(json!({
        "clusters": output.clusters,
        "below_min_nodes": output.below_min_nodes,
    }))
}

/// `cluster-by-id` forwarder.
fn call_cluster_by_id(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
    let id = extract_string(args, "id")?;
    let cluster = backend.cluster_by_id(&id).map_err(backend_to_rpc)?;
    Ok(serde_json::to_value(cluster).unwrap_or(Value::Null))
}

/// `list-embedding-models` forwarder.
fn call_list_embedding_models(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let models = backend.list_embedding_models().map_err(backend_to_rpc)?;
    let rendered: Vec<Value> = models
        .into_iter()
        .map(|info| {
            json!({
                "name": info.name,
                "bare_id": info.bare_id,
                "digest": info.digest,
                "size_bytes": info.size_bytes,
                "is_embedding_model": info.is_embedding_model,
            })
        })
        .collect();
    Ok(json!({ "models": rendered }))
}

/// `set-embedding-model` forwarder.
fn call_set_embedding_model(backend: &dyn McpBackend, args: &Value) -> Result<Value, JsonRpcError> {
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
    Ok(json!({
        "provider_id": spec.provider_id,
        "model_id": spec.model_id,
        "model_version": spec.model_version,
        "dimensions": spec.dimensions,
    }))
}

/// `session-config` forwarder.
fn call_session_config(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let snapshot = backend.session_config().map_err(backend_to_rpc)?;
    Ok(json!({
        "root": snapshot.root,
        "min_nodes": snapshot.min_nodes,
        "languages": snapshot.languages,
        "incremental": snapshot.incremental,
        "embedding_provenance": snapshot.embedding_provenance,
        "cache_stats": {
            "hits": snapshot.cumulative_cache_stats.hits,
            "misses": snapshot.cumulative_cache_stats.misses,
        },
        "generation": backend.generation(),
    }))
}

/// Requires explicit user consent for model-changing tool calls.
fn require_user_initiated(args: &Value) -> Result<(), JsonRpcError> {
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

/// Extracts a required string field from `args`.
fn extract_string(args: &Value, field: &str) -> Result<String, JsonRpcError> {
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

/// Extracts a required non-negative integer field from `args`.
fn extract_u64(args: &Value, field: &str) -> Result<u64, JsonRpcError> {
    args.get(field).and_then(Value::as_u64).ok_or_else(|| {
        JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("missing or non-integer parameter {field:?}"),
        )
    })
}

/// Rejects byte ranges with `end <= start`.
fn reject_inverted_range(start: u64, end: u64) -> Result<(), JsonRpcError> {
    if end < start {
        return Err(JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("end_byte ({end}) must be >= start_byte ({start})"),
        ));
    }
    Ok(())
}

/// Maps a [`BackendError`] onto the JSON-RPC error envelope.
#[must_use]
pub fn backend_to_rpc(err: BackendError) -> JsonRpcError {
    match err {
        BackendError::UnparseableInput(message) => {
            JsonRpcError::new(ErrorCode::UnparseableInput, message)
        }
        BackendError::UnsupportedLanguage(lang) => JsonRpcError::new(
            ErrorCode::UnsupportedLanguage,
            format!("language {lang:?} is not registered"),
        ),
        BackendError::Path(inner) => {
            JsonRpcError::new(ErrorCode::PathOutsideRoot, inner.to_string())
        }
        BackendError::UnknownCluster(id) => JsonRpcError::new(
            ErrorCode::InvalidParams,
            format!("no cluster with id {id:?}"),
        ),
        other => JsonRpcError::new(ErrorCode::BackendError, other.to_string()),
    }
}
