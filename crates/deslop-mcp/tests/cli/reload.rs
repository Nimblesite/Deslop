//! Live-reload tests: `filesChanged` notifications, the synchronous
//! `rescan` tool, generation/incremental reporting, and the
//! server-pushed notifications a reload emits.

use anyhow::Result;
use common::{
    mcp_calls::{broad_cluster_count, call_tool, finish_after_tools_list, structured_tool_result},
    mcp_child::{
        mutate_two_and_notify, poll_total_clusters_below, spawn_and_init,
        two_file_workspace_with_state, McpChild, FIXTURE_MIN_NODES, SECOND_FILE_NAME,
    },
    mcp_wire::{
        BROAD_RESULT_LIMIT, CSHARP_LANGUAGE, DEFAULT_MIN_NODES, DUPLICATES_TOOL, ID_FIELD,
        LANGUAGES_POINTER, LIMIT_PARAM, OFFSET_PARAM, PATHS_FIELD, REPORT_URI, RESCAN_TOOL,
        SESSION_TOOL, TOTAL_CLUSTERS_POINTER,
    },
    value_get,
};

use crate::common;

use serde_json::{json, Value};

/// [MCP-TOOLS] `rescan` wraps the fresh page: `{ generation, summary, page }`.
const RESCAN_TOTAL_CLUSTERS_POINTER: &str = "/page/total_clusters";
const RESCAN_CLUSTERS_POINTER: &str = "/page/clusters";
const RESCAN_PAGE_LIMIT_POINTER: &str = "/page/page/limit";
/// JSON field: a notification frame's method name.
const METHOD_FIELD: &str = "method";
/// Replacement body that dissolves the Alpha/Beta clone.
const UNIQUE_REPLACEMENT_BODY: &str = "namespace Solo { class Only { public int Go() => 1; } }\n";
/// The mutated fixture file whose clone disappears.
const BETA_FILE_NAME: &str = "Beta.cs";

#[test]
fn mark_changed_is_idempotent_across_second_session() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let first_count = broad_cluster_count(&mut child)?;
    assert!(first_count >= 1, "expected at least one cluster initially");
    let _ = child.finish();
    std::fs::write(
        temp.path().join(SECOND_FILE_NAME),
        "namespace Lone { class Only { public int Go() => 1; } }\n",
    )?;
    common::mcp_child::generate_state_file(temp.path(), FIXTURE_MIN_NODES)?;
    let mut second = McpChild::spawn(temp.path(), &[])?;
    let _ = common::mcp_child::init_session(&mut second)?;
    let rerun_count = broad_cluster_count(&mut second)?;
    assert!(
        rerun_count < first_count,
        "after mutating Two.cs, cluster count must drop; was {first_count}, now {rerun_count}"
    );
    let _ = second.finish();
    Ok(())
}

#[test]
fn files_changed_notification_triggers_reanalysis() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let before_count = broad_cluster_count(&mut child)?;
    assert!(before_count >= 1, "expected at least one cluster");
    // Edit Two.cs so the clone disappears, regenerate the state file, then notify.
    mutate_two_and_notify(&mut child, temp.path())?;
    // mark_changed triggers the reload; poll until the report settles to
    // its post-reload state so the watcher-debounced pass cannot race the
    // read under heavy CI load (see poll_total_clusters_below).
    let after = poll_total_clusters_below(
        &mut child,
        DUPLICATES_TOOL,
        &common::mcp_calls::broad_page_arguments(),
        before_count,
    )?;
    let after_count = value_get(&after, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0);
    assert!(
        after_count < before_count,
        "filesChanged notification must reload the state file and drop the Two.cs clone; was {before_count}, now {after_count}"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_77_session_reports_incremental_true_after_mutation_reload() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let before_count = broad_cluster_count(&mut child)?;
    assert!(before_count >= 1, "expected a cluster before mutation");
    let before_config = structured_tool_result(&call_tool(&mut child, SESSION_TOOL, &json!({}))?)?;
    let before_generation = value_get(&before_config, "/generation")?
        .as_u64()
        .unwrap_or(0);
    assert!(
        before_generation >= 1,
        "initial generation should load state"
    );
    assert!(
        value_get(&before_config, LANGUAGES_POINTER)?
            .as_array()
            .is_some_and(|languages| languages.iter().any(|value| value == CSHARP_LANGUAGE)),
        "session should report csharp before mutation: {before_config}"
    );

    mutate_two_and_notify(&mut child, temp.path())?;

    let after_page = poll_total_clusters_below(
        &mut child,
        DUPLICATES_TOOL,
        &common::mcp_calls::broad_page_arguments(),
        before_count,
    )?;
    let after_count = value_get(&after_page, TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0);
    assert!(
        after_count < before_count,
        "mutation reload should remove the stale duplicate cluster"
    );
    let after_config = structured_tool_result(&call_tool(&mut child, SESSION_TOOL, &json!({}))?)?;
    // [MCP-IPC-CLIENT] min_nodes now comes from the live LSP, not
    // the test's `generate_state_file` invocation — LSP defaults
    // to 30.
    assert_eq!(
        value_get(&after_config, "/min_nodes")?.as_u64(),
        Some(DEFAULT_MIN_NODES)
    );
    assert!(
        value_get(&after_config, LANGUAGES_POINTER)?.is_array(),
        "session should keep languages shaped as an array: {after_config}"
    );
    assert!(
        value_get(&after_config, "/generation")?
            .as_u64()
            .unwrap_or(0)
            > before_generation,
        "filesChanged reload should advance the MCP generation"
    );
    assert_eq!(
        value_get(&after_config, "/incremental")?.as_bool(),
        Some(true),
        "issue #77/#81: session must report live incremental mode after mutation reload"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn issue_89_rescan_tool_reloads_state_file_and_returns_fresh_duplicates() -> Result<()> {
    // [MCP-IPC-CLIENT] rescan triggers the LSP's
    // `deslop.lsp.refreshReport` over IPC and the next read sees the
    // re-analysed state. Mutating source (not the seed cache) is the
    // only way to influence the LSP under the new architecture.
    let workspace = common::copied_fixture()?;
    let mut child = McpChild::spawn(workspace.path(), &[])?;
    let _ = common::mcp_child::init_session(&mut child)?;
    // Flush the LSP cold-pass install before measuring `before` so
    // the post-mutation rescan does not race a delayed background
    // commit that would re-introduce the stale cluster.
    let _flush = call_tool(&mut child, RESCAN_TOOL, &json!({}))?;
    let before_count = broad_cluster_count(&mut child)?;
    assert!(
        before_count >= 1,
        "expected at least one cluster before edit"
    );

    // Replace Beta.cs with a unique implementation so the duplicate
    // cluster between Alpha.cs and Beta.cs disappears after rescan.
    std::fs::write(
        workspace.path().join(BETA_FILE_NAME),
        UNIQUE_REPLACEMENT_BODY,
    )?;

    let after = structured_tool_result(&call_tool(
        &mut child,
        RESCAN_TOOL,
        &json!({
            (PATHS_FIELD): [workspace.path().join(BETA_FILE_NAME).to_string_lossy().into_owned()],
            (OFFSET_PARAM): 0,
            (LIMIT_PARAM): BROAD_RESULT_LIMIT
        }),
    )?)?;
    let after_count = value_get(&after, RESCAN_TOTAL_CLUSTERS_POINTER)?
        .as_u64()
        .unwrap_or(0);
    assert!(
        after_count < before_count,
        "issue #89: rescan must synchronously trigger LSP re-analysis and return a fresh page; was {before_count}, now {after_count}"
    );
    assert_eq!(
        value_get(&after, RESCAN_PAGE_LIMIT_POINTER)?.as_u64(),
        Some(BROAD_RESULT_LIMIT),
        "issue #89: rescan must echo the requested page limit"
    );
    assert!(
        value_get(&after, RESCAN_CLUSTERS_POINTER)?.is_array(),
        "issue #89: rescan must return a clusters page"
    );
    let _ = child.finish();
    Ok(())
}

#[test]
fn files_changed_notification_with_empty_paths_is_a_noop() -> Result<()> {
    let mut child = spawn_and_init()?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ (PATHS_FIELD): [] }),
    )?;
    // Server must remain responsive after a no-op notification.
    finish_after_tools_list(child)
}

