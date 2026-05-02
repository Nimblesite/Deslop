//! MCP `tools/list` + `tools/call` for the eight [MCP-TOOLS] tools.
//!
//! Each entry binds a [`ToolDefinition`] — schema + agent-facing
//! description — to a dispatch function that forwards to the active
//! [`McpBackend`]. The [MCP-AGENT-PROMPT-GUIDANCE] descriptions are
//! authored for an LLM planner, not a human reader.

use serde_json::{json, Value};

use crate::backend::McpBackend;
use crate::protocol::JsonRpcError;

mod handlers;
mod schemas;

use handlers::{
    call_cluster_by_id, call_find_similar, call_list_embedding_models, call_report_for_file,
    call_report_for_range, call_report_get, call_report_query, call_session_config,
    call_set_embedding_model,
};
use schemas::{
    schema_cluster_by_id, schema_empty, schema_find_similar, schema_report_for_file,
    schema_report_for_range, schema_report_get, schema_report_query, schema_set_embedding_model,
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
            crate::protocol::ErrorCode::MethodNotFound,
            format!("no tool named {other:?}"),
        )),
    }
}

/// Maps a [`BackendError`] onto the JSON-RPC error envelope.
#[must_use]
pub fn backend_to_rpc(err: crate::backend::BackendError) -> JsonRpcError {
    use crate::backend::BackendError;
    use crate::protocol::ErrorCode;
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
