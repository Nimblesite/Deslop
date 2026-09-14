//! The authored clone corpus every equivalence suite is judged against
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]), plus the cold ground
//! truth and the positive assertions that keep an equivalence
//! comparison from ever passing on an empty report.
//!
//! One definition, two suites: `incremental_equivalence.rs` walks edit
//! histories across separate *processes* (the batch CLI warm path),
//! `live_session_equivalence.rs` walks them inside one long-lived
//! [`deslop_core::PipelineSession`] (the live splice path). Both owe the
//! same cold report for the same corpus state, so they must be scored
//! against the same corpus and the same assertions — restating either
//! per suite is how two answers to one contract get written down.
//!
//! Every corpus file is byte-distinct on purpose: the store is
//! content-addressed (blob path = blake3 of the file bytes), so a
//! byte-identical pair would hit within the cold run itself and the cold
//! miss count would no longer equal the parseable-file count. Distinct
//! one-line banners keep the clone copies byte-distinct while the
//! function they share stays byte-identical.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{
    cluster_count, cluster_file_set, cluster_size, clusters, clusters_hidden,
    expect_cluster_spanning, field, incremental::*, metric_field, occurrence_paths, occurrences,
    per_file_metrics, signals::assert_no_pair_surface_on_cluster, Result,
};

/// `--min-nodes` low enough that the eleven-line clone body fingerprints
/// as one clusterable subtree.
pub(crate) const MIN_NODES: u32 = 8;

/// The duplicated function every `dup_*.rs` file carries, byte for byte.
/// Eleven lines, so each copy spans lines 2..=12 under its one-line
/// banner comment.
pub(crate) const DUPLICATE_FN: &str = "pub fn accumulate_totals(items: &[i32]) -> i32 {
    let mut total = 0;
    for item in items {
        if *item > 0 {
            total += item * 2;
        } else {
            total -= item;
        }
    }
    total
}
";

/// The non-duplicate function `dup_c.rs` is rewritten into for the edit
/// and revert scenarios — a shape shared with nothing else in the
/// corpus, so the rewrite removes exactly one clone occurrence.
pub(crate) const REPLACEMENT_FN: &str = "pub fn count_vowels(text: &str) -> usize {
    text.chars().filter(|ch| \"aeiou\".contains(*ch)).count()
}
";

/// Structurally unrelated filler: string assembly, nothing like the
/// clone's loop-and-branch shape, so it can never join the cluster.
const FILLER_LABELS: &str = "pub fn widget_label(count: usize, name: &str) -> String {
    let mut label = String::from(name);
    label.push(':');
    label.push_str(&count.to_string());
    label
}
";

/// Structurally unrelated filler: a struct plus a clamp expression.
const FILLER_BOUNDS: &str = "pub struct Bounds {
    pub low: i64,
    pub high: i64,
}

pub fn clamp_to_bounds(bounds: &Bounds, value: i64) -> i64 {
    value.max(bounds.low).min(bounds.high)
}
";

/// Banner for the copy added in the file-add scenarios — shared by the
/// mutation and the fresh ground-truth tree so both hold the same bytes.
pub(crate) const DELTA_BANNER: &str = "delta copy joins after the warm run";

/// Banner for `dup_c.rs` — shared by [`corpus`] and the rename
/// scenario's ground-truth tree, where `dup_moved.rs` keeps its bytes.
pub(crate) const GAMMA_BANNER: &str = "gamma copy rounds out the trio";

/// The three files carrying the clone in the baseline corpus.
pub(crate) const DUPLICATE_TRIO: [&str; 3] = ["dup_a.rs", "dup_b.rs", "dup_c.rs"];

/// The two clone carriers that survive editing or deleting `dup_c.rs`.
pub(crate) const DUPLICATE_PAIR: [&str; 2] = ["dup_a.rs", "dup_b.rs"];

/// All four clone carriers once `dup_d.rs` is added.
pub(crate) const DUPLICATE_QUAD: [&str; 4] = ["dup_a.rs", "dup_b.rs", "dup_c.rs", "dup_d.rs"];

/// A clone carrier whose name sorts **before** every baseline file.
/// `dup_d.rs` sorts last, so appending it to the flat store lands it in
/// the right place by luck; this one does not, and a splice that
/// appends instead of inserting in path order renders its occurrence
/// out of order ([PIPELINE-DETERMINISM]).
pub(crate) const EARLY_CARRIER: &str = "aa_dup.rs";

/// Banner for [`EARLY_CARRIER`] — shared by the mutation and the fresh
/// ground-truth tree so both hold the same bytes.
pub(crate) const EARLY_BANNER: &str = "early-sorting copy joins mid-session";

