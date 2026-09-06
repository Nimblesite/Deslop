//! E2E regression for GH #133 [CLONE-NOISE-PY-MODULE-CONSTANT-TABLE].
//!
//! Two unrelated Python modules that are each just a run of module-level
//! `NAME = <literal>` constant assignments — a table of SQL query strings
//! in one file, a table of registry/config values in another — normalise
//! to the *same* structural subtree once identifiers, literals, and
//! comments are stripped. They reach `structural=1.00, token_jaccard=1.00`
//! and cluster as duplicates even though they share no control flow,
//! behaviour, or abstraction. A table of distinct named constants is data,
//! not extractable logic.
//!
//! The fix suppresses such a cluster ONLY when the members differ in raw
//! bytes, so a constants module copied verbatim into two files still
//! surfaces as genuine duplication. This test pins both directions: the
//! unrelated constant tables are hidden, and the verbatim copy stays
//! visible across both files.

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd
        .args(["--min-nodes", "4", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

/// Resolves the named fixture and runs the constant-table report over it.
fn fixture_report(fixture_name: &str) -> Result<(std::path::PathBuf, Value)> {
    let scan_root = fixture(fixture_name);
    let report = run_report(&scan_root)?;
    Ok((scan_root, report))
}

/// Collects every visible cluster whose occurrences contain `needle`.
fn clusters_touching(report: &Value, scan_root: &Path, needle: &str) -> Result<Vec<Vec<String>>> {
    let mut hits = Vec::new();
    for cluster in clusters(report) {
        let texts = occurrence_texts(scan_root, cluster)?;
        if texts.iter().any(|text| text.contains(needle)) {
            hits.push(texts);
        }
    }
    Ok(hits)
}

// GH #133 acceptance: a module of SQL query string constants and a module
// of registry/config value constants must NOT cluster as duplicate logic
// merely because both are runs of `NAME = <literal>` assignments.
#[test]
fn unrelated_constant_tables_do_not_cluster() -> Result<()> {
    let (scan_root, report) = fixture_report("python-issue-133-constant-table")?;
    let sql = clusters_touching(&report, &scan_root, "_PUBLIC_FUNCTIONS_SQL")?;
    let registry = clusters_touching(&report, &scan_root, "WORKSPACE_IMAGE_NAMESPACE")?;
    assert!(
        sql.is_empty() && registry.is_empty(),
        "a table of SQL-string constants and a table of registry/config \
         constants are unrelated data, not duplication, and must not \
         cluster: sql={sql:#?} registry={registry:#?}"
    );
    Ok(())
}

// GH #133 over-suppression guard: a constants module copied verbatim into
// two files (identical bytes) IS real duplication and must still surface —
// the suppression keys on raw-byte divergence, not on the constant-table
// shape alone.
#[test]
fn verbatim_copied_constants_still_surface() -> Result<()> {
    let (scan_root, report) = fixture_report("python-issue-133-genuine-copy")?;
    let copied = clusters_touching(&report, &scan_root, "DESLOP_GENUINE_COPY_MARKER")?;
    assert!(
        !copied.is_empty(),
        "a constants module copied verbatim into two files must still \
         surface as duplication: {:#?}",
        clusters(&report)
    );
    let spans_both_files = clusters(&report).iter().try_fold(false, |found, cluster| {
        let paths = occurrence_paths(cluster);
        let texts = occurrence_texts(&scan_root, cluster)?;
        let touches_marker = texts
            .iter()
            .any(|text| text.contains("DESLOP_GENUINE_COPY_MARKER"));
        let left = paths
            .iter()
            .any(|path| path.contains("feature_defaults.py"));
        let right = paths
            .iter()
            .any(|path| path.contains("feature_defaults_copy.py"));
        Ok::<bool, anyhow::Error>(found || (touches_marker && left && right))
    })?;
    assert!(
        spans_both_files,
        "the surviving clone must span both copies of the constants module: {:#?}",
        clusters(&report)
    );
    Ok(())
}

// GH #133 precision guard: a module whose entries include an interpolated
// f-string embeds expressions, so it is not an inert constant table. Two
// such modules must NOT be suppressed — the filter keys on *plain* literal
// values, and anything that can carry logic keeps clustering for review.
#[test]
fn interpolated_template_modules_still_surface() -> Result<()> {
    let (scan_root, report) = fixture_report("python-issue-133-precision")?;
    let templated = clusters_touching(&report, &scan_root, "BANNER = f\"Welcome to")?;
    assert!(
        !templated.is_empty(),
        "modules carrying interpolated f-string templates are not inert \
         constant tables and must still surface: {:#?}",
        clusters(&report)
    );
    Ok(())
}
