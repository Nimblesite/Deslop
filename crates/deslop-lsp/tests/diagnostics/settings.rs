//! [SEVERITY-TESTING] Settings update a real LSP without another scan.

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use tower_lsp::lsp_types::{
    ClientCapabilities, DiagnosticWorkspaceClientCapabilities, WorkspaceClientCapabilities,
};

use crate::common::{
    notification, path as json_path, read_frame, session::FixtureSession, write_frame,
};

const FIXTURE: &str = "csharp-small";
const PRIMARY: &str = "Alpha.cs";
const DIAGNOSTIC: &str = "textDocument/diagnostic";
const CONFIG_CHANGED: &str = "workspace/didChangeConfiguration";
const REPORT: &str = deslop_lsp::custom_methods::REPORT_GET;
const WARNING: u64 = 2;
const ERROR: u64 = 1;
const PUBLISH: &str = "textDocument/publishDiagnostics";
const WORKSPACE: &str = "workspace";
const NO_PUBLICATIONS: usize = 0;
const WORKSPACE_REFRESH: &str = "workspace/diagnostic/refresh";
const PEER: &str = "Beta.cs";
const PUBLISHED_FILES: usize = 2;
const ONE_DIAGNOSTIC: usize = 1;
const FIRST: usize = 0;
const CLONE_ID: &str = "54a6b248b64f425d";
const CLONE_KIND: &str = "nearly_identical";
const CLONE_MASS: u64 = 58;
const CLONE_RANK: u64 = 1;
const CLONE_MESSAGE: &str = "Nearly identical code × 2 — mass 58";
const RANGE_START: u64 = 0;
const RANGE_END_LINE: u64 = 18;
const RANGE_END_CHARACTER: u64 = 1;
const CANONICAL: &str = "Canonical";
const FILE_PAIRS: [(&str, &str); PUBLISHED_FILES] = [(PRIMARY, PEER), (PEER, PRIMARY)];
const MISSING_DIAGNOSTIC: &str = "the fixture must publish its real clone";
const MISSING_REFRESH_ID: &str = "refresh must have a request id";
const PUBLICATION_ITEMS: &[&str] = &["params", "diagnostics"];

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
        let items = publication_items(&frame)?;
        assert_eq!(items, pull(session)?, "pull and push must agree");
        return Ok(items);
    }
}

fn workspace_session() -> Result<FixtureSession> {
    let mut session = FixtureSession::open(FIXTURE)?;
    configure_workspace(&mut session, true)?;
    assert_level(&pushed_for_primary(&mut session)?, WARNING)?;
    Ok(session)
}

fn configure_workspace(session: &mut FixtureSession, enabled: bool) -> Result<()> {
    configure(session, &json!({"enabled": enabled, "scope": WORKSPACE}))
}

#[test]
fn workspace_diagnostics_match_pull_and_clear_when_disabled() -> Result<()> {
    let mut session = workspace_session()?;
    configure_workspace(&mut session, false)?;
    assert!(
        pushed_for_primary(&mut session)?.is_empty(),
        "disabled diagnostics must clear the Problems panel"
    );
    Ok(())
}

// [SEVERITY-CONFIG] Unchanged publications must not queue obsolete settings behind a clear.
#[test]
fn reapplying_workspace_settings_keeps_diagnostics_without_duplicate_publications() -> Result<()> {
    let mut session =
        FixtureSession::open_with_initialization(FIXTURE, &workspace_initialization())?;
    let initial = publication_batch(&mut session)?;
    assert_publications(&session, &initial, WARNING)?;
    let before = pull(&mut session)?;
    assert_level(&before, WARNING)?;
    configure_workspace(&mut session, true)?;
    let replay = publication_batch(&mut session)?;
    assert_eq!(replay.len(), NO_PUBLICATIONS, "{replay:?}");
    assert_eq!(pull(&mut session)?, before);
    assert_workspace_controls(&mut session)?;
    Ok(())
}

