//! The shared contract every noise-family pin is judged by
//! ([CLONE-NOISE], [RANK-STRUCTURAL-ONLY], [RANK-SCORE], [METRICS-REPO]).
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
//!
//! What the control is, is not a matter of degree. It is two copies of
//! one function with no byte between them, so its bucket, its every
//! signal, its occurrence count and its rank are all determined before
//! the tool runs. This module asserts them as determined values. An
//! earlier revision accepted any of three act-now buckets and said
//! nothing about rank, hidden occurrences or the metric figures — a
//! fusion regression that stopped saturating a byte-proven copy, or a
//! ranking change that buried it under the noise it was staged against,
//! passed every noise pin in the tree.

use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    approx, cluster_bucket, cluster_count, cluster_id, cluster_size, cluster_spanning, clusters,
    clusters_hidden, expect_cluster_spanning, field, metric_field, occurrence_files,
    occurrence_is_hidden, occurrences, signal,
    signals::{
        signal_dump, ACT_NOW_BUCKETS, ACT_NOW_FUSED, HONEST_SHAPE_ONLY_BUCKETS, IDENTICAL_BUCKET,
    },
    verdict::{assert_type1_identical_signals, duplicated_loc, loc_as_f64},
    visible_cluster_lines, visible_duplicated_lines, Result,
};

/// The number of visible clusters a fully-suppressed noise fixture may
/// publish: the control, and nothing else.
const SOLE_VISIBLE_CLUSTER: usize = 1;

/// The report field that proves the scan opened a file, rather than
/// skipping it and leaving the family absent for a reason the pin never
/// intended to assert.
const FILES_ANALYSED_FIELD: &str = "files_analysed";

/// Asserts the two-sided contract: no visible cluster spans
/// `family_files`, the suppression is counted **exactly**
/// `expected_hidden` times, and the byte-identical control is published
/// first, whole, and saturated.
pub(crate) fn assert_family_hidden_with_control(
    report: &Value,
    label: &str,
    family_files: &[&str],
    control_files: &[&str],
    expected_hidden: u64,
) -> Result<()> {
    assert_family_hidden(report, label, family_files, expected_hidden);
    assert_control_visible(report, label, control_files)
}

/// The suppression half: nothing spanning the family reaches the report,
/// and `clusters_hidden` records exactly the decisions the fixture
/// determines.
///
/// The count is `==`, not `>=`. `clusters_hidden` is a whole-run
/// counter ([`deslop_core::report`] derives it as the number of clusters
/// the render pass hid), so `>= 1` cannot attribute a suppression to
/// *this* family: it is equally satisfied when the family never
/// clustered while something unrelated was hidden. Worse, `>=` is signed
/// the wrong way — every over-suppression regression moves the number
/// **up**, the one direction a lower bound cannot see, and
/// over-suppression is the false-negative direction. Measured on
/// `python-issue-107-chained-dict-assert`: with one of the three pytest
/// modules deleted the run still reports `clusters_hidden == 1`, and
/// every `>= 1` assertion in that pin still passes. The value is fixed
/// by the checked-in fixture, `min_nodes`, `--embeddings off` and
/// `--no-incremental`; nothing about it is environmental.
fn assert_family_hidden(report: &Value, label: &str, family_files: &[&str], expected_hidden: u64) {
    assert!(
        cluster_spanning(report, family_files).is_none(),
        "{label}: the family is scaffolding, not duplication — no cluster may span \
         {family_files:?}: {published:#?}",
        published = published_summary(report),
    );
    assert_eq!(
        clusters_hidden(report),
        expected_hidden,
        "{label}: the family must be actively hidden and counted, not merely absent \
         — an uncounted disappearance is indistinguishable from a detector that \
         stopped looking, and a *higher* count is a filter that has begun eating \
         code this fixture never staged: {report:#}"
    );
}

/// The false-negative control half: the authored copy in the same run
/// survives whatever hid the family — published first, in the one bucket
/// a byte-identical copy may carry, with every signal saturated, both
/// occurrences present and neither of them hidden.
fn assert_control_visible(report: &Value, label: &str, control_files: &[&str]) -> Result<()> {
    let control = expect_cluster_spanning(report, control_files)?;
    assert_control_verdict(control, label);
    assert_control_shows_both_copies(control, label, control_files);
    assert_control_occurrences_are_shown(control, label);
    assert_control_is_ranked_first(report, control, label);
    Ok(())
}

/// Bucket half. The act-now membership test admits three buckets where
/// the fixture determines one, so both are asserted: the wide one names
/// the failure a reader recognises, the exact one is what actually
/// holds.
fn assert_control_verdict(control: &Value, label: &str) {
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(control)),
        "{label}: the control clone is copied byte for byte; a suppression wide \
         enough to demote it has eaten real duplication — bucket={bucket} \
         fused={fused:.4}",
        bucket = cluster_bucket(control),
        fused = signal(control, "fused"),
    );
    assert_control_is_byte_proven(control, label);
}

