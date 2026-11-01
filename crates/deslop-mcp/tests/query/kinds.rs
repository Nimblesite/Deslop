//! [MCP-TOOL-FILTERS-KINDS] Configured clone kinds and per-call overrides.

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use deslop_core::{buckets::ClusterKind, report::ReportCluster};
use serde_json::{json, Value};

use crate::common::{
    call_tool, copied_fixture, spawn_lsp_with_args, wait_for_state_then_init_mcp, ChildKillOnDrop,
    McpHandle,
};

/// A persisted selection whose fixture has no matching cluster.
const IDENTICAL_CONFIG: &str = "[total]\ninclude = [\"identical\"]\n";
const CONFIG_NAME: &str = ".deslop.toml";
const IDENTICAL: &str = "identical";
const NEARLY_IDENTICAL: &str = "nearly_identical";
const DUPLICATES: &str = "duplicates";
const FULL: &str = "full";
const PAGE_LIMIT: usize = 10;
const OCCURRENCE_BUDGET: usize = 100;
const EXPECTED_CLUSTERS: usize = 2;
const EXPECTED_COPIES: usize = 2;
const NO_MATCHES: usize = 0;
const FIRST_RANK: usize = 1;
const SECOND_RANK: usize = 2;
const FIRST_PATHS: [&str; EXPECTED_COPIES] = ["Alpha.cs", "Beta.cs"];
const SECOND_PATHS: [&str; EXPECTED_COPIES] = ["Delta.cs", "Gamma.cs"];
const INVALID_KIND: &str = "not_a_clone_kind";
const INVALID_PARAMS: i64 = -32_602;
const RESCAN: &str = "rescan";
const FIND_SIMILAR: &str = "find-similar";
const CSHARP: &str = "csharp";
const SNIPPET: &str = include_str!("../fixtures/csharp-mcp/Alpha.cs");
const SECOND_SNIPPET: &str = include_str!("../fixtures/csharp-mcp/Gamma.cs");
const LIST_TOOLS: [&str; 3] = [DUPLICATES, FIND_SIMILAR, RESCAN];
const FILTER_FIELDS: [&str; 5] = [
    "kinds",
    "languages",
    "severities",
    "path_contains",
    "min_size",
];
const SEVERITIES: [&str; 5] = ["error", "warning", "information", "hint", "none"];
const OTHER_LANGUAGE: &str = "rust";
const MISSING_PATH: &str = "missing.cs";
const TOOLS_LIST: &str = "tools/list";
const SINGLE_RESULT: usize = 1;
const FIXTURE_EXTENSION: &str = ".cs";
const NEGATIVE_SIZE: i64 = -1;
const INTERSECTION_CASES: usize = 32;

/// The same complete page shape for inherited and explicit selections.
fn arguments(kinds: Option<&[&str]>) -> Value {
    let mut args = serde_json::Map::from_iter([
        ("detail".to_owned(), json!(FULL)),
        ("limit".to_owned(), json!(PAGE_LIMIT)),
        ("max_occurrences".to_owned(), json!(OCCURRENCE_BUDGET)),
    ]);
    args.extend(kinds.map(|kinds| ("kinds".to_owned(), json!(kinds))));
    Value::Object(args)
}

/// Changes only the kind selection on a known object-shaped request.
fn set_kinds(args: &mut Value, kinds: Value) -> Result<()> {
    let _previous = args
        .as_object_mut()
        .context("request must be an object")?
        .insert("kinds".to_owned(), kinds);
    Ok(())
}

/// A missing wire field is an assertion failure, never an implicit null.
fn field<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value> {
    value
        .pointer(pointer)
        .ok_or_else(|| anyhow!("response is missing asserted field {pointer}: {value}"))
}

