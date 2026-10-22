//! [LIVE-WORKSPACE-SCOPE] Editor scratch files cannot join workspace reports.

#![cfg(feature = "live")]

use std::fs;

use anyhow::Result;
use deslop_core::live::LiveError;

use crate::common::{copy_fixture, live_session};

const FIXTURE: &str = "csharp-small";
const INSIDE_FILE: &str = "Alpha.cs";
const SCRATCH_FILE: &str = "scratch.cs";
const WORKSPACE_FILE_COUNT: usize = 2;
const WORKSPACE_WITH_UNSAVED_FILE: usize = 3;
const REMOVED_DIRECTORY: &str = "removed";

/// [LIVE-WORKSPACE-SCOPE] Closing a deleted directory's buffer must discard its source.
#[test]
fn closing_a_buffer_after_its_parent_directory_is_removed_restores_the_baseline() -> Result<()> {
    let workspace = copy_fixture(FIXTURE)?;
    let mut session = live_session(workspace.path())?;
    let before = session.report();
    let text = fs::read_to_string(workspace.path().join(INSIDE_FILE))?;
    let directory = workspace.path().join(REMOVED_DIRECTORY);
    fs::create_dir(&directory)?;
    let file = directory.join(SCRATCH_FILE);
    fs::write(&file, &text)?;
    let _opened = session.open_buffer(&file, text)?;
    assert_eq!(session.report().files_analysed, WORKSPACE_WITH_UNSAVED_FILE);
    fs::remove_file(&file)?;
    fs::remove_dir(&directory)?;
    let _closed = session.close_buffer(&file)?;
    assert_eq!(session.report().files_analysed, before.files_analysed);
    assert_eq!(session.report().clusters, before.clusters);
    assert_eq!(
        serde_json::to_value(&session.report().metrics)?,
        serde_json::to_value(&before.metrics)?
    );
    Ok(())
}

/// [LIVE-WORKSPACE-SCOPE] Opening external source must preserve the entire rendered report.
#[test]
fn external_editor_buffer_is_rejected_without_changing_workspace_metrics_or_clusters() -> Result<()>
{
    let workspace = copy_fixture(FIXTURE)?;
    let outside = tempfile::tempdir()?;
    let mut session = live_session(workspace.path())?;
    let before = session.report();
    assert_eq!(before.files_analysed, WORKSPACE_FILE_COUNT);
    assert!(
        !before.clusters.is_empty(),
        "the fixture must retain its real clone"
    );
    let text = fs::read_to_string(workspace.path().join(INSIDE_FILE))?;
    let scratch = outside.path().join(SCRATCH_FILE);
    fs::write(&scratch, &text)?;
    let result = session.open_buffer(&scratch, text);
    assert!(matches!(
        result,
        Err(LiveError::PathOutsideWorkspace { .. })
    ));
    assert_eq!(session.report().files_analysed, before.files_analysed);
    assert_eq!(session.report().clusters, before.clusters);
    assert_eq!(
        serde_json::to_value(&session.report().metrics)?,
        serde_json::to_value(&before.metrics)?
    );
    Ok(())
}

/// [LIVE-WORKSPACE-SCOPE] A new editor document under the root is valid before save.
#[test]
fn unsaved_workspace_source_survives_path_validation_and_closes_back_to_the_baseline() -> Result<()>
{
    let workspace = copy_fixture(FIXTURE)?;
    let mut session = live_session(workspace.path())?;
    let before = session.report();
    let text = fs::read_to_string(workspace.path().join(INSIDE_FILE))?;
    let unsaved = workspace.path().join(SCRATCH_FILE);
    let _opened = session.open_buffer(&unsaved, text)?;
    let opened = session.report();
    assert_eq!(opened.files_analysed, WORKSPACE_WITH_UNSAVED_FILE);
    assert!(opened
        .clusters
        .iter()
        .flat_map(|cluster| &cluster.occurrences)
        .any(|occurrence| occurrence.path.ends_with(SCRATCH_FILE)));
    let _closed = session.close_buffer(&unsaved)?;
    assert_eq!(session.report().files_analysed, before.files_analysed);
    assert_eq!(session.report().clusters, before.clusters);
    assert_eq!(
        serde_json::to_value(&session.report().metrics)?,
        serde_json::to_value(&before.metrics)?
    );
    Ok(())
}
