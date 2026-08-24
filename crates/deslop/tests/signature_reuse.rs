//! Warm runs must reuse persisted `MinHash` signatures instead of
//! rebuilding every one from token streams
//! ([PIPELINE-INCREMENTAL-ANALYSIS-REUSE]).
//!
//! A signature is a pure function of one subtree's normalised token
//! k-grams, so a fully-warm pass may rebuild none of them. The pass
//! must say so on the structured tracing surface — timing assertions
//! are banned — and the reuse can never be bought with a report
//! difference: outside `cache_stats`, the warm report IS the cold
//! report.
//!
//! Contract, per `fingerprint corpus built` event: cold default run →
//! `signatures_reused=0`, `signatures_built=F` where F is the total
//! fingerprint count; fully-warm run → `signatures_built=0`,
//! `signatures_reused=F`; a disabled store (`--no-incremental` or the
//! config opt-out) → `signatures_built=F`, `signatures_reused=0` with
//! zero hits *and* zero misses, because a store that is never consulted
//! accounts for no file at all. Conservation (`built + reused == F`)
//! holds on every pass and is asserted on every pass.
//!
//! The event-parsing and report-comparison helpers live in
//! `common::incremental`, the corpus in `common::seeded` — shared with
//! `cache_blob_integrity.rs`, so there is one definition of "reused,
//! not rebuilt" across the tree.

use std::{fs, path::Path};

use crate::common::{incremental::*, seeded::*, store::*, *};

// [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] A fully-warm pass rebuilds no
// signature, attaches one per fingerprint from the parse store, and
// renders the cold report unchanged.
#[test]
fn warm_run_reuses_persisted_signatures_instead_of_rebuilding() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_corpus(&scan_root)?;

    // The whole cold-fills / warm-serves / warm-owes-cold contract, over
    // the three seeded files.
    let cycle = cold_then_warm(&scan_root, tmp.path(), SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;

    // And it was a real corpus on both passes, not a blind one.
    assert_seeded_corpus(&cycle.cold, "cold")?;
    assert_seeded_corpus(&cycle.warm, "warm")?;
    Ok(())
}

// [PIPELINE-INCREMENTAL-ANALYSIS-REUSE] `--no-incremental` never
// consults the store, so it must build every signature on every pass —
// twice over, proving the opt-out cannot silently start reusing — while
// rendering the same report the store-backed passes render. A disabled
// store records zero hits and zero misses; the store-on conservation
// rule does not apply to it.
/// Runs the same disabled-store invocation twice, each pass into its
/// own out dir, and asserts the whole disabled contract on both: exact
/// `{0, 0}` on both cache surfaces, every signature built, identical
/// fingerprint counts, a real seeded corpus, and the second report
/// field-for-field equal to the first. Returns the first pass so the
/// caller can compare it onward.
fn assert_two_disabled_passes(
    scan_root: &Path,
    out_root: &Path,
    extra_args: &[&str],
    scenario: &str,
) -> Result<(serde_json::Value, ReuseCounters)> {
    let first_out = out_root.join("first");
    let (first, first_events) = run_store_on(scan_root, &first_out, SEEDED_MIN_NODES, extra_args)?;
    let second_out = out_root.join("second");
    let (second, second_events) =
        run_store_on(scan_root, &second_out, SEEDED_MIN_NODES, extra_args)?;
    assert_seeded_corpus(&first, scenario)?;
    for (label, events) in [("first", &first_events), ("second", &second_events)] {
        events.assert_invariants(&format!("{scenario} {label}"));
        events.assert_store_disabled(&format!("{scenario} {label}"));
    }
    assert_cache_stats(&first, 0, 0, scenario);
    assert_cache_stats(&second, 0, 0, scenario);
    assert_eq!(
        first_events.fingerprints, second_events.fingerprints,
        "{scenario}: two disabled passes over one corpus must fingerprint \
         identically: {first_events:?} vs {second_events:?}"
    );
    assert_reports_equal(&second, &first, scenario);
    Ok((first, first_events))
}

