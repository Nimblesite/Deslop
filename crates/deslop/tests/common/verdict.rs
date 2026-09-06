//! Shared verdict assertions: what the report says about a fixture as a
//! whole ([METRICS-REPO]) and about the shape of a single expected
//! cluster.
//!
//! Every accuracy control asks one of two questions — "this stayed
//! visible, and the metrics counted it" or "this was suppressed, and the
//! metrics did not" — so both belong here rather than restated per
//! binary. Restating them is what let the metric assertions drift: one
//! suite checked `duplication_percent` while its siblings checked only
//! `duplicated_loc`, and a percentage inflated by a shape match would
//! have passed every one of them.

use std::{collections::BTreeSet, ops::RangeInclusive, path::Path};

use anyhow::anyhow;
use serde_json::Value;

use super::{
    assert_occurrence_extents, cluster_size, clusters, clusters_hidden, expect_cluster_spanning,
    field, line_count, metric_field, occurrence_files, occurrence_texts, per_file_metrics, signals,
    signals::assert_no_pair_surface_on_cluster, signals::has_verbatim_pair,
    visible_duplicated_lines, Result,
};

/// `metrics.duplicated_loc`, defaulting to `0` so a missing metric
/// fails an at-least assertion instead of passing it.
pub(crate) fn duplicated_loc(report: &Value) -> u64 {
    metric_field(report, "duplicated_loc")
        .as_u64()
        .unwrap_or_default()
}

/// Reads a file's duplicated-line count, requiring its metric row and
/// count to be present so an omitted file cannot masquerade as rejected.
pub(crate) fn duplicated_loc_for_path(report: &Value, path: &str) -> Result<u64> {
    per_file_metrics(report)
        .iter()
        .find(|metric| field(metric, "path").as_str() == Some(path))
        .and_then(|metric| field(metric, "duplicated_loc").as_u64())
        .ok_or_else(|| anyhow::anyhow!("missing duplicated_loc for {path}: {report:#}"))
}

/// Asserts the report attributes at least `minimum` duplicated lines —
/// the metric half of every "this stayed visible" control. A cluster
/// that is reported but contributes no lines is a metric defect, so
/// visibility assertions pair with this one.
pub(crate) fn assert_duplicated_loc_at_least(report: &Value, minimum: u64) {
    let actual = duplicated_loc(report);
    assert!(
        actual >= minimum,
        "visible duplication must count toward the metrics: \
         duplicated_loc={actual}, expected >= {minimum}, report={report:#}"
    );
}

/// Asserts the report is fully suppressed: no visible contributing
/// cluster, no duplicated lines and a zero percentage, while
/// `clusters_hidden` still shows the detector saw the shape. Proving a
/// shape match may never inflate the percentage.
pub(crate) fn assert_fully_suppressed(report: &Value, minimum_hidden: u64) {
    let visible = metric_field(report, "clusters_total")
        .as_u64()
        .unwrap_or(u64::MAX);
    assert_eq!(
        visible, 0,
        "every cluster here is suppressed, so none may count as a visible \
         contributing cluster: {report:#}"
    );
    let lines = metric_field(report, "duplicated_loc")
        .as_u64()
        .unwrap_or(u64::MAX);
    assert_eq!(
        lines, 0,
        "suppressed clusters must add zero duplicated lines: {report:#}"
    );
    assert_duplication_percent_zero(report);
    let hidden = clusters_hidden(report);
    assert!(
        hidden >= minimum_hidden,
        "the suppressed shapes must still be detected and counted toward \
         visibility telemetry: clusters_hidden={hidden}, expected >= \
         {minimum_hidden}, report={report:#}"
    );
}

/// Asserts the duplication percentage is zero — the metric may not be
/// influenced by a shape match the report never rendered.
pub(crate) fn assert_duplication_percent_zero(report: &Value) {
    let percent = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!(
        (0.0..=0.0001).contains(&percent),
        "duplication_percent must be 0 when every cluster is suppressed — the \
         metric is not influenced by shape matches: got {percent}, report={report:#}"
    );
}

