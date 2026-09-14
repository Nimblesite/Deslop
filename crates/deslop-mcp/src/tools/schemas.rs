//! JSON schemas for the canonical MCP tools.

use deslop_core::pipeline::language_ids;
use serde_json::{json, Map, Value};

/// Default cluster count returned by paginated tools.
const DEFAULT_LIMIT: usize = 5;
/// Default full-detail occurrence budget.
const DEFAULT_MAX_OCCURRENCES: usize = 15;

/// Empty-parameter schema.
pub(super) fn schema_empty() -> Value {
    object_schema(&Map::new(), &[])
}

/// Schema for the one duplicate-report tool.
pub(super) fn schema_duplicates() -> Value {
    let mut properties = page_properties();
    properties.extend(filter_properties());
    properties.extend(scope_properties());
    object_schema(&properties, &[])
}

/// Schema for a forced refresh followed by a duplicates page.
pub(super) fn schema_rescan() -> Value {
    let mut properties = page_properties();
    properties.extend(filter_properties());
    let _old = properties.insert(
        "paths".to_owned(),
        json!({"type": "array", "items": {"type": "string", "minLength": 1}}),
    );
    object_schema(&properties, &[])
}

/// Schema for preventative similarity lookup.
pub(super) fn schema_find_similar() -> Value {
    let mut properties = filter_properties();
    properties.extend(scope_properties());
    let _old = properties.insert("snippet".to_owned(), json!({"type": "string"}));
    let _old = properties.insert(
        "language".to_owned(),
        json!({"type": "string", "enum": language_enum()}),
    );
    let _old = properties.insert(
        "limit".to_owned(),
        json!({"type": "integer", "minimum": 1, "default": DEFAULT_LIMIT}),
    );
    let _old = properties.insert(
        "max_occurrences".to_owned(),
        json!({"type": "integer", "minimum": 1, "default": DEFAULT_MAX_OCCURRENCES}),
    );
    object_schema(&properties, &[])
}

/// Schema for exact pair evidence.
pub(super) fn schema_compare_pair() -> Value {
    let mut properties = Map::new();
    let _old = properties.insert("left".to_owned(), endpoint_schema());
    let _old = properties.insert("right".to_owned(), endpoint_schema());
    object_schema(&properties, &["left", "right"])
}

/// Schema for one cluster drill-in.
pub(super) fn schema_cluster_by_id() -> Value {
    let mut properties = Map::new();
    let _old = properties.insert("id".to_owned(), json!({"type": "string", "minLength": 1}));
    let _old = properties.insert(
        "offset".to_owned(),
        json!({"type": "integer", "minimum": 0, "default": 0}),
    );
    object_schema(&properties, &["id"])
}

/// Schema for embedding/session management.
pub(super) fn schema_session() -> Value {
    let mut properties = Map::new();
    let _old = properties.insert(
        "action".to_owned(),
        json!({
            "type": "string",
            "enum": ["get", "list-embedding-models", "set-embedding-model"],
            "default": "get"
        }),
    );
    let _old = properties.insert(
        "provider_id".to_owned(),
        json!({"type": "string", "enum": ["ollama"]}),
    );
    let _old = properties.insert(
        "model_id".to_owned(),
        json!({"type": "string", "minLength": 1}),
    );
    let _old = properties.insert("endpoint".to_owned(), json!({"type": "string"}));
    let _old = properties.insert(
        "user_initiated".to_owned(),
        json!({"type": "boolean", "const": true}),
    );
    object_schema(&properties, &[])
}

/// Schema for the read-only merge planner.
pub(super) fn schema_merge_plan() -> Value {
    let mut properties = Map::new();
    let _old = properties.insert("id".to_owned(), json!({"type": "string", "minLength": 1}));
    object_schema(&properties, &["id"])
}

/// Shared cluster-owned filter block.
fn filter_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let _old = properties.insert(
        "languages".to_owned(),
        json!({"type": "array", "items": {"type": "string", "enum": language_enum()}}),
    );
    let _old = properties.insert("path_contains".to_owned(), json!({"type": "string"}));
    let _old = properties.insert(
        "severities".to_owned(),
        json!({
            "type": "array",
            "items": {"type": "string", "enum": ["worst", "top10", "mid", "faint"]}
        }),
    );
    let _old = properties.insert(
        "min_size".to_owned(),
        json!({"type": "integer", "minimum": 0}),
    );
    properties
}

/// Shared pagination and detail block.
fn page_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let _old = properties.insert(
        "offset".to_owned(),
        json!({"type": "integer", "minimum": 0, "default": 0}),
    );
    let _old = properties.insert(
        "limit".to_owned(),
        json!({"type": "integer", "minimum": 1, "default": DEFAULT_LIMIT}),
    );
    let _old = properties.insert(
        "detail".to_owned(),
        json!({"type": "string", "enum": ["full", "summary"], "default": "full"}),
    );
    let _old = properties.insert(
        "max_occurrences".to_owned(),
        json!({"type": "integer", "minimum": 1, "default": DEFAULT_MAX_OCCURRENCES}),
    );
    let _old = properties.insert(
        "include_per_file".to_owned(),
        json!({"type": "boolean", "default": false}),
    );
    properties
}

/// Optional file/range scope block.
fn scope_properties() -> Map<String, Value> {
    let mut properties = Map::new();
    let _old = properties.insert("path".to_owned(), json!({"type": "string"}));
    let _old = properties.insert(
        "start_byte".to_owned(),
        json!({"type": "integer", "minimum": 0}),
    );
    let _old = properties.insert(
        "end_byte".to_owned(),
        json!({"type": "integer", "minimum": 0}),
    );
    properties
}

/// Exact endpoint schema shared by left and right.
fn endpoint_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "minLength": 1},
            "start_byte": {"type": "integer", "minimum": 0},
            "end_byte": {"type": "integer", "minimum": 0}
        },
        "required": ["path", "start_byte", "end_byte"],
        "additionalProperties": false
    })
}

/// Registered language enum derived from the parser registry.
fn language_enum() -> Value {
    Value::Array(language_ids().into_iter().map(Value::from).collect())
}

/// Builds one strict object schema.
fn object_schema(properties: &Map<String, Value>, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}
