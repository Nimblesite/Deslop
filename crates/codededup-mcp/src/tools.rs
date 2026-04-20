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

/// Canonical tool registry. Order matches the [MCP-TOOLS] table in
/// `docs/specs/mcp.md`.
#[must_use]
pub const fn all_tools() -> &'static [ToolDefinition] {
    &TOOLS
}

/// Renders `all_tools()` into the JSON shape MCP's `tools/list`
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
        "report-get" => call_report_get(backend),
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
const TOOLS: [ToolDefinition; 8] = [
    ToolDefinition {
        name: "report-get",
        description:
            "Fetch the current full duplication report. Worst offenders first. Call this at session start, or when you want a full picture of the codebase's clone landscape.",
        input_schema: schema_empty,
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
            "Switch the live embedding model. Invalidates only the embedding layer; structural + LSH caches stay warm.",
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
            "endpoint": { "type": "string", "description": "Optional override (Ollama only)." }
        },
        "required": ["provider_id", "model_id"],
        "additionalProperties": false,
    })
}

/// `report-get` forwarder.
fn call_report_get(backend: &dyn McpBackend) -> Result<Value, JsonRpcError> {
    let report = backend.report_get().map_err(backend_to_rpc)?;
    serde_json::to_value(&*report).map_err(|err| serialise_failure(&err))
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
    serde_json::to_value(cluster).map_err(|err| serialise_failure(&err))
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

/// Maps a `serde_json` serialisation failure onto an internal error.
fn serialise_failure(err: &serde_json::Error) -> JsonRpcError {
    JsonRpcError::new(
        ErrorCode::InternalError,
        format!("failed to serialise tool result: {err}"),
    )
}