/// Pins the actual ranked pairs so an empty detector cannot pass the test.
fn assert_nearly_identical_page(page: &Value) -> Result<()> {
    let clusters: Vec<ReportCluster> = serde_json::from_value(field(page, "/clusters")?.clone())?;
    assert_eq!(clusters.len(), EXPECTED_CLUSTERS);
    let expected = [(FIRST_RANK, FIRST_PATHS), (SECOND_RANK, SECOND_PATHS)];
    for (cluster, (rank, paths)) in clusters.iter().zip(expected) {
        assert_eq!(cluster.kind, ClusterKind::NearlyIdentical);
        assert_eq!(cluster.rank, rank);
        assert_eq!(cluster.occurrence_count, EXPECTED_COPIES);
        assert_eq!(cluster.occurrences.len(), EXPECTED_COPIES);
        assert!(!cluster.id.is_empty(), "the pair has a stable identity");
        let actual: Vec<_> = cluster.occurrences.iter().map(|item| &item.path).collect();
        assert_eq!(actual, paths.map(PathBuf::from).iter().collect::<Vec<_>>());
    }
    assert_eq!(field(page, "/total_clusters")?, &json!(EXPECTED_CLUSTERS));
    Ok(())
}

/// A configured identical-only query must agree with its counted kinds.
fn assert_identical_only_page(page: &Value) -> Result<()> {
    assert_eq!(field(page, "/metrics/total_includes")?, &json!([IDENTICAL]));
    assert_eq!(
        field(page, "/clusters")?,
        &json!([]),
        "identical-only configuration must exclude the fixture's nearly identical pairs"
    );
    assert_eq!(field(page, "/total_clusters")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/total_occurrences")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/page/returned")?, &json!(NO_MATCHES));
    assert_eq!(field(page, "/filters/kinds")?, &json!([IDENTICAL]));
    Ok(())
}

/// One explicit selection must not change what the next omitted selection inherits.
fn check_queries(mcp: &mut McpHandle) -> Result<()> {
    let override_nearly = arguments(Some(&[NEARLY_IDENTICAL]));
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?)?;
    let override_identical = arguments(Some(&[IDENTICAL]));
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &override_identical)?)?;
    assert_nearly_identical_page(&call_tool(mcp, DUPLICATES, &override_nearly)?)?;
    assert_identical_only_page(&call_tool(mcp, DUPLICATES, &arguments(None))?)?;
    Ok(())
}

/// Every cluster-returning tool inherits the identical-only policy.
fn check_shared_filters(mcp: &mut McpHandle) -> Result<()> {
    let refreshed = call_tool(mcp, RESCAN, &arguments(None))?;
    assert_identical_only_page(field(&refreshed, "/page")?)?;
    let inherited = json!({"snippet": SNIPPET, "language": CSHARP});
    assert_identical_only_page(&call_tool(mcp, FIND_SIMILAR, &inherited)?)?;
    let mut explicit = inherited;
    set_kinds(&mut explicit, json!([NEARLY_IDENTICAL]))?;
    let found = call_tool(mcp, FIND_SIMILAR, &explicit)?;
    assert!(field(&found, "/clusters")?
        .as_array()
        .is_some_and(|rows| !rows.is_empty()));
    assert_eq!(field(&found, "/filters/kinds")?, &json!([NEARLY_IDENTICAL]));
    Ok(())
}

/// Invalid overrides must be rejected rather than ignored as a broader selection.
fn check_invalid_kinds(mcp: &mut McpHandle) -> Result<()> {
    for invalid in [json!([INVALID_KIND]), json!(IDENTICAL), Value::Null] {
        let mut args = arguments(None);
        set_kinds(&mut args, invalid)?;
        assert_invalid_arguments(mcp, DUPLICATES, &args)?;
    }
    Ok(())
}

/// [MCP-TOOL-FILTERS-KINDS] Real LSP and MCP processes share one configured policy.
#[test]
fn configured_clone_kinds_are_inherited_and_explicit_arguments_override_one_call() -> Result<()> {
    let workspace = copied_fixture()?;
    fs::write(workspace.path().join(CONFIG_NAME), IDENTICAL_CONFIG)?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp = ChildKillOnDrop(lsp);
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    check_queries(&mut mcp)?;
    check_shared_filters(&mut mcp)?;
    check_invalid_kinds(&mut mcp)
}

