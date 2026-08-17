//! An incremental pass owes the cold report
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
//!
//! docs/specs/pipeline.md: "An incremental pass owes the cold report.
//! For any corpus state reachable by any sequence of edits, the report
//! produced by an incremental pass must equal the report a cold pass
//! produces for that same state — field for field: cluster ids,
//! occurrence paths and byte ranges, bucket, every signal, ranking
//! order, `metrics`, and `clusters_hidden`. `cache_stats` is the sole
//! permitted difference."
//!
//! Each test walks one edit history over a throwaway scan root (the
//! fingerprint store always lands at `<scan_root>/.deslop/cache`, so a
//! checked-in fixture is never scanned with the store on), then
//! compares the incremental report against a cold report of the same
//! corpus state as full JSON documents with exactly the top-level
//! `cache_stats` member removed from both sides. Explicit `cache_stats`
//! assertions prove the store really served each pass, and explicit
//! cluster and metric assertions keep the comparison from ever passing
//! on an empty report — pinning today's behaviour so future downstream
//! reuse ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]) inherits an enforced
//! contract.
//!
//! Every corpus file is byte-distinct on purpose: the store is
//! content-addressed (blob path = blake3 of the file bytes), so a
//! byte-identical pair would hit within the cold run itself and the
//! cold miss count would no longer equal the parseable-file count.
//! Distinct one-line banners keep the clone copies byte-distinct while
//! the function they share stays byte-identical.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

mod common;
use crate::common::{incremental::*, *};

/// `--min-nodes` low enough that the eleven-line clone body
/// fingerprints as one clusterable subtree.
const MIN_NODES: u32 = 8;

/// The duplicated function every `dup_*.rs` file carries, byte for
/// byte. Eleven lines, so each copy spans lines 2..=12 under its
/// one-line banner comment.
const DUPLICATE_FN: &str = "pub fn accumulate_totals(items: &[i32]) -> i32 {
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
const REPLACEMENT_FN: &str = "pub fn count_vowels(text: &str) -> usize {
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

/// Banner for the copy added in the file-add scenario — shared by the
/// mutation and the fresh ground-truth tree so both hold the same bytes.
const DELTA_BANNER: &str = "delta copy joins after the warm run";

/// Banner for `dup_c.rs` — shared by [`corpus`] and the rename
/// scenario's ground-truth tree, where `dup_moved.rs` keeps its bytes.
const GAMMA_BANNER: &str = "gamma copy rounds out the trio";

/// The three files carrying the clone in the baseline corpus.
const DUPLICATE_TRIO: [&str; 3] = ["dup_a.rs", "dup_b.rs", "dup_c.rs"];

/// The two clone carriers that survive editing or deleting `dup_c.rs`.
const DUPLICATE_PAIR: [&str; 2] = ["dup_a.rs", "dup_b.rs"];

/// All four clone carriers once `dup_d.rs` is added.
const DUPLICATE_QUAD: [&str; 4] = ["dup_a.rs", "dup_b.rs", "dup_c.rs", "dup_d.rs"];

/// One clone copy: a distinct one-line banner comment above the shared
/// function, keeping every copy byte-distinct (see module doc).
fn dup_source(banner: &str) -> String {
    format!("// {banner}\n{DUPLICATE_FN}")
}

