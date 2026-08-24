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

use serde_json::Value;

use super::{
    cluster_size, clusters, clusters_hidden, expect_cluster_spanning, metric_field,
    occurrence_files, occurrence_texts, signal, Result,
};
use std::path::Path;

/// `metrics.duplicated_loc`, defaulting to `0` so a missing metric
/// fails an at-least assertion instead of passing it.
pub(crate) fn duplicated_loc(report: &Value) -> u64 {
    metric_field(report, "duplicated_loc")
        .as_u64()
        .unwrap_or_default()
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
/// `files` with `size` occurrences, shape evidence at or above
/// `min_structural`, and at least `minimum_loc` duplicated lines — the
/// shared spine of every "this cross-file copy stayed visible" control.
/// Returns the reported texts so each control can still pin *what* the
/// duplication is.
///
/// `min_structural` is a parameter rather than a fixed `0.99` because
/// the two controls hold different bars honestly. A byte-identical copy
/// still saturates and its caller still demands it. A near-copy's
/// reported view spans the divergence, so it measures real but
/// non-exact overlap ([FUSION-SHARED-SUBTREE]) — demanding saturation
/// there demands the fragment view. Passing the bar in keeps the strong
/// assertion strong where it is true, instead of lowering it for both.
pub(crate) fn expect_cross_file_duplicate(
    scan_root: &Path,
    report: &Value,
    files: &[&str],
    size: u64,
    minimum_loc: u64,
    min_structural: f64,
) -> Result<Vec<String>> {
    let cluster = expect_cluster_spanning(report, files)?;
    assert_eq!(
        cluster_size(cluster),
        size,
        "one occurrence per copied file: {cluster:#}"
    );
    let structural = signal(cluster, "structural");
    assert!(
        structural >= min_structural,
        "the copies share a shape, so the structural signal must reach \
         {min_structural}, got {structural}: {cluster:#}"
    );
    assert_duplicated_loc_at_least(report, minimum_loc);
    occurrence_texts(scan_root, cluster)
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

/// Lossless `u64 → f64` for LOC counts and cluster sizes (all far below
/// `2^32`), mirroring the renderer's own clamp-then-widen order. Every
/// metric re-derivation in the golden suites goes through it so no
/// assertion silently loses precision on the way to a comparison.
pub(crate) fn loc_as_f64(value: u64) -> Result<f64> {
    Ok(f64::from(u32::try_from(value)?))
}

/// The complete signal vector a **byte-identical** Type-1 clone must
/// render with embeddings off. Every value is determined by the authored
/// corpus, so there is no band to hide inside ([FUSED-THRESHOLD]) and no
/// signal is exempt from the pin.
///
/// The content triple (`agreement`, `rename_consistency`,
/// `literal_fraction`) belongs here as much as the shape pair does: it
/// feeds [FUSION-CONTENT-GATE] through `buckets::content_support`, so a
/// value that drifts here can promote or demote a cluster. Nothing
/// differs between two byte-identical copies, so there is nothing for a
/// rename to fail to explain and `rename_consistency` is exactly `1.0`;
/// a discounted reading is the engine charging a byte-proven copy for
/// evidence it is not missing ([REPAIR-RENAME-ANCHOR-MASS]). Pinning
/// only the four shape/fused signals is how a stale content figure sat
/// inside a committed golden while the golden's own contract half
/// declared it correct.
pub(crate) const TYPE1_IDENTICAL_SIGNALS: &[(&str, f64)] = &[
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("shape", 1.0),
    ("embedding_cos", 0.0),
    ("fused", 1.0),
    ("agreement", 1.0),
    ("rename_consistency", 1.0),
    ("literal_fraction", 0.0),
];

/// Asserts `cluster` carries exactly [`TYPE1_IDENTICAL_SIGNALS`].
/// `label` names the corpus or language under test so a failure says
/// which byte-identical pair moved.
pub(crate) fn assert_type1_identical_signals(cluster: &Value, label: &str) {
    for (name, expected) in TYPE1_IDENTICAL_SIGNALS {
        let actual = signal(cluster, name);
        assert!(
            (actual - expected).abs() <= 1e-9,
            "{label}: signal `{name}` must be {expected}, got {actual}. Both \
             copies are byte-identical, so every signal is determined. A \
             signal that moves while the source does not is either a \
             corrupted- or misaddressed-blob \
             ([PIPELINE-INCREMENTAL-INTEGRITY]) or an unblessed engine \
             change: {cluster:#}"
        );
    }
}
