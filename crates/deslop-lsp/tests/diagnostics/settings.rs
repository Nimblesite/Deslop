//! [SEVERITY-TESTING] Settings update a real LSP without another scan.

use anyhow::{ensure, Result};
use serde_json::{json, Value};

use crate::common::{notification, read_frame, session::FixtureSession, write_frame};

const FIXTURE: &str = "csharp-small";
const PRIMARY: &str = "Alpha.cs";
const DIAGNOSTIC: &str = "textDocument/diagnostic";
const CONFIG_CHANGED: &str = "workspace/didChangeConfiguration";
const REPORT: &str = deslop_lsp::custom_methods::REPORT_GET;
const WARNING: u64 = 2;
const ERROR: u64 = 1;
const PUBLISH: &str = "textDocument/publishDiagnostics";

fn pull(session: &mut FixtureSession) -> Result<Vec<Value>> {
    let params = json!({"textDocument": {"uri": session.file_uri(PRIMARY)?}});
    let response = session.call(DIAGNOSTIC, &params)?;
    response
        .pointer("/result/items")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("diagnostic response must contain items: {response}"))
}

fn configure(session: &mut FixtureSession, diagnostics: &Value) -> Result<()> {
    let params = json!({"settings": {"deslop": {"diagnostics": diagnostics}}});
    write_frame(&mut session.stdin, &notification(CONFIG_CHANGED, &params)?)
}

fn assert_level(items: &[Value], severity: u64) -> Result<()> {
    ensure!(!items.is_empty(), "fixture must publish its real clones");
    for item in items {
        assert_eq!(item.get("severity").and_then(Value::as_u64), Some(severity));
        assert!(item
            .pointer("/data/cluster_id")
            .and_then(Value::as_str)
            .is_some());
        assert!(item
            .pointer("/relatedInformation/0/location/uri")
            .and_then(Value::as_str)
            .is_some());
    }
    Ok(())
}

#[test]
fn settings_enable_override_and_clear_diagnostics_without_reanalysis() -> Result<()> {
    let mut session = FixtureSession::open(FIXTURE)?;
    let before = session.call(REPORT, &json!({}))?;
    assert!(pull(&mut session)?.is_empty(), "diagnostics default to off");
    configure(&mut session, &json!({"enabled": true}))?;
    assert_level(&pull(&mut session)?, WARNING)?;
    let overrides = json!({"identical": "error", "nearly_identical": "error"});
    configure(
        &mut session,
        &json!({"enabled": true, "severityByKind": overrides}),
    )?;
    assert_level(&pull(&mut session)?, ERROR)?;
    configure(
        &mut session,
        &json!({"enabled": false, "severityByKind": overrides}),
    )?;
    assert!(
        pull(&mut session)?.is_empty(),
        "master switch overrides Error"
    );
    let after = session.call(REPORT, &json!({}))?;
    assert_eq!(
        before.get("result"),
        after.get("result"),
        "diagnostic settings cannot alter the report"
    );
    Ok(())
}

fn pushed_for_primary(session: &mut FixtureSession) -> Result<Vec<Value>> {
    let uri = session.file_uri(PRIMARY)?;
    loop {
        let frame = read_frame(&mut session.stdout)?;
        if frame.get("method") != Some(&json!(PUBLISH))
            || frame.pointer("/params/uri") != Some(&json!(uri))
        {
            continue;
        }
        let items = frame
            .pointer("/params/diagnostics")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("publication needs diagnostic items: {frame}"))?;
        assert_eq!(items, pull(session)?, "pull and push must agree");
        return Ok(items);
    }
}

#[test]
fn workspace_diagnostics_match_pull_and_clear_when_disabled() -> Result<()> {
    let mut session = FixtureSession::open(FIXTURE)?;
    configure(
        &mut session,
        &json!({"enabled": true, "scope": "workspace"}),
    )?;
    assert_level(&pushed_for_primary(&mut session)?, WARNING)?;
    configure(
        &mut session,
        &json!({"enabled": false, "scope": "workspace"}),
    )?;
    assert!(
        pushed_for_primary(&mut session)?.is_empty(),
        "disabled diagnostics must clear the Problems panel"
    );
    Ok(())
}