/// The five-file baseline corpus: three byte-distinct copies of the
/// clone plus two structurally unrelated fillers.
fn corpus() -> Vec<(String, String)> {
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
fn corpus_with_dup_c(file_name: &str, source: &str) -> Vec<(String, String)> {
    corpus()
        .into_iter()
        .map(|(name, text)| match name.as_str() {
            "dup_c.rs" => (file_name.to_owned(), source.to_owned()),
            _ => (name, text),
        })
        .collect()
}

/// Writes `(file_name, source)` pairs into a freshly created `root`.
fn seed_tree(root: &Path, files: &[(String, String)]) -> Result<()> {
    fs::create_dir_all(root)?;
    for (file_name, source) in files {
        fs::write(root.join(file_name), source)?;
    }
    Ok(())
}

/// Runs `deslop` over `scan_root` and returns the JSON report. Passing
/// `incremental` runs the default store-on path; otherwise
/// `--no-incremental` is added and the store is never consulted.
fn run(scan_root: &Path, incremental: bool) -> Result<Value> {
    run_report_with_store(scan_root, MIN_NODES, Store::incremental(incremental))
}

/// Seeds a fresh root with [`corpus`], then asserts the baseline cold
/// run fills the store ({hits: 0, misses: 5}) and the warm run serves
/// it in full ({hits: 5, misses: 0}).
fn seeded_warm_root() -> Result<(tempfile::TempDir, PathBuf, Value, Value)> {
    let guard = tempfile::tempdir()?;
    let root = guard.path().join("src");
    seed_tree(&root, &corpus())?;
    let cold = run(&root, true)?;
    assert_cache_stats(&cold, 0, 5, "baseline cold");
    let warm = run(&root, true)?;
    assert_cache_stats(&warm, 5, 0, "baseline warm");
    Ok((guard, root, cold, warm))
}

/// A store-off (`--no-incremental`) pass over a fresh tree holding
/// `files` — the cold ground truth every incremental report is owed.
fn cold_truth(files: &[(String, String)]) -> Result<Value> {
    let guard = tempfile::tempdir()?;
    let root = guard.path().join("src");
    seed_tree(&root, files)?;
    let truth = run(&root, false)?;
    assert_cache_stats(&truth, 0, 0, "no-incremental ground truth");
    Ok(truth)
}

/// `files.len()` as the `u64` the wire counts use, following the
/// saturating conversion precedent in `common::line_count` (a saturated
/// value fails the assertion loudly instead of panicking).
fn expected_count(files: &[&str]) -> u64 {
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

/// Asserts the clone cluster spanning exactly `files`: bucket
/// `identical`, one occurrence per file, saturated structural and token
/// signals, and the clone's line span in every occurrence.
fn assert_identical_cluster(report: &Value, files: &[&str], label: &str) -> Result<()> {
    let clone = expect_cluster_spanning(report, files)?;
    assert_eq!(
        cluster_bucket(clone),
        "identical",
        "{label}: clone bucket: {report}"
    );
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
    assert!(
        approx(signal(clone, "structural"), 1.0),
        "{label}: byte-identical copies must saturate the structural signal: {report}"
    );
    assert!(
        approx(signal(clone, "token_jaccard"), 1.0),
        "{label}: byte-identical copies must saturate the token signal: {report}"
    );
    assert_clone_lines(clone, report, label);
    Ok(())
}

/// Asserts the report's corpus-level shape — analysed-file count, zero
/// hidden clusters, exactly one visible cluster, and the duplicated-file
/// metric — then the clone cluster itself. These positive assertions
/// keep [`assert_reports_equal`] from ever passing on an empty report.
fn assert_report_shape(
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

/// Every relative path the report mentions: cluster occurrence paths
/// plus `metrics.per_file` rows.
fn all_report_paths(report: &Value) -> Vec<String> {
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

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Cold-store and warm-store
// passes over one root must both render the exact report a store-off
// pass renders for an identical fresh tree; the store may only ever
// show up in cache_stats.
#[test]
fn cold_and_warm_cached_runs_match_the_uncached_cold_report() -> Result<()> {
    let (_guard, _root, cold, warm) = seeded_warm_root()?;
    let truth = cold_truth(&corpus())?;
    for (label, report) in [
        ("baseline cold", &cold),
        ("baseline warm", &warm),
        ("ground truth", &truth),
    ] {
        assert_report_shape(report, 5, &DUPLICATE_TRIO, label)?;
    }
    assert_reports_equal(&cold, &truth, "cold cached pass vs uncached pass");
    assert_reports_equal(&warm, &truth, "warm cached pass vs uncached pass");
    assert_reports_equal(&warm, &cold, "warm cached pass vs cold cached pass");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Rewriting one file after
// the warm run (re-parsing only that file: hits 4, misses 1) must
// render the exact report a cold pass renders for a fresh tree already
// holding the post-edit state — here the edit removes one clone
// occurrence, shrinking the cluster from trio to pair.
#[test]
fn editing_one_file_matches_the_cold_report_of_the_post_edit_tree() -> Result<()> {
    let (_guard, root, cold, _warm) = seeded_warm_root()?;
    fs::write(root.join("dup_c.rs"), REPLACEMENT_FN)?;
    let incremental = run(&root, true)?;
    assert_cache_stats(&incremental, 4, 1, "post-edit incremental");
    assert_ne!(
        without_cache_stats(&incremental),
        without_cache_stats(&cold),
        "rewriting dup_c.rs must change the report — otherwise this scenario compares nothing"
    );
    let truth = cold_truth(&corpus_with_dup_c("dup_c.rs", REPLACEMENT_FN))?;
    assert_report_shape(&incremental, 5, &DUPLICATE_PAIR, "post-edit incremental")?;
    assert_report_shape(&truth, 5, &DUPLICATE_PAIR, "post-edit ground truth")?;
    assert_reports_equal(&incremental, &truth, "one-file edit");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Adding a byte-distinct
// fourth copy after the warm run must miss for exactly the new file
// (hits 5, misses 1) and render the exact report a cold pass renders
// for a fresh six-file tree — the cluster grows from trio to quad.
#[test]
fn adding_a_duplicate_file_matches_the_cold_report_of_the_grown_tree() -> Result<()> {
    let (_guard, root, _cold, _warm) = seeded_warm_root()?;
    fs::write(root.join("dup_d.rs"), dup_source(DELTA_BANNER))?;
    let incremental = run(&root, true)?;
    assert_cache_stats(&incremental, 5, 1, "post-add incremental");
    let mut grown = corpus();
    grown.push(("dup_d.rs".to_owned(), dup_source(DELTA_BANNER)));
    let truth = cold_truth(&grown)?;
    assert_report_shape(&incremental, 6, &DUPLICATE_QUAD, "post-add incremental")?;
    assert_report_shape(&truth, 6, &DUPLICATE_QUAD, "post-add ground truth")?;
    assert_reports_equal(&incremental, &truth, "file add");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Deleting one clone
// carrier after the warm run must serve every surviving file from the
// store (hits 4, misses 0 — discovery, not the store, decides corpus
// membership) and render the exact report a cold pass renders for a
// fresh four-file tree.
#[test]
fn deleting_a_file_matches_the_cold_report_of_the_shrunk_tree() -> Result<()> {
    let (_guard, root, _cold, _warm) = seeded_warm_root()?;
    fs::remove_file(root.join("dup_c.rs"))?;
    let incremental = run(&root, true)?;
    assert_cache_stats(&incremental, 4, 0, "post-delete incremental");
    let survivors: Vec<(String, String)> = corpus()
        .into_iter()
        .filter(|(name, _)| name != "dup_c.rs")
        .collect();
    let truth = cold_truth(&survivors)?;
    assert_report_shape(&incremental, 4, &DUPLICATE_PAIR, "post-delete incremental")?;
    assert_report_shape(&truth, 4, &DUPLICATE_PAIR, "post-delete ground truth")?;
    assert_reports_equal(&incremental, &truth, "file delete");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Renaming a file without
// touching its bytes must still hit the content-addressed store for
// every file (hits 5, misses 0 — the blob key is the content hash, not
// the path), while every reported path carries the new name, exactly as
// a cold pass over a fresh tree with the renamed layout reports it.
#[test]
fn renaming_a_file_hits_the_content_addressed_store_and_matches_cold() -> Result<()> {
    let (_guard, root, _cold, _warm) = seeded_warm_root()?;
    fs::rename(root.join("dup_c.rs"), root.join("dup_moved.rs"))?;
    let incremental = run(&root, true)?;
    assert_cache_stats(&incremental, 5, 0, "post-rename incremental");
    let truth = cold_truth(&corpus_with_dup_c(
        "dup_moved.rs",
        &dup_source(GAMMA_BANNER),
    ))?;
    let renamed_trio = ["dup_a.rs", "dup_b.rs", "dup_moved.rs"];
    assert_report_shape(&incremental, 5, &renamed_trio, "post-rename incremental")?;
    assert_report_shape(&truth, 5, &renamed_trio, "post-rename ground truth")?;
    let paths = all_report_paths(&incremental);
    assert_eq!(
        paths.iter().filter(|path| *path == "dup_moved.rs").count(),
        2,
        "renamed file must appear as one occurrence and one per-file metric row: {incremental}"
    );
    assert_eq!(
        paths.iter().filter(|path| *path == "dup_c.rs").count(),
        0,
        "the old name must vanish from every reported path: {incremental}"
    );
    assert_reports_equal(&incremental, &truth, "same-bytes rename");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Editing a file and then
// reverting it to its exact previous bytes must land back on the
// original cold report, with the reverted file served from the store
// (hits 5, misses 0 — the original blob is still addressable after the
// mid-edit run stored the edited one).
#[test]
fn reverting_an_edit_restores_the_original_cold_report_with_full_hits() -> Result<()> {
    let (_guard, root, cold, _warm) = seeded_warm_root()?;
    let original_bytes = fs::read(root.join("dup_c.rs"))?;
    fs::write(root.join("dup_c.rs"), REPLACEMENT_FN)?;
    let edited = run(&root, true)?;
    assert_cache_stats(&edited, 4, 1, "mid-edit incremental");
    assert_ne!(
        without_cache_stats(&edited),
        without_cache_stats(&cold),
        "the edit must actually change the report, or the revert below proves nothing"
    );
    fs::write(root.join("dup_c.rs"), &original_bytes)?;
    let reverted = run(&root, true)?;
    assert_cache_stats(&reverted, 5, 0, "post-revert incremental");
    assert_report_shape(&reverted, 5, &DUPLICATE_TRIO, "post-revert incremental")?;
    assert_reports_equal(&reverted, &cold, "revert vs the original cold report");
    Ok(())
}