/// Asserts the report publishes exactly `expected` visible clusters and
/// hides none, returning them. `why` states what a different count would
/// mean, so the failure names the defect rather than the number.
pub(crate) fn expect_visible_only<'a>(
    report: &'a Value,
    expected: usize,
    why: &str,
) -> &'a [Value] {
    let visible = clusters(report);
    assert_eq!(visible.len(), expected, "{why} report={report:#}");
    assert_eq!(
        clusters_hidden(report),
        0,
        "{why} nothing here is proven scaffolding, so no cluster may be \
         hidden: {report:#}"
    );
    visible
}

/// The sole visible cluster, asserted to exist.
pub(crate) fn expect_sole_cluster<'a>(report: &'a Value, why: &str) -> Result<&'a Value> {
    let visible = expect_visible_only(report, 1, why);
    visible
        .first()
        .ok_or_else(|| anyhow::anyhow!("the visible cluster asserted above is missing"))
}

/// Asserts a cluster covers `size` occurrences, all inside `file`.
pub(crate) fn assert_single_file_cluster(cluster: &Value, size: u64, file: &str) {
    assert_eq!(
        cluster_size(cluster),
        size,
        "every member must be an occurrence of the one cluster: {cluster:#}"
    );
    let expected: Vec<String> = (0..size).map(|_| file.to_owned()).collect();
    assert_eq!(
        occurrence_files(cluster),
        expected,
        "single-file cluster by construction: {cluster:#}"
    );
}

/// Asserts the report publishes a cross-file duplicate spanning
/// `files` with `size` occurrences and at least `minimum_loc` duplicated
/// lines — the shared spine of every "this cross-file copy stayed
/// visible" control. Returns the reported texts so each control can
/// still pin *what* the duplication is.
///
/// `byte_identical` is the byte-level truth the deleted structural bar
/// used to proxy. A byte-identical copy is proven by the corpus itself
/// ([`has_verbatim_pair`]); a byte-distinct near-copy (rename / literal
/// drift) is asserted on the same evidence, negated. Either way the
/// cluster must be admitted and carry the mass-only surface
/// ([PIPELINE-CLUSTER-CLOSURE]).
pub(crate) fn expect_cross_file_duplicate(
    scan_root: &Path,
    report: &Value,
    files: &[&str],
    size: u64,
    minimum_loc: u64,
    byte_identical: bool,
) -> Result<Vec<String>> {
    let cluster = expect_cluster_spanning(report, files)?;
    assert_eq!(
        cluster_size(cluster),
        size,
        "one occurrence per copied file: {cluster:#}"
    );
    assert_eq!(
        has_verbatim_pair(scan_root, cluster)?,
        byte_identical,
        "the fixture bytes decide whether this is a byte-proven copy or a \
         byte-distinct rename, and the report must carry the cluster \
         either way: {cluster:#}"
    );
    assert_no_pair_surface_on_cluster(cluster, "cross-file duplicate");
    assert_duplicated_loc_at_least(report, minimum_loc);
    occurrence_texts(scan_root, cluster)
}

/// Asserts every string in `evidence` reached the reported occurrence
/// text. `why` says what the evidence is, so a failure names the missing
/// proof rather than the needle.
///
/// The evidence a control names is the whole point of publishing the
/// cluster — the member that holds the copy, the literal that makes one
/// call dead, the helper that parameterising would absorb — so the loop
/// that checks for it lives here rather than being restated per binary.
pub(crate) fn assert_reported(texts: &[String], evidence: &[&str], why: &str) {
    for needle in evidence {
        assert!(
            texts.iter().any(|text| text.contains(needle)),
            "{why}; {needle} must be reported: {texts:#?}"
        );
    }
}

/// Asserts every name appears somewhere in the cluster's reported text,
/// returning the texts for any further per-test assertions.
pub(crate) fn assert_cluster_mentions(
    scan_root: &Path,
    cluster: &Value,
    names: &[&str],
) -> Result<Vec<String>> {
    let texts = occurrence_texts(scan_root, cluster)?;
    for name in names {
        assert!(
            texts.iter().any(|text| text.contains(name)),
            "{name} is part of the duplication and must be reported: {texts:#?}"
        );
    }
    Ok(texts)
}

/// A ratio rendered as a percentage.
const PERCENT_SCALE: f64 = 100.0;

/// Lossless `u64 → f64` for LOC counts and cluster sizes (all far below
/// `2^32`), mirroring the renderer's own clamp-then-widen order. Every
/// metric re-derivation in the golden suites goes through it so no
/// assertion silently loses precision on the way to a comparison.
pub(crate) fn loc_as_f64(value: u64) -> Result<f64> {
    Ok(f64::from(u32::try_from(value)?))
}

