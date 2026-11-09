//! `duplicates` page-contract tests: ranking by mass, the occurrence
//! budget, default paging, the slim summary shape, and the
//! cluster-by-id round-trip into the deep dive.

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{call_tool, init_and_tool_payload, structured_tool_result},
    mcp_child::McpChild,
    mcp_pages::{
        assert_clusters_omit, page_clusters, shipped_occurrences, DEEP_DIVE_KEY_REASON,
        RETIRED_CLUSTER_KEYS, RETIRED_CLUSTER_SUMMARY_KEYS, RETIRED_KEY_REASON,
        SUMMARY_DEEP_DIVE_KEYS,
    },
    mcp_wire::{
        CLUSTERS_POINTER, CLUSTER_BY_ID_TOOL, DETAIL_FIELD, DETAIL_SUMMARY, DUPLICATES_TOOL,
        END_BYTE_FIELD, ID_FIELD, ID_POINTER, LANGUAGE_FIELD, LIMIT_PARAM, MASS_FIELD,
        OCCURRENCES_FIELD, OFFSET_PARAM, PATH_FIELD, QUERY_PAGE_LIMIT,
        REQUESTED_TOP_OFFENDERS_COUNT, SCHEMA_DOC_FIELD, SEVERITY_FIELD, SMALL_PAGE_LIMIT,
        START_BYTE_FIELD, TOTAL_CLUSTERS_POINTER,
    },
    value_array, value_get,
};

use crate::common;

use serde_json::{json, Value};

/// JSON pointer: the echoed page limit.
const PAGE_LIMIT_POINTER: &str = "/page/limit";
/// JSON pointer: the unfiltered occurrence count.
const TOTAL_OCCURRENCES_POINTER: &str = "/total_occurrences";
/// JSON pointer: a summary row's own copy of its worst occurrence.
const FIRST_OCCURRENCE_POINTER: &str = "/first_occurrence";
/// Default per-response occurrence budget ([MCP-OCCURRENCE-BUDGET]).
const DEFAULT_MAX_OCCURRENCES: u64 = 15;
/// Cluster count the wider ranking checks request.
const DEFAULT_TOP_OFFENDERS_COUNT: usize = 5;
/// Page limit the standard paging assertions use.
const STANDARD_PAGE_LIMIT: u64 = 10;
/// Window size when checking a mass sequence is non-increasing.
const PAIR_WINDOW_SIZE: usize = 2;
/// Offset increment that is guaranteed past the end.
const OUT_OF_RANGE_OFFSET_INCREMENT: u64 = 100;
/// Serialised-page ceiling: the "fat" report-get era shipped 2.4MB and
/// blew out every agent context; pages must stay far below it.
const PAGE_BYTE_BUDGET: usize = 50_000;

