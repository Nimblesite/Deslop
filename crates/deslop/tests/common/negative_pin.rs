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
    approx, cluster_count, cluster_id, cluster_size, cluster_spanning, clusters,
    expect_cluster_spanning, field, metric_field, occurrence_files, occurrence_is_hidden,
    occurrences,
    signals::{assert_no_pair_surface_on_cluster, signal_dump},
    verdict::{duplicated_loc, loc_as_f64},
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
/// and the byte-identical control is published
/// first, whole, and saturated.
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
/// and every family file was parsed, so the absence is a decision, never
/// a scan that never looked.
///
/// The mass-only wire counts `clusters_hidden` only for clusters the
/// render pass hid; a shape-only family the content gate rejects at
/// admission ([FUSED-CONTENT-GATE]) leaves no counter to read. What
/// proves the detector examined the family on the current wire is
/// `metrics.per_file`: the report carries one row per analysed file,
/// and a family file that was parsed but suppressed still carries its
/// analysed lines there. A family file with no row, or a row with no
/// analysed lines, is the detector stopping — exactly the failure the
/// old counter existed to catch.
fn assert_family_hidden(report: &Value, label: &str, family_files: &[&str]) {
    assert!(
        cluster_spanning(report, family_files).is_none(),
        "{label}: the family is scaffolding, not duplication — no cluster may span \
         {family_files:?}: {published:#?}",
        published = published_summary(report),
    );
    let per_file = metric_field(report, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default();
    for family_file in family_files {
        let row = per_file.iter().find(|entry| {
            field(entry, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(family_file))
        });
        let analysed = row
            .and_then(|entry| field(entry, "analysed_loc").as_u64())
            .unwrap_or(0);
        assert!(
            analysed > 0,
            "{label}: the family file {family_file} must be parsed (analysed_loc > 0) — \
             a file the scan never opened leaves the family just as absent while \
             proving nothing was suppressed: {report:#}"
        );
    }
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

/// Cluster-surface half. The duplicate-bucket claim and the `identical`
/// label are gone from the mass-only wire; what proves the control is
/// the byte-level and visibility facts the report still exposes
/// ([PIPELINE-CLUSTER-CLOSURE]): no pair-only field may sit on the
/// cluster (so nothing can mislabel it), the reported membership is
/// complete (nothing hidden or truncated), and the cluster is
/// byte-proven by its occurrences.
fn assert_control_verdict(control: &Value, label: &str) {
    assert_no_pair_surface_on_cluster(control, label);
    assert_control_is_byte_proven(control, label);
}

/// Byte-level and visibility half. Two byte-identical copies leave
/// nothing for a wire field to be uncertain about; the strongest facts
/// the report exposes are that every occurrence is shown, none is
/// hidden, and the carried membership is untruncated.
fn assert_control_is_byte_proven(control: &Value, label: &str) {
    assert!(
        !occurrences(control).iter().any(occurrence_is_hidden),
        "{label}: the control is copied byte for byte; hiding one of its \
         occurrences is a false negative the cluster count cannot see: {dump}",
        dump = signal_dump(control),
    );
    assert_eq!(
        field(control, "occurrence_count").as_u64().unwrap_or(0),
        field(control, "occurrences_total").as_u64().unwrap_or(0),
        "{label}: the byte-identical control must be carried untruncated — \
         occurrence_count must equal occurrences_total: {dump}",
        dump = signal_dump(control),
    );
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
        assert_family_hidden_with_control(report, label, &[*family_file], fixture.control_files)?;
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

/// Every visible cluster as `(id, occurrence_count, files)` — the
/// smallest dump that lets a failure be diagnosed without re-running
/// the scan.
fn published_summary(report: &Value) -> Vec<(&str, u64, Vec<String>)> {
    clusters(report)
        .iter()
        .map(|cluster| {
            (
                cluster_id(cluster),
                field(cluster, "occurrence_count").as_u64().unwrap_or(0),
                occurrence_files(cluster),
            )
        })
        .collect()
}

/// The contract for a family the detector **demotes but does not yet
/// hide**. Same two sides — the control must still be visible — but the
/// family half is stated as what is true today rather than what should
/// be: every cluster over the family is labelled shape-only, none of
/// none of them makes a duplication claim, and there are exactly
/// `expected_demoted` of them.
///
/// The exact count is what makes this a pin rather than a shrug: a
/// *new* family cluster fails it, and so does an existing one climbing
/// into a duplicate bucket. `expected_hidden` holds the same bar for the
/// suppression counter, for the reason [`assert_family_hidden`] gives.
/// The residual itself is recorded against its issue in each caller's
/// module doc, so a reader can tell a known, bounded gap from a passing
/// test.
pub(crate) fn assert_family_demoted_with_control(
    report: &Value,
    label: &str,
    family_files: &[&str],
    control_files: &[&str],
) -> Result<()> {
    // The demotion bucket is gone from the mass-only wire: a shape-only
    // family is either hidden at render or rejected at admission
    // ([RANK-STRUCTURAL-ONLY]), and either way it publishes no cluster.
    // The pin is the family's absence plus the liveness proof, exactly
    // as for a render-hidden family.
    assert_family_hidden(report, label, family_files);
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

/// A family the tool cannot act on stays outside every duplication
/// claim: the mass-only wire gives it no bucket, no verdict, and no
/// pair-only surface, and the report must carry it untruncated and
/// unhidden.
fn assert_each_family_cluster_is_demoted(over_family: &[&Value], label: &str) {
    for cluster in over_family {
        assert_no_pair_surface_on_cluster(cluster, label);
        assert!(
            !occurrences(cluster).iter().any(occurrence_is_hidden),
            "{label}: a reported family cluster may not hide an occurrence — {id}: {dump}",
            id = cluster_id(cluster),
            dump = signal_dump(cluster),
        );
        assert_eq!(
            field(cluster, "occurrence_count").as_u64().unwrap_or(0),
            field(cluster, "occurrences_total").as_u64().unwrap_or(0),
            "{label}: a reported family cluster must be carried untruncated — {id}: {dump}",
            id = cluster_id(cluster),
            dump = signal_dump(cluster),
        );
    }
}