/// [MCP-TOOL-FILTERS-KINDS] The default selection and explicit empty set differ.
#[test]
fn default_clone_kinds_and_explicit_empty_selection_are_reported_truthfully() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp = ChildKillOnDrop(lsp);
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    let inherited = call_tool(&mut mcp, DUPLICATES, &arguments(None))?;
    assert_nearly_identical_page(&inherited)?;
    assert_eq!(
        field(&inherited, "/filters/kinds")?,
        &json!([IDENTICAL, NEARLY_IDENTICAL])
    );
    let empty = call_tool(&mut mcp, DUPLICATES, &arguments(Some(&[])))?;
    assert_eq!(field(&empty, "/clusters")?, &json!([]));
    assert_eq!(field(&empty, "/filters/kinds")?, &json!([]));
    assert_eq!(field(&empty, "/total_clusters")?, &json!(NO_MATCHES));
    assert_nearly_identical_page(&call_tool(&mut mcp, DUPLICATES, &arguments(None))?)
}

/// [MCP-TOOL-FILTERS] Every result-list tool advertises and applies the same filters.
#[test]
fn every_result_list_applies_shared_filter_arguments_before_pagination() -> Result<()> {
    let workspace = copied_fixture()?;
    let lsp = spawn_lsp_with_args(workspace.path(), &["--ipc-transport", "tcp"])?;
    let _lsp = ChildKillOnDrop(lsp);
    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;
    assert_filter_schemas(&mut mcp)?;
    let baseline = call_tool(&mut mcp, DUPLICATES, &arguments(None))?;
    assert_nearly_identical_page(&baseline)?;
    let clusters: Vec<ReportCluster> =
        serde_json::from_value(field(&baseline, "/clusters")?.clone())?;
    for tool in LIST_TOOLS {
        check_filter_matrix(&mut mcp, tool, &clusters)?;
    }
    Ok(())
}

/// Schemas share complete filter definitions, including closed enum values.
fn assert_filter_schemas(mcp: &mut McpHandle) -> Result<()> {
    let response = mcp.request(TOOLS_LIST, &json!({}))?;
    let tools = field(&response, "/result/tools")?
        .as_array()
        .context("tool definitions")?;
    let canonical = tool_schema(tools, DUPLICATES)?;
    for name in LIST_TOOLS {
        let tool = tool_schema(tools, name)?;
        for filter in FILTER_FIELDS {
            let pointer = format!("/inputSchema/properties/{filter}");
            assert_eq!(
                field(tool, &pointer)?,
                field(canonical, &pointer)?,
                "{name}: {filter}"
            );
        }
    }
    Ok(())
}

/// Select the public tool definition being asserted.
fn tool_schema<'a>(tools: &'a [Value], name: &str) -> Result<&'a Value> {
    tools
        .iter()
        .find(|tool| tool["name"] == name)
        .context("tool schema")
}

/// Query the same two ranked pairs through each public result-list route.
fn filtered_page(mcp: &mut McpHandle, tool: &str, filters: &Value) -> Result<Value> {
    let result = call_tool(mcp, tool, &tool_arguments(tool, filters)?)?;
    if tool == RESCAN {
        Ok(field(&result, "/page")?.clone())
    } else {
        Ok(result)
    }
}

/// Build one canonical request for successful and rejected argument checks.
fn tool_arguments(tool: &str, filters: &Value) -> Result<Value> {
    let mut args = arguments(None);
    let object = args.as_object_mut().context("query object")?;
    object.extend(filters.as_object().context("filter object")?.clone());
    if tool == FIND_SIMILAR {
        let _old = object.insert(
            "snippet".to_owned(),
            json!(format!("{SNIPPET}\n{SECOND_SNIPPET}")),
        );
        let _old = object.insert("language".to_owned(), json!(CSHARP));
    }
    Ok(args)
}

/// Full rows must retain identity, ranking, kind, every member, and honest totals.
fn assert_filtered_page(page: &Value, expected: &[ReportCluster], filters: &Value) -> Result<()> {
    assert_eq!(field(page, "/clusters")?, &serde_json::to_value(expected)?);
    assert_eq!(field(page, "/total_clusters")?, &json!(expected.len()));
    let occurrences: usize = expected
        .iter()
        .map(|cluster| cluster.occurrence_count)
        .sum();
    assert_eq!(field(page, "/total_occurrences")?, &json!(occurrences));
    assert_eq!(field(page, "/page/returned")?, &json!(expected.len()));
    assert_eq!(field(page, "/page/has_more")?, &json!(false));
    for (name, value) in filters.as_object().context("filter object")? {
        assert_eq!(field(page, &format!("/filters/{name}"))?, value);
    }
    Ok(())
}

