//! Cross-language comparison configuration E2E coverage.
//!
//! Drives the CLI against the mixed-language fixture so the public
//! `.deslop.toml` contract is tested through the same path users run.

use std::{collections::BTreeSet, fs, path::Path, path::PathBuf};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

mod common;
use crate::common::*;

#[test]
fn default_run_does_not_report_cross_language_clusters() -> Result<()> {
    let report = run_mixed_fixture(None)?;
    assert!(
        !has_cross_language_cluster(&report),
        "default report must not compare clones across languages: {report}"
    );
    assert!(
        !clusters(&report)?.is_empty(),
        "same-language duplication should still be reported"
    );
    Ok(())
}

#[test]
fn config_can_enable_cross_language_clusters() -> Result<()> {
    let report = run_mixed_fixture(Some("[analysis]\nallow_cross_language_comparison = true\n"))?;
    assert!(
        has_cross_language_cluster(&report),
        "explicit opt-in should keep cross-language clusters available: {report}"
    );
    Ok(())
}

fn run_mixed_fixture(config: Option<&str>) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = Command::cargo_bin("deslop")?;
    let _args = cmd
        .arg(fixture("mixed-small"))
        .arg("--min-nodes")
        .arg("10")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output);
    if let Some(contents) = config {
        let path = tmp.path().join("deslop.toml");
        fs::write(&path, contents)?;
        let _config_args = cmd.arg("--config").arg(path);
    }
    let _assertion = cmd.assert().success();
    let report_path = with_ext(&output, "json");
    let json = fs::read_to_string(report_path)?;
    serde_json::from_str(&json).map_err(Into::into)
}

fn has_cross_language_cluster(report: &Value) -> bool {
    clusters(report).is_ok_and(|items| {
        items.iter().any(|cluster| {
            occurrence_extensions(cluster).is_ok_and(|extensions| extensions.len() > 1)
        })
    })
}

fn clusters(report: &Value) -> Result<&Vec<Value>> {
    report["clusters"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("report.clusters must be an array"))
}

fn occurrence_extensions(cluster: &Value) -> Result<BTreeSet<String>> {
    let occurrences = cluster["occurrences"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("cluster.occurrences must be an array"))?;
    Ok(occurrences
        .iter()
        .filter_map(|occurrence| occurrence["path"].as_str())
        .filter_map(extension)
        .map(ToOwned::to_owned)
        .collect())
}

fn extension(path: &str) -> Option<&str> {
    Path::new(path).extension()?.to_str()
}

fn with_ext(base: &Path, ext: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".");
    name.push(ext);
    path.set_file_name(name);
    path
}