#[test]
fn duplicates_returns_full_clusters_ranked_by_mass() -> Result<()> {
    let (child, payload) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (LIMIT_PARAM): REQUESTED_TOP_OFFENDERS_COUNT }),
    )?;
    let total = value_get(&payload, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be present"))?;
    assert!(total >= 1, "fixture must have at least one cluster");
    assert_eq!(
        value_get(&payload, PAGE_LIMIT_POINTER)?.as_u64(),
        Some(REQUESTED_TOP_OFFENDERS_COUNT as u64),
        "page must echo the requested limit"
    );
    let clusters_arr = page_clusters(&payload)?;
    assert!(
        clusters_arr.len() <= REQUESTED_TOP_OFFENDERS_COUNT,
        "returned {} clusters but requested max 3",
        clusters_arr.len()
    );
    let first = clusters_arr
        .first()
        .ok_or_else(|| anyhow!("at least one cluster expected"))?;
    assert!(
        first.get(OCCURRENCES_FIELD).is_some_and(Value::is_array),
        "duplicates full detail must return the occurrences array: {first}"
    );
    assert!(
        first
            .get(MASS_FIELD)
            .and_then(Value::as_u64)
            .is_some_and(|mass| mass > 0),
        "duplicates must return positive mass: {first}"
    );
    assert!(
        first
            .get(ID_FIELD)
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty()),
        "duplicates must return the stable cluster id: {first}"
    );
    assert!(
        first
            .get(SEVERITY_FIELD)
            .and_then(Value::as_str)
            .is_some_and(|band| !band.is_empty()),
        "duplicates must return the mass-derived rank band: {first}"
    );
    assert_clusters_omit(
        std::slice::from_ref(first),
        &RETIRED_CLUSTER_KEYS,
        RETIRED_KEY_REASON,
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_defaults_to_five_pages_worst_first_by_mass() -> Result<()> {
    let (child, payload) = init_and_tool_payload(DUPLICATES_TOOL, &json!({}))?;
    assert_eq!(
        value_get(&payload, PAGE_LIMIT_POINTER)?.as_u64(),
        Some(SMALL_PAGE_LIMIT),
        "omitting limit must default to 5"
    );
    let clusters_arr = page_clusters(&payload)?;
    assert!(
        u64::try_from(clusters_arr.len()).is_ok_and(|len| len <= SMALL_PAGE_LIMIT),
        "default limit=5 must not return more than 5 clusters"
    );
    let masses: Vec<u64> = clusters_arr
        .iter()
        .filter_map(|cluster| cluster.get(MASS_FIELD).and_then(Value::as_u64))
        .collect();
    assert_eq!(
        masses.len(),
        clusters_arr.len(),
        "every cluster must have a mass"
    );
    let sorted = masses
        .windows(PAIR_WINDOW_SIZE)
        .all(|pair| matches!(pair, [first, second] if first >= second));
    assert!(sorted, "clusters must be worst-first by mass: {masses:?}");
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_max_occurrences_caps_response_and_reports_total() -> Result<()> {
    // Issue #136 [MCP-OCCURRENCE-BUDGET]. A full-detail `duplicates` response
    // on a real workspace can ship 50+ occurrences per cluster — large enough
    // to crash some MCP clients (e.g. Codex's rmcp_client). The fix: a
    // `max_occurrences` budget across returned clusters that keeps the
    // response small while surfacing the true unfiltered count via
    // `total_occurrences` so the agent knows what was filtered.
    // cluster-by-id remains the way to fetch the full occurrence list of any
    // one cluster.
    let mut child = common::mcp_child::spawn_and_init()?;

    let baseline = call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({ (LIMIT_PARAM): DEFAULT_TOP_OFFENDERS_COUNT }),
    )?;
    let baseline_payload = structured_tool_result(&baseline)?;
    let baseline_total = value_get(&baseline_payload, TOTAL_OCCURRENCES_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("total_occurrences must be a non-negative integer"))?;
    assert!(
        baseline_total > 0,
        "fixture must contain at least one occurrence: {baseline_payload}"
    );

    // Pick a budget strictly below the baseline total so truncation MUST
    // fire (either per-cluster or by dropping trailing clusters).
    let budget = baseline_total
        .checked_sub(1)
        .filter(|&b| b > 0)
        .unwrap_or(1);
    let result = call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({ (LIMIT_PARAM): DEFAULT_TOP_OFFENDERS_COUNT, "max_occurrences": budget }),
    )?;
    let payload = structured_tool_result(&result)?;
    assert_eq!(
        value_get(&payload, TOTAL_OCCURRENCES_POINTER)?.as_u64(),
        Some(baseline_total),
        "total_occurrences must equal the unfiltered count, not the budgeted count"
    );
    let clusters_arr = page_clusters(&payload)?;
    let returned = shipped_occurrences(&clusters_arr);
    assert!(
        returned <= budget,
        "budget={budget} must yield at most {budget} occurrences total across returned clusters; got {returned}"
    );
    // The budget can manifest two ways: (a) a cluster shipped with
    // `occurrences_truncated=true` (its tail dropped), or (b) trailing
    // clusters were dropped entirely (returned_clusters < total_clusters
    // limited by `n`). Either is a valid truncation signal — assert at
    // least one fired given baseline_total exceeds the budget of 2.
    let total_clusters = value_get(&payload, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters missing"))?;
    let n_requested = value_get(&payload, PAGE_LIMIT_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("page.limit missing"))?;
    let cap = total_clusters.min(n_requested);
    let returned_clusters = clusters_arr.len() as u64;
    let truncation_marker_present = clusters_arr.iter().any(|cluster| {
        cluster
            .get("occurrences_truncated")
            .and_then(Value::as_bool)
            == Some(true)
    });
    let dropped_following_cluster = returned_clusters < cap;
    assert!(
        truncation_marker_present || dropped_following_cluster,
        "budget={budget} with baseline_total={baseline_total} must drop trailing clusters or set occurrences_truncated; got {returned_clusters}/{cap} clusters back: {clusters_arr:#?}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_default_max_occurrences_is_fifteen() -> Result<()> {
    let (child, payload) = init_and_tool_payload(DUPLICATES_TOOL, &json!({}))?;
    let clusters = page_clusters(&payload)?;
    let shipped = shipped_occurrences(&clusters);
    assert!(
        shipped <= DEFAULT_MAX_OCCURRENCES,
        "omitting max_occurrences must cap shipped occurrences at 15 \
         ([MCP-OCCURRENCE-BUDGET]); shipped {shipped}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_clusters_carry_no_per_cluster_labels() -> Result<()> {
    // Issue #134 pinned that structural-only matches must not be labelled
    // as ordinary nearly_identical duplication. The mass-only wire goes
    // further: a published cluster owns identity, extent, membership, mass
    // and mass-derived rank only ([PIPELINE-CLUSTER-CLOSURE]) — no bucket,
    // no signal block, no verdict can mislabel it on any surface.
    let (child, payload) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (LIMIT_PARAM): DEFAULT_TOP_OFFENDERS_COUNT }),
    )?;
    let clusters = page_clusters(&payload)?;
    assert!(
        !clusters.is_empty(),
        "fixture must surface at least one cluster for the label check"
    );
    assert_clusters_omit(&clusters, &RETIRED_CLUSTER_KEYS, RETIRED_KEY_REASON);
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_returns_paginated_page() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): STANDARD_PAGE_LIMIT }),
    )?;
    assert!(
        page.get("report_schema_version").is_none(),
        "report pages must not expose internal report-format revisions"
    );
    assert!(
        page.get(SCHEMA_DOC_FIELD).is_none(),
        "schema_doc must live behind schema-doc/deslop://schema, not every report page"
    );
    let total = value_get(&page, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters must be a number"))?;
    let returned = value_get(&page, "/page/returned")?
        .as_u64()
        .ok_or_else(|| anyhow!("page.returned missing"))?;
    assert_eq!(value_get(&page, "/page/offset")?, json!(0));
    assert_eq!(
        value_get(&page, PAGE_LIMIT_POINTER)?,
        json!(STANDARD_PAGE_LIMIT)
    );
    assert!(
        returned <= STANDARD_PAGE_LIMIT,
        "returned ({returned}) must respect requested limit"
    );
    assert!(
        total >= returned,
        "total_clusters ({total}) must be >= returned ({returned})"
    );
    assert!(total >= 1, "fixture should surface at least one cluster");
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_applies_default_offset_when_omitted() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (LIMIT_PARAM): STANDARD_PAGE_LIMIT }),
    )?;
    assert_eq!(
        value_get(&page, "/page/offset")?,
        json!(0),
        "omitting offset must page from the worst cluster"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_applies_default_limit_when_omitted() -> Result<()> {
    let (child, page) = init_and_tool_payload(DUPLICATES_TOOL, &json!({ (OFFSET_PARAM): 0 }))?;
    assert_eq!(
        value_get(&page, PAGE_LIMIT_POINTER)?,
        json!(SMALL_PAGE_LIMIT),
        "omitting limit must default to 5"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_clusters_are_slim_summaries_only() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): STANDARD_PAGE_LIMIT,
            (DETAIL_FIELD): DETAIL_SUMMARY,
        }),
    )?;
    let array = page_clusters(&page)?;
    assert!(!array.is_empty(), "fixture should produce >= 1 cluster");
    assert_clusters_omit(&array, &RETIRED_CLUSTER_SUMMARY_KEYS, RETIRED_KEY_REASON);
    assert_clusters_omit(&array, &SUMMARY_DEEP_DIVE_KEYS, DEEP_DIVE_KEY_REASON);
    for cluster in &array {
        for required in [
            ID_FIELD,
            MASS_FIELD,
            SEVERITY_FIELD,
            "size_nodes",
            "occurrence_count",
            LANGUAGE_FIELD,
            "first_occurrence",
        ] {
            assert!(
                cluster.get(required).is_some(),
                "ClusterSummary missing required field {required:?}: {cluster}"
            );
        }
        let first_occ = value_get(cluster, FIRST_OCCURRENCE_POINTER)?;
        for occ_field in [
            PATH_FIELD,
            START_BYTE_FIELD,
            END_BYTE_FIELD,
            "start_line",
            "end_line",
        ] {
            assert!(
                first_occ.get(occ_field).is_some(),
                "first_occurrence missing {occ_field:?}: {first_occ}"
            );
        }
        for line_field in ["start_line", "end_line"] {
            let line = value_get(&first_occ, &format!("/{line_field}"))?
                .as_i64()
                .ok_or_else(|| anyhow!("first_occurrence {line_field} must be an integer"))?;
            assert!(
                line >= 1,
                "first_occurrence {line_field} must be one-based: {first_occ}"
            );
        }
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_summary_first_occurrence_belongs_to_full_cluster() -> Result<()> {
    let (mut child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): STANDARD_PAGE_LIMIT,
            (DETAIL_FIELD): DETAIL_SUMMARY,
        }),
    )?;
    let clusters = value_array(&page, CLUSTERS_POINTER)?;
    assert!(!clusters.is_empty(), "fixture should produce >= 1 cluster");
    for summary in &clusters {
        assert_first_occurrence_matches_full_cluster(&mut child, summary)?;
    }
    let _ = child.finish();
    Ok(())
}

