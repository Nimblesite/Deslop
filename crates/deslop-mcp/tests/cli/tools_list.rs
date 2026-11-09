//! `tools/list` contract tests: the normative tool surface, its order,
//! schemas, and the descriptions that steer agents to the right tool.

use anyhow::{anyhow, Result};
use common::{
    mcp_calls::{call_tool, structured_tool_result},
    mcp_child::spawn_and_init,
    mcp_wire::{
        CLUSTER_BY_ID_TOOL, COMPARE_PAIR_TOOL, CSHARP_LANGUAGE, DUPLICATES_TOOL, END_BYTE_FIELD,
        FIND_SIMILAR_TOOL, LANGUAGES_FIELD, LANGUAGES_POINTER, LANGUAGE_FIELD, LIMIT_PARAM,
        MIN_SIZE_FIELD, PATH_CONTAINS_FIELD, PATH_FIELD, RESCAN_TOOL, SCHEMA_DOC_TOOL,
        SESSION_TOOL, SEVERITIES_FIELD, SNIPPET_FIELD, START_BYTE_FIELD, TOOLS_LIST_METHOD,
        TOOLS_LIST_POINTER,
    },
    str_field, value_get, NAME_FIELD,
};

use crate::common;

use serde_json::{json, Value};

/// Tool name: the merge planner ([MCP-TOOLS] eighth tool).
const MERGE_PLAN_TOOL: &str = "merge-plan";
/// Retired tool names that must never be advertised again.
const RETIRED_TOOLS: [&str; 8] = [
    "top-offenders",
    "report-get",
    "report-query",
    "report-for-file",
    "report-for-range",
    "session-config",
    "list-embedding-models",
    "set-embedding-model",
];
/// Issue #170/#198: languages the find-similar enum must list.
const REQUIRED_ENUM_LANGUAGES: [&str; 4] = [CSHARP_LANGUAGE, "rust", "python", "dart"];

#[test]
fn tools_list_returns_all_tools_with_schemas() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request(TOOLS_LIST_METHOD, &json!({}))?;
    let tools = value_get(&response, TOOLS_LIST_POINTER)?;
    let names: Vec<String> = tools
        .as_array()
        .ok_or_else(|| anyhow!("tools not array"))?
        .iter()
        .filter_map(|tool| {
            tool.get(NAME_FIELD)
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    // [MCP-TOOLS] the normative surface: seven core analysis tools plus the
    // separately specified merge planner. The retired twelve-tool
    // analysis-query surface must never reappear.
    let expected_order = [
        FIND_SIMILAR_TOOL,
        DUPLICATES_TOOL,
        COMPARE_PAIR_TOOL,
        CLUSTER_BY_ID_TOOL,
        RESCAN_TOOL,
        SESSION_TOOL,
        SCHEMA_DOC_TOOL,
        MERGE_PLAN_TOOL,
    ];
    assert_eq!(
        names, expected_order,
        "tools/list must return exactly the normative surface, in registry order"
    );
    for retired in RETIRED_TOOLS {
        assert!(
            !names.iter().any(|candidate| candidate == retired),
            "retired tool must not be advertised: {retired}"
        );
    }
    for tool in tools.as_array().unwrap_or(&Vec::new()) {
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!description.is_empty(), "tool missing description: {tool}");
        assert!(
            tool.get("inputSchema").is_some_and(Value::is_object),
            "tool missing inputSchema: {tool}"
        );
    }
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_113_find_similar_description_leads_with_prevention() -> Result<()> {
    let mut child = spawn_and_init()?;
    let response = child.request(TOOLS_LIST_METHOD, &json!({}))?;
    let tools_value = value_get(&response, TOOLS_LIST_POINTER)?;
    let tools = tools_value
        .as_array()
        .ok_or_else(|| anyhow!("tools/list result.tools must be an array"))?;
    let find_similar_tools: Vec<&Value> = tools
        .iter()
        .filter(|tool| tool.get(NAME_FIELD).and_then(Value::as_str) == Some(FIND_SIMILAR_TOOL))
        .collect();
    assert_eq!(
        find_similar_tools.len(),
        1,
        "issue #113: tools/list must expose exactly one find-similar tool: {tools:?}"
    );
    let tool = find_similar_tools
        .first()
        .ok_or_else(|| anyhow!("find-similar tool must be present"))?;
    let description = str_field(tool, "description")?;
    assert!(
        description.starts_with("Call before writing code to prevent duplication"),
        "issue #113: find-similar description must lead with prevention guidance: {description}"
    );
    assert!(
        description.contains("mass-ranked clusters"),
        "issue #113: find-similar description must name its mass-ranked product: {description}"
    );
    assert!(
        description.contains(COMPARE_PAIR_TOOL),
        "issue #113: find-similar description must route pair-evidence questions to compare-pair: {description}"
    );
    let schema = tool
        .get("inputSchema")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("find-similar tool must include an input schema object"))?;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("find-similar schema must include properties"))?;
    for field in [
        PATH_FIELD,
        START_BYTE_FIELD,
        END_BYTE_FIELD,
        SNIPPET_FIELD,
        LANGUAGE_FIELD,
        LIMIT_PARAM,
        "max_occurrences",
        LANGUAGES_FIELD,
        PATH_CONTAINS_FIELD,
        SEVERITIES_FIELD,
        MIN_SIZE_FIELD,
    ] {
        assert!(
            properties.contains_key(field),
            "issue #113: find-similar schema must document {field}: {properties:?}"
        );
    }
    // Issue #170/#198: the `language` enum is derived from the core parser
    // registry, so it must list every first-class language — including
    // `dart`, which `session-config` already reports. A hand-maintained
    // enum let `dart` fall off and made the filter unusable on Dart repos.
    let language_enum = properties
        .get(LANGUAGE_FIELD)
        .and_then(|language| language.get("enum"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("find-similar language must be a closed enum: {properties:?}"))?;
    let languages: Vec<&str> = language_enum.iter().filter_map(Value::as_str).collect();
    for expected in REQUIRED_ENUM_LANGUAGES {
        assert!(
            languages.contains(&expected),
            "issue #170/#198: find-similar language enum must include {expected}, got {languages:?}"
        );
    }
    // Issue #255: the advertised enum must equal the engine's *live*
    // registered languages, not the MCP binary's compile-time set — the
    // two silently drifted under MCP/engine version skew and disabled the
    // Rule-zero gate for newly detected languages. `session` reports the
    // live set, so the enum must match it exactly.
    let mut advertised: Vec<String> = languages.iter().map(|value| (*value).to_owned()).collect();
    let session = structured_tool_result(&call_tool(&mut child, SESSION_TOOL, &json!({}))?)?;
    let mut detected: Vec<String> = value_get(&session, LANGUAGES_POINTER)?
        .as_array()
        .ok_or_else(|| anyhow!("session-config languages must be an array"))?
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect();
    detected.sort();
    advertised.sort();
    assert_eq!(
        advertised, detected,
        "issue #255: find-similar language enum must equal the engine's detected languages"
    );
    let _ = child.finish();
    Ok(())
}
