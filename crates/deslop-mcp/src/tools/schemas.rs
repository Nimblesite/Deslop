//! JSON schema builders for each MCP tool's input parameters.

use deslop_core::{buckets::ClusterKind, pipeline::language_ids};
use serde_json::{json, Value};

/// JSON Schema keyword declaring a value's type.
const TYPE_KEY: &str = "type";
/// JSON Schema keyword introducing an object's property map.
const PROPERTIES_KEY: &str = "properties";
/// JSON Schema keyword controlling undeclared properties.
const ADDITIONAL_PROPERTIES_KEY: &str = "additionalProperties";
/// JSON Schema keyword listing an object's mandatory properties.
const REQUIRED_KEY: &str = "required";
/// JSON Schema keyword bounding a number from below.
const MINIMUM_KEY: &str = "minimum";
/// JSON Schema keyword giving a property's default value.
const DEFAULT_KEY: &str = "default";
/// JSON Schema keyword carrying human-readable documentation.
const DESCRIPTION_KEY: &str = "description";
/// JSON Schema keyword restricting a value to a fixed set.
const ENUM_KEY: &str = "enum";
/// JSON Schema keyword bounding a string's length from below.
const MIN_LENGTH_KEY: &str = "minLength";
/// JSON Schema `object` type name.
const OBJECT_TYPE: &str = "object";
/// JSON Schema `integer` type name.
const INTEGER_TYPE: &str = "integer";
/// JSON Schema `string` type name.
const STRING_TYPE: &str = "string";
/// JSON Schema `boolean` type name.
const BOOLEAN_TYPE: &str = "boolean";
/// JSON Schema `number` type name.
const NUMBER_TYPE: &str = "number";
/// Tool parameter naming the first cluster of a page.
const OFFSET_PROPERTY: &str = "offset";
/// Tool parameter naming a page's maximum cluster count.
const LIMIT_PROPERTY: &str = "limit";
/// Tool parameter opting into per-file duplication totals.
const INCLUDE_PER_FILE_PROPERTY: &str = "include_per_file";
/// Tool parameter naming the file a request is scoped to.
const PATH_PROPERTY: &str = "path";
/// Tool parameter carrying a range's inclusive start byte.
const START_BYTE_PROPERTY: &str = "start_byte";
/// Tool parameter carrying a range's exclusive end byte.
const END_BYTE_PROPERTY: &str = "end_byte";
/// Tool parameter capping the occurrences a response may carry.
const MAX_OCCURRENCES_PROPERTY: &str = "max_occurrences";
/// Tool parameter naming the cluster id a request is scoped to.
const ID_PROPERTY: &str = "id";
/// Lower bound for parameters where zero is meaningful (an offset).
const MINIMUM_ZERO: u8 = 0;
/// Lower bound for parameters where zero is meaningless (a count).
const MINIMUM_ONE: u8 = 1;
/// Default number of clusters `top-offenders` returns.
const DEFAULT_TOP_COUNT: u8 = 5;
/// Default occurrence budget across a response's clusters.
const DEFAULT_MAX_OCCURRENCES: u8 = 15;
/// Shared documentation for every `path` parameter.
const PATH_DESCRIPTION: &str = "Absolute or workspace-relative path.";
/// Shared documentation for the compact occurrence budget.
const COMPACT_MAX_OCCURRENCES_DESCRIPTION: &str =
    "Total occurrence budget across returned clusters. See top-offenders for semantics.";

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
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {},
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `report-get`. Both pagination knobs are required so the
/// agent always states its context budget explicitly
/// ([MCP-TOOL-REPORT-PAGINATION]).
pub(super) fn schema_report_get() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (OFFSET_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO, (DESCRIPTION_KEY): "Zero-based cluster index to start at." },
            (LIMIT_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO, (DESCRIPTION_KEY): "Max clusters in this page. Pick a sensible value for your context window." },
            (INCLUDE_PER_FILE_PROPERTY): { (TYPE_KEY): BOOLEAN_TYPE, (DEFAULT_KEY): false, (DESCRIPTION_KEY): "Include the per-file duplication breakdown (one row per analysed file). Off by default: on a large workspace it alone can exceed the whole result budget." }
        },
        (REQUIRED_KEY): [OFFSET_PROPERTY, LIMIT_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `report-query`. Same pagination contract as `report-get`
/// plus optional filter knobs ([MCP-TOOL-REPORT-QUERY]).
pub(super) fn schema_report_query() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (OFFSET_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            (LIMIT_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            "language": { (TYPE_KEY): STRING_TYPE, (ENUM_KEY): language_enum(), (DESCRIPTION_KEY): "Match clusters whose detected source language equals this id." },
            "bucket": { (TYPE_KEY): STRING_TYPE, (ENUM_KEY): bucket_enum(), (DESCRIPTION_KEY): "Match clusters whose canonical bucket equals this id." },
            "path_contains": { (TYPE_KEY): STRING_TYPE, (DESCRIPTION_KEY): "Case-sensitive substring match against any occurrence path on the cluster." },
            "min_score": { (TYPE_KEY): NUMBER_TYPE, (DESCRIPTION_KEY): "Inclusive ranking-score floor." },
            "min_size": { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO, (DESCRIPTION_KEY): "Inclusive subtree-node-count floor (canonical_node_count)." },
            (INCLUDE_PER_FILE_PROPERTY): { (TYPE_KEY): BOOLEAN_TYPE, (DEFAULT_KEY): false, (DESCRIPTION_KEY): "Include the per-file duplication breakdown (one row per analysed file). Off by default: on a large workspace it alone can exceed the whole result budget." }
        },
        (REQUIRED_KEY): [OFFSET_PROPERTY, LIMIT_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `report-for-file`.
pub(super) fn schema_report_for_file() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (PATH_PROPERTY): { (TYPE_KEY): STRING_TYPE, (DESCRIPTION_KEY): PATH_DESCRIPTION },
            (MAX_OCCURRENCES_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_MAX_OCCURRENCES, (DESCRIPTION_KEY): "Total occurrence budget across returned clusters. Worst-first; cluster that overruns the budget is truncated and following clusters are dropped. Result still reports total_occurrences for the unfiltered count." }
        },
        (REQUIRED_KEY): [PATH_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `report-for-range`.
pub(super) fn schema_report_for_range() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (PATH_PROPERTY): { (TYPE_KEY): STRING_TYPE, (DESCRIPTION_KEY): PATH_DESCRIPTION },
            (START_BYTE_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            (END_BYTE_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            (MAX_OCCURRENCES_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_MAX_OCCURRENCES, (DESCRIPTION_KEY): COMPACT_MAX_OCCURRENCES_DESCRIPTION }
        },
        (REQUIRED_KEY): [PATH_PROPERTY, START_BYTE_PROPERTY, END_BYTE_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `find-similar`.
pub(super) fn schema_find_similar() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (PATH_PROPERTY): { (TYPE_KEY): STRING_TYPE },
            (START_BYTE_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            (END_BYTE_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO },
            "snippet": { (TYPE_KEY): STRING_TYPE },
            "language": {
                (TYPE_KEY): STRING_TYPE,
                (ENUM_KEY): language_enum()
            },
            "top_n": { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ZERO, (DEFAULT_KEY): DEFAULT_TOP_COUNT },
            (MAX_OCCURRENCES_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_MAX_OCCURRENCES, (DESCRIPTION_KEY): COMPACT_MAX_OCCURRENCES_DESCRIPTION }
        },
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `top-offenders`.
pub(super) fn schema_top_offenders() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            "n": { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_TOP_COUNT, (DESCRIPTION_KEY): "Max clusters to return." },
            (MAX_OCCURRENCES_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_MAX_OCCURRENCES, (DESCRIPTION_KEY): "Total occurrence budget across returned clusters. Worst-first: clusters are added until the next one would push occurrences past this budget; the overrunning cluster is truncated and following clusters are dropped. The result reports total_occurrences for the unfiltered count, plus occurrences_truncated per cluster." }
        },
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `rescan`.
pub(super) fn schema_rescan() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            "paths": {
                (TYPE_KEY): "array",
                "items": { (TYPE_KEY): STRING_TYPE, (MIN_LENGTH_KEY): MINIMUM_ONE },
                (DESCRIPTION_KEY): "Optional absolute or workspace-relative paths changed by the caller."
            },
            "n": { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_TOP_COUNT, (DESCRIPTION_KEY): "Max clusters to return after reload." },
            (MAX_OCCURRENCES_PROPERTY): { (TYPE_KEY): INTEGER_TYPE, (MINIMUM_KEY): MINIMUM_ONE, (DEFAULT_KEY): DEFAULT_MAX_OCCURRENCES, (DESCRIPTION_KEY): COMPACT_MAX_OCCURRENCES_DESCRIPTION }
        },
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// `merge-plan` input schema ([AUTOFIX-MERGE-MCP]).
pub(super) fn schema_merge_plan() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (ID_PROPERTY): { (TYPE_KEY): STRING_TYPE, (DESCRIPTION_KEY): "Stable cluster identifier (from any report tool or LSP diagnostic)." }
        },
        (REQUIRED_KEY): [ID_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `cluster-by-id`.
pub(super) fn schema_cluster_by_id() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            (ID_PROPERTY): { (TYPE_KEY): STRING_TYPE, (MIN_LENGTH_KEY): MINIMUM_ONE }
        },
        (REQUIRED_KEY): [ID_PROPERTY],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}

/// Schema for `set-embedding-model`.
pub(super) fn schema_set_embedding_model() -> Value {
    json!({
        (TYPE_KEY): OBJECT_TYPE,
        (PROPERTIES_KEY): {
            "provider_id": { (TYPE_KEY): STRING_TYPE, (ENUM_KEY): ["ollama"] },
            "model_id": { (TYPE_KEY): STRING_TYPE, (MIN_LENGTH_KEY): MINIMUM_ONE },
            "endpoint": { (TYPE_KEY): STRING_TYPE, (DESCRIPTION_KEY): "Optional override (Ollama only)." },
            "user_initiated": {
                (TYPE_KEY): BOOLEAN_TYPE,
                "const": true,
                (DESCRIPTION_KEY): "Must be true only when a human explicitly requested this model switch."
            }
        },
        (REQUIRED_KEY): ["provider_id", "model_id", "user_initiated"],
        (ADDITIONAL_PROPERTIES_KEY): false,
    })
}