/// The four clone carriers once [`EARLY_CARRIER`] is added, in the
/// ascending path order the report must render them in.
pub(crate) const DUPLICATE_QUAD_EARLY: [&str; 4] =
    ["aa_dup.rs", "dup_a.rs", "dup_b.rs", "dup_c.rs"];

/// One clone copy: a distinct one-line banner comment above the shared
/// function, keeping every copy byte-distinct (see module doc).
pub(crate) fn dup_source(banner: &str) -> String {
    format!("// {banner}\n{DUPLICATE_FN}")
}

/// The five-file baseline corpus: three byte-distinct copies of the
/// clone plus two structurally unrelated fillers.
pub(crate) fn corpus() -> Vec<(String, String)> {
    vec![
        ("dup_a.rs".to_owned(), dup_source("alpha owner")),
        ("dup_b.rs".to_owned(), dup_source("beta copy, same shape")),
        ("dup_c.rs".to_owned(), dup_source(GAMMA_BANNER)),
        ("filler_labels.rs".to_owned(), FILLER_LABELS.to_owned()),
        ("filler_bounds.rs".to_owned(), FILLER_BOUNDS.to_owned()),
    ]
}

/// [`corpus`] with the `dup_c.rs` entry replaced by `(file_name,
/// source)` — builds the edit scenario's ground-truth tree (same name,
/// new source) and the rename scenario's (new name, same bytes).
pub(crate) fn corpus_with_dup_c(file_name: &str, source: &str) -> Vec<(String, String)> {
    corpus()
        .into_iter()
        .map(|(name, text)| match name.as_str() {
            "dup_c.rs" => (file_name.to_owned(), source.to_owned()),
            _ => (name, text),
        })
        .collect()
}

/// [`corpus`] without `dup_c.rs` — the ground-truth tree for every
/// scenario that deletes that carrier.
pub(crate) fn corpus_without_dup_c() -> Vec<(String, String)> {
    corpus()
        .into_iter()
        .filter(|(name, _)| name != "dup_c.rs")
        .collect()
}

/// [`corpus`] plus one more byte-distinct clone carrier — the
/// ground-truth tree for every scenario that adds a carrier, whatever
/// the new file sorts next to.
pub(crate) fn corpus_with_carrier(file_name: &str, banner: &str) -> Vec<(String, String)> {
    let mut grown = corpus();
    grown.push((file_name.to_owned(), dup_source(banner)));
    grown
}

/// [`corpus`] plus the byte-distinct fourth copy `dup_d.rs`.
pub(crate) fn corpus_with_dup_d() -> Vec<(String, String)> {
    corpus_with_carrier("dup_d.rs", DELTA_BANNER)
}

/// Writes `(file_name, source)` pairs into a freshly created `root`.
pub(crate) fn seed_tree(root: &Path, files: &[(String, String)]) -> Result<()> {
    fs::create_dir_all(root)?;
    for (file_name, source) in files {
        fs::write(root.join(file_name), source)?;
    }
    Ok(())
}

/// Runs `deslop` over `scan_root` and returns the JSON report. Passing
/// `incremental` runs the default store-on path; otherwise
/// `--no-incremental` is added and the store is never consulted.
pub(crate) fn run(scan_root: &Path, incremental: bool) -> Result<Value> {
    run_report_with_store(scan_root, MIN_NODES, Store::incremental(incremental))
}

/// A throwaway scan root at `<guard>/src` seeded with `files`. The
/// guard comes back because dropping it deletes the tree. Every scenario
/// starts here — the parse store always lands at
/// `<scan_root>/.deslop/cache`, so a checked-in fixture is never scanned
/// with the store on.
pub(crate) fn seeded_scan_root(files: &[(String, String)]) -> Result<(tempfile::TempDir, PathBuf)> {
    let guard = tempfile::tempdir()?;
    let root = guard.path().join("src");
    seed_tree(&root, files)?;
    Ok((guard, root))
}

/// A store-off (`--no-incremental`) pass over a fresh tree holding
/// `files` — the cold ground truth every incremental report is owed.
pub(crate) fn cold_truth(files: &[(String, String)]) -> Result<Value> {
    let (_guard, root) = seeded_scan_root(files)?;
    let truth = run(&root, false)?;
    assert_cache_stats(&truth, 0, 0, "no-incremental ground truth");
    Ok(truth)
}

/// `files.len()` as the `u64` the wire counts use, following the
/// saturating conversion precedent in `common::line_count` (a saturated
/// value fails the assertion loudly instead of panicking).
pub(crate) fn expected_count(files: &[&str]) -> u64 {
    u64::try_from(files.len()).unwrap_or(u64::MAX)
}