fn assert_publications(session: &FixtureSession, frames: &[Value], severity: u64) -> Result<()> {
    assert_eq!(frames.len(), PUBLISHED_FILES);
    for ((path, peer), frame) in FILE_PAIRS.iter().zip(frames) {
        assert_publication_uri(session, path, frame)?;
        let items = publication_items(frame)?;
        assert_level(&items, severity)?;
        assert_eq!(items.len(), ONE_DIAGNOSTIC);
        let item = items
            .get(FIRST)
            .ok_or_else(|| anyhow::anyhow!(MISSING_DIAGNOSTIC))?;
        assert_clone_item(session, peer, item)?;
    }
    Ok(())
}

fn assert_clone_item(session: &FixtureSession, peer: &str, item: &Value) -> Result<()> {
    assert_eq!(
        item.get("data"),
        Some(
            &json!({"cluster_id": CLONE_ID, "kind": CLONE_KIND, "mass": CLONE_MASS, "rank": CLONE_RANK})
        )
    );
    assert_eq!(item.get("message"), Some(&json!(CLONE_MESSAGE)));
    assert_eq!(item.get("range"), Some(&fixture_range()));
    assert_eq!(
        item.get("relatedInformation"),
        Some(
            &json!([{"location": {"range": fixture_range(), "uri": session.file_uri(peer)?}, "message": CANONICAL}])
        )
    );
    Ok(())
}

fn assert_publication_uri(session: &FixtureSession, path: &str, frame: &Value) -> Result<()> {
    assert_eq!(
        frame.pointer("/params/uri"),
        Some(&json!(session.file_uri(path)?))
    );
    Ok(())
}

fn publication_items(frame: &Value) -> Result<Vec<Value>> {
    serde_json::from_value(json_path(frame, PUBLICATION_ITEMS).clone())
        .with_context(|| format!("publication needs diagnostic items: {frame}"))
}

fn fixture_range() -> Value {
    json!({"start": {"line": RANGE_START, "character": RANGE_START},
        "end": {"line": RANGE_END_LINE, "character": RANGE_END_CHARACTER}})
}

fn assert_workspace_controls(session: &mut FixtureSession) -> Result<()> {
    let overrides = json!({"identical": "error", "nearly_identical": "error"});
    configure(
        session,
        &json!({"enabled": true, "scope": WORKSPACE, "severityByKind": overrides}),
    )?;
    let changed = publication_batch(session)?;
    assert_publications(session, &changed, ERROR)?;
    assert_level(&pull(session)?, ERROR)?;
    configure_workspace(session, false)?;
    let cleared = publication_batch(session)?;
    assert_cleared_publications(session, &cleared)?;
    configure_workspace(session, true)?;
    let restored = publication_batch(session)?;
    assert_publications(session, &restored, WARNING)
}

fn assert_cleared_publications(session: &FixtureSession, cleared: &[Value]) -> Result<()> {
    assert_eq!(cleared.len(), PUBLISHED_FILES);
    for ((path, _), frame) in FILE_PAIRS.iter().zip(cleared) {
        assert_publication_uri(session, path, frame)?;
        assert_eq!(
            frame.pointer("/params/diagnostics"),
            Some(&json!(Vec::<Value>::new()))
        );
    }
    Ok(())
}

fn workspace_initialization() -> Value {
    let capabilities = ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            diagnostic: Some(DiagnosticWorkspaceClientCapabilities {
                refresh_support: Some(true),
            }),
            ..WorkspaceClientCapabilities::default()
        }),
        ..ClientCapabilities::default()
    };
    json!({"capabilities": capabilities, "initializationOptions": {
        "deslop": {"diagnostics": {"enabled": true, "scope": WORKSPACE}}
    }})
}

// A real refresh request follows the complete publication batch; acknowledge it as a client.
fn publication_batch(session: &mut FixtureSession) -> Result<Vec<Value>> {
    let mut publications = Vec::new();
    loop {
        let frame = read_frame(&mut session.stdout)?;
        if frame.get("method") == Some(&json!(WORKSPACE_REFRESH)) {
            let id = frame
                .get("id")
                .ok_or_else(|| anyhow::anyhow!(MISSING_REFRESH_ID))?;
            let reply = json!({"jsonrpc": "2.0", "id": id, "result": null});
            write_frame(&mut session.stdin, &serde_json::to_string(&reply)?)?;
            return Ok(publications);
        }
        if frame.get("method") == Some(&json!(PUBLISH)) {
            publications.push(frame);
        }
    }
}