/// Asserts the summary row's `first_occurrence` also appears in the
/// `cluster-by-id` deep dive for the same cluster.
fn assert_first_occurrence_matches_full_cluster(
    child: &mut McpChild,
    summary: &Value,
) -> Result<()> {
    let id = value_get(summary, ID_POINTER)?;
    let first = value_get(summary, "/first_occurrence")?;
    let cluster = structured_tool_result(&call_tool(
        child,
        CLUSTER_BY_ID_TOOL,
        &json!({ (ID_FIELD): id }),
    )?)?;
    let occurrences = value_array(&cluster, "/occurrences")?;
    assert!(
        occurrences.iter().any(|occ| same_occurrence(occ, &first)),
        "first_occurrence must be present in cluster-by-id occurrences: {summary}"
    );
    Ok(())
}

/// Whether two occurrence objects name the same extent (path + byte span).
fn same_occurrence(left: &Value, right: &Value) -> bool {
    let left_path = left.get(PATH_FIELD).and_then(Value::as_str);
    let right_path = right.get(PATH_FIELD).and_then(Value::as_str);
    let left_start = left.get(START_BYTE_FIELD).and_then(Value::as_u64);
    let right_start = right.get(START_BYTE_FIELD).and_then(Value::as_u64);
    let left_end = left.get(END_BYTE_FIELD).and_then(Value::as_u64);
    let right_end = right.get(END_BYTE_FIELD).and_then(Value::as_u64);
    left_path == right_path && left_start == right_start && left_end == right_end
}

