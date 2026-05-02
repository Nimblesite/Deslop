//! MCP `tools/list` + `tools/call` for the [MCP-TOOLS] tools.
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
    call_report_for_range, call_report_get, call_report_query, call_rescan, call_schema_doc,
    call_session_config, call_set_embedding_model, call_top_offenders,
};
use schemas::{
    schema_cluster_by_id, schema_empty, schema_find_similar, schema_report_for_file,
    schema_report_for_range, schema_report_get, schema_report_query, schema_rescan,
    schema_set_embedding_model, schema_top_offenders,
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

/// Static tool registry. `top-offenders` is the primary entry point.
const TOOLS: [ToolDefinition; 12] = [
    ToolDefinition {
        name: "top-offenders",
        description:
            "Top N duplicate clusters with full data (occurrences, interpretation, signals, bucket, score). Default n=5. Start here — one call gives everything needed to fix duplication.",
        input_schema: schema_top_offenders,
    },
    ToolDefinition {
        name: "rescan",
        description:
            "Synchronously reload the latest LSP state after edits, then return fresh top offenders. Use when watcher lag or stale ranges are suspected.",
        input_schema: schema_rescan,
    },
    ToolDefinition {
        name: "report-get",
        description: "Paginated slim cluster list, worst-first. Use top-offenders for full data.",
        input_schema: schema_report_get,
    },
    ToolDefinition {
        name: "report-query",
        description:
            "Slim paginated list with AND-combined filters: language, bucket, path_contains, min_score, min_size.",
        input_schema: schema_report_query,
    },
    ToolDefinition {
        name: "schema-doc",
        description:
            "One-shot report schema markdown. Call once for field meanings; report pages omit it by default.",
        input_schema: schema_empty,
    },
    ToolDefinition {
        name: "report-for-file",
        description: "All clusters whose occurrences touch this file.",
        input_schema: schema_report_for_file,
    },
    ToolDefinition {
        name: "report-for-range",
        description: "Clusters overlapping a byte range. Call before refactoring a specific block.",
        input_schema: schema_report_for_range,
    },
    ToolDefinition {
        name: "find-similar",
        description:
            "Find clusters similar to a byte range or snippet. Call before writing to avoid introducing new clones.",
        input_schema: schema_find_similar,
    },
    ToolDefinition {
        name: "cluster-by-id",
        description: "Full cluster record by stable id (shown in report text and LSP diagnostics).",
        input_schema: schema_cluster_by_id,
    },
    ToolDefinition {
        name: "list-embedding-models",
        description: "Enumerate available embedding models.",
        input_schema: schema_empty,
    },
    ToolDefinition {
        name: "set-embedding-model",
        description: "Switch the embedding model. Requires user_initiated=true.",
        input_schema: schema_set_embedding_model,
    },
    ToolDefinition {
        name: "session-config",
        description: "Session metadata: root, min-nodes, languages, generation counter.",
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
        "top-offenders" => call_top_offenders(backend, arguments),
        "rescan" => call_rescan(backend, arguments),
        "report-get" => call_report_get(backend, arguments),
        "report-query" => call_report_query(backend, arguments),
        "schema-doc" => call_schema_doc(backend),
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
