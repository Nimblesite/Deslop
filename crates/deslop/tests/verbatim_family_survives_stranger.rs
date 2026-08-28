//! E2E pin for gh #458 (C3 / AC6) — [FUSION-CLUSTER-SIGNALS] and
//! [FUSION-CONTENT-GATE]: a cluster containing a proven copy family must
//! keep its act-now bucket when an unrelated weak member joins, and the
//! rendered signals of the evidence already in it must not change.
//!
//! The `verbatim-plus-stranger` fixture holds four byte-identical files
//! plus a shape-identical stranger with different identifiers and
//! literals (content agreement ≈ 0.04 — as unrelated as an admitted
//! member can be). Baker (1995) defines duplication per pair: the
//! copies p-match at 1.0, the stranger does not, and the stranger's
//! presence cannot erase the copies' proof. The content gate's anchor is
//! a member of the largest token-identical family, so the four copies
//! score 1.0 against it and only the stranger's own low agreement is
//! averaged in — the cluster cannot fall below
//! `CONTENT_SUPPORT_FLOOR` (0.7) while the family holds a strict
//! majority, whatever the stranger contributes (the mean over 3
//! verbatim + the stranger is at least 0.75).

use anyhow::Result;
use serde_json::Value;

use deslop_core::buckets::CONTENT_SUPPORT_FLOOR;

use crate::common::*;

/// The mixed cluster: four verbatim copies plus the stranger.
const MIXED_CLUSTER_ID: &str = "0c3021fd6641a9c6";
/// The family-only cluster the same run reports for the copies alone.
const FAMILY_CLUSTER_ID: &str = "1a9c15f5c7f7b5fd";
/// The four verbatim copies.
const COPY_FILES: [&str; 4] = ["copy_0.ts", "copy_1.ts", "copy_2.ts", "copy_3.ts"];
/// The shape-identical stranger with unrelated content.
const STRANGER_FILE: &str = "stranger.ts";

/// Runs the fixture with embeddings off.
fn run_family_report() -> Result<Value> {
    run_report_args(
        &fixture("verbatim-plus-stranger"),
        &["--min-nodes", "15", "--embeddings", "off"],
    )
}

/// [FUSION-CLUSTER-SIGNALS] gh #458 C3 — the stranger cannot demote the
/// proven family or rewrite its evidence: the mixed cluster stays
/// act-now, its rendered signals are the byte-identical pair's own
/// 1.0/1.0, and the family-only cluster renders the same evidence.
#[test]
fn a_verbatim_family_survives_an_unrelated_stranger() -> Result<()> {
    let report = run_family_report()?;

    let mixed = cluster_by_id(&report, MIXED_CLUSTER_ID).ok_or_else(|| {
        anyhow::anyhow!(
            "the mixed cluster {} must exist: {report:#}",
            MIXED_CLUSTER_ID
        )
    })?;
    let family = cluster_by_id(&report, FAMILY_CLUSTER_ID).ok_or_else(|| {
        anyhow::anyhow!("the family cluster {FAMILY_CLUSTER_ID} must exist")
    })?;

    // Membership: the mixed cluster holds all four copies plus the
    // stranger; the family cluster holds the copies alone.
    assert_eq!(
        occurrences(mixed).len(),
        5,
        "the mixed cluster holds the four copies plus the stranger"
    );
    let mut files: Vec<String> = COPY_FILES.iter().map(|path| (*path).to_owned()).collect();
    files.push(STRANGER_FILE.to_owned());
    assert_eq!(
        cluster_file_set(mixed),
        files.into_iter().collect(),
        "the mixed cluster reports exactly the family and the stranger"
    );
    assert_eq!(
        cluster_file_set(family),
        COPY_FILES.iter().map(|path| (*path).to_owned()).collect(),
        "the family cluster reports exactly the four copies"
    );

    // Evidence: both clusters display the byte-identical pair's own
    // 1.0/1.0 — adding the stranger changed nothing about the rendered
    // signals of the evidence already there (AC6).
    for (cluster, label) in [(&mixed, "mixed"), (&family, "family")] {
        assert_eq!(
            signal(cluster, "structural"),
            1.0,
            "the {label} cluster must display the copy pair's structural: {cluster:#}"
        );
        assert_eq!(
            signal(cluster, "token_jaccard"),
            1.0,
            "the {label} cluster must display the copy pair's token evidence: {cluster:#}"
        );
    }
    let mixed_source = signal_source_paths(mixed)?;
    assert_eq!(
        mixed_source.0,
        COPY_FILES[0],
        "the mixed cluster must name a copy as the evidence source, got {}",
        mixed_source.0
    );
    assert_eq!(
        mixed_source.1,
        COPY_FILES[1],
        "the mixed cluster must name a copy as the evidence source, got {}",
        mixed_source.1
    );

    // Admission: the stranger's near-zero agreement cannot sink the
    // cluster below the support floor — the anchor is a verbatim copy,
    // so the copies score 1.0 and only the stranger's own agreement is
    // averaged in.
    let agreement = field(mixed, "signals")
        .get("agreement")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow::anyhow!("the mixed cluster must report agreement"))?;
    assert!(
        agreement >= CONTENT_SUPPORT_FLOOR,
        "the mixed cluster's agreement {agreement} must stay above the \
         support floor {CONTENT_SUPPORT_FLOOR} — the stranger must not \
         demote a proven family"
    );
    assert_eq!(
        cluster_bucket(mixed),
        "nearly_identical",
        "the mixed cluster's agreement 0.7609 must keep it in the act-now \
         nearly-identical band — the stranger cannot demote a proven family"
    );
    assert_eq!(
        cluster_bucket(family),
        "identical",
        "the copies alone are identical code"
    );

    Ok(())
}

/// Reads the two paths named by the cluster's signal source.
fn signal_source_paths(cluster: &Value) -> Result<(String, String)> {
    let source = field(cluster, "signal_source");
    let left = source
        .get("left")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("signal_source must name the left index"))?;
    let right = source
        .get("right")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("signal_source must name the right index"))?;
    let occurrences = occurrences(cluster);
    let left_path = occurrence_path(
        occurrences
            .get(left as usize)
            .ok_or_else(|| anyhow::anyhow!("signal_source.left {left} out of range"))?,
    )?
    .split('/')
    .last()
    .ok_or_else(|| anyhow::anyhow!("path has no file name"))?
    .to_owned();
    let right_path = occurrence_path(
        occurrences
            .get(right as usize)
            .ok_or_else(|| anyhow::anyhow!("signal_source.right {right} out of range"))?,
    )?
    .split('/')
    .last()
    .ok_or_else(|| anyhow::anyhow!("path has no file name"))?
    .to_owned();
    Ok((left_path, right_path))
}

/// Finds a rendered cluster by its stable id.
fn cluster_by_id<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_id(cluster) == id)
}
