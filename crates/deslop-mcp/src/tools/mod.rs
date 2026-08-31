//! MCP `tools/list` + `tools/call` for the [MCP-TOOLS] tools.
//!
//! Each entry binds a [`ToolDefinition`] — schema + agent-facing
//! description — to a dispatch function that forwards to the active
//! [`McpBackend`]. The [MCP-AGENT-PROMPT-GUIDANCE] descriptions are
//! authored for an LLM planner, not a human reader. Per,
//! every description is one-sentence-short (≤200 chars) so the full
//! `tools/list` payload stays well under the wire budgets of stricter
//! MCP clients (Codex's `rmcp_client` in particular). Long-form
//! rationale and worked examples live in the `deslop://schema`
//! resource — agents fetch it on demand via `schema-doc`.

use deslop_core::pipeline::language_ids;
use serde_json::{json, Value};
use tracing::debug;

use crate::backend::McpBackend;
use crate::protocol::{jsonrpc_error, JsonRpcError};

mod handlers;
mod limits;
mod schemas;

use handlers::{
    call_cluster_by_id, call_compare_pair, call_duplicates, call_find_similar, call_merge_plan,
    call_rescan, call_schema_doc, call_session,
};
use limits::cap_tool_result;
use schemas::{
    schema_cluster_by_id, schema_compare_pair, schema_duplicates, schema_empty,
    schema_find_similar, schema_merge_plan, schema_rescan, schema_session,
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

/// Static registry: exactly seven analysis tools plus the separately-specified
/// merge refactor tool.
const TOOLS: [ToolDefinition; 8] = [
    ToolDefinition {
        name: "find-similar",
        description: "Call before writing code to prevent duplication. Returns mass-ranked clusters only; use compare-pair for pair evidence.",
        input_schema: schema_find_similar,
    },
    ToolDefinition {
        name: "duplicates",
        description: "Duplicate clusters ranked by mass. Start here when fixing duplicates.",
        input_schema: schema_duplicates,
    },
    ToolDefinition {
        name: "compare-pair",
        description: "Compare exactly two occurrences and return their pair admission evidence.",
        input_schema: schema_compare_pair,
    },
    ToolDefinition {
        name: "cluster-by-id",
        description: "Full mass-only cluster record by stable id, with pageable occurrences.",
        input_schema: schema_cluster_by_id,
    },
    ToolDefinition {
        name: "rescan",
        description: "Force the LSP refresh and return a fresh filtered duplicates page with change counts.",
        input_schema: schema_rescan,
    },
    ToolDefinition {
        name: "session",
        description: "Get session metadata, list embedding models, or apply an explicitly user-selected model.",
        input_schema: schema_session,
    },
    ToolDefinition {
        name: "schema-doc",
        description: "One-shot report schema markdown. Call once for field meanings.",
        input_schema: schema_empty,
    },
    ToolDefinition {
        name: "merge-plan",
        description: "Mechanical plan for a cluster BEFORE hand-editing: same-file clusters merge into one helper; cross-file identical definitions consolidate to one canonical copy. Verdict + WorkspaceEdit. Read-only.",
        input_schema: schema_merge_plan,
    },
];

/// Live language ids the running engine registers, or the compiled
/// fallback set when the engine is unreachable. `tools/list` must
/// advertise this set so the `language` enum can never claim a language
/// the runtime validator would reject under MCP/engine version skew
///.
#[must_use]
pub fn advertised_languages(backend: &dyn McpBackend) -> Vec<String> {
    backend.session_config().map_or_else(
        |_| language_ids().into_iter().map(str::to_owned).collect(),
        |config| config.languages,
    )
}

/// Renders the tool registry into the JSON shape MCP's `tools/list`
/// response expects. `languages` is the live engine's registered set
/// (see [`advertised_languages`]); every schema `language` enum is
/// rewritten to it so the advertised gate matches the validator.
#[must_use]
pub fn tools_list_payload(languages: &[String]) -> Value {
    debug!(language_count = languages.len(), "tools_list_advertised");
    let mut items: Vec<Value> = TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": (tool.input_schema)(),
            })
        })
        .collect();
    apply_live_language_enum(&mut items, languages);
    json!({ "tools": items })
}

