//! E2E regression for GH #97 [CLONE-NOISE-PY-PARAMETRIC-INVARIANT-TESTS].
//!
//! `def test_register_<variant>()` style tests assert the same invariant
//! against each value of an enum discriminator. After identifier
//! normalisation the bodies collapse to identical Type-2 clones, but
//! each test name records a distinct spec assertion: collapsing them
//! into one cluster would silently lose coverage granularity. Bodies
//! that vary only by enum-style identifier access (`X.K8S` vs
//! `X.DOCKER`) inside `test_*` functions must be dropped.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

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

fn occurrence_text(scan_root: &Path, occurrence: &Value) -> Result<String> {
    let path = occurrence
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("reported occurrence is missing path"))?;
    let source = fs::read_to_string(scan_root.join(path))?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    source
        .get(start..end)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("reported occurrence range is invalid"))
}

fn occurrence_byte(occurrence: &Value, field: &str) -> Result<usize> {
    occurrence
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| anyhow!("reported occurrence is missing {field}"))
}

fn parametric_test_offenders(report: &Value, scan_root: &Path) -> Result<Vec<String>> {
    let mut summaries = Vec::new();
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            if text.contains("register_host(") || text.contains("register_dispatcher(") {
                summaries.push(text.replace('\n', " "));
            }
        }
    }
    Ok(summaries)
}

#[test]
fn parametric_enum_invariant_tests_do_not_cluster() -> Result<()> {
    let scan_root = fixture("python-issue-97-parametric-invariant-tests");
    let report = run_report(&scan_root)?;
    let offenders = parametric_test_offenders(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "`test_register_<variant>` bodies varying only by enum member \
         access must not surface as duplicates — each test name is its \
         own spec assertion: {offenders:#?}"
    );
    Ok(())
}
