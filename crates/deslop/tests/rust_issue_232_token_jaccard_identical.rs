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

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::Value;

use crate::common::scan_dir::run_report_min_nodes;
use crate::common::{
    occurrence_texts,
    signals::{assert_no_pair_surface_on_cluster, assert_structural_only_contract},
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn byte_identical_clones_are_byte_proven_from_the_fixture() -> Result<()> {
    let scan_root = fixture("rust-issue-232-token-jaccard");
    let report = run_report_min_nodes(&scan_root, "30")?;
    let clusters = report
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("report missing clusters: {report:#}"))?;
    assert!(
        !clusters.is_empty(),
        "fixture must produce the byte-identical clone so the byte-proven \
         fact is exercised: {report:#}"
    );
    // #232 acceptance on the mass-only wire: the sibling-window clone is
    // reported (admission) and the byte-identical `render_header` +
    // `render_footer` block is *inside* both reported occurrences — the
    // byte truth that the window token-Jaccard used to proxy. The
    // published view may be the wider near-miss that legitimately
    // subsumes it ([PIPELINE-CLUSTER-SUBSUME]), so the pin is the block
    // appearing verbatim in both reported slices, not a byte-proven
    // *cluster* of its own.
    assert!(
        clusters.len() == 1,
        "the fixture must report exactly one cluster (the whole-file \
         near-miss subsumes the nested render window): {report:#}"
    );
    for cluster in clusters {
        assert_structural_only_contract(cluster, "rust-issue-232");
        assert_no_pair_surface_on_cluster(cluster, "rust-issue-232");
        let texts = occurrence_texts(&scan_root, cluster)?;
        let both_carry_block = texts.len() >= 2
            && texts
                .iter()
                .all(|text| text.contains("render_header") && text.contains("render_footer"));
        assert!(
            both_carry_block,
            "both reported occurrences must carry the byte-identical \
             render_header + render_footer block: {texts:#?}"
        );
    }
    Ok(())
}