#[test]
fn duplicates_offset_past_end_returns_empty_page() -> Result<()> {
    let (mut child, probe) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): 1 }),
    )?;
    let total = value_get(&probe, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .ok_or_else(|| anyhow!("total_clusters missing"))?;
    let past = total.saturating_add(OUT_OF_RANGE_OFFSET_INCREMENT);
    let page = structured_tool_result(&call_tool(
        &mut child,
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): past, (LIMIT_PARAM): STANDARD_PAGE_LIMIT }),
    )?)?;
    assert_eq!(
        value_get(&page, "/page/returned")?,
        json!(0),
        "offset past end must return zero clusters"
    );
    assert!(
        value_get(&page, CLUSTERS_POINTER)?
            .as_array()
            .is_some_and(Vec::is_empty),
        "clusters[] must be empty when offset is past the end"
    );
    assert_eq!(
        value_get(&page, TOTAL_CLUSTERS_POINTER)?
            .as_u64()
            .unwrap_or(u64::MAX),
        total,
        "total_clusters must not change when paging past the end"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn duplicates_page_stays_under_byte_budget() -> Result<()> {
    let (child, page) = init_and_tool_payload(
        DUPLICATES_TOOL,
        &json!({ (OFFSET_PARAM): 0, (LIMIT_PARAM): QUERY_PAGE_LIMIT }),
    )?;
    let serialised = serde_json::to_string(&page)?;
    assert!(
        serialised.len() < PAGE_BYTE_BUDGET,
        "duplicates page exceeded 50KB budget: was {} bytes",
        serialised.len()
    );
    let _ = child.finish();
    Ok(())
}
