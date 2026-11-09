//! `duplicates` filter and scope tests: language / severity / min-size /
//! path narrowing, the filters echo, and the path-scoped page variants.

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{
        assert_tool_rejects_invalid_params, call_tool, init_and_tool_payload,
        init_and_tool_response, structured_tool_result,
    },
    mcp_child::workspace_with_extra_file,
    mcp_pages::{assert_empty_page, page_clusters},
    mcp_wire::{
        BROAD_RESULT_LIMIT, CLUSTERS_POINTER, CSHARP_LANGUAGE, DETAIL_FIELD, DETAIL_SUMMARY,
        DUPLICATES_TOOL, END_BYTE_FIELD, LANGUAGE_FIELD, LIMIT_PARAM, MIN_SIZE_FIELD, OFFSET_PARAM,
        PATH_FIELD, QUERY_PAGE_LIMIT, SEVERITIES_FIELD, SEVERITY_FIELD, SMALL_PAGE_LIMIT,
        START_BYTE_FIELD, TOTAL_CLUSTERS_POINTER,
    },
    value_array, value_get,
};

use crate::common;

use serde_json::{json, Value};

/// JSON pointer: the echo of the filters a page actually applied.
const FILTERS_POINTER: &str = "/filters";
/// [MCP-TOOLS] A knob retired with the twelve-tool surface.
const RETIRED_MIN_SCORE_KNOB: &str = "min_score";
/// A clone kind the retired `bucket` filter used to accept.
const LOOSELY_SIMILAR_KIND: &str = "loosely_similar";
/// `min_size` the echo test applies.
const MIN_SIZE_FILTER: u64 = 10;
/// `min_size` no fixture cluster can satisfy.
const EXCLUSIVE_MIN_SIZE: u64 = 99_999;
/// `start_byte` that inverts any range end below it.
const INVERTED_RANGE_START: u64 = BROAD_RESULT_LIMIT;
/// Substring no fixture path contains.
const NONMATCHING_PATH_SUBSTRING: &str = "ZZZ_NEVER_MATCHES_ANYTHING";
/// Severity band the fixture is guaranteed to carry.
const WARNING_SEVERITY: &str = "warning";
/// `end_byte` far past any fixture file's length.
const GENEROUS_RANGE_END: u64 = 10_000;

