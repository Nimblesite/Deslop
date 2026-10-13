//! [LIVE-OPEN-BUFFERS] [LIVE-EMBEDDING-CONSENT] Model selection must preserve unsaved editor text.

use anyhow::Result;
use serde_json::Value;

use super::{cluster_spanning, did_open, EDITED_FILE, FIXTURE, FIXTURE_FILE_COUNT, PEER_FILE};
use crate::{
    common::{
        at, path as json_path,
        reports::{
            assert_initialize_contract, assert_report_shell, assert_report_shell_without_clusters,
            cluster_paths, has_suffix, report_clusters, wait_for_report,
        },
        session::FixtureSession,
        write_frame,
    },
    lsp_embedding_determinism::assert_model_selection,
    mock_ollama::MockOllama,
};

/// The provider served by the existing deterministic HTTP harness.
const PROVIDER_ID: &str = "ollama";
/// Opening an empty editor buffer removes the clone without changing disk.
const EMPTY_BUFFER: &str = "";
/// The fixture's ordered report contains one two-occurrence cluster.
const CLUSTER_COUNT: usize = 1;
/// Both fixture files implement the same behavior with renamed identifiers.
const CLUSTER_KIND: &str = "nearly_identical";

#[test]
fn selected_model_preserves_unsaved_buffer_report() -> Result<()> {
    let server = MockOllama::spawn()?;
    let mut session = FixtureSession::open(FIXTURE)?;
    assert_initialize_contract(&session.init);
    assert_initial_clone(&mut session)?;
    open_empty_buffer(&mut session)?;
    assert_model_selection(
        &mut session.stdin,
        &mut session.stdout,
        PROVIDER_ID,
        server.endpoint(),
    )?;
    let refreshed = wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        json_path(report, &["embedding_provenance", "provider_id"]) == PROVIDER_ID
    })?;
    assert_empty_report(&refreshed)?;
    Ok(())
}

/// Establishes that the real server sees the fixture's ranked clone before editing.
fn assert_initial_clone(session: &mut FixtureSession) -> Result<()> {
    let initial = wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        at(report, "files_analysed") == FIXTURE_FILE_COUNT
    })?;
    assert_report_shell(&initial, FIXTURE_FILE_COUNT);
    let cluster = cluster_spanning(&initial, EDITED_FILE, PEER_FILE)?;
    assert_eq!(report_clusters(&initial)?.len(), CLUSTER_COUNT);
    assert_eq!(report_clusters(&initial)?.first(), Some(&cluster));
    assert_eq!(at(&cluster, "occurrences_total"), FIXTURE_FILE_COUNT);
    assert_eq!(at(&cluster, "kind"), CLUSTER_KIND, "{cluster}");
    let paths = cluster_paths(&cluster);
    assert!(has_suffix(&paths, EDITED_FILE));
    assert!(has_suffix(&paths, PEER_FILE));
    Ok(())
}

/// Removes the clone through the real editor protocol and observes its published report.
fn open_empty_buffer(session: &mut FixtureSession) -> Result<()> {
    let uri = session.file_uri(EDITED_FILE)?;
    write_frame(&mut session.stdin, &did_open(&uri, EMPTY_BUFFER)?)?;
    let edited = wait_for_report(&mut session.stdin, &mut session.stdout, |report| {
        report_clusters(report).is_ok_and(Vec::is_empty)
    })?;
    assert_empty_report(&edited)
}

/// Every published report must retain the editor's deletion after a model refresh.
fn assert_empty_report(report: &Value) -> Result<()> {
    assert_report_shell_without_clusters(report, FIXTURE_FILE_COUNT);
    assert!(
        report_clusters(report)?.is_empty(),
        "the unsaved empty buffer must remain authoritative after model selection: {report}"
    );
    Ok(())
}
