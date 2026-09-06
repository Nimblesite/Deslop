//! JSON schema builders for each MCP tool's input parameters.

use deslop_core::{buckets::ClusterKind, pipeline::language_ids};
use serde_json::{json, Value};

/// The closed `language` enum, derived from the core parser registry so the
/// tool schemas can never drift from the set of supported languages
/// ([MCP-TOOL-REPORT-QUERY]). Fixes the omission of `dart`.
fn language_enum() -> Value {
    Value::Array(language_ids().into_iter().map(Value::from).collect())
}

/// The closed `bucket` enum, derived from the canonical [`ClusterKind`]
/// registry so the filter can never drift from the buckets the engine
/// emits ([CLONE-BUCKETS]) — the #170/#198 anti-drift lesson applied to
/// buckets. Notably includes `structural_only` so agents can exclude
/// (or isolate) demoted shape-only families ([RANK-STRUCTURAL-ONLY],
///).
fn bucket_enum() -> Value {
    Value::Array(
        ClusterKind::all()
            .into_iter()
            .map(|kind| Value::from(kind.wire_label()))
            .collect(),
    )
}

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
            "limit": { "type": "integer", "minimum": 0, "description": "Max clusters in this page. Pick a sensible value for your context window." },
            "include_per_file": { "type": "boolean", "default": false, "description": "Include the per-file duplication breakdown (one row per analysed file). Off by default: on a large workspace it alone can exceed the whole result budget." }
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
            "language": { "type": "string", "enum": language_enum(), "description": "Match clusters whose detected source language equals this id." },
            "bucket": { "type": "string", "enum": bucket_enum(), "description": "Match clusters whose canonical bucket equals this id." },
            "path_contains": { "type": "string", "description": "Case-sensitive substring match against any occurrence path on the cluster." },
            "min_score": { "type": "number", "description": "Inclusive ranking-score floor." },
            "min_size": { "type": "integer", "minimum": 0, "description": "Inclusive subtree-node-count floor (canonical_node_count)." },
            "include_per_file": { "type": "boolean", "default": false, "description": "Include the per-file duplication breakdown (one row per analysed file). Off by default: on a large workspace it alone can exceed the whole result budget." }
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
            "path": { "type": "string", "description": "Absolute or workspace-relative path." },
            "max_occurrences": { "type": "integer", "minimum": 1, "default": 15, "description": "Total occurrence budget across returned clusters. Worst-first; cluster that overruns the budget is truncated and following clusters are dropped. Result still reports total_occurrences for the unfiltered count." }
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
            "end_byte": { "type": "integer", "minimum": 0 },
            "max_occurrences": { "type": "integer", "minimum": 1, "default": 15, "description": "Total occurrence budget across returned clusters. See top-offenders for semantics." }
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
                "enum": language_enum()
            },
            "top_n": { "type": "integer", "minimum": 0, "default": 5 },
            "max_occurrences": { "type": "integer", "minimum": 1, "default": 15, "description": "Total occurrence budget across returned clusters. See top-offenders for semantics." }
        },
        "additionalProperties": false,
    })
}

/// Schema for `top-offenders`.
pub(super) fn schema_top_offenders() -> Value {
    json!({
        "type": "object",
        "properties": {
            "n": { "type": "integer", "minimum": 1, "default": 5, "description": "Max clusters to return." },
            "max_occurrences": { "type": "integer", "minimum": 1, "default": 15, "description": "Total occurrence budget across returned clusters. Worst-first: clusters are added until the next one would push occurrences past this budget; the overrunning cluster is truncated and following clusters are dropped. The result reports total_occurrences for the unfiltered count, plus occurrences_truncated per cluster." }
        },
        "additionalProperties": false,
    })
}

/// Schema for `rescan`.
pub(super) fn schema_rescan() -> Value {
    json!({
        "type": "object",
        "properties": {
            "paths": {
                "type": "array",
                "items": { "type": "string", "minLength": 1 },
                "description": "Optional absolute or workspace-relative paths changed by the caller."
            },
            "n": { "type": "integer", "minimum": 1, "default": 5, "description": "Max clusters to return after reload." },
            "max_occurrences": { "type": "integer", "minimum": 1, "default": 15, "description": "Total occurrence budget across returned clusters. See top-offenders for semantics." }
        },
        "additionalProperties": false,
    })
}

/// `merge-plan` input schema ([AUTOFIX-MERGE-MCP]).
pub(super) fn schema_merge_plan() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "description": "Stable cluster identifier (from any report tool or LSP diagnostic)." }
        },
        "required": ["id"],
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
            "provider_id": { "type": "string", "enum": ["ollama"] },
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