/// [MCP-RESOURCES-SUBSCRIPTIONS] Only subscribers receive standard updates.
#[test]
fn report_edits_send_standard_updates_only_while_subscribed() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    let init = child.initialize("resource-subscription-test")?;
    assert_eq!(
        init.pointer("/result/capabilities/resources/subscribe"),
        Some(&json!(true))
    );
    let params = json!({ "uri": REPORT_URI });
    assert_subscription_response(&mut child, "resources/subscribe", &params)?;
    mutate_two_and_notify(&mut child, temp.path())?;
    assert_resource_updated(&child.read_frame()?);
    assert_report_changed(&child.read_frame()?);

    assert_subscription_response(&mut child, "resources/unsubscribe", &params)?;
    let source = temp.path().join(SECOND_FILE_NAME);
    std::fs::write(&source, include_str!("../fixtures/csharp-mcp/Beta.cs"))?;
    child.notify(
        "notifications/deslop/filesChanged",
        &json!({ (PATHS_FIELD): [source] }),
    )?;
    assert_report_changed(&child.read_frame()?);
    finish_after_tools_list(child)
}

#[test]
fn report_edits_send_only_custom_updates_before_subscribing() -> Result<()> {
    let (temp, mut child) = two_file_workspace_with_state()?;
    mutate_two_and_notify(&mut child, temp.path())?;
    assert_report_changed(&child.read_frame()?);
    finish_after_tools_list(child)
}

fn assert_subscription_response(child: &mut McpChild, method: &str, params: &Value) -> Result<()> {
    let response = child.request(method, params)?;
    assert_eq!(
        response.get("result"),
        Some(&json!({})),
        "{method}: {response}"
    );
    Ok(())
}

fn assert_resource_updated(frame1: &Value) {
    assert_eq!(
        frame1.get(METHOD_FIELD).and_then(Value::as_str),
        Some("notifications/resources/updated"),
        "first pushed frame must be resources/updated: {frame1}"
    );
    assert!(
        frame1.get(ID_FIELD).is_none(),
        "notification must not carry an id: {frame1}"
    );
    assert_eq!(
        frame1.pointer("/params/uri").and_then(Value::as_str),
        Some(REPORT_URI),
        "resources/updated must name deslop://report: {frame1}"
    );
}

fn assert_report_changed(frame2: &Value) {
    assert_eq!(
        frame2.get(METHOD_FIELD).and_then(Value::as_str),
        Some("notifications/deslop/reportChanged"),
        "expected a deslop/reportChanged notification: {frame2}"
    );
    assert!(
        frame2.get(ID_FIELD).is_none(),
        "notification must not carry an id: {frame2}"
    );
    assert!(
        frame2
            .pointer("/params/generation")
            .and_then(Value::as_u64)
            .is_some(),
        "reportChanged must include a numeric generation: {frame2}"
    );
}
