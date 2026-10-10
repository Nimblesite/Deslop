//! [LIVE-OPEN-BUFFERS] Disk removal cannot evict an editor's authoritative source.

use std::fs;

use anyhow::Result;
use deslop_lsp::commands::REFRESH_REPORT;
use serde_json::{json, Value};
use tower_lsp::lsp_types::Url;

use super::{did_close, did_open, report_for_range, EDITED_FILE, FIXTURE, FIXTURE_FILE_COUNT};
use crate::common::{
    at,
    reports::{assert_report_shell, report_clusters, wait_for_report},
    session::FixtureSession,
    watched_file_changed, write_frame,
};

/// The standard command dispatch method for a synchronous full refresh.
const EXECUTE_COMMAND: &str = "workspace/executeCommand";
/// The canonical report query, which does not target a missing source for re-insertion.
const REPORT_GET: &str = "deslop/reportGet";
/// Closing the removed source leaves its one saved peer in the corpus.
const CLOSED_FILE_COUNT: u64 = 1;
/// Query the start of the fixture's clone without assuming a line boundary.
const OCCURRENCE_START: u64 = 0;

#[test]
fn deleted_disk_source_survives_reconciliation_and_refresh_until_buffer_closes() -> Result<()> {
    let mut session = FixtureSession::open(FIXTURE)?;
    let initial = wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        at(report, "files_analysed") == FIXTURE_FILE_COUNT
    })?;
    assert_report_shell(&initial, FIXTURE_FILE_COUNT);
    let uri = remove_open_source(&mut session)?;
    assert_editor_report(&mut session, &initial)?;
    reconcile_directory(&mut session, &initial)?;
    refresh_workspace(&mut session, &initial)?;
    close_deleted_source(&mut session, &uri)?;
    Ok(())
}

/// Remove the saved file while the editor still holds its complete clone text.
fn remove_open_source(session: &mut FixtureSession) -> Result<Url> {
    let file = session.workspace.path().join(EDITED_FILE);
    let uri = session.file_uri(EDITED_FILE)?;
    let text = fs::read_to_string(&file)?;
    write_frame(&mut session.stdin, &did_open(&uri, &text)?)?;
    fs::remove_file(&file)?;
    Ok(uri)
}

/// File membership, all ranked clusters and every metric must match the editor's source.
fn assert_editor_report(session: &mut FixtureSession, initial: &Value) -> Result<()> {
    let response = session.call(REPORT_GET, &json!({}))?;
    let current = at(&response, "result");
    assert_report_shell(current, FIXTURE_FILE_COUNT);
    assert_eq!(at(current, "clusters"), at(initial, "clusters"));
    assert_eq!(at(current, "metrics"), at(initial, "metrics"));
    Ok(())
}

/// A directory event uses the same complete source inventory as a file event.
fn reconcile_directory(session: &mut FixtureSession, initial: &Value) -> Result<()> {
    write_frame(
        &mut session.stdin,
        &watched_file_changed(session.workspace.path())?,
    )?;
    assert_editor_report(session, initial)
}

/// Rebuilding from scratch must replay a buffer whose disk file no longer exists.
fn refresh_workspace(session: &mut FixtureSession, initial: &Value) -> Result<()> {
    let refreshed = session.call(EXECUTE_COMMAND, &json!({ "command": REFRESH_REPORT }))?;
    assert!(refreshed.get("error").is_none(), "{refreshed}");
    assert_editor_report(session, initial)
}

/// Closing the last authoritative source publishes the removal and clears range queries.
fn close_deleted_source(session: &mut FixtureSession, uri: &Url) -> Result<()> {
    write_frame(&mut session.stdin, &did_close(uri)?)?;
    let closed = wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        at(report, "files_analysed") == CLOSED_FILE_COUNT
    })?;
    assert_eq!(at(&closed, "files_analysed"), CLOSED_FILE_COUNT);
    assert!(report_clusters(&closed)?.is_empty(), "{closed}");
    assert_deleted_range_empty(session)
}

/// The removed source must disappear from range queries as well as the workspace report.
fn assert_deleted_range_empty(session: &mut FixtureSession) -> Result<()> {
    let file = session.workspace.path().join(EDITED_FILE);
    assert!(report_for_range(
        &mut session.stdin,
        &mut session.stdout,
        &file,
        OCCURRENCE_START,
    )?
    .is_empty());
    Ok(())
}