#[test]
fn no_incremental_runs_always_build_every_signature() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_corpus(&scan_root)?;

    let (first, first_events) = assert_two_disabled_passes(
        &scan_root,
        tmp.path(),
        &["--no-incremental"],
        "--no-incremental",
    )?;

    // And the store-off report is the store-on report.
    let (warm, warm_events) =
        run_store_on(&scan_root, &tmp.path().join("warm"), SEEDED_MIN_NODES, &[])?;
    warm_events.assert_invariants("store-on after store-off");
    assert_eq!(
        warm_events.fingerprints, first_events.fingerprints,
        "the store must not change how many subtrees are fingerprinted: \
         {warm_events:?} vs {first_events:?}"
    );
    assert_reports_equal(&warm, &first, "store-on pass vs --no-incremental pass");
    Ok(())
}

// [CONFIG-INCREMENTAL-OPTOUT] `[analysis] incremental = false` in
// `.deslop.toml` is the config-file escape hatch: it disables persisted
// processing with no per-invocation flag, for every surface that loads
// the config. Two default-flag runs must both behave exactly like
// `--no-incremental` — zero hits, zero misses, every signature built —
// and the store directory must never be created on disk.
#[test]
fn config_file_opt_out_disables_persisted_processing() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_corpus(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[analysis]\nincremental = false\n",
    )?;

    // No per-invocation flag at all — the config alone must disable the
    // store, to exactly the contract `--no-incremental` satisfies.
    let (opted_out, _events) =
        assert_two_disabled_passes(&scan_root, tmp.path(), &[], "config-opt-out")?;
    assert!(
        !store_dir(&scan_root).exists(),
        "an opted-out run must never create the parse store on disk"
    );

    // The two opt-out spellings are one behaviour, so they owe the same
    // report — otherwise the escape hatch is a second analysis path.
    let flagged = run_report_with_store(&scan_root, SEEDED_MIN_NODES, Store::Off)?;
    assert_reports_equal(
        &opted_out,
        &flagged,
        "config opt-out vs --no-incremental over one corpus",
    );
    Ok(())
}

// [CONFIG-INCREMENTAL-OPTOUT] The opt-out must ignore a store that is
// already warm, not merely decline to create one. A run that consulted
// an existing store while claiming to be opted out would serve
// persisted analysis the operator explicitly turned off — and the "never
// creates it" assertion above cannot see that, because the directory is
// already there.
//
// The store is filled and proven warm first, so the opted-out passes run
// against three valid, hit-ready blobs. They must report zero hits and
// zero misses, build every signature, leave the blobs byte-for-byte
// untouched — neither read nor rewritten — and still render the report
// the warm pass rendered.
#[test]
fn config_opt_out_ignores_an_already_warm_store() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_corpus(&scan_root)?;

    let cycle = cold_then_warm(&scan_root, tmp.path(), SEEDED_MIN_NODES, SEEDED_FILE_COUNT)?;
    assert_seeded_corpus(&cycle.warm, "warm before opt-out")?;
    let warm_blobs = blob_bytes(&scan_root)?;
    assert_eq!(
        warm_blobs.len() as u64,
        SEEDED_FILE_COUNT,
        "the warm store must hold one blob per file before the opt-out"
    );

    fs::write(
        scan_root.join(".deslop.toml"),
        "[analysis]\nincremental = false\n",
    )?;
    let (opted_out, _events) = assert_two_disabled_passes(
        &scan_root,
        &tmp.path().join("after"),
        &[],
        "warm-then-opt-out",
    )?;

    assert_eq!(
        blob_bytes(&scan_root)?,
        warm_blobs,
        "an opted-out pass must neither consult nor rewrite the blobs that \
         were already on disk"
    );
    assert_reports_equal(
        &opted_out,
        &cycle.warm,
        "config opt-out over a warm store vs the store-backed warm pass",
    );
    Ok(())
}
