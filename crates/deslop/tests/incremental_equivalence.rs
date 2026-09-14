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

use std::{fs, path::PathBuf};

use serde_json::Value;

use crate::common::{clone_corpus::*, incremental::*, *};

/// Seeds a fresh root with [`corpus`], then asserts the baseline cold
/// run fills the store ({hits: 0, misses: 5}) and the warm run serves
/// it in full ({hits: 5, misses: 0}).
fn seeded_warm_root() -> Result<(tempfile::TempDir, PathBuf, Value, Value)> {
    let (guard, root) = seeded_scan_root(&corpus())?;
    let cold = run(&root, true)?;
    assert_cache_stats(&cold, 0, 5, "baseline cold");
    let warm = run(&root, true)?;
    assert_cache_stats(&warm, 5, 0, "baseline warm");
    Ok((guard, root, cold, warm))
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
    let truth = cold_truth(&corpus_with_dup_d())?;
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
    let truth = cold_truth(&corpus_without_dup_c())?;
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
    assert_reported_path_count(
        &incremental,
        "dup_moved.rs",
        2,
        "the renamed file must appear as one occurrence and one per-file metric row",
    );
    assert_reported_path_count(
        &incremental,
        "dup_c.rs",
        0,
        "the old name must vanish from every reported path",
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
