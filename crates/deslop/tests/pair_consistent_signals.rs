//! [FUSED-PAIR-SIGNALS] E2E proof that transitive closure never promotes
//! pair evidence into a cluster. Pair evidence belongs to an explicit
//! two-endpoint comparison; cluster reports carry membership and mass only.

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// The mixed cluster containing the byte-identical pair and three lookalikes.
const MIXED_CLUSTER_ID: &str = "3015b03cf2ead794";
/// The two-member cluster holding only the byte-identical pair.
const PAIR_CLUSTER_ID: &str = "22ccedd3ee6b95f6";
/// The byte-identical copy seeded next to `ledger_a.ts`.
const COPY_STEM: &str = "ledger_a_copy.ts";

/// Every file path the mixed cluster reports after structural-family partitioning.
const MIXED_CLUSTER_FILES: [&str; 5] = [
    "ledger_a.ts",
    COPY_STEM,
    "ledger_b.ts",
    "ledger_d.ts",
    "ledger_e.ts",
];

/// The cluster carrying exactly the given id.
fn cluster_by_id<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_id(cluster) == id)
}

const FORBIDDEN_CLUSTER_FIELDS: [&str; 10] = [
    "signals",
    "signal_source",
    "structural",
    "token_jaccard",
    "embedding_cos",
    "pair_agreement",
    "pair_rename_consistency",
    "classification",
    "bucket",
    "weight",
];

fn cluster_mass(cluster: &Value) -> Result<u64> {
    field(cluster, "mass")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("cluster has no mass: {cluster:#}"))
}

fn assert_mass_only(cluster: &Value) -> Result<()> {
    let nodes = field(cluster, "canonical_node_count")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("cluster has no canonical node count: {cluster:#}"))?;
    let copies = field(cluster, "occurrence_count")
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("cluster has no occurrence count: {cluster:#}"))?
        .saturating_sub(1);
    assert_eq!(
        cluster_mass(cluster)?,
        nodes.saturating_mul(copies),
        "mass must be canonical nodes times additional visible occurrences: {cluster:#}"
    );
    for forbidden in FORBIDDEN_CLUSTER_FIELDS {
        assert!(
            cluster.get(forbidden).is_none(),
            "pair-only field {forbidden} leaked onto a cluster: {cluster:#}"
        );
    }
    Ok(())
}

/// Seeded `ts-mixed-band` plus the byte-identical copy, run with the
/// issue's own flags (`--min-nodes 15 --embeddings off`).
fn run_pair_mean_report() -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let fixtures = fixture("ts-mixed-band");
    for entry in std::fs::read_dir(&fixtures)? {
        let entry = entry?;
        let target = tmp.path().join(entry.file_name());
        let _bytes = std::fs::copy(entry.path(), target)?;
    }
    let _bytes = std::fs::copy(fixtures.join("ledger_a.ts"), tmp.path().join(COPY_STEM))?;
    run_report_args(tmp.path(), &["--min-nodes", "15", "--embeddings", "off"])
}

#[test]
fn mixed_closure_reports_membership_and_mass_only() -> Result<()> {
    let report = run_pair_mean_report()?;

    let mixed = cluster_by_id(&report, MIXED_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("mixed cluster {MIXED_CLUSTER_ID} missing: {report:#}"))?;
    assert_eq!(
        occurrences(mixed).len(),
        MIXED_CLUSTER_FILES.len(),
        "the mixed cluster must report exactly its partitioned structural family"
    );
    assert_eq!(
        occurrence_paths(mixed),
        MIXED_CLUSTER_FILES,
        "the mixed cluster's occurrence set must contain the copy pair and three lookalikes"
    );
    assert_eq!(
        field(mixed, "occurrence_count").as_u64(),
        Some(MIXED_CLUSTER_FILES.len() as u64),
        "occurrence_count must match the reported occurrences"
    );

    let pair = cluster_by_id(&report, PAIR_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("pair cluster {PAIR_CLUSTER_ID} missing: {report:#}"))?;
    assert_eq!(
        occurrences(pair).len(),
        2,
        "the pair cluster holds one pair"
    );

    assert_mass_only(mixed)?;
    assert_mass_only(pair)?;

    let rank_mixed = field(mixed, "rank").as_u64().unwrap_or(u64::MAX);
    let rank_pair = field(pair, "rank").as_u64().unwrap_or(u64::MAX);
    assert!(
        cluster_mass(mixed)? > cluster_mass(pair)?,
        "five occurrences must carry more duplicated mass than two: {report:#}"
    );
    assert!(
        rank_mixed < rank_pair,
        "five copies of the ledgers outrank two copies, got {rank_mixed} vs {rank_pair}"
    );
    Ok(())
}

#[test]
fn no_closure_serializes_pair_evidence() -> Result<()> {
    let report = run_pair_mean_report()?;
    let mixed = cluster_by_id(&report, MIXED_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("mixed cluster {MIXED_CLUSTER_ID} missing: {report:#}"))?;
    let pair = cluster_by_id(&report, PAIR_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("pair cluster {PAIR_CLUSTER_ID} missing: {report:#}"))?;
    assert_mass_only(mixed)?;
    assert_mass_only(pair)?;
    Ok(())
}
