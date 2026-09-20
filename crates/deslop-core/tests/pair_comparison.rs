//! Black-box explicit pair-comparison contract ([FUSED-PAIR-SIGNALS]).

#![cfg(feature = "live")]

use std::{fs, sync::Arc};

use anyhow::{anyhow, Context, Result};
use deslop_core::{
    buckets::ClusterKind,
    live::{LiveApi, LiveService},
    report::{
        PairClassification, PairComparison, PairComparisonParams, PairEndpoint, PairTextIdentity,
        Report, ReportCluster, ReportOccurrence,
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
/// `SOURCE` re-indented by one extra level on every line after the first:
/// the same code, laid out differently.
const INDENTED_SOURCE: &str = "pub fn calculate(input: u32, offset: u32) -> u32 {\n        let first = input + offset;\n        let second = first + input;\n        let third = second + offset;\n        third\n    }\n";

#[tokio::test]
async fn explicit_pair_comparison_owns_exact_admission_evidence() -> Result<()> {
    let fixture = PairFixture::new(SOURCE, SOURCE)?;
    let cluster = fixture.cluster_of_kind(
        ClusterKind::Identical,
        "two byte-identical copies fold to the identical kind",
    )?;
    let left = endpoint_for(cluster, LEFT_FILE)?;
    let right = endpoint_for(cluster, RIGHT_FILE)?;

    let comparison = fixture
        .compare(&left, &right, "compare exact endpoints")
        .await?;
    assert_eq!(
        comparison.left, left,
        "response must echo the selected left endpoint"
    );
    assert_eq!(
        comparison.right, right,
        "response must echo the selected right endpoint"
    );
    assert_exact_evidence(&comparison);

    let reversed = fixture
        .compare(&right, &left, "compare reversed endpoints")
        .await?;
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
async fn an_indentation_only_copy_reports_indentation_as_its_whole_difference() -> Result<()> {
    let fixture = PairFixture::new(SOURCE, INDENTED_SOURCE)?;
    let cluster = fixture.cluster_of_kind(
        ClusterKind::Identical,
        "[CLONE-BUCKETS-IDENTICAL] indentation does not change copied source content",
    )?;
    let left = endpoint_for(cluster, LEFT_FILE)?;
    let right = endpoint_for(cluster, RIGHT_FILE)?;

    let comparison = fixture
        .compare(&left, &right, "compare re-indented endpoints")
        .await?;
    let evidence = &comparison.evidence;
    assert_eq!(
        evidence.text_identity,
        PairTextIdentity::IndentationOnly,
        "the only difference is indentation: {comparison:#?}"
    );
    assert_eq!(
        evidence.classification,
        Some(PairClassification::Identical),
        "whitespace-folded source is identical: {comparison:#?}"
    );
    assert_metric(evidence.structural, 1.0, "same normalised shape");
    assert_metric(evidence.agreement, 1.0, "same authored content");
    assert!(
        evidence.admitted,
        "the re-indented copy is an admitted edge"
    );
    Ok(())
}

#[tokio::test]
async fn content_rejected_pair_never_enters_cluster_closure() -> Result<()> {
    let fixture = PairFixture::new(UNRELATED_LEFT, UNRELATED_RIGHT)?;
    let left = source_endpoint(LEFT_FILE, UNRELATED_LEFT);
    let right = source_endpoint(RIGHT_FILE, UNRELATED_RIGHT);

    let comparison = fixture
        .compare(&left, &right, "compare content-rejected pair")
        .await?;
    assert_rejected_evidence(&comparison);
    let report = &fixture.report;
    assert!(
        !report
            .clusters
            .iter()
            .filter(|cluster| cluster.kind.is_clone())
            .any(|cluster| {
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
    assert_eq!(report.metrics.clusters_total, 0);
    assert_eq!(report.metrics.duplicated_loc, 0);
    assert!(report
        .clusters
        .iter()
        .all(|finding| finding.mass == 0 && finding.rank == 0));
    Ok(())
}

/// One live session over a two-file workspace, with the report it
/// produced and the service every comparison runs through. Owns the
/// temp directory so the workspace outlives the session that reads it.
struct PairFixture {
    /// Keeps the workspace on disk for the test's duration.
    _workspace: tempfile::TempDir,
    /// The report the session produced before the service took it.
    report: Arc<Report>,
    /// The service every `pair_compare` in the test runs through.
    service: LiveService,
}

impl PairFixture {
    /// Writes `left` and `right` as the two endpoints of a fresh
    /// workspace and starts one live session over them.
    fn new(left: &str, right: &str) -> Result<Self> {
        let workspace = tempfile::tempdir().context("pair workspace")?;
        fs::write(workspace.path().join(LEFT_FILE), left).context("write left endpoint")?;
        fs::write(workspace.path().join(RIGHT_FILE), right).context("write right endpoint")?;
        let session = live_session_at(workspace.path(), MIN_NODES)?;
        let report = session.report();
        Ok(Self {
            _workspace: workspace,
            report,
            service: LiveService::new(Arc::new(Mutex::new(session))),
        })
    }

    /// The single cross-file cluster, asserted to carry `kind`.
    fn cluster_of_kind(&self, kind: ClusterKind, why: &str) -> Result<&ReportCluster> {
        let cluster = two_file_cluster(&self.report)?;
        assert_eq!(cluster.kind, kind, "{why}: {cluster:#?}");
        Ok(cluster)
    }

    /// Compares `left` against `right` through the live service.
    async fn compare(
        &self,
        left: &PairEndpoint,
        right: &PairEndpoint,
        why: &'static str,
    ) -> Result<PairComparison> {
        self.service
            .pair_compare(&PairComparisonParams {
                left: left.clone(),
                right: right.clone(),
            })
            .await
            .context(why)
    }
}

fn assert_exact_evidence(comparison: &PairComparison) {
    let evidence = &comparison.evidence;
    assert_metric(evidence.structural, 1.0, "exact structural overlap");
    assert_metric(evidence.token_jaccard, 1.0, "Merkle token correction");
    assert_metric(evidence.embedding_cos, 0.0, "embeddings-off cosine");
    assert_metric(evidence.agreement, 1.0, "byte-identical agreement");
    assert_metric(evidence.rename_consistency, 1.0, "identity rename mapping");
    assert_metric(evidence.literal_fraction, 0.0, "empty literal population");
    assert_eq!(
        evidence.text_identity,
        PairTextIdentity::ByteIdentical,
        "the same bytes on both sides"
    );
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
    assert_eq!(
        evidence.text_identity,
        PairTextIdentity::Different,
        "different names and literals are not an indentation difference"
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
    // [CLONE-BUCKETS-ROUTING] Rejected content is not necessarily negligible.
    assert!(
        evidence.agreement.max(evidence.rename_consistency)
            > deslop_core::config::RoutingTuning::default().shape_only_max_content
    );
    assert_eq!(evidence.classification, None);
    assert_eq!(
        evidence.explanation,
        "rejected: pair fails content corroboration"
    );
}

fn assert_metric(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= f64::EPSILON,
        "{label}: expected {expected}, got {actual}"
    );
}

/// The one cluster holding exactly the left and right fixture files.
fn two_file_cluster(report: &Report) -> Result<&ReportCluster> {
    report
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
        .ok_or_else(|| anyhow!("missing two-file clone: {report:#?}"))
}

fn endpoint_for(cluster: &ReportCluster, file: &str) -> Result<PairEndpoint> {
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
