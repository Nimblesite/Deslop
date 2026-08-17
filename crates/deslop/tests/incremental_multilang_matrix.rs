//! Per-language invalidation matrix over the six-language fixture
//! ([PIPELINE-INCREMENTAL-INVALIDATION],
//! [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE],
//! [PIPELINE-INCREMENTAL-ANALYSIS-ADDRESSING]).
//!
//! `incremental_multilang_golden.rs` pins what one cold and one fully
//! warm pass render. This suite drives the states *between* them, once
//! per language: touch one file, delete one file, revert one file, and
//! collide two parsers on identical bytes.
//!
//! The defect class these exist for is silent and cross-cutting. The
//! parse store is content-addressed per language, so a key that lost its
//! language component, an invalidation that swept too wide, or a hit
//! path that served a stale tree would all keep rendering a plausible
//! report — with one language's clone attributed to another's subtree, a
//! false positive and a false negative in the same pass. Every assertion
//! here therefore pins two things at once: the exact store accounting
//! (`cache_stats` and the `fingerprint corpus built` counters), and the
//! rendered clusters of the languages that were *not* touched.
//!
//! Timing is never asserted — reuse is proven by counters, never by a
//! stopwatch.

use std::fs;

mod common;
use crate::common::{incremental::*, multilang::*, *};

// [PIPELINE-INCREMENTAL-INVALIDATION] Touching one file in one language
// must invalidate exactly that file's store entry: the other eleven
// still hit, and because the edit appends at end-of-file no span moves,
// so all six clusters must render byte-identically to the warm baseline.
// A store that keyed on anything coarser than the file would invalidate
// siblings and show up as extra misses; one that keyed too loosely would
// serve the stale blob and show up as a twelfth hit.
#[test]
fn touching_one_language_invalidates_exactly_one_store_entry() -> Result<()> {
    for case in MULTILANG_CASES {
        let language = case.language;
        let corpus = WarmCorpus::warm()?;
        let original = append_newline(&corpus.file(case.alpha))?;

        let label = format!("touched {language}");
        let report = corpus.rerun_after_one_touch(&format!("touch-{language}"), &label)?;

        // The corpus is still fully recognised, nothing moved, and the
        // partially-warm report owes the cold render of this same state.
        corpus.assert_unmoved_and_equivalent(&report, &label)?;
        assert_other_languages_unchanged(&report, corpus.baseline(), case, &label)?;

        // The original blob was never evicted: restoring the bytes
        // restores a full-hit pass ([PIPELINE-INCREMENTAL-ANALYSIS-ADDRESSING] —
        // the store is content-addressed, so reverting is a hit, not a
        // re-parse).
        fs::write(corpus.file(case.alpha), &original)?;
        let (reverted, revert_events) = corpus.rerun(&format!("revert-{language}"))?;
        let revert_label = format!("reverted {language}");
        assert_warm_pass(
            &reverted,
            &revert_events,
            MULTILANG_FILE_COUNT,
            &revert_label,
        );
        assert_reports_equal(&reverted, corpus.baseline(), &revert_label);
    }
    Ok(())
}

// [PIPELINE-INCREMENTAL-INVALIDATION] Deleting one language's second
// copy must remove exactly that language's cluster. The eleven survivors
// all still hit — a deletion invalidates nothing — and the five
// untouched languages must render identically, so a delete can never
// perturb a neighbour's attribution.
#[test]
fn deleting_one_language_leaves_every_other_language_intact() -> Result<()> {
    for case in MULTILANG_CASES {
        let language = case.language;
        let corpus = WarmCorpus::warm()?;
        fs::remove_file(corpus.file(case.beta))?;

        let (report, events) = corpus.rerun(&format!("delete-{language}"))?;
        let label = format!("deleted {language} beta");

        assert_eq!(
            field(&report, "files_analysed").as_u64(),
            Some(MULTILANG_FILE_COUNT - 1),
            "{label}: one file was removed: {report:#}"
        );
        assert_warm_pass(&report, &events, MULTILANG_FILE_COUNT - 1, &label);

        // The orphaned language reports nothing at all — neither the
        // surviving copy alone, nor a stale pairing against the deleted
        // file's cached tree.
        assert!(
            cluster_spanning(&report, &case.files()).is_none(),
            "{label}: the pair cannot cluster once half of it is gone: {report:#}"
        );
        for cluster in clusters(&report) {
            assert!(
                !occurrence_files(cluster)
                    .iter()
                    .any(|name| name == case.beta),
                "{label}: a deleted file must not survive in the report via its \
                 cached blob — that is a pure false positive: {cluster:#}"
            );
            assert!(
                !occurrence_files(cluster)
                    .iter()
                    .any(|name| name == case.alpha),
                "{label}: the surviving copy has no partner and must not cluster \
                 with itself: {cluster:#}"
            );
        }
        assert_eq!(
            lang_of_every_cluster(&report)?.len(),
            MULTILANG_CASES.len() - 1,
            "{label}: exactly the five untouched languages remain: {report:#}"
        );

        assert_other_languages_unchanged(&report, corpus.baseline(), case, &label)?;
        assert_reports_equal(&report, &corpus.cold_reference()?, &label);
    }
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-ADDRESSING] The store key carries the language
// id, not just the source hash. Two byte-identical files routed to
// different parsers must therefore occupy two entries: both miss on the
// cold pass, both hit on the warm one.
//
// This is the assertion that fails loudly if the language component is
// ever dropped from the key. Without it the second file would be served
// the first parser's tree — silently, with a plausible report — and
// every downstream fingerprint for that file would describe a grammar it
// was never parsed under.
#[test]
fn identical_bytes_under_two_parsers_never_share_a_store_entry() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let (scan_root, cold, cold_events) = seeded_cold_pass(tmp.path(), seed_twins, MIN, 2)?;

    let (warm, warm_events) = run_store_on(&scan_root, &tmp.path().join("warm"), MIN, &[])?;
    assert_warm_pass(&warm, &warm_events, 2, "twin warm");
    assert_eq!(
        warm_events.fingerprints, cold_events.fingerprints,
        "two entries in, two entries out: {warm_events:?} vs {cold_events:?}"
    );
    assert_reports_equal(&warm, &cold, "twin-extension warm vs cold");
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] Every language edited in
// turn, cumulatively, without ever re-warming from scratch. After each
// step the store holds a mixture of fresh and long-standing blobs, and
// the report still owes a cold render of that exact state. A drift that
// only appears after several deltas — an accumulating store, a
// signature list that slips out of alignment by one — surfaces here and
// nowhere else.
#[test]
fn a_chain_of_per_language_edits_stays_equivalent_at_every_step() -> Result<()> {
    let corpus = WarmCorpus::warm()?;
    let mut expected_misses = 0_u64;

    for (step, case) in MULTILANG_CASES.iter().enumerate() {
        let language = case.language;
        let _original = append_newline(&corpus.file(case.alpha))?;
        expected_misses += 1;

        // Each step invalidates one more file; every previously edited
        // file must hit again from the blob its own edit wrote.
        let label = format!("chain step {step} ({language})");
        let report = corpus.rerun_after_one_touch(&format!("chain-{step}-{language}"), &label)?;

        corpus.assert_unmoved_and_equivalent(&report, &label)?;
    }

    assert_eq!(
        expected_misses,
        MULTILANG_CASES.len() as u64,
        "every language must have been edited exactly once"
    );
    Ok(())
}
