//! Issue #134: a same-shape family whose members differ in *substance*
//! must NOT be labeled `nearly_identical`. The fixture is three
//! handlers sharing one 96-node skeleton whose renamed identifiers map
//! consistently but whose loop strides (`+ 1` / `+ 2` / `+ 3`) diverge
//! at the aligned literal position: [FUSED-CONTENT-GATE] measures zero
//! literal preservation, so no content evidence vouches for the family
//! and it stays a hidden structural-only match (test scaffolding,
//! generated boilerplate). The divergent literal is what separates this
//! family from a genuine Type-2 clone — an identical-logic rename with
//! its literals preserved is the *reportable* side of the same line
//! (`fused_golden_bands.rs`, `type2_rename_anchor_floor.rs`,
//! [TECH-PMATCH-BAKER]).
//!
//! Acceptance: no cluster in the rendered report carries
//! `bucket=nearly_identical` together with `structural >= 0.99`,
//! `token_jaccard < 0.05`, and `embedding_cos < 0.05`.

use anyhow::Result;
use deslop_core::report::PairClassification;

use crate::common::{
    signals::{
        assert_no_pair_surface_on_cluster, assert_pair_metric, assert_structural_only_contract,
        compare_pair, occurrence_for_file,
    },
    *,
};

const MIN_NODES: u32 = 30;
const LEFT_FILE: &str = "Alpha.cs";
const RIGHT_FILE: &str = "Beta.cs";
const EXACT_SCORE: f64 = 1.0;
const EXPECTED_MASS: u64 = 216;
const EXPECTED_DUPLICATION_PERCENT: f64 = 96.0;

#[test]
fn issue_134_structural_only_clusters_are_not_nearly_identical() -> Result<()> {
    let scan_root = fixture("csharp-issue-134-structural-only");
    let report = run_report(&scan_root, MIN_NODES)?;
    let visible_contributing = report
        .pointer("/metrics/clusters_total")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::MAX);
    assert_eq!(
        visible_contributing, 1,
        "the admitted three-member closure must contribute exactly one visible cluster: {report}"
    );
    let clusters_hidden = report
        .get("clusters_hidden")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert!(
        clusters_hidden == 0,
        "the mass-only contract has no bucket-era structural-only hiding: {report}"
    );
    let duplication_percent = report
        .pointer("/metrics/duplication_percent")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(-1.0);
    assert!(
        (duplication_percent - EXPECTED_DUPLICATION_PERCENT).abs() <= f64::EPSILON,
        "the visible closure must contribute its exact duplicated-line percentage: got {duplication_percent}"
    );
    for cluster in clusters(&report) {
        assert_no_pair_surface_on_cluster(cluster, "issue #134");
        assert_structural_only_contract(cluster, "issue #134");
    }
    let cluster = expect_cluster_spanning(&report, &[LEFT_FILE, RIGHT_FILE])?;
    assert_eq!(
        field(cluster, "mass").as_u64(),
        Some(EXPECTED_MASS),
        "cluster mass must be node count × duplicate copies: {cluster:#}"
    );
    let comparison = compare_pair(
        &scan_root,
        MIN_NODES,
        occurrence_for_file(cluster, LEFT_FILE)?,
        occurrence_for_file(cluster, RIGHT_FILE)?,
    )?;
    let evidence = &comparison.evidence;
    assert_pair_metric(
        evidence.structural,
        EXACT_SCORE,
        "shared normalized skeleton",
    );
    assert!(
        evidence.content_required,
        "saturated shape must require pair content: {comparison:#?}"
    );
    assert!(
        evidence.content_ok,
        "the pair's measured authored-content population must decide the guard: {comparison:#?}"
    );
    assert!(
        evidence.admitted,
        "the explicit pair must agree with the admitted edge contract: {comparison:#?}"
    );
    assert_eq!(
        evidence.classification,
        Some(PairClassification::NearlyIdentical)
    );
    Ok(())
}
