//! E2E pin for gh #458 (C3 / AC6) — [FUSED-CLUSTER-SIGNALS] and
//! [FUSED-CONTENT-GATE]: a proven copy family must keep its act-now
//! evidence when an unrelated shape-identical stranger joins the
//! corpus — and the stranger must not be laundered into the family's
//! act-now cluster or double-report the family's mass.
//!
//! The `verbatim-plus-stranger` fixture holds four byte-identical files
//! plus a shape-identical stranger with different identifiers and
//! different literal constants (its content agreement against a copy is
//! 0.0436 — a false duplicate by any measure: the formulas it computes
//! differ at every literal position). Baker (1995) defines duplication
//! per pair: the copies p-match at 1.0, the stranger does not. The
//! family's evidence is the four copies' own 1.0/1.0 triple — the
//! stranger's presence must not demote it (AC6) and must not certify
//! the stranger as a duplicate (majority cannot certify occurrence
//! truth — a false-positive member must split out, not ride the
//! family's anchor to an act-now bucket).

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// The one cluster the report may certify for the copies: the whole-
/// file family view that the enclosure arm of the subsumption election
/// keeps ([PIPELINE-CLUSTER-SUBSUME]). Proven by control: running the
/// same fixture WITHOUT the stranger renders exactly this cluster, so
/// the stranger's presence must change nothing about the family's
/// certification.
const FAMILY_CLUSTER_ID: &str = "b031611e89d9c258";
/// The four verbatim copies.
const COPY_FILES: [&str; 4] = ["copy_0.ts", "copy_1.ts", "copy_2.ts", "copy_3.ts"];
/// The shape-identical stranger with unrelated content (agreement 0.0436).
const STRANGER_FILE: &str = "stranger.ts";
/// Act-now bucket labels a false-positive member must never reach.
const ACT_NOW_BUCKETS: [&str; 2] = ["identical", "nearly_identical"];

/// Runs the fixture with embeddings off.
fn run_family_report() -> Result<Value> {
    run_report_args(
        &fixture("verbatim-plus-stranger"),
        &["--min-nodes", "15", "--embeddings", "off"],
    )
}

/// [FUSED-CLUSTER-SIGNALS] gh #458 C3 — the stranger cannot demote the
/// proven family and cannot be laundered into it:
/// 1. the copies are reported in exactly ONE cluster (no
///    double-reporting of duplicated mass),
/// 2. that cluster is the four copies alone, identical, agreement 1.0,
///    rendering the byte-identical pair's own 1.0/1.0 triple,
/// 3. no cluster certifies the stranger as act-now — its content
///    agreement (0.0436) is below the support floor, so it must split
///    out or stay hidden.
#[test]
fn a_verbatim_family_survives_an_unrelated_stranger() -> Result<()> {
    let report = run_family_report()?;

    // 1. The copies appear in exactly one cluster: the family's mass is
    //    counted once, never twice (a duplicate of a duplicate would
    //    inflate the ranking weight).
    let family_clusters: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| {
            occurrences(cluster)
                .iter()
                .any(|occurrence| is_copy(occurrence))
        })
        .collect();
    assert_eq!(
        family_clusters.len(),
        1,
        "the copies must be reported in exactly one cluster, got {} — \
         overlapping clusters double-count the duplicated mass: {report:#}",
        family_clusters.len()
    );

    // 2. The family cluster: the four copies alone, identical code, the
    //    byte-identical pair's own 1.0/1.0 evidence, named source.
    let family = family_clusters[0];
    assert_eq!(
        cluster_id(family),
        FAMILY_CLUSTER_ID,
        "the family cluster keeps its stable id"
    );
    assert_eq!(occurrences(family).len(), 4, "the family holds its four copies");
    assert_eq!(
        cluster_file_set(family),
        COPY_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "no stranger may ride inside the family's cluster"
    );
    assert_eq!(
        cluster_bucket(family),
        "identical",
        "the copies alone are identical code and keep their act-now bucket"
    );
    assert_eq!(
        signal(family, "structural"),
        1.0,
        "the family renders the byte-identical pair's own structural evidence"
    );
    assert_eq!(
        signal(family, "token_jaccard"),
        1.0,
        "the family renders the byte-identical pair's own token evidence"
    );
    let source = field(family, "signal_source");
    let left_index = source
        .get("left")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("signal_source.left must exist"))? as usize;
    let right_index = source
        .get("right")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("signal_source.right must exist"))? as usize;
    let left = occurrence_path(occurrences(family).get(left_index).ok_or_else(|| {
        anyhow::anyhow!("signal_source.left {left_index} out of range")
    })?)?;
    let right = occurrence_path(occurrences(family).get(right_index).ok_or_else(|| {
        anyhow::anyhow!("signal_source.right {right_index} out of range")
    })?)?;
    assert!(
        is_copy_named(left) && is_copy_named(right),
        "the evidence shown must be the copies' own pair, got {left:?}/{right:?}"
    );

    // 3. The stranger is never certified act-now: wherever it appears,
    //    its cluster's bucket is below the act-now band. Majority vote
    //    of four verbatim copies cannot launder a 0.0436-agreement
    //    occurrence into "duplicate" (FUSION-CONTENT-GATE).
    for cluster in clusters(&report) {
        let has_stranger = occurrences(cluster)
            .iter()
            .any(|occurrence| is_stranger(occurrence));
        if has_stranger {
            let bucket = cluster_bucket(cluster);
            assert!(
                !ACT_NOW_BUCKETS.contains(&bucket),
                "a cluster certifying the unrelated stranger as {bucket} is a \
                 false positive: {cluster:#}"
            );
        }
    }

    Ok(())
}

/// Whether an occurrence's file is one of the verbatim copies.
fn is_copy(occurrence: &Value) -> bool {
    occurrence_path(occurrence)
        .map(|path| {
            COPY_FILES.contains(&path.rsplit('/').next().unwrap_or_default())
        })
        .unwrap_or(false)
}

/// Whether an occurrence's file is the stranger.
fn is_stranger(occurrence: &Value) -> bool {
    occurrence_path(occurrence)
        .map(|path| path.rsplit('/').next().unwrap_or_default() == STRANGER_FILE)
        .unwrap_or(false)
}

/// Whether a rendered path names one of the verbatim copies.
fn is_copy_named(path: &str) -> bool {
    COPY_FILES.contains(&path.rsplit('/').next().unwrap_or_default())
}
