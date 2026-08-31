//! The temp-workspace scaffold every standalone regression test opens
//! with: a [`tempfile::TempDir`] plus an empty `src` scan root inside
//! it. Hand-rolling those three lines per test was the suite's largest
//! scaffolding duplication cluster ([CI-DESLOP] ledger, gh #397), so
//! the pairing lives here once and every test binds the returned
//! [`TempDir`] to keep the workspace alive.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use tempfile::TempDir;

use crate::common::deslop_cmd;

/// Creates the temp workspace with an empty `<dir_name>` scan root
/// inside it and returns both. Dropping the [`TempDir`] deletes the
/// workspace, so callers must bind it for as long as they read the
/// scan root or any report written beside it.
///
/// # Errors
///
/// Returns an error when the workspace or scan root cannot be created.
pub fn temp_scan_dir(dir_name: &str) -> Result<(TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join(dir_name);
    fs::create_dir_all(&scan_root)?;
    Ok((tmp, scan_root))
}

/// Runs `deslop <scan_root> --min-nodes <min_nodes> --embeddings off`
/// into a fresh temp workspace, asserts success, and returns the parsed
/// JSON report. Six per-issue suites carried byte-identical copies of
/// this helper differing only in the node floor ([CI-DESLOP] ledger,
/// gh #397); the report it returns is the caller's only contract.
///
/// The temp workspace is dropped on return — the report is fully
/// materialised in the returned [`serde_json::Value`].
///
/// # Errors
///
/// Returns an error when the command fails or the report is missing or
/// is not valid JSON.
pub fn run_report_min_nodes(scan_root: &Path, min_nodes: &str) -> Result<serde_json::Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", min_nodes, "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}
