//! A **live session** owes the cold report
//! ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
//!
//! docs/specs/pipeline.md scopes the equivalence contract to "any corpus
//! state reachable by any sequence of edits" — not to any sequence of
//! *processes*. `incremental_equivalence.rs` walks its edit histories
//! across separate CLI invocations, so every pass there re-discovers the
//! tree and rebuilds the in-memory corpus from scratch; the only reuse
//! under test is the on-disk parse store.
//!
//! The live path reuses far more. A long-lived
//! [`deslop_core::PipelineSession`] keeps the flat fingerprint,
//! signature and tree store, the per-file sources, languages, line
//! counts, boilerplate ranges and path map alive in memory and
//! *splices* one file's records in place per change
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]). Every one of those maps is a
//! chance to serve state the current tree no longer justifies, and a
//! spliced corpus that keeps a stale record reports a cluster that does
//! not exist — a false positive that no amount of parse-store integrity
//! can catch, because the store was never consulted for it.
//!
//! These tests drive that path black-box through the binary:
//! `--rerun-add` / `--rerun-remove` mutate the tree *between*
//! `PipelineSession::initialise` and `PipelineSession::update_files`
//! inside one invocation, so the report the CLI writes is the report a
//! spliced session produced. Each is compared field for field against a
//! `--no-incremental` pass over a fresh tree already holding the
//! post-change state, with `cache_stats` the sole permitted difference.
//!
//! The store is left **on** throughout: the live path is where the
//! store and the splice compose, and pinning them apart would leave the
//! composition untested.

use std::{ffi::OsString, path::Path};

use serde_json::Value;

mod common;
use crate::common::{clone_corpus::*, incremental::*, rerun_ops::*, *};

/// Runs one CLI invocation that initialises a session over `scan_root`,
/// applies `ops` between the two generations, and replays the changed
/// paths through `PipelineSession::update_files`. Returns the report the
/// spliced session rendered.
///
/// The store stays on (see module doc), and the run writes into its own
/// throwaway output directory so the single timestamped log stays
/// unambiguous.
fn spliced_session(scan_root: &Path, ops: &[(&str, OsString)]) -> Result<Value> {
    let out = tempfile::tempdir()?;
    let flattened: Vec<OsString> = ops
        .iter()
        .flat_map(|(flag, value)| [OsString::from(flag), value.clone()])
        .collect();
    let extra: Vec<&str> = flattened
        .iter()
        .map(|arg| arg.to_str().unwrap_or_default())
        .collect();
    let (report, _events) = run_store_on(scan_root, out.path(), MIN_NODES, &extra)?;
    Ok(report)
}

/// Asserts the spliced report is not the baseline it started from. Every
/// scenario below mutates the corpus, so a report that did not move
/// means the splice never happened and the equivalence comparison would
/// be proving nothing.
fn assert_splice_moved_the_report(spliced: &Value, baseline: &Value, scenario: &str) {
    assert_ne!(
        without_cache_stats(spliced),
        without_cache_stats(baseline),
        "{scenario}: the live change must actually move the report, or this \
         scenario compares a session against itself"
    );
}

/// One mid-session add, judged end to end: stage a byte-distinct clone
/// copy outside the scan root, land it at `<root>/<file_name>` between
/// the two generations, and prove the spliced report equals a cold pass
/// over a fresh tree already holding it — same cluster, same spans, same
/// signals, same ranking, same metrics.
fn assert_added_carrier_matches_cold(
    guard: &Path,
    root: &Path,
    carrier: (&str, &str),
    expected_files: &[&str],
) -> Result<()> {
    let (file_name, banner) = carrier;
    let spec = staged_spec(
        guard,
        &format!("staged_{file_name}"),
        &dup_source(banner),
        &root.join(file_name),
    )?;
    let spliced = spliced_session(root, &[("--rerun-add", spec)])?;
    let truth = cold_truth(&corpus_with_carrier(file_name, banner))?;
    let label = format!("live session: {file_name} added mid-session");
    assert_report_shape(&spliced, 6, expected_files, &label)?;
    assert_report_shape(&truth, 6, expected_files, &format!("{label} ground truth"))?;
    assert_reports_equal(&spliced, &truth, &label);
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] A file that appears
// mid-session must join the corpus exactly as a fresh cold pass over the
// grown tree sees it: the cluster grows from trio to quad, and every
// span, id, signal, ranking position and metric matches field for field.
#[test]
fn a_file_added_mid_session_matches_the_cold_report_of_the_grown_tree() -> Result<()> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    assert_added_carrier_matches_cold(
        guard.path(),
        &root,
        ("dup_d.rs", DELTA_BANNER),
        &DUPLICATE_QUAD,
    )
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] + [PIPELINE-DETERMINISM]
// The same add, with a name that sorts *ahead* of every file already in
// the corpus. `dup_d.rs` sorts last, so a splice that appends instead of
// inserting at the file's sort position lands it correctly by luck;
// `aa_dup.rs` does not. The corpus store holds one span per file in
// ascending workspace-relative-path order and a render borrows those
// slices as they are, so an appended span renders its occurrence — and
// the `summary` line built from it — out of order while every other
// reading stays identical.
#[test]
fn an_early_sorting_add_splices_into_path_order_not_arrival_order() -> Result<()> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    assert_added_carrier_matches_cold(
        guard.path(),
        &root,
        (EARLY_CARRIER, EARLY_BANNER),
        &DUPLICATE_QUAD_EARLY,
    )
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Overwriting a live clone
// carrier mid-session must evict its old fingerprints, signatures,
// boilerplate ranges and line count along with them. A splice that
// replaced only some of those would keep reporting the occurrence the
// file no longer holds — a false positive the cold pass proves is not
// there.
#[test]
fn a_file_edited_mid_session_matches_the_cold_report_of_the_edited_tree() -> Result<()> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    let carrier = root.join("dup_c.rs");
    let spec = staged_spec(guard.path(), "staged_c.rs", REPLACEMENT_FN, &carrier)?;
    let baseline = run(&root, true)?;
    let spliced = spliced_session(&root, &[("--rerun-add", spec)])?;
    assert_splice_moved_the_report(&spliced, &baseline, "live session: file edit");
    let truth = cold_truth(&corpus_with_dup_c("dup_c.rs", REPLACEMENT_FN))?;
    assert_report_shape(&spliced, 5, &DUPLICATE_PAIR, "spliced edit")?;
    assert_report_shape(&truth, 5, &DUPLICATE_PAIR, "edited ground truth")?;
    assert_reports_equal(&spliced, &truth, "live session: file edit");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] A file deleted mid-session