/// [MCP-TOOLS] A knob retired with the twelve-tool surface must neither
/// narrow the mass-only page nor appear in its filters echo — a client
/// must never be able to misread the filter as applied.
fn assert_retired_knob_is_ignored(knob: &str, value: Value) -> Result<()> {
    let plain = json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): QUERY_PAGE_LIMIT });
    let (baseline_child, baseline) = init_and_tool_payload(DUPLICATES_TOOL, &plain)?;
    let unfiltered_total = value_get(&baseline, TOTAL_CLUSTERS_POINTER)?.as_u64();
    let _ = baseline_child.finish();

    let mut with_knob = plain;
    let displaced = with_knob
        .as_object_mut()
        .ok_or_else(|| anyhow!("page arguments must be a JSON object"))?
        .insert(knob.to_owned(), value);
    assert!(
        displaced.is_none(),
        "the plain page arguments must not already carry {knob}"
    );
    let (child, page) = init_and_tool_payload(DUPLICATES_TOOL, &with_knob)?;
    let knobbed_total = value_get(&page, TOTAL_CLUSTERS_POINTER)?.as_u64();
    assert_eq!(
        knobbed_total, unfiltered_total,
        "retired {knob} must not narrow the mass-only page: \
         with={knobbed_total:?} without={unfiltered_total:?}"
    );
    let filters = value_get(&page, FILTERS_POINTER)?;
    assert!(
        filters.get(knob).is_none(),
        "filters echo must not carry a retired {knob} row: {filters}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_accepts_dart_language_filter() -> Result<()> {
    // Issue #170/#198: `duplicates` only *filters* already-detected clusters
    // by language — no parsing — yet the enum omitted `dart`, so a Dart
    // filter failed JSON-Schema validation with InvalidParams and there was
    // no workaround on Dart repos. The enum is derived from the core parser
    // registry, so the filter must be accepted (returning a, possibly empty,
    // page) rather than rejected.
    let (child, response) = init_and_tool_response(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): SMALL_PAGE_LIMIT,
            "languages": ["dart"],
        }),
    )?;
    assert!(
        response.get("error").is_none(),
        "issue #170/#198: duplicates must accept languages=[\"dart\"] at the \
         schema layer, not reject it as InvalidParams: {response}"
    );
    let page = structured_tool_result(
        response
            .get("result")
            .ok_or_else(|| anyhow!("duplicates must return a result: {response}"))?,
    )?;
    let clusters = value_get(&page, CLUSTERS_POINTER)?;
    assert!(
        clusters.is_array(),
        "duplicates must return a clusters array even when the language \
         filter matches nothing: {page}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_filters_by_language() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            "languages": [CSHARP_LANGUAGE],
            (DETAIL_FIELD): DETAIL_SUMMARY,
        }),
    )?;
    let array = page_clusters(&page)?;
    assert!(
        !array.is_empty(),
        "fixture should match >= 1 csharp cluster"
    );
    for cluster in array {
        assert_eq!(
            cluster.get(LANGUAGE_FIELD).and_then(Value::as_str),
            Some(CSHARP_LANGUAGE),
            "language filter not applied: {cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_filters_by_unknown_language_returns_empty() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            "languages": ["cobol"],
        }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_filters_by_path_contains() -> Result<()> {
    let (mut child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): QUERY_PAGE_LIMIT, "path_contains": "Alpha" }),
    )?;
    let array = value_array(&page, CLUSTERS_POINTER)?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted clone family"
    );
    // Every cluster on a path-filtered page must have at least one
    // occurrence whose path matches the filter — the filter narrows by
    // any occurrence, not only first_occurrence.
    for cluster in &array {
        let matched = cluster
            .pointer("/occurrences")
            .and_then(Value::as_array)
            .is_some_and(|occurrences| {
                occurrences.iter().any(|occurrence| {
                    occurrence
                        .pointer("/path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| path.contains("Alpha"))
                })
            });
        assert!(
            matched,
            "path_contains=Alpha page must only carry clusters with an Alpha occurrence: {cluster}"
        );
    }
    let unfiltered = structured_tool_result(&call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): 1 }),
    )?)?;
    let unfiltered_total = value_get(&unfiltered, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0);
    let filtered_total = value_get(&page, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0);
    assert!(
        filtered_total <= unfiltered_total,
        "filtered total ({filtered_total}) must be <= unfiltered total ({unfiltered_total})"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_filters_by_min_size() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            (MIN_SIZE_FIELD): 20,
            (DETAIL_FIELD): DETAIL_SUMMARY,
        }),
    )?;
    let clusters = value_get(&page, CLUSTERS_POINTER)?;
    for cluster in clusters.as_array().unwrap_or(&Vec::new()) {
        let size = cluster
            .get("size_nodes")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(
            size >= 20,
            "min_size=20 violated: cluster size_nodes={size}, cluster={cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_ignores_retired_min_score_argument() -> Result<()> {
    // The retired twelve-tool surface filtered pages by a fused pair
    // score. [MCP-TOOLS] deletes that surface wholesale: the mass-only
    // page must ignore the retired knob entirely rather than silently
    // re-filtering by a score that no longer exists on the wire.
    assert_retired_knob_is_ignored(RETIRED_MIN_SCORE_KNOB, json!(9_999_999.0))
}

#[test]
fn duplicates_filters_by_min_size_excludes_above_max() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            (MIN_SIZE_FIELD): EXCLUSIVE_MIN_SIZE,
        }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_ignores_retired_bucket_argument() -> Result<()> {
    // Buckets are retired from the wire with the twelve-tool surface.
    // The page must ignore a retired bucket filter and its echo must
    // not carry a bucket row, so no client can misread the filter as
    // applied ([MCP-TOOLS] normative cutover).
    assert_retired_knob_is_ignored("bucket", json!(LOOSELY_SIMILAR_KIND))
}

