//! [OUTPUT-REPORT-REUSE] Real editor export commands retain the live generation.

use anyhow::{Context, Result};
use deslop_lsp::commands::{REFRESH_REPORT, RENDER_HTML_REPORT, REPORT_JSON};
use serde_json::{json, Value};

use crate::common::{at, session::FixtureSession};

/// Two renamed C# methods with one real duplicate group.
const FIXTURE: &str = "csharp-small";
/// The duplicate eliminated during the edit portion of the scenario.
const DELETED_COPY: &str = "Beta.cs";
/// Real LSP report query method.
const REPORT_GET: &str = "deslop/reportGet";
/// Real LSP generation query method.
const REPORT_DELTA: &str = "deslop/reportDelta";
/// Execute-command transport used by VS Code's HTML-report button.
const EXECUTE: &str = "workspace/executeCommand";
/// Number of repeated button clicks checked per snapshot.
const EXPORT_REPETITIONS: usize = 3;
/// Ranking and occurrence counts prove the fixture is visible before testing reuse.
const FIRST_RANK: u64 = 1;
/// Number of source copies in the initial duplicate.
const OCCURRENCES: u64 = 2;

#[test]
fn editor_html_and_json_exports_reuse_the_current_live_generation() -> Result<()> {
    let mut lsp = FixtureSession::open(FIXTURE)?;
    let report = current_report(&mut lsp)?;
    let first = at(&report, "clusters")
        .as_array()
        .context("clusters")?
        .first()
        .context("fixture must contain real duplicates")?;
    assert_eq!(at(first, "rank"), FIRST_RANK);
    assert_eq!(at(first, "occurrence_count"), OCCURRENCES);
    assert_snapshot_exports(&mut lsp, &report)?;
    std::fs::remove_file(lsp.workspace.path().join(DELETED_COPY))?;
    let _refresh = lsp.call(EXECUTE, &json!({"command": REFRESH_REPORT}))?;
    let changed = current_report(&mut lsp)?;
    assert_eq!(at(&changed, "clusters"), &json!([]));
    assert_snapshot_exports(&mut lsp, &changed)
}

/// Reads the canonical structured snapshot, through the editor's real transport.
///
/// [LIVE-OPEN-BUFFERS] The live answer also names the open buffers it
/// analysed, which is live transport state, not part of the report an export
/// writes; with no buffer open it names none, and is compared without it.
fn current_report(lsp: &mut FixtureSession) -> Result<Value> {
    let response = lsp.call(REPORT_GET, &json!({}))?;
    let mut report = response.get("result").cloned().context("report result")?;
    let analysed = report
        .as_object_mut()
        .and_then(|fields| fields.remove("analysed_buffers"));
    assert_eq!(
        analysed,
        Some(json!([])),
        "no buffer is open, so the live answer names none"
    );
    Ok(report)
}

/// Generation equality detects a refresh even when its calculated groups are equal.
fn generation(lsp: &mut FixtureSession) -> Result<u64> {
    let response = lsp.call(REPORT_DELTA, &json!({"since_generation": 0}))?;
    response
        .pointer("/result/to_generation")
        .and_then(Value::as_u64)
        .context("live generation")
}

/// Both standalone exports must preserve the published report and generation.
fn assert_snapshot_exports(lsp: &mut FixtureSession, expected: &Value) -> Result<()> {
    let before = generation(lsp)?;
    for _click in 0..EXPORT_REPETITIONS {
        assert_rendered_snapshot(lsp, expected)?;
        assert_eq!(generation(lsp)?, before, "export must not advance analysis");
    }
    Ok(())
}

/// Compares full calculated JSON and a real HTML document with the live snapshot.
fn assert_rendered_snapshot(lsp: &mut FixtureSession, expected: &Value) -> Result<()> {
    let html = command_string(lsp, RENDER_HTML_REPORT)?;
    assert!(
        html.starts_with("<!doctype html"),
        "standalone HTML document"
    );
    assert!(html.contains("Deslop report"), "populated report heading");
    let exported: Value = serde_json::from_str(&command_string(lsp, REPORT_JSON)?)?;
    assert_eq!(
        &exported, expected,
        "native JSON must reuse every calculated field"
    );
    assert_eq!(
        &current_report(lsp)?,
        expected,
        "canonical snapshot must stay unchanged"
    );
    Ok(())
}

/// Executes precisely the command VS Code requests, without a fake LSP.
fn command_string(lsp: &mut FixtureSession, command: &str) -> Result<String> {
    let response = lsp.call(EXECUTE, &json!({"command": command, "arguments": []}))?;
    response
        .get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .context("rendered report string")
}
