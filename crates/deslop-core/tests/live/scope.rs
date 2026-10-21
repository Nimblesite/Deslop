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