#[test]
fn duplicates_filters_by_nonmatching_path_returns_empty() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            "path_contains": NONMATCHING_PATH_SUBSTRING
        }),
    )?;
    assert_empty_page(&page)?;
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_filters_by_matching_severity_includes_clusters() -> Result<()> {
    // The retired bucket filter is superseded by the mass-derived
    // severity axis ([MCP-TOOL-FILTERS]): every returned cluster's
    // rank band must sit inside the requested set.
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): QUERY_PAGE_LIMIT,
            (SEVERITIES_FIELD): [WARNING_SEVERITY],
            (DETAIL_FIELD): DETAIL_SUMMARY,
        }),
    )?;
    let clusters = value_array(&page, CLUSTERS_POINTER)?;
    assert!(
        !clusters.is_empty(),
        "fixture has at least one worst-severity cluster"
    );
    for cluster in &clusters {
        assert_eq!(
            cluster.get(SEVERITY_FIELD).and_then(Value::as_str),
            Some(WARNING_SEVERITY),
            "severity filter not applied: {cluster}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_echoes_filters_in_response() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): SMALL_PAGE_LIMIT,
            "languages": [CSHARP_LANGUAGE],
            (MIN_SIZE_FIELD): MIN_SIZE_FILTER,
        }),
    )?;
    let filters = value_get(&page, FILTERS_POINTER)?;
    assert_eq!(filters.get("languages"), Some(&json!([CSHARP_LANGUAGE])));
    assert_eq!(filters.get(MIN_SIZE_FIELD), Some(&json!(MIN_SIZE_FILTER)));
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_scope_path_returns_only_matching_clusters() -> Result<()> {
    let (child, payload) =
        init_and_tool_payload(DUPLICATES_TOOL, &json!({ (PATH_FIELD): "Alpha.cs" }))?;
    let array = page_clusters(&payload)?;
    assert!(
        !array.is_empty(),
        "Alpha.cs participates in the planted Type-2 clone"
    );
    for cluster in array {
        let occurrences = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("occurrences missing"))?;
        let touches_alpha = occurrences
            .iter()
            .filter_map(|occ| occ.get(PATH_FIELD).and_then(Value::as_str))
            .any(|path| path.ends_with("Alpha.cs"));
        assert!(
            touches_alpha,
            "cluster must touch Alpha.cs, got {occurrences:?}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_scope_range_rejects_inverted_range() -> Result<()> {
    assert_tool_rejects_invalid_params(
        DUPLICATES_TOOL,
        &json!({ (PATH_FIELD): "Alpha.cs", (START_BYTE_FIELD): INVERTED_RANGE_START, (END_BYTE_FIELD): 1 }),
    )
}

#[test]
fn duplicates_scope_range_returns_empty_when_path_has_no_clusters() -> Result<()> {
    let (_workspace, mut child) = workspace_with_extra_file(
        "Lonely.cs",
        "namespace Lonely { class Solo { public int Uniq() => 42; } }",
    )?;
    let result = call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({
            (PATH_FIELD): "Lonely.cs",
            (START_BYTE_FIELD): 0,
            (END_BYTE_FIELD): GENEROUS_RANGE_END,
        }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, CLUSTERS_POINTER)?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "a unique-content file should not participate in any cluster"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_scope_path_on_unknown_path_returns_empty_clusters() -> Result<()> {
    let (_workspace, mut child) =
        workspace_with_extra_file("Ghost.cs", "namespace G { class G {} }")?;
    let result = call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({ (PATH_FIELD): "Ghost.cs" }),
    )?;
    let payload = structured_tool_result(&result)?;
    let clusters = value_get(&payload, CLUSTERS_POINTER)?;
    assert!(
        clusters.as_array().is_some_and(Vec::is_empty),
        "unknown file should produce no clusters"
    );
    let _ = child.finish();
    Ok(())
}
