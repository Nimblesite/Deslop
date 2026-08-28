//! E2E pin for gh #458 — [RANK-MASS-SUM]: the ranking weight is the sum
//! of duplicated mass, never confidence-discounted, so a five-member
//! medium-evidence cluster outranks a two-member byte-identical pair
//! when its total mass is larger — and a weak member cannot move the
//! bucket or the rank through dilution.
//!
//! The `rank-mass` fixture holds two unrelated families: five
//! near-miss files (`mid_0.ts`..`mid_4.ts`, 948 canonical nodes each,
//! content agreement ~0.86) and a byte-identical pair (`big_a.ts` /
//! `big_b.ts`, 3513 canonical nodes each). The five-member cluster
//! carries 948 × 4 = 3792 duplicated nodes; the pair carries 3513. Mass
//! says the five-member cluster ranks worst-first; the old formula
//! multiplied mass by the fused confidence (0.857), erasing 543 mass
//! points and ranking the pair above it (gh #458, "erases duplicated
//! line mass at ranking"). Juergens & Hummel (ICSE 2009) and
//! Islam/Mondal/Roy (SANER 2019) define clone harm as copies × extent —
//! the mass to fix — which is what the weight sums; confidence already
//! did its job at admission (Baker 1995: a pair either p-matches or it
//! does not), so it must not re-discount the mass at ranking.

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// Finds a rendered cluster by its stable id.
fn cluster_by_id<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_id(cluster) == id)
}

/// The five-member near-miss cluster: 5 × 948 = 3792 duplicated nodes.
const MID_CLUSTER_ID: &str = "02bb5e80ee7e2d96";
/// The byte-identical pair: 1 × 3513 = 3513 duplicated nodes.
const BIG_CLUSTER_ID: &str = "2a6f2840074b7094";
/// Every file the five-member cluster reports.
const MID_FILES: [&str; 5] = [
    "mid_0.ts",
    "mid_1.ts",
    "mid_2.ts",
    "mid_3.ts",
    "mid_4.ts",
];
/// The two files of the byte-identical pair.
const BIG_FILES: [&str; 2] = ["big_a.ts", "big_b.ts"];

/// Runs the `rank-mass` fixture with embeddings off, like the issue.
fn run_rank_mass_report() -> Result<Value> {
    run_report_args(
        &fixture("rank-mass"),
        &["--min-nodes", "15", "--embeddings", "off"],
    )
}

/// [RANK-MASS-SUM] gh #458 — mass outranks confidence: the five-member
/// near-miss cluster (3792 duplicated nodes) must rank above the
/// byte-identical pair (3513) even though the pair's evidence is
/// perfect. The old weight formula multiplied mass by the fused
/// confidence, so the pair ranked 1 and the five-member cluster 2 —
/// fourteen per cent more mass to fix, pushed below a smaller clone.
#[test]
fn mass_outranks_confidence_when_mass_is_larger() -> Result<()> {
    let report = run_rank_mass_report()?;

    let mid = cluster_by_id(&report, MID_CLUSTER_ID).ok_or_else(|| {
        anyhow::anyhow!(
            "the five-member cluster {} must exist: {report:#}",
            MID_CLUSTER_ID
        )
    })?;
    let big = cluster_by_id(&report, BIG_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("the pair cluster {BIG_CLUSTER_ID} must exist"))?;

    // Both clusters, exact membership, exact buckets.
    assert_eq!(
        occurrences(mid).len(),
        5,
        "the mid cluster holds five occurrences"
    );
    assert_eq!(
        cluster_file_set(mid),
        MID_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "the five near-miss files must all be reported"
    );
    assert_eq!(
        cluster_bucket(mid),
        "nearly_identical",
        "the five-member cluster keeps its act-now bucket"
    );
    assert_eq!(occurrences(big).len(), 2, "the pair cluster holds one pair");
    assert_eq!(
        cluster_file_set(big),
        BIG_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "the byte-identical pair must be reported"
    );
    assert_eq!(
        cluster_bucket(big),
        "identical",
        "the byte-identical pair is identical code"
    );

    // The mass arithmetic, asserted as numbers — no calcs outside Rust.
    let mid_nodes = field(mid, "canonical_node_count").as_u64().unwrap_or(0);
    let big_nodes = field(big, "canonical_node_count").as_u64().unwrap_or(0);
    let mid_mass = mid_nodes.saturating_mul(occurrences(mid).len().saturating_sub(1) as u64);
    let big_mass = big_nodes.saturating_mul(occurrences(big).len().saturating_sub(1) as u64);
    assert!(
        mid_mass > big_mass,
        "the fixture must give the five-member cluster more duplicated mass \
         ({mid_mass}) than the pair ({big_mass}) — otherwise the test pins \
         nothing"
    );

    // Ranking: the larger mass ranks worse-first, regardless of the
    // pair's perfect confidence.
    let rank_mid = field(mid, "rank").as_u64().unwrap_or(u64::MAX);
    let rank_big = field(big, "rank").as_u64().unwrap_or(u64::MAX);
    assert!(
        rank_mid < rank_big,
        "3792 duplicated nodes must rank above 3513: the five-member cluster \
         ranks {rank_mid}, the pair {rank_big} — the fused confidence must \
         not erase duplicated-line mass"
    );
    let weight_mid = field(mid, "weight").as_f64().unwrap_or(0.0);
    let weight_big = field(big, "weight").as_f64().unwrap_or(0.0);
    assert!(
        weight_mid > weight_big,
        "the weight must be the mass sum: {weight_mid} > {weight_big}"
    );

    Ok(())
}
