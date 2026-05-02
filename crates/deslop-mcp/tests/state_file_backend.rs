//! Focused tests for `StateFileBackend`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use deslop_mcp::{McpBackend, SessionBackendConfig, StateFileBackend};
use serde_json::Value;
use tempfile::TempDir;

fn fixture_root() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/csharp-mcp"
    ))
}

fn copied_fixture_root() -> Result<TempDir> {
    let temp = TempDir::new()?;
    copy_dir_all(fixture_root(), temp.path())?;
    Ok(temp)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            let _bytes = fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn backend_for(root: &Path) -> Result<StateFileBackend> {
    Ok(StateFileBackend::initialise(SessionBackendConfig {
        root: root.to_path_buf(),
        config_path: None,
    })?)
}

#[test]
fn issue_90_report_get_reloads_state_file_between_plain_calls() -> Result<()> {
    let workspace = copied_fixture_root()?;
    let backend = backend_for(workspace.path())?;
    let before = backend.report_get()?;
    let before_generation = backend.generation();
    let first_id = before
        .clusters
        .first()
        .map(|cluster| cluster.id.clone())
        .context("fixture should start with at least one cluster")?;
    assert!(
        !before.clusters.is_empty(),
        "fixture should expose duplicate clusters before mutation"
    );
    assert!(
        before_generation >= 1,
        "first report_get should load the state file and bump generation"
    );

    let state_file = workspace.path().join(".deslop-cache/live-report.json");
    remove_all_clusters(&state_file)?;

    let after = backend.report_get()?;
    let after_generation = backend.generation();
    assert!(
        after.clusters.is_empty(),
        "issue #90: plain report_get calls must not return a stale cached snapshot"
    );
    assert!(
        !after.clusters.iter().any(|cluster| cluster.id == first_id),
        "removed cluster {first_id} must not survive in the next plain MCP snapshot"
    );
    assert!(
        after_generation > before_generation,
        "reloading the changed state file should advance generation"
    );
    Ok(())
}

fn remove_all_clusters(state_file: &PathBuf) -> Result<()> {
    let mut state: Value = serde_json::from_slice(&fs::read(state_file)?)?;
    let clusters = state
        .get_mut("clusters")
        .and_then(Value::as_array_mut)
        .context("fixture state missing clusters")?;
    assert!(
        !clusters.is_empty(),
        "fixture state should have clusters before mutation"
    );
    clusters.clear();
    fs::write(state_file, serde_json::to_vec_pretty(&state)?)?;
    Ok(())
}