/// [METRICS-REPO] Asserts the report's percentage is exactly the report's
/// own line counts, at the repo level and for every file it lists. The
/// headline figure is the reader's to check: a percentage that does not
/// divide the lines beside it is a transparency defect whatever the
/// clusters say.
pub(crate) fn assert_percent_matches_lines(report: &Value) -> Result<()> {
    let rows = std::iter::once(("<repo>", field(report, "metrics"))).chain(
        per_file_metrics(report)
            .iter()
            .map(|row| (field(row, "path").as_str().unwrap_or("?"), row)),
    );
    for (label, row) in rows {
        let analysed = field(row, "analysed_loc").as_u64().unwrap_or(0);
        let duplicated = field(row, "duplicated_loc").as_u64().unwrap_or(0);
        let percent = field(row, "duplication_percent").as_f64().unwrap_or(-1.0);
        // Through `loc_as_f64`, like every other metric re-derivation
        // here: a raw `as f64` would round a count silently, which is
        // the one thing an assertion about an exact division may not do.
        let expected = if analysed == 0 {
            0.0
        } else {
            loc_as_f64(duplicated)? / loc_as_f64(analysed)? * PERCENT_SCALE
        };
        assert!(
            (percent - expected).abs() < 0.0001,
            "{label}: duplication_percent must be duplicated_loc / analysed_loc \
             — {duplicated}/{analysed} is {expected}, the report says {percent}: \
             {report:#}"
        );
    }
    Ok(())
}

/// The whole published contract for a report whose one finding is a
/// same-file pair: exactly one visible cluster with nothing hidden, two
/// occurrences at `spans` in `file`, the wire mass formula, no pair-only
/// surface, rank one, `distinct` distinct occurrence texts, and metrics
/// that both count the pair's lines and divide into the reported
/// percentage. `why` states what a suppression would prove.
///
/// Returns the cluster's occurrence texts, so each control still pins the
/// evidence that varies across its own pair.
pub(crate) fn expect_only_finding_is_the_pair(
    scan_root: &Path,
    report: &Value,
    file: &str,
    spans: &[RangeInclusive<u64>],
    distinct: usize,
    why: &str,
) -> Result<Vec<String>> {
    assert!(
        metric_field(report, "analysed_loc").as_u64().unwrap_or(0) > 0,
        "{why} the pair's file must be parsed (analysed_loc > 0) — a scan \
         that never opened it proves nothing: {report:#}"
    );
    let cluster = expect_visible_only(report, 1, why)
        .first()
        .ok_or_else(|| anyhow!("one visible cluster asserted above"))?;
    assert_single_file_cluster(cluster, 2, file);
    assert_occurrence_extents(cluster, file, spans)?;
    // [RANK-MASS-SUM] / [PIPELINE-CLUSTER-CLOSURE] mass is the wire
    // formula over the visible membership, nothing is hidden behind
    // `report_hide`, and no pair-only evidence reaches a cluster surface.
    signals::assert_structural_only_contract(cluster, why);
    signals::assert_no_pair_surface_on_cluster(cluster, why);
    assert_eq!(
        signals::rank_of(report, cluster)?,
        0,
        "{why} the file's one finding is its worst offender: {report:#}"
    );
    assert_eq!(
        field(cluster, "rank_band").as_str(),
        Some("worst"),
        "{why} the only finding sits in the worst band: {cluster:#}"
    );
    assert_eq!(
        signals::distinct_texts(scan_root, cluster)?.len(),
        distinct,
        "{why} the occurrences' byte truth decides whether this is a \
         verbatim copy or a near-miss, and it must match the fixture: {cluster:#}"
    );
    // [METRICS-REPO] the pair's own lines, counted once and divided honestly.
    let expected: BTreeSet<u64> = spans.iter().cloned().flatten().collect();
    assert_eq!(
        visible_duplicated_lines(report).get(file),
        Some(&expected),
        "{why} only the pair's own lines are duplicated: {report:#}"
    );
    assert_eq!(
        duplicated_loc(report),
        line_count(&expected),
        "{why} the metric counts the pair's lines: {report:#}"
    );
    assert_percent_matches_lines(report)?;
    occurrence_texts(scan_root, cluster)
}