/// Bucket and signals, exactly ([CLONE-BUCKETS-ROUTING],
/// [FUSED-THRESHOLD]). Two byte-identical copies leave nothing for a
/// signal to be uncertain about, so every one of them is determined.
/// A fusion regression that stops saturating a byte-proven copy is
/// precisely what `TYPE1_IDENTICAL_SIGNALS` exists to catch, and no
/// noise pin was reaching for it.
fn assert_control_is_byte_proven(control: &Value, label: &str) {
    assert_eq!(
        cluster_bucket(control),
        IDENTICAL_BUCKET,
        "{label}: the control is copied byte for byte, so `{IDENTICAL_BUCKET}` is \
         the only honest bucket — any other act-now label claims the copies differ \
         somewhere they do not: {dump}",
        dump = signal_dump(control),
    );
    assert_type1_identical_signals(control, label);
}

/// Occurrence count and span: one occurrence per copied file, and no
/// file the control does not own.
fn assert_control_shows_both_copies(control: &Value, label: &str, control_files: &[&str]) {
    assert_eq!(
        usize::try_from(cluster_size(control)).unwrap_or(usize::MAX),
        control_files.len(),
        "{label}: every copy of the control clone must be shown — hiding one \
         occurrence is a false negative the cluster count cannot see: {dump}",
        dump = signal_dump(control),
    );
    assert_eq!(
        occurrence_files(control).len(),
        control_files.len(),
        "{label}: the control clone spans exactly its own files: {files:?}",
        files = occurrence_files(control),
    );
}

/// Occurrence half: both copies are *shown*. `size` counts members, so a
/// member rendered `hidden: true` keeps the count right while dropping
/// out of the report a human reads and out of every line metric. Today
/// the LOC assertion catches that by arithmetic accident — both halves
/// happen to be the same length — which is a coupling, not a contract.
fn assert_control_occurrences_are_shown(control: &Value, label: &str) {
    assert!(
        !occurrences(control).iter().any(occurrence_is_hidden),
        "{label}: a byte-proven copy may not carry a hidden occurrence — it would \
         vanish from duplicated_loc while the cluster size still reads whole: \
         {control:#}"
    );
}

/// Ranking order ([RANK-SCORE], `docs/specs/noise.md`
/// §CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE, which requires the control
/// "stays visible **and ranked first**"). A saturated byte-identical
/// copy outranks anything else these fixtures stage, so it heads the
/// report. A control that survives but ranks behind the noise it was
/// staged against is a finding the reader never reaches — and the spec
/// stated this requirement with nothing asserting it.
fn assert_control_is_ranked_first(report: &Value, control: &Value, label: &str) {
    assert!(
        clusters(report)
            .first()
            .is_some_and(|first| std::ptr::eq(first, control)),
        "{label}: the byte-identical control must be the report's first finding, \
         not something a reader scrolls past demoted noise to reach: {published:#?}",
        published = published_summary(report),
    );
}

/// The metric half, stated once instead of once per suite. A family
/// hidden from `clusters` that still feeds `duplicated_loc` is the
/// defect moved, not fixed — and a report that publishes a *second* view
/// of the control satisfies an unchanged line-set total, which is why
/// the cluster counts are pinned beside it.
pub(crate) fn assert_control_is_the_only_published_cluster(
    report: &Value,
    label: &str,
    control_files: &[&str],
    control_loc: u64,
) -> Result<()> {
    assert_eq!(
        (
            duplicated_loc(report),
            metric_field(report, "clusters_total").as_u64(),
            metric_field(report, "duplicated_files").as_u64(),
            cluster_count(report),
        ),
        (
            control_loc,
            u64::try_from(SOLE_VISIBLE_CLUSTER).ok(),
            u64::try_from(control_files.len()).ok(),
            SOLE_VISIBLE_CLUSTER,
        ),
        "{label}: the control clone is the whole of this report's duplication — \
         its lines, its one cluster, its own files, and nothing else published: \
         {lines:#?}",
        lines = visible_cluster_lines(report),
    );
    assert_duplication_percent_re_derives(report, label)
}

/// [METRICS-REPO] `duplication_percent` must be exactly
/// `duplicated_loc / analysed_loc × 100`. Pinning the calculation rather
/// than a magic float keeps the assertion honest when a fixture gains a
/// line, and still fails the moment a suppressed family leaks into
/// either term. A percentage a reader cannot re-derive is not a
/// transparent figure.
fn assert_duplication_percent_re_derives(report: &Value, label: &str) -> Result<()> {
    let duplicated = duplicated_loc(report);
    let analysed = metric_field(report, "analysed_loc")
        .as_u64()
        .unwrap_or_default();
    let reported = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    let expected = 100.0 * loc_as_f64(duplicated)? / loc_as_f64(analysed)?;
    assert!(
        approx(reported, expected),
        "{label}: duplication_percent must be duplicated/analysed × 100 \
         ({duplicated}/{analysed} = {expected}), got {reported}: {report:#}"
    );
    Ok(())
}

