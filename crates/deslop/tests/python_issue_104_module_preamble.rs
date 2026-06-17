//! E2E regression for GH #104 [CLONE-NOISE-PY-MODULE-PREAMBLE].
//!
//! A sibling-window fingerprint can cover a contiguous run of >=2
//! module-level definitions — a test-module preamble of several small
//! helpers/fixtures. Two unrelated test files that happen to open with
//! the same number of equally-shaped helpers then cluster as duplicates
//! (`structural=1.00`, `token_jaccard=1.00`) even though every helper's
//! body differs. Matching a "block of declarations" is not duplication.
//!
//! The fix suppresses such a cluster ONLY when no two members share
//! identical definition bodies — so a genuinely copy-pasted helper
//! module (same bodies across files) still surfaces. This test pins both
//! directions: the differently-bodied preamble is hidden, and the
//! verbatim-copied helper module stays visible.

use std::{fs, path::Path};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = Command::cargo_bin("deslop")?
        .arg(scan_root)
        .arg("--min-nodes")
        .arg("4")
        .arg("--embeddings")
        .arg("off")
        .arg("--output")
        .arg(&output)
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn occurrence_texts(scan_root: &Path, cluster: &Value) -> Result<Vec<String>> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|occurrence| occurrence_text(scan_root, occurrence))
        .collect()
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

// GH #104 acceptance: a module preamble of differently-bodied helpers
// from two unrelated test files must NOT surface as duplicate logic.
#[test]
fn differently_bodied_module_preamble_does_not_surface() -> Result<()> {
    let scan_root = fixture("python-issue-104-module-preamble");
    let report = run_report(&scan_root)?;
    let encode = clusters_touching(&report, &scan_root, "def encode_payload(")?;
    let build = clusters_touching(&report, &scan_root, "def build_url(")?;
    assert!(
        encode.is_empty() && build.is_empty(),
        "two test files that merely open with the same number of equally \
         shaped — but differently bodied — helpers must not cluster as \
         duplication: encode={encode:#?} build={build:#?}"
    );
    Ok(())
}

// GH #104 over-suppression guard: a helper module copied verbatim into
// two files (identical bodies) IS real duplication and must still
// surface — the suppression keys on body divergence, not name divergence.
#[test]
fn verbatim_copied_helper_module_still_surfaces() -> Result<()> {
    let scan_root = fixture("python-issue-104-genuine-copy");
    let report = run_report(&scan_root)?;
    let copied = clusters_touching(&report, &scan_root, "def checksum(")?;
    assert!(
        !copied.is_empty(),
        "a verbatim-copied helper module must still surface as duplication: \
         {:#?}",
        clusters(&report)
    );
    let spans_both_files = clusters(&report).iter().try_fold(false, |found, cluster| {
        let paths: Vec<String> = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|occ| {
                occ.get("path")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect();
        let texts = occurrence_texts(&scan_root, cluster)?;
        let touches_checksum = texts.iter().any(|text| text.contains("checksum"));
        let left = paths.iter().any(|path| path.contains("billing_helpers.py"));
        let right = paths
            .iter()
            .any(|path| path.contains("billing_helpers_copy.py"));
        Ok::<bool, anyhow::Error>(found || (touches_checksum && left && right))
    })?;
    assert!(
        spans_both_files,
        "the surviving clone must span both copies of the helper module: {:#?}",
        clusters(&report)
    );
    Ok(())
}

// GH #104 hardest case: a verbatim-copied preamble (service_a/service_b)
// and a same-shaped-but-differently-bodied lookalike (widget_helpers) all
// share one structural fingerprint, so the clusterer merges them into a
// single cluster. A names-only guard would suppress the whole cluster —
// destroying the genuine service_a/service_b clone. The body-equivalence
// guard must keep it: two members share identical bodies, so the cluster
// carries real duplication and stays visible.
#[test]
fn verbatim_copy_among_lookalikes_still_surfaces() -> Result<()> {
    let scan_root = fixture("python-issue-104-mixed-copy-lookalike");
    let report = run_report(&scan_root)?;
    let surfaces_genuine_copy = clusters(&report).iter().try_fold(false, |found, cluster| {
        let paths: Vec<String> = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|occ| {
                occ.get("path")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .collect();
        let texts = occurrence_texts(&scan_root, cluster)?;
        let touches_token = texts.iter().any(|text| text.contains("encode_token"));
        let a = paths.iter().any(|path| path.contains("service_a.py"));
        let b = paths.iter().any(|path| path.contains("service_b.py"));
        Ok::<bool, anyhow::Error>(found || (touches_token && a && b))
    })?;
    assert!(
        surfaces_genuine_copy,
        "a verbatim copy hiding among same-shape lookalikes must still \
         surface — the guard keys on body divergence, not names: {:#?}",
        clusters(&report)
    );
    Ok(())
}
