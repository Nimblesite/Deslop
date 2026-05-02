//! JSON schema builders for each MCP tool's input parameters.

use serde_json::{json, Value};

/// Empty-parameter schema.
pub(super) fn schema_empty() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
    })
}

/// Schema for `report-get`. Both pagination knobs are required so the
/// agent always states its context budget explicitly
/// ([MCP-TOOL-REPORT-PAGINATION]).
pub(super) fn schema_report_get() -> Value {
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
pub(super) fn schema_report_query() -> Value {
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
pub(super) fn schema_report_for_file() -> Value {
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
pub(super) fn schema_report_for_range() -> Value {
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
pub(super) fn schema_find_similar() -> Value {
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
pub(super) fn schema_cluster_by_id() -> Value {
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
pub(super) fn schema_set_embedding_model() -> Value {
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
