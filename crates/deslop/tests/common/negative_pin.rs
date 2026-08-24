//! The shared contract every noise-family pin is judged by
//! ([CLONE-NOISE], [RANK-STRUCTURAL-ONLY]).
//!
//! A suppression test that asserts only "the family produced no
//! cluster" is an instrument that cannot go red for the reason that
//! matters: a detector that had gone blind — a filter widened until it
//! eats real code, a parser that stopped producing candidates, a
//! renderer that dropped every cluster — passes it perfectly. Every one
//! of the shipped false positives this repository tracks was reported
//! *alongside* real duplication in the same repository, so the honest
//! contract is a two-sided one:
//!
//! > the family stays hidden **while a real clone in the same run stays
//! > visible**.
//!
//! Each noise fixture therefore stages its own false-negative control —
//! a byte-identical function pair, unrelated to the family — and every
//! pin asserts both halves against this one helper, so no fixture can
//! drift into asserting the weaker half alone.

use serde_json::Value;

use super::{
    cluster_bucket, cluster_id, cluster_size, cluster_spanning, clusters, clusters_hidden,
    expect_cluster_spanning, occurrence_files, signal,
    signals::{ACT_NOW_BUCKETS, ACT_NOW_FUSED, HONEST_SHAPE_ONLY_BUCKETS},
    Result,
};

/// Asserts the two-sided contract: no visible cluster spans
/// `family_files`, the suppression is *counted* rather than merely
/// absent, and exactly one visible act-now cluster spans
/// `control_files` with both of its occurrences shown.
pub(crate) fn assert_family_hidden_with_control(
    report: &Value,
    label: &str,
    family_files: &[&str],
    control_files: &[&str],
) -> Result<()> {
    assert_family_hidden(report, label, family_files);
    assert_control_visible(report, label, control_files)
}

/// The suppression half: nothing spanning the family reaches the report,
/// and `clusters_hidden` records that a decision was taken. A family
/// that merely failed to cluster proves nothing about the filter.
fn assert_family_hidden(report: &Value, label: &str, family_files: &[&str]) {
    assert!(
        cluster_spanning(report, family_files).is_none(),
        "{label}: the family is scaffolding, not duplication — no cluster may span \
         {family_files:?}: {published:#?}",
        published = published_summary(report),
    );
    assert!(
        clusters_hidden(report) >= 1,
        "{label}: the family must be actively hidden and counted, not merely absent \
         — an uncounted disappearance is indistinguishable from a detector that \
         stopped looking: {report:#}"
    );
}

/// The false-negative control half: the authored clone in the same run
/// survives whatever hid the family, in an act-now bucket, with both
/// occurrences shown.
fn assert_control_visible(report: &Value, label: &str, control_files: &[&str]) -> Result<()> {
    let control = expect_cluster_spanning(report, control_files)?;
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(control)),
        "{label}: the control clone is copied byte for byte; a suppression wide \
         enough to demote it has eaten real duplication — bucket={bucket} \
         fused={fused:.4}",
        bucket = cluster_bucket(control),
        fused = signal(control, "fused"),
    );
    assert_eq!(
        cluster_size(control),
        2,
        "{label}: both copies of the control clone must be shown — hiding one \
         occurrence is a false negative the cluster count cannot see"
    );
    assert_eq!(
        occurrence_files(control).len(),
        control_files.len(),
        "{label}: the control clone spans exactly its own files: {files:?}",
        files = occurrence_files(control),
    );
    Ok(())
}

/// Every visible cluster as `(id, bucket, files)` — the smallest dump
/// that lets a failure be diagnosed without re-running the scan.
fn published_summary(report: &Value) -> Vec<(&str, &str, Vec<String>)> {
    clusters(report)
        .iter()
        .map(|cluster| {
            (
                cluster_id(cluster),
                cluster_bucket(cluster),
                occurrence_files(cluster),
            )
        })
        .collect()
}

/// The contract for a family the detector **demotes but does not yet
/// hide**. Same two sides — the control must still be visible — but the
/// family half is stated as what is true today rather than what should
/// be: every cluster over the family is labelled shape-only, none of
/// them reaches the act-now line, and there are exactly
/// `expected_demoted` of them.
///
/// The exact count is what makes this a pin rather than a shrug: a
/// *new* family cluster fails it, and so does an existing one climbing
/// into an act-now bucket. The residual itself is recorded against its
/// issue in each caller's module doc, so a reader can tell a known,
/// bounded gap from a passing test.
pub(crate) fn assert_family_demoted_with_control(
    report: &Value,
    label: &str,
    family_files: &[&str],
    control_files: &[&str],
    expected_demoted: usize,
) -> Result<()> {
    let over_family: Vec<&Value> = clusters(report)
        .iter()
        .filter(|cluster| {
            occurrence_files(cluster)
                .iter()
                .any(|file| family_files.contains(&file.as_str()))
        })
        .collect();
    assert_eq!(
        over_family.len(),
        expected_demoted,
        "{label}: the family's known residual is {expected_demoted} demoted \
         cluster(s); anything else is a change that must be looked at, not \
         absorbed: {published:#?}",
        published = published_summary(report),
    );
    for cluster in over_family {
        assert!(
            HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
            "{label}: a family the tool cannot act on must at least be labelled \
             shape-only — {id} is {bucket}",
            id = cluster_id(cluster),
            bucket = cluster_bucket(cluster),
        );
        assert!(
            signal(cluster, "fused") < ACT_NOW_FUSED,
            "{label}: {id} reached the act-now line at {fused:.4}; an agent told \
             not to write this code would be told wrong",
            id = cluster_id(cluster),
            fused = signal(cluster, "fused"),
        );
    }
    assert_control_visible(report, label, control_files)
}