/// Every occurrence must cover the clone body's lines 2..=12 — line 1
/// is each file's distinct banner comment.
fn assert_clone_lines(clone: &Value, report: &Value, label: &str) {
    for occurrence in occurrences(clone) {
        assert_eq!(
            field(occurrence, "start_line").as_u64(),
            Some(2),
            "{label}: clone must start on line 2, under the banner comment: {report}"
        );
        assert_eq!(
            field(occurrence, "end_line").as_u64(),
            Some(12),
            "{label}: clone must end on line 12: {report}"
        );
    }
}

/// Asserts the clone cluster spanning exactly `files`: one occurrence
/// per file, the clone's line span in every occurrence, and a clean
/// cluster surface ([PIPELINE-CLUSTER-CLOSURE]) — the byte-proven
/// verbatim fact the `identical` bucket and signal block used to proxy
/// is asserted from the corpus by [`assert_report_shape`]'s caller
/// (`incremental_equivalence` / `live_session_equivalence` author the
/// corpus as byte-identical copies).
fn assert_identical_cluster(report: &Value, files: &[&str], label: &str) -> Result<()> {
    let clone = expect_cluster_spanning(report, files)?;
    assert_eq!(
        cluster_size(clone),
        expected_count(files),
        "{label}: clone occurrence count: {report}"
    );
    let expected_files: BTreeSet<String> = files.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        cluster_file_set(clone),
        expected_files,
        "{label}: files the clone spans: {report}"
    );
    assert_no_pair_surface_on_cluster(clone, label);
    assert_clone_lines(clone, report, label);
    assert_occurrences_in_path_order(clone, label);
    Ok(())
}

/// [PIPELINE-DETERMINISM] Occurrences render in ascending
/// workspace-relative-path order, because the corpus store holds one
/// span per file in exactly that order and a render borrows the flat
/// slices as they are.
///
/// This is the assertion a live splice is judged by: appending a
/// changed file's records instead of inserting them at the file's sort
/// position leaves every other reading identical — same cluster id,
/// same spans, same signals, same rank — and moves only this order and
/// the `summary` line built from it. Without it, a session that renders
/// its corpus in edit-arrival order passes every other check here.
fn assert_occurrences_in_path_order(clone: &Value, label: &str) {
    let paths = occurrence_paths(clone);
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "{label}: occurrences must render in ascending path order, not in the \
         order a session happened to splice them: {clone:#}"
    );
}

/// Asserts the report's corpus-level shape — analysed-file count, zero
/// hidden clusters, exactly one visible cluster, and the duplicated-file
/// metric — then the clone cluster itself. These positive assertions
/// keep [`assert_reports_equal`] from ever passing on an empty report.
pub(crate) fn assert_report_shape(
    report: &Value,
    files_analysed: u64,
    files: &[&str],
    label: &str,
) -> Result<()> {
    assert_eq!(
        (
            field(report, "files_analysed").as_u64(),
            clusters_hidden(report)
        ),
        (Some(files_analysed), 0),
        "{label}: (files_analysed, clusters_hidden): {report}"
    );
    assert_eq!(
        (
            metric_field(report, "clusters_total").as_u64(),
            cluster_count(report)
        ),
        (Some(1), 1),
        "{label}: exactly one visible cluster, none hidden: {report}"
    );
    assert_eq!(
        metric_field(report, "duplicated_files").as_u64(),
        Some(expected_count(files)),
        "{label}: metrics.duplicated_files: {report}"
    );
    assert_identical_cluster(report, files, label)
}

/// Asserts `path` is named exactly `expected` times across every
/// reported path — cluster occurrences and `metrics.per_file` rows
/// together. A renamed or removed file must vanish from *both*, and a
/// new one must reach both; checking either alone lets the other keep a
/// stale row.
pub(crate) fn assert_reported_path_count(report: &Value, path: &str, expected: usize, why: &str) {
    let actual = all_report_paths(report)
        .iter()
        .filter(|reported| reported.as_str() == path)
        .count();
    assert_eq!(
        actual, expected,
        "{why}: {path} named {actual} times, expected {expected}: {report}"
    );
}

/// Every relative path the report mentions: cluster occurrence paths
/// plus `metrics.per_file` rows.
pub(crate) fn all_report_paths(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .flat_map(occurrence_paths)
        .chain(
            per_file_metrics(report)
                .iter()
                .map(|row| field(row, "path").as_str().unwrap_or_default().to_owned()),
        )
        .collect()
}
