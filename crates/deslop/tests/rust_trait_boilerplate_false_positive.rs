//! E2E regression for GH #75: the three Rust language plug-ins all
//! implement the same `LanguageParser` trait surface. That adapter
//! boilerplate is required by the trait contract and must not rank as
//! duplicate business logic.
//! Tests [CLONE-NOISE-RUST-LANGPARSER]

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::common::scan_dir::run_report_min_nodes;
use crate::common::*;

fn deslop_core_lang_dir() -> Result<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .context("deslop crate must live under crates/")?
        .join("deslop-core")
        .join("src")
        .join("lang"))
}

fn cluster_paths(cluster: &Value) -> BTreeSet<&str> {
    occurrences(cluster)
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(Value::as_str))
        .collect()
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
                let first_line = text
                    .lines()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("empty reported occurrence: {text:?}"))?
                    .trim();
                snippets.push(format!("{}: {}", occurrence_path(occurrence)?, first_line));
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
    let report = run_report_min_nodes(&scan_root, "30")?;
    let offenders = language_parser_adapter_clusters(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "LanguageParser trait adapter impls must not surface as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
