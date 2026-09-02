//! Regression test for issue #135.
//!
//! Inside one MCP session, the `generation` returned by `rescan` must
//! match the `generation` returned by the immediately following
//! `duplicates` and `session`. Agents use this counter to detect
//! stale results — three different counters in the same session means
//! they cannot tell which MCP result reflects the current codebase.

#![cfg(unix)]

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

use crate::common;
use common::{
    copied_fixture, spawn_lsp_and_wait_for_socket, structured_content,
    wait_for_state_then_init_mcp, McpHandle,
};

/// Issue #135: `rescan`, `session`, and `duplicates` must all
/// report the same `generation` for the same report state.
#[test]
fn issue_135_rescan_generation_matches_report_get_and_session_config() -> Result<()> {
    let workspace = copied_fixture()?;
    let beta = workspace.path().join("Beta.cs");
    let _lsp_guard = spawn_lsp_and_wait_for_socket(workspace.path())?;

    let mut mcp = wait_for_state_then_init_mcp(workspace.path())?;

    // Edit Beta.cs so the LSP must do at least one re-analysis when
    // rescan asks it to refresh. This bumps the LSP-internal generation
    // counter past the MCP backend's local counter, exposing the bug.
    std::fs::write(
        &beta,
        b"namespace Solo { class Only { public int Go() => 1; } }\n",
    )?;

    let rescan = mcp.request(
        "tools/call",
        &json!({
            "name": "rescan",
            "arguments": {
                "paths": [beta.to_string_lossy().into_owned()],
                "n": 1
            }
        }),
    )?;
    let rescan_structured = structured_content(&rescan, "rescan")?;
    let rescan_generation = read_generation(&rescan_structured, "rescan", &rescan)?;

    let session = mcp.request("tools/call", &json!({ "name": "session", "arguments": {} }))?;
    let session_structured = structured_content(&session, "session")?;
    let session_generation = read_generation(&session_structured, "session", &session)?;

    let report = call_report_get(&mut mcp)?;
    let report_structured = structured_content(&report, "duplicates")?;
    let report_generation = read_generation(&report_structured, "duplicates", &report)?;

    ensure!(
        rescan_generation == session_generation,
        "issue #135: rescan generation ({rescan_generation}) must match the next session generation ({session_generation}); rescan={rescan_structured} session={session_structured}"
    );
    ensure!(
        rescan_generation == report_generation,
        "issue #135: rescan generation ({rescan_generation}) must match the next duplicates generation ({report_generation}); rescan={rescan_structured} report={report_structured}"
    );
    Ok(())
}

fn call_report_get(mcp: &mut McpHandle) -> Result<Value> {
    mcp.request(
        "tools/call",
        &json!({
            "name": "duplicates",
            "arguments": { "offset": 0, "limit": 0 }
        }),
    )
}

fn read_generation(structured: &Value, tool: &str, response: &Value) -> Result<u64> {
    structured
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            anyhow!("{tool} structured content missing numeric generation: structured={structured} response={response}")
        })
}
