//! E2E regression for GH #232.
//!
//! `token_jaccard` was reported as `0.00` for clusters in the `identical`
//! bucket whose occurrences are byte-for-byte identical, while
//! `nearly_identical` clusters correctly reported `1.00`. The signal was
//! zeroed on any synthetic **sibling-window** fingerprint: its byte range
//! matches no single AST node, so the non-language signature path
//! (`token_stream_for_fingerprint` → `locate`) failed to resolve it and
//! fell back to a byte-offset-seeded signature that differs between two
//! files even when the code is identical, driving the estimated Jaccard
//! to ~0.
//!
//! The fixture's `render_header` + `render_footer` block is byte-identical
//! across `alpha.rs` and `beta.rs`; the surrounding lead/tail functions
//! differ structurally so the reported clone is exactly that window.
//!
//! Acceptance: a byte-equivalent (`identical`-bucket) clone MUST report
//! `token_jaccard ≈ 1.0`. The Jaccard of two identical token multisets is
//! 1.0 by definition, so any `identical` cluster reporting `< 0.9` is the
//! GH #232 regression.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

use crate::common::{
    deslop_cmd,
    signals::{
        assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
    },
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_report(scan_root: &Path) -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let output = tmp.path().join("report");
    let _assertion = deslop_cmd(scan_root, &output)?
        .args(["--min-nodes", "30", "--embeddings", "off"])
        .assert()
        .success();
    let body = fs::read_to_string(output.with_extension("json"))?;
    Ok(serde_json::from_str(&body)?)
}

#[test]
fn byte_identical_clones_are_byte_proven_from_the_fixture() -> Result<()> {
    let scan_root = fixture("rust-issue-232-token-jaccard");
    let report = run_report(&scan_root)?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(
        !clusters.is_empty(),
        "fixture must produce the byte-identical clone so the byte-proven \
         fact is exercised: {report:#}"
    );
    // #232 acceptance on the mass-only wire: every reported cluster whose
    // occurrences slice to identical source bytes must be byte-proven —
    // the `token_jaccard` value that used to assert it is pair-scoped now
    // ([PIPELINE-CLUSTER-CLOSURE]).
    for cluster in &clusters {
        if has_verbatim_pair(&scan_root, cluster)? {
            assert_structural_only_contract(cluster, "rust-issue-232");
            assert_no_pair_surface_on_cluster(cluster, "rust-issue-232");
        }
    }
    let byte_proven = clusters
        .iter()
        .filter(|cluster| has_verbatim_pair(&scan_root, cluster).unwrap_or(false))
        .count();
    assert!(
        byte_proven >= 1,
        "the fixture's render_header + render_footer window is byte-identical \
         across alpha.rs and beta.rs and must be reported as a byte-proven \
         clone: {report:#}"
    );
    Ok(())
}