/// Rewrites the closed `language` enum of every tool schema that filters
/// by language (`find-similar`, `report-query`) to `languages` — the
/// live engine's registered set. Schemas ship a compile-time enum as a
/// fallback; this override keeps the advertised gate in lock-step with
/// the runtime validator so it can never reject a language the engine
/// supports.
fn apply_live_language_enum(tools: &mut [Value], languages: &[String]) {
    let enum_values: Vec<Value> = languages.iter().cloned().map(Value::from).collect();
    for tool in tools {
        for pointer in [
            "/inputSchema/properties/language/enum",
            "/inputSchema/properties/languages/items/enum",
        ] {
            if let Some(slot) = tool.pointer_mut(pointer) {
                *slot = Value::Array(enum_values.clone());
            }
        }
    }
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
/// The returned [`Value`] is passed through [`cap_tool_result`] so a
/// pathological payload (multi-MB top-offenders on a huge monorepo)
/// can never blow out a strict MCP client (Codex's `rmcp_client` in
/// particular — see). The cap is silent on small results
/// and adds a `truncated` flag plus pointer to `report-get` on large
/// ones.
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
    let payload = dispatch_inner(backend, name, arguments)?;
    Ok(cap_tool_result(name, payload))
}

/// Inner dispatch table — kept separate from [`dispatch_tool_call`]
/// so the cap pass is one wrapping line in production code and
/// trivially bypassable in unit tests that target individual
/// handlers.
fn dispatch_inner(
    backend: &dyn McpBackend,
    name: &str,
    arguments: &Value,
) -> Result<Value, JsonRpcError> {
    match name {
        "find-similar" => call_find_similar(backend, arguments),
        "duplicates" => call_duplicates(backend, arguments),
        "compare-pair" => call_compare_pair(backend, arguments),
        "cluster-by-id" => call_cluster_by_id(backend, arguments),
        "rescan" => call_rescan(backend, arguments),
        "session" => call_session(backend, arguments),
        "schema-doc" => call_schema_doc(backend),
        "merge-plan" => call_merge_plan(backend, arguments),
        other => Err(jsonrpc_error(
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
            jsonrpc_error(ErrorCode::UnparseableInput, message)
        }
        BackendError::UnsupportedLanguage(lang) => jsonrpc_error(
            ErrorCode::UnsupportedLanguage,
            format!("language {lang:?} is not registered"),
        ),
        BackendError::Path(inner) => jsonrpc_error(ErrorCode::PathOutsideRoot, inner.to_string()),
        BackendError::UnknownCluster(id) => jsonrpc_error(
            ErrorCode::InvalidParams,
            format!("no cluster with id {id:?}"),
        ),
        BackendError::LspNotRunning { socket_path } => lsp_not_running_rpc(&socket_path),
        other => jsonrpc_error(ErrorCode::BackendError, other.to_string()),
    }
}

/// Builds the `-32004` error for a missing LSP, attaching a structured
/// recovery payload so agents can decide whether to retry and where the
/// on-disk fallback lives. The numeric code and the
/// human message are reproduced verbatim for wire
/// back-compat; only the previously-`null` `data` field is enriched.
fn lsp_not_running_rpc(socket_path: &std::path::Path) -> JsonRpcError {
    let message = crate::backend::BackendError::LspNotRunning {
        socket_path: socket_path.to_path_buf(),
    }
    .to_string();
    JsonRpcError {
        code: crate::protocol::ErrorCode::BackendError as i32,
        message,
        data: Some(lsp_recovery_data(socket_path)),
    }
}

/// Assembles the machine-readable recovery payload for a missing LSP.
/// The cache-fallback and TCP-discovery paths are derived from the
/// socket's parent directory (the `.deslop/cache` dir), so an agent
/// can read the last analysis while the LSP is down and knows where
/// the TCP endpoint record would appear ([MCP-IPC-DISCOVERY]).
fn lsp_recovery_data(socket_path: &std::path::Path) -> Value {
    let cache_fallback = socket_path
        .parent()
        .map(|dir| dir.join(deslop_core::live::LIVE_REPORT_FILE_NAME));
    let port_file = socket_path
        .parent()
        .map(|dir| dir.join(deslop_core::live::transport::IPC_PORT_FILE_NAME));
    json!({
        "reason": "lsp_not_running",
        "retry_after_ms": 500,
        "socket_path": socket_path.display().to_string(),
        "port_file_path": port_file.map(|path| path.display().to_string()),
        "cache_fallback": {
            "path": cache_fallback.map(|path| path.display().to_string()),
            "format": "live-report-json",
            "note": "If the LSP stays down after retry, read this on-disk live-report cache for the last analysis.",
        },
    })
}
