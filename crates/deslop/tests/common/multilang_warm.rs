//! Warm-store scenarios over the `incremental-multilang` fixture
//! ([PIPELINE-INCREMENTAL], [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
//!
//! Separated from the fixture vocabulary in [`super::multilang`] because
//! the two answer different questions: that module says what the fixture
//! *is* and what each language must render, while this one drives the
//! parse store through a warm baseline, a targeted mutation, and the
//! reuse accounting that mutation must produce.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{
    incremental::{
        assert_pass, assert_reports_equal, cold_then_warm, run_report_with_store, run_store_on,
        ColdThenWarm, ReuseCounters, Store,
    },
    multilang::{
        assert_multilang_contract, expect_lang_clone, seed_multilang, MULTILANG_CASES,
        MULTILANG_FILE_COUNT, MULTILANG_MIN_NODES,
    },
    seed, Result,
};

/// A warm scan root: the fixture seeded into `<tmp>/src` and scanned
/// once with the store on, so every file has a persisted blob. Returns
/// the baseline report every later assertion in that test compares to.
pub(crate) struct WarmCorpus {
    /// Owns the temp directory for the lifetime of the scenario.
    tmp: tempfile::TempDir,
    /// The seeded, already-warmed scan root.
    scan_root: PathBuf,
    /// The cold and fully-warm reports the corpus renders before any
    /// mutation; `cycle.warm` is the baseline every scenario compares to.
    pub(crate) cycle: ColdThenWarm,
}

impl WarmCorpus {
    /// Seeds the fixture and warms the store, asserting the cold pass
    /// missed on all twelve files and the warm pass hit on all twelve —
    /// so a scenario can never start from a store that was quietly
    /// empty, and "the edit caused this miss" is provable.
    pub(crate) fn warm() -> Result<Self> {
        let tmp = tempfile::tempdir()?;
        let scan_root = tmp.path().join("src");
        seed_multilang(&scan_root)?;

        let cycle = cold_then_warm(&scan_root, tmp.path(), MIN, MULTILANG_FILE_COUNT)?;
        assert_multilang_contract(&cycle.warm, "warm baseline")?;

        Ok(Self {
            tmp,
            scan_root,
            cycle,
        })
    }

    /// The fully-warm report rendered before any mutation.
    pub(crate) fn baseline(&self) -> &Value {
        &self.cycle.warm
    }

    /// Runs the corpus again with the store on, into a scenario-specific
    /// output directory, and returns the report with its counters.
    pub(crate) fn rerun(&self, label: &str) -> Result<(Value, ReuseCounters)> {
        run_store_on(&self.scan_root, &self.tmp.path().join(label), MIN, &[])
    }

    /// The same corpus state rendered from scratch with the store off —
    /// the reference [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE] judges
    /// an incremental report against. Scanned in a pristine copy so the
    /// warm store cannot influence it.
    pub(crate) fn cold_reference(&self) -> Result<Value> {
        let fresh = self.tmp.path().join("reference-root");
        seed(&self.scan_root, &fresh)?;
        run_report_with_store(&fresh, MIN, Store::Off)
    }

    /// Absolute path of one fixture file in the scan root.
    pub(crate) fn file(&self, name: &str) -> PathBuf {
        self.scan_root.join(name)
    }

    /// Asserts a post-edit report still recognises all six authored
    /// clones, renders every language's cluster exactly as the warm
    /// baseline did, and owes the cold render of that very same corpus
    /// state ([PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
    pub(crate) fn assert_unmoved_and_equivalent(&self, report: &Value, label: &str) -> Result<()> {
        assert_multilang_contract(report, label)?;
        assert_every_language_cluster_unchanged(report, self.baseline(), label)?;
        assert_reports_equal(report, &self.cold_reference()?, label);
        Ok(())
    }

    /// Re-runs after exactly one file was touched and asserts the store
    /// invalidated exactly that one: eleven files served, one re-parsed,
    /// and the signature work split across both paths rather than
    /// collapsing onto either. Returns the report for the caller's own
    /// scenario assertions.
    pub(crate) fn rerun_after_one_touch(&self, out_dir: &str, label: &str) -> Result<Value> {
        let (report, events) = self.rerun(out_dir)?;
        assert_pass(&report, &events, MULTILANG_FILE_COUNT - 1, 1, label);
        assert!(
            events.signatures_built > 0,
            "{label}: the re-parsed file must rebuild its own signatures: {events:?}"
        );
        assert!(
            events.signatures_reused > 0,
            "{label}: the {} untouched files must still reuse theirs: {events:?}",
            MULTILANG_FILE_COUNT - 1
        );
        Ok(report)
    }
}

/// Shorthand for the fixture's pinned subtree floor.
pub(crate) const MIN: u32 = MULTILANG_MIN_NODES;

/// A body that parses under both the TypeScript and the JavaScript
/// grammar, so a `twin.ts`/`twin.js` pair can be byte-identical. Shared
/// by the addressing test (`incremental_multilang_matrix.rs`) and the
/// cross-partition blob-integrity test (`cache_blob_integrity.rs`) —
/// both exist to prove the store's language component is load-bearing.
pub(crate) const TWIN_SOURCE: &str = "export function reconcileEntries(entries, floor) {\n\
    \x20 let balance = 0;\n\
    \x20 for (const entry of entries) {\n\
    \x20   if (entry > floor) {\n\
    \x20     balance += entry * 2;\n\
    \x20   } else {\n\
    \x20     balance -= Math.trunc(entry / 2);\n\
    \x20   }\n\
    \x20 }\n\
    \x20 return balance;\n\
}\n";

/// Seeds `scan_root` with the byte-identical `twin.ts`/`twin.js` pair
/// and proves the two really are byte-identical — without that the
/// language component of the store key is not under test at all.
pub(crate) fn seed_twins(scan_root: &Path) -> Result<()> {
    fs::create_dir_all(scan_root)?;
    fs::write(scan_root.join("twin.ts"), TWIN_SOURCE)?;
    fs::write(scan_root.join("twin.js"), TWIN_SOURCE)?;
    assert_eq!(
        fs::read(scan_root.join("twin.ts"))?,
        fs::read(scan_root.join("twin.js"))?,
        "the two twins must be byte-identical for the key to be under test at all"
    );
    Ok(())
}

/// Appends one newline, returning the file's original bytes. A trailing
/// newline changes the raw bytes — and so the blake3 store key — without
/// moving a single byte offset inside the file, which is what lets the
/// caller demand a cache miss *and* an unchanged cluster in the same
/// breath.
pub(crate) fn append_newline(path: &Path) -> Result<Vec<u8>> {
    let original = fs::read(path)?;
    let mut touched = original.clone();
    touched.push(b'\n');
    fs::write(path, &touched)?;
    Ok(original)
}

/// Asserts every language's cluster is field-for-field what it was in
/// `baseline` — including the language that was touched.
pub(crate) fn assert_every_language_cluster_unchanged(
    report: &Value,
    baseline: &Value,
    label: &str,
) -> Result<()> {
    for case in MULTILANG_CASES {
        let language = case.language;
        assert_eq!(
            expect_lang_clone(report, case)?,
            expect_lang_clone(baseline, case)?,
            "{label}: the {language} cluster changed under an edit that moved no \
             byte offsets — a re-parse must reproduce exactly what the store held"
        );
    }
    Ok(())
}
