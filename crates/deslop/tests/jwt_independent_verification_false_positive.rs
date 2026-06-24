//! E2E regression for GH #114: tests that independently re-implement
//! HS256 signing to verify a production JWT minter are intentionally
//! duplicative. Extracting a shared helper would make the tests verify
//! the same implementation they are meant to check.
//! Tests [CLONE-NOISE-PY-JWT-HS256]

use std::{collections::BTreeSet, fs, path::Path};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let mut cmd = deslop_cmd(scan_root, &output)?;
    let _assertion = cmd
        .args(["--min-nodes", "10", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

fn jwt_verifier_clusters(report: &Value, scan_root: &Path) -> Result<Vec<String>> {
    let mut offenders = Vec::new();
    for cluster in clusters(report) {
        let mut paths = BTreeSet::new();
        let mut snippets = Vec::new();
        let mut crypto_snippets = 0usize;
        for occurrence in occurrences(cluster) {
            let path = occurrence_path(occurrence)?;
            let text = occurrence_text(scan_root, occurrence)?;
            let _inserted = paths.insert(path);
            if text.contains("hmac.new") && text.contains("hashlib.sha256") {
                crypto_snippets = crypto_snippets.saturating_add(1);
            }
            snippets.push(format!(
                "{path}: {}",
                text.lines().next().unwrap_or_default()
            ));
        }
        let has_production = paths.iter().any(|path| path.ends_with("jwt_minter.py"));
        let test_count = paths
            .iter()
            .filter(|path| path.starts_with("tests/"))
            .count();
        if has_production && test_count >= 1 && crypto_snippets >= 2 {
            offenders.push(format!(
                "bucket={:?}, paths={paths:?}, snippets={snippets:?}",
                cluster.get("bucket").and_then(Value::as_str),
            ));
        }
    }
    Ok(offenders)
}

#[test]
fn independent_hs256_test_verifiers_do_not_surface_as_duplicate_logic() -> Result<()> {
    let scan_root = fixture("python-jwt-independent-verification");
    let production = scan_root.join("src/agent_backend/agent_workspace/jwt_minter.py");
    let coverage_test = scan_root.join("tests/test_agent_workspace_coverage.py");
    let fly_test = scan_root.join("tests/test_fly_host.py");
    assert!(
        production.is_file(),
        "production JWT minter fixture must exist"
    );
    assert!(
        coverage_test.is_file(),
        "coverage verifier fixture must exist"
    );
    assert!(fly_test.is_file(), "fly-host verifier fixture must exist");
    assert!(
        fs::read_to_string(&production)?.contains("hmac.new"),
        "production fixture must implement HMAC signing"
    );
    assert!(
        fs::read_to_string(&coverage_test)?.contains("expected_hs256"),
        "coverage test fixture must independently compute expected HS256"
    );
    assert!(
        fs::read_to_string(&fly_test)?.contains("expected_hs256"),
        "fly-host test fixture must independently compute expected HS256"
    );

    let report = run_report(&scan_root)?;
    let offenders = jwt_verifier_clusters(&report, &scan_root)?;
    assert!(
        offenders.is_empty(),
        "independent JWT/HMAC test verifiers must not rank as duplicate logic: {offenders:#?}"
    );
    Ok(())
}