/// Which files the metrics attribute a duplicated line to — the
/// path-level statement of "the family was suppressed". The repo-level
/// total cannot say *which* file earned it, so a family leaking into the
/// CI duplication gate under a compensating change is invisible to it.
pub(crate) fn assert_only_the_control_files_carry_duplicated_lines(
    report: &Value,
    label: &str,
    control_files: &[&str],
) {
    let counted: BTreeSet<String> = visible_duplicated_lines(report).into_keys().collect();
    let expected: BTreeSet<String> = control_files
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(
        counted,
        expected,
        "{label}: only the control clone's own files may carry a duplicated line — \
         a family file here is the suppression leaking into the duplication gate: \
         {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

/// The whole of what a fully-suppressed noise fixture must show, in one
/// value, so a pin cannot state three quarters of it.
///
/// Five suites were each spelling the same four-call block out by hand.
/// A block copied five times is a block that drifts five ways: the
/// moment one of them dropped the metric half or the analysed count, it
/// would still read like a complete pin. Stating the fixture's numbers
/// as data and the contract once as code makes the contract the thing
/// that is shared and the numbers the thing that varies.
#[derive(Debug)]
pub(crate) struct SuppressedFamily<'fixture> {
    /// The files holding the scaffolding family. Judged **one at a
    /// time**: [`cluster_spanning`] matches a cluster containing *all*
    /// the names it is given, so handing it the whole family at once
    /// asks only that no single cluster pools every file — a bar a
    /// family split across two clusters clears while still publishing.
    pub(crate) family_files: &'fixture [&'fixture str],
    /// The byte-identical false-negative control staged beside it.
    pub(crate) control_files: &'fixture [&'fixture str],
    /// Clusters the render pass must hide — exactly, never at least.
    pub(crate) expected_hidden: u64,
    /// Duplicated lines the control accounts for, which is the whole of
    /// this report's duplication.
    pub(crate) control_loc: u64,
    /// Files the scan must have read, so a skipped file cannot pass
    /// itself off as a suppression.
    pub(crate) files_analysed: u64,
}

/// Asserts every half of the contract: the family hidden per file and
/// counted exactly, the byte-identical control published first, whole
/// and saturated, the metrics counting that control and nothing else —
/// down to which file each duplicated line is charged to — and every
/// file analysed.
pub(crate) fn assert_suppressed_family(
    report: &Value,
    label: &str,
    fixture: &SuppressedFamily<'_>,
) -> Result<()> {
    for family_file in fixture.family_files {
        assert_family_hidden_with_control(
            report,
            label,
            &[*family_file],
            fixture.control_files,
            fixture.expected_hidden,
        )?;
    }
    assert_control_is_the_only_published_cluster(
        report,
        label,
        fixture.control_files,
        fixture.control_loc,
    )?;
    assert_only_the_control_files_carry_duplicated_lines(report, label, fixture.control_files);
    assert_every_file_was_analysed(report, label, fixture);
    Ok(())
}

/// A suppression is only proven when the scan read the files. A file the
/// walker skipped leaves the family just as absent while proving
/// nothing, so the analysed count is pinned beside every other half.
fn assert_every_file_was_analysed(report: &Value, label: &str, fixture: &SuppressedFamily<'_>) {
    assert_eq!(
        field(report, FILES_ANALYSED_FIELD).as_u64(),
        Some(fixture.files_analysed),
        "{label}: every family file {family:?} and every control file {control:?} must be \
         analysed — a file the scan never opened leaves the family just as absent while \
         proving nothing was suppressed: {report:#}",
        family = fixture.family_files,
        control = fixture.control_files,
    );
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
/// into an act-now bucket. `expected_hidden` holds the same bar for the
/// suppression counter, for the reason [`assert_family_hidden`] gives.
/// The residual itself is recorded against its issue in each caller's
/// module doc, so a reader can tell a known, bounded gap from a passing
/// test.
pub(crate) fn assert_family_demoted_with_control(
    report: &Value,
    label: &str,
    family_files: &[&str],
    control_files: &[&str],
    expected_demoted: usize,
    expected_hidden: u64,
) -> Result<()> {
    let over_family = clusters_over_family(report, family_files);
    assert_eq!(
        over_family.len(),
        expected_demoted,
        "{label}: the family's known residual is {expected_demoted} demoted \
         cluster(s); anything else is a change that must be looked at, not \
         absorbed: {published:#?}",
        published = published_summary(report),
    );
    assert_eq!(
        clusters_hidden(report),
        expected_hidden,
        "{label}: the sub-families this fixture suppresses are a determined \
         count, and a *higher* one is a filter that has begun eating code the \
         fixture never staged: {report:#}"
    );
    assert_each_family_cluster_is_demoted(&over_family, label);
    assert_control_visible(report, label, control_files)
}

/// Every visible cluster holding at least one occurrence in the family.
fn clusters_over_family<'a>(report: &'a Value, family_files: &[&str]) -> Vec<&'a Value> {
    clusters(report)
        .iter()
        .filter(|cluster| {
            occurrence_files(cluster)
                .iter()
                .any(|file| family_files.contains(&file.as_str()))
        })
        .collect()
}

/// A family the tool cannot act on must at least be labelled shape-only
/// and stay below the act-now line.
fn assert_each_family_cluster_is_demoted(over_family: &[&Value], label: &str) {
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
}
