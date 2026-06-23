//! E2E regression for [METRICS-REPO]: the duplication metric counts only
//! the clusters the *visible* report carries.
//!
//! A cluster dropped as noise / structural-only sibling boilerplate — here
//! the kwargs-constructor family in `python-issue-100-kwargs-ctor`, hidden
//! via [CLONE-NOISE-PY-KWARGS-CTOR] — must not inflate per-file or repo
//! `duplicated_loc`. Before the fix `compute_repo_metrics` ran over the raw
//! pre-filter cluster set, so a file whose only match was a hidden cluster
//! reported phantom duplication (the kwargs fixture showed 56%, the repo's
//! own test files showed 100%) even though `report-for-file` carried no
//! cluster for it at all.

mod common;

use crate::common::*;

#[test]
fn duplication_metric_excludes_hidden_clusters() -> Result<()> {
    let scan_root = fixture("python-issue-100-kwargs-ctor");
    let report = run_report(&scan_root, 4)?;

    let visible = visible_duplicated_lines(&report);

    // Per file: `duplicated_loc` must equal the lines the *visible* clusters
    // cover — never a line that exists only inside a hidden cluster.
    for metric in per_file_metrics(&report) {
        let path = field(metric, "path").as_str().unwrap_or_default();
        let expected = visible
            .iter()
            .find(|(occurrence_path, _)| path.ends_with(occurrence_path.as_str()))
            .map_or(0, |(_, lines)| line_count(lines));
        let reported = field(metric, "duplicated_loc").as_u64().unwrap_or(u64::MAX);
        assert_eq!(
            reported, expected,
            "{path}: metric reports {reported} duplicated lines but the visible \
             clusters cover {expected} — hidden clusters must not be counted"
        );
    }

    // Repo headline: `duplicated_loc` is the union size across visible clusters.
    let expected_total = visible_duplicated_loc(&report);
    let reported_total = metric_field(&report, "duplicated_loc")
        .as_u64()
        .unwrap_or(u64::MAX);
    assert_eq!(
        reported_total, expected_total,
        "repo duplicated_loc {reported_total} must equal the {expected_total} lines \
         covered by visible clusters"
    );

    // `clusters_total` counts only visible clusters with at least two live
    // (non-hidden) occurrences.
    let expected_clusters = u64::try_from(
        clusters(&report)
            .iter()
            .filter(|cluster| live_occurrences(cluster) >= 2)
            .count(),
    )
    .unwrap_or(u64::MAX);
    let reported_clusters = metric_field(&report, "clusters_total")
        .as_u64()
        .unwrap_or(u64::MAX);
    assert_eq!(
        reported_clusters, expected_clusters,
        "clusters_total {reported_clusters} must match the {expected_clusters} visible \
         contributing clusters"
    );

    Ok(())
}
