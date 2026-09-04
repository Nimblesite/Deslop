//! Black-box explicit pair-comparison contract ([FUSED-PAIR-SIGNALS]).

#![cfg(feature = "live")]

use std::{fs, sync::Arc};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    live::{LiveApi, LiveService},
    report::{
        PairClassification, PairComparison, PairComparisonParams, PairEndpoint, ReportOccurrence,
    },
};
use tokio::sync::Mutex;

use crate::common::*;

const MIN_NODES: u32 = 4;
const CONTENT_FLOOR: f64 = 0.7;
const LEFT_FILE: &str = "left.rs";
const RIGHT_FILE: &str = "right.rs";
const SOURCE: &str = "pub fn calculate(input: u32, offset: u32) -> u32 {\n    let first = input + offset;\n    let second = first + input;\n    let third = second + offset;\n    third\n}\n";
const UNRELATED_LEFT: &str =
    "pub fn alpha(order: u32) -> u32 {\n    let fee = order + 11;\n    fee * 3\n}\n";
const UNRELATED_RIGHT: &str =
    "pub fn beta(user: u32) -> u32 {\n    let score = user + 97;\n    score * 8\n}\n";

#[tokio::test]
async fn explicit_pair_comparison_owns_exact_admission_evidence() -> Result<()> {
    let workspace = tempfile::tempdir().context("pair workspace")?;
    fs::write(workspace.path().join(LEFT_FILE), SOURCE).context("write left endpoint")?;
    fs::write(workspace.path().join(RIGHT_FILE), SOURCE).context("write right endpoint")?;
    let session = live_session_at(&workspace.path(), MIN_NODES)?;
    let report = session.report();
    let cluster = report
        .clusters
        .iter()
        .find(|cluster| {
            cluster.occurrences.len() == 2
                && cluster
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.path.ends_with(LEFT_FILE))
                && cluster
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.path.ends_with(RIGHT_FILE))
        })
        .ok_or_else(|| anyhow!("missing exact two-file clone: {report:#?}"))?;
    let left = endpoint_for(cluster, LEFT_FILE)?;
    let right = endpoint_for(cluster, RIGHT_FILE)?;
    let service = LiveService::new(Arc::new(Mutex::new(session)));

    let comparison = service
        .pair_compare(&PairComparisonParams {
            left: left.clone(),
            right: right.clone(),
        })
        .await
        .context("compare exact endpoints")?;

    assert_eq!(
        comparison.left, left,
        "response must echo the selected left endpoint"
    );
    assert_eq!(
        comparison.right, right,
        "response must echo the selected right endpoint"
    );
    assert_exact_evidence(&comparison);

    let reversed = service
        .pair_compare(&PairComparisonParams {
            left: right.clone(),
            right: left.clone(),
        })
        .await
        .context("compare reversed endpoints")?;
    assert_eq!(
        reversed.left, right,
        "reversal must preserve the caller's left endpoint"
    );
    assert_eq!(
        reversed.right, left,
        "reversal must preserve the caller's right endpoint"
    );
    assert_eq!(
        reversed.evidence, comparison.evidence,
        "symmetric evidence must be order-invariant"
    );
    Ok(())
}

#[tokio::test]
async fn content_rejected_pair_never_enters_cluster_closure() -> Result<()> {
    let workspace = tempfile::tempdir().context("content-gate workspace")?;
    fs::write(workspace.path().join(LEFT_FILE), UNRELATED_LEFT).context("write unrelated left")?;
    fs::write(workspace.path().join(RIGHT_FILE), UNRELATED_RIGHT)
        .context("write unrelated right")?;
    let session = live_session_at(&workspace.path(), MIN_NODES)?;
    let report = session.report();
    let left = source_endpoint(LEFT_FILE, UNRELATED_LEFT);
    let right = source_endpoint(RIGHT_FILE, UNRELATED_RIGHT);
    let service = LiveService::new(Arc::new(Mutex::new(session)));

    let comparison = service
        .pair_compare(&PairComparisonParams {
            left: left.clone(),
            right: right.clone(),
        })
        .await
        .context("compare content-rejected pair")?;

    assert_rejected_evidence(&comparison);
    assert!(
        !report.clusters.iter().any(|cluster| {
            cluster
                .occurrences
                .iter()
                .any(|occurrence| occurrence.path.ends_with(LEFT_FILE))
                && cluster
                    .occurrences
                    .iter()
                    .any(|occurrence| occurrence.path.ends_with(RIGHT_FILE))
        }),
        "a rejected pair must never enter closure: {report:#?}"
    );
    Ok(())
}

fn assert_exact_evidence(comparison: &PairComparison) {
    let evidence = &comparison.evidence;
    assert_metric(evidence.structural, 1.0, "exact structural overlap");
    assert_metric(evidence.token_jaccard, 1.0, "Merkle token correction");
    assert_metric(evidence.embedding_cos, 0.0, "embeddings-off cosine");
    assert_metric(evidence.agreement, 1.0, "byte-identical agreement");
    assert_metric(evidence.rename_consistency, 1.0, "identity rename mapping");
    assert_metric(evidence.literal_fraction, 0.0, "empty literal population");
    assert_metric(evidence.fused_score, 1.0, "bounded maximum");
    assert!(
        evidence.content_required,
        "exact shape requires pair content"
    );
    assert!(evidence.content_ok, "exact content clears its guard");
    assert!(evidence.admitted, "the exact pair must be an admitted edge");
    assert_eq!(evidence.classification, Some(PairClassification::Identical));
    assert_eq!(
        evidence.explanation,
        "admitted: exact pair clears every admission guard"
    );
}

fn assert_rejected_evidence(comparison: &PairComparison) {
    let evidence = &comparison.evidence;
    assert_metric(evidence.structural, 1.0, "normalised shape identity");
    assert_metric(evidence.token_jaccard, 1.0, "Merkle token correction");
    assert_metric(evidence.embedding_cos, 0.0, "embeddings-off cosine");
    assert!(
        evidence.agreement < CONTENT_FLOOR,
        "raw content must expose the mismatch: {comparison:#?}"
    );
    assert!(
        evidence.rename_consistency < CONTENT_FLOOR,
        "changed literals must defeat rename evidence: {comparison:#?}"
    );
    assert!(
        evidence.content_required,
        "saturated shape requires pair content"
    );
    assert!(
        !evidence.content_ok,
        "neither content population clears the guard"
    );
    assert!(
        !evidence.admitted,
        "the content-rejected pair is not an edge"
    );
    assert_eq!(
        evidence.classification,
        Some(PairClassification::StructuralOnly)
    );
    assert_eq!(
        evidence.explanation,
        "rejected: saturated normalised evidence lacks required pair content support"
    );
}

fn assert_metric(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= f64::EPSILON,
        "{label}: expected {expected}, got {actual}"
    );
}

fn endpoint_for(cluster: &deslop_core::report::ReportCluster, file: &str) -> Result<PairEndpoint> {
    cluster
        .occurrences
        .iter()
        .find(|occurrence| occurrence.path.ends_with(file))
        .map(endpoint)
        .ok_or_else(|| anyhow!("cluster has no {file} occurrence: {cluster:#?}"))
}

fn endpoint(occurrence: &ReportOccurrence) -> PairEndpoint {
    PairEndpoint {
        path: occurrence.path.clone(),
        start_byte: occurrence.start_byte,
        end_byte: occurrence.end_byte,
    }
}

fn source_endpoint(path: &str, source: &str) -> PairEndpoint {
    PairEndpoint {
        path: path.into(),
        start_byte: 0,
        end_byte: source.trim_end().len(),
    }
}