/// Positive selections and empty intersections use the same canonical cluster rows.
fn check_filter_matrix(mcp: &mut McpHandle, tool: &str, clusters: &[ReportCluster]) -> Result<()> {
    let cases = [
        (json!({}), clusters),
        (json!({"languages": [CSHARP]}), clusters),
        (json!({"languages": []}), clusters),
        (json!({"severities": []}), clusters),
        (json!({"kinds": [NEARLY_IDENTICAL]}), clusters),
    ];
    for (filters, expected) in cases {
        assert_filtered_page(&filtered_page(mcp, tool, &filters)?, expected, &filters)?;
    }
    check_empty_filters(mcp, tool)?;
    check_severity_filters(mcp, tool, clusters)?;
    check_size_and_intersection(mcp, tool, clusters)?;
    check_filter_intersections(mcp, tool, clusters)?;
    check_invalid_filter_arguments(mcp, tool, clusters)
}

/// Malformed filters must return `InvalidParams` and leave the next query intact.
fn check_invalid_filter_arguments(
    mcp: &mut McpHandle,
    tool: &str,
    clusters: &[ReportCluster],
) -> Result<()> {
    for filters in malformed_filter_cases() {
        let args = tool_arguments(tool, &filters)?;
        assert_invalid_arguments(mcp, tool, &args)?;
        assert_unfiltered(mcp, tool, clusters)?;
    }
    Ok(())
}

/// Errors must be typed protocol failures rather than empty successful pages.
fn assert_invalid_arguments(mcp: &mut McpHandle, tool: &str, args: &Value) -> Result<()> {
    let response = mcp.request("tools/call", &json!({"name": tool, "arguments": args}))?;
    assert_eq!(
        field(&response, "/error/code")?,
        &json!(INVALID_PARAMS),
        "{tool}: {args}"
    );
    Ok(())
}

/// Cover wrong container types, wrong element types, negative sizes and nulls.
fn malformed_filter_cases() -> Vec<Value> {
    vec![
        json!({"kinds": NEARLY_IDENTICAL}),
        json!({"kinds": [INVALID_KIND]}),
        json!({"languages": CSHARP}),
        json!({"languages": [false]}),
        json!({"severities": SEVERITIES[NO_MATCHES]}),
        json!({"severities": [false]}),
        json!({"path_contains": [MISSING_PATH]}),
        json!({"path_contains": null}),
        json!({"min_size": NEGATIVE_SIZE}),
        json!({"min_size": false}),
    ]
}

/// A filtered or rejected request must leave the complete next query intact.
fn assert_unfiltered(mcp: &mut McpHandle, tool: &str, clusters: &[ReportCluster]) -> Result<()> {
    let filters = json!({});
    assert_filtered_page(&filtered_page(mcp, tool, &filters)?, clusters, &filters)
}

/// Every combination of matching and rejecting predicates must preserve AND semantics.
fn check_filter_intersections(
    mcp: &mut McpHandle,
    tool: &str,
    clusters: &[ReportCluster],
) -> Result<()> {
    let cases = intersection_cases(clusters)?;
    assert_eq!(cases.len(), INTERSECTION_CASES);
    for (filters, matches) in cases {
        assert_eq!(
            filters.as_object().context("filters")?.len(),
            FILTER_FIELDS.len()
        );
        let expected = if matches { clusters } else { &[] };
        assert_filtered_page(&filtered_page(mcp, tool, &filters)?, expected, &filters)?;
        assert_unfiltered(mcp, tool, clusters)?;
    }
    Ok(())
}