// must leave the corpus entirely — occurrences, per-file metric row and
// analysed-file count together. `drop_path` closes the file's span in
// every flat vector, so a mis-sized close would mis-attribute a
// surviving file's signatures and manufacture a cluster from nothing.
#[test]
fn a_file_removed_mid_session_matches_the_cold_report_of_the_shrunk_tree() -> Result<()> {
    let (_guard, root) = seeded_scan_root(&corpus())?;
    let removed = root.join("dup_c.rs");
    let spliced = spliced_session(&root, &[("--rerun-remove", OsString::from(&removed))])?;
    let truth = cold_truth(&corpus_without_dup_c())?;
    assert_report_shape(&spliced, 4, &DUPLICATE_PAIR, "spliced remove")?;
    assert_report_shape(&truth, 4, &DUPLICATE_PAIR, "shrunk ground truth")?;
    assert_reported_path_count(
        &spliced,
        "dup_c.rs",
        0,
        "a file removed mid-session must vanish from every reported path",
    );
    assert_reports_equal(&spliced, &truth, "live session: file remove");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] The composition, in one
// pass: a file arrives, another is rewritten out of the cluster, a third
// is deleted. Each splice reorders the flat store's per-file spans
// relative to the last, so this is the scenario a splice that got its
// offsets right only for the append case fails. The trio becomes
// `dup_a`, `dup_b`, `dup_d`.
#[test]
fn add_edit_and_remove_in_one_pass_match_the_cold_report_of_the_result() -> Result<()> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    let added = root.join("dup_d.rs");
    let carrier = root.join("dup_c.rs");
    let removed = root.join("filler_bounds.rs");
    let baseline = run(&root, true)?;
    let spliced = spliced_session(
        &root,
        &[
            (
                "--rerun-add",
                staged_spec(
                    guard.path(),
                    "staged_d.rs",
                    &dup_source(DELTA_BANNER),
                    &added,
                )?,
            ),
            (
                "--rerun-add",
                staged_spec(guard.path(), "staged_c.rs", REPLACEMENT_FN, &carrier)?,
            ),
            ("--rerun-remove", OsString::from(&removed)),
        ],
    )?;
    assert_splice_moved_the_report(&spliced, &baseline, "live session: compound change");
    let mut expected: Vec<(String, String)> = corpus_with_dup_c("dup_c.rs", REPLACEMENT_FN)
        .into_iter()
        .filter(|(name, _)| name != "filler_bounds.rs")
        .collect();
    expected.push(("dup_d.rs".to_owned(), dup_source(DELTA_BANNER)));
    let truth = cold_truth(&expected)?;
    let survivors = ["dup_a.rs", "dup_b.rs", "dup_d.rs"];
    assert_report_shape(&spliced, 5, &survivors, "spliced compound change")?;
    assert_report_shape(&truth, 5, &survivors, "compound ground truth")?;
    assert_reports_equal(&spliced, &truth, "live session: add + edit + remove");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Reverting inside one
// session is the content-addressed path the batch suite pins across
// processes: the session splices the carrier out and back, and the
// second splice must land on the baseline report exactly. A session that
// carried any per-file state forward from the intermediate state — a
// stale boilerplate range, a leftover line count, a fingerprint span
// that was never closed — diverges here even though the tree on disk is
// byte-identical to where it started.
#[test]
fn editing_and_reverting_inside_one_session_lands_on_the_baseline_report() -> Result<()> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    let carrier = root.join("dup_c.rs");
    let original = dup_source(GAMMA_BANNER);
    let baseline = run(&root, true)?;
    let spliced = spliced_session(
        &root,
        &[
            (
                "--rerun-add",
                staged_spec(guard.path(), "staged_edit.rs", REPLACEMENT_FN, &carrier)?,
            ),
            (
                "--rerun-add",
                staged_spec(guard.path(), "staged_revert.rs", &original, &carrier)?,
            ),
        ],
    )?;
    assert_report_shape(&spliced, 5, &DUPLICATE_TRIO, "spliced revert")?;
    assert_reports_equal(
        &spliced,
        &baseline,
        "live session: edit then revert vs the baseline report",
    );
    Ok(())
}
