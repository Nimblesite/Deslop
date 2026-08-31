//! [FUSED-PAIR-SIGNALS] E2E proof that transitive closure never promotes
//! pair evidence into a cluster. Pair evidence belongs to an explicit
//! two-endpoint comparison; cluster reports carry membership and mass only.

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// The byte-identical copy seeded next to `ledger_a.ts`.
const COPY_STEM: &str = "ledger_a_copy.ts";

/// The copy pair's two files.
const PAIR_FILES: [&str; 2] = ["ledger_a.ts", COPY_STEM];

/// The first cluster whose occurrence set covers every file in `files`.
fn cluster_covering<'a>(report: &'a Value, files: &[&str]) -> Option<&'a Value> {
    clusters(report).iter().find(|cluster| {
        files.iter().all(|file| {
            occurrence_paths(cluster)
                .iter()
                .any(|path| path.ends_with(file))
        })
    })
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

    // The byte-identical copy pair must publish as its own cluster — the
    // strongest wire fact the fixture stages ([PIPELINE-CLUSTER-CLOSURE]).
    let pair = clusters(&report)
        .iter()
        .find(|cluster| {
            occurrences(cluster).len() == 2
                && PAIR_FILES.iter().all(|file| {
                    occurrence_paths(cluster)
                        .iter()
                        .any(|path| path.ends_with(file))
                })
        })
        .ok_or_else(|| anyhow::anyhow!("byte-identical copy pair missing: {report:#}"))?;

    // The mixed band must also publish: the copy sweeps lookalikes into a
    // larger component, and the report must show that membership with the
    // mass-only surface ([FUSED-PAIR-SIGNALS]).
    let mixed = cluster_covering(&report, &["ledger_a.ts", COPY_STEM, "ledger_d.ts"])
        .filter(|cluster| !std::ptr::eq(*cluster, pair))
        .ok_or_else(|| anyhow::anyhow!("mixed band cluster missing: {report:#}"))?;
    assert!(
        occurrences(mixed).len() > occurrences(pair).len(),
        "the mixed cluster must carry more occurrences than the pair: {report:#}"
    );

    assert_mass_only(mixed)?;
    assert_mass_only(pair)?;

    let rank_mixed = field(mixed, "rank").as_u64().unwrap_or(u64::MAX);
    let rank_pair = field(pair, "rank").as_u64().unwrap_or(u64::MAX);
    assert!(
        cluster_mass(mixed)? > cluster_mass(pair)?,
        "more occurrences must carry more duplicated mass than two: {report:#}"
    );
    assert!(
        rank_mixed < rank_pair,
        "the larger mixed cluster outranks the pair, got {rank_mixed} vs {rank_pair}"
    );
    Ok(())
}

#[test]
fn no_closure_serializes_pair_evidence() -> Result<()> {
    let report = run_pair_mean_report()?;
    for cluster in clusters(&report) {
        assert_mass_only(cluster)?;
    }
    Ok(())
}
