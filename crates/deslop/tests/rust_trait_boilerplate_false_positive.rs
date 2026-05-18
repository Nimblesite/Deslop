//! E2E regression for GH #75: the three Rust language plug-ins all
//! implement the same `LanguageParser` trait surface. That adapter
//! boilerplate is required by the trait contract and must not rank as
//! duplicate business logic.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use assert_cmd::Command;
use serde_json::Value;

fn deslop_core_lang_dir() -> Result<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .context("deslop crate must live under crates/")?
        .join("deslop-core")
        .join("src")
        .join("lang"))
}

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("30")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn occurrences(cluster: &Value) -> &[Value] {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn cluster_paths(cluster: &Value) -> BTreeSet<&str> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(Value::as_str))
        .collect()
}

fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence_path(occurrence)?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("reported occurrence range is invalid for {path}"))
}

fn occurrence_path(occurrence: &Value) -> Result<&str> {
    occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))
}

fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("reported occurrence is missing {field}"))
}

fn language_parser_adapter_clusters(report: &Value, scan_root: &Path) -> Result<Vec<String>> {
    let target_files = BTreeSet::from(["csharp.rs", "python.rs", "rust_lang.rs"]);
    let mut offenders = Vec::new();
    for cluster in clusters(report) {
        let paths = cluster_paths(cluster);
        if !target_files.is_subset(&paths) {
            continue;
        }
        let mut snippets = Vec::new();
        for occurrence in occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            if is_language_parser_adapter_text(&text) {
                snippets.push(format!(
                    "{}: {}",
                    occurrence_path(occurrence)?,
                    text.lines().next().unwrap_or_default().trim(),
                ));
            }
        }
        if snippets.len() >= target_files.len() {
            offenders.push(format!(
                "bucket={:?}, paths={paths:?}, snippets={snippets:?}",
                cluster.get("bucket").and_then(Value::as_str),
            ));
        }
    }
    Ok(offenders)
}

fn is_language_parser_adapter_text(text: &str) -> bool {
    text.contains("impl LanguageParser for")
        || (text.contains("fn id(&self)")
            && text.contains("fn file_extensions(&self)")
            && text.contains("parse_and_normalize"))
}

#[test]
fn rust_language_parser_trait_impl_boilerplate_does_not_surface() -> Result<()> {
    let scan_root = deslop_core_lang_dir()?;
    assert!(
        scan_root.join("rust_lang.rs").is_file(),
        "test must scan deslop-core/src/lang"
    );
    let report = run_report(&scan_root)?;
    let offenders = language_parser_adapter_clusters(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "LanguageParser trait adapter impls must not surface as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