/// Multi-value selections are unions; the five distinct predicates intersect.
fn intersection_cases(clusters: &[ReportCluster]) -> Result<Vec<(Value, bool)>> {
    let included = [
        json!([IDENTICAL, NEARLY_IDENTICAL]),
        json!([OTHER_LANGUAGE, CSHARP]),
        json!(SEVERITIES),
        json!(FIXTURE_EXTENSION),
        json!(NO_MATCHES),
    ];
    let excluded = rejecting_filter_values(clusters)?;
    FILTER_FIELDS.iter().zip(included).zip(excluded).try_fold(
        vec![(json!({}), true)],
        |cases, ((name, included), excluded)| {
            expand_intersections(cases, name, &included, &excluded)
        },
    )
}

/// Use valid values which reject every fixture cluster for each predicate.
fn rejecting_filter_values(clusters: &[ReportCluster]) -> Result<[Value; FILTER_FIELDS.len()]> {
    let absent_severity = SEVERITIES
        .iter()
        .find(|severity| clusters.iter().all(|row| row.severity != **severity))
        .context("fixture must leave one valid severity unrepresented")?;
    Ok([
        json!([IDENTICAL]),
        json!([OTHER_LANGUAGE]),
        json!([absent_severity]),
        json!(MISSING_PATH),
        json!(size_excluding_all(clusters)?),
    ])
}

/// Extend the Cartesian product without dropping prior rejecting predicates.
fn expand_intersections(
    cases: Vec<(Value, bool)>,
    name: &str,
    included: &Value,
    excluded: &Value,
) -> Result<Vec<(Value, bool)>> {
    let mut expanded = Vec::new();
    for (filters, prior_match) in cases {
        for (value, matches) in [(included, true), (excluded, false)] {
            let mut filters = filters.clone();
            let _old = filters
                .as_object_mut()
                .context("filters")?
                .insert(name.to_owned(), value.clone());
            expanded.push((filters, prior_match && matches));
        }
    }
    Ok(expanded)
}

/// Each restrictive filter must independently exclude nonmatching clusters.
fn check_empty_filters(mcp: &mut McpHandle, tool: &str) -> Result<()> {
    for filters in [
        json!({"languages": [OTHER_LANGUAGE]}),
        json!({"kinds": [IDENTICAL]}),
        json!({"kinds": []}),
        json!({"path_contains": MISSING_PATH}),
    ] {
        assert_filtered_page(&filtered_page(mcp, tool, &filters)?, &[], &filters)?;
    }
    Ok(())
}

/// Every advertised diagnostic severity selects exactly its owning clusters.
fn check_severity_filters(
    mcp: &mut McpHandle,
    tool: &str,
    clusters: &[ReportCluster],
) -> Result<()> {
    for severity in SEVERITIES {
        let filters = json!({"severities": [severity]});
        let expected: Vec<_> = clusters
            .iter()
            .filter(|row| row.severity == severity)
            .cloned()
            .collect();
        assert_filtered_page(&filtered_page(mcp, tool, &filters)?, &expected, &filters)?;
    }
    Ok(())
}

/// Combined filters must select a later pair before the one-row limit is applied.
fn check_size_and_intersection(
    mcp: &mut McpHandle,
    tool: &str,
    clusters: &[ReportCluster],
) -> Result<()> {
    let too_large = json!({"min_size": size_excluding_all(clusters)?});
    assert_filtered_page(&filtered_page(mcp, tool, &too_large)?, &[], &too_large)?;
    let selected = clusters.last().context("second pair")?;
    let filters = json!({"kinds": [NEARLY_IDENTICAL], "languages": [CSHARP],
        "severities": [selected.severity], "path_contains": SECOND_PATHS[FIRST_RANK],
        "min_size": selected.canonical_node_count});
    let mut controls = filters.clone();
    let _old = controls
        .as_object_mut()
        .context("controls")?
        .insert("limit".to_owned(), json!(SINGLE_RESULT));
    let page = filtered_page(mcp, tool, &controls)?;
    assert_filtered_page(&page, std::slice::from_ref(selected), &filters)?;
    Ok(())
}

/// Select a checked threshold strictly above every fixture cluster's size.
fn size_excluding_all(clusters: &[ReportCluster]) -> Result<usize> {
    clusters
        .iter()
        .map(|row| row.canonical_node_count)
        .max()
        .context("fixture sizes")?
        .checked_add(SINGLE_RESULT)
        .context("fixture size overflow")
}
