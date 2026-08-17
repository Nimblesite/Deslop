//! The `incremental-multilang` fixture: six languages, one authored
//! Type-1 clone pair each, scanned as a single corpus
//! ([PIPELINE-INCREMENTAL], [PIPELINE-INCREMENTAL-ANALYSIS-EQUIVALENCE]).
//!
//! The parse store keys on `(language_id, tool_version, min_nodes,
//! blake3(source))`, so a mixed-language corpus is the only shape that
//! can expose a store that leaks one language's tree into another's
//! slot. This module owns the fixture's vocabulary — which files pair
//! up, what each pair must render as, and how to isolate one language's
//! clusters from the rest — so the golden suite and the per-language
//! invalidation matrix can never disagree about what the fixture means.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{
    cluster_bucket, cluster_size, clusters, expect_cluster_spanning, field, fixture,
    incremental::{
        assert_pass, assert_reports_equal, cold_then_warm, run_report_with_store, run_store_on,
        ColdThenWarm, ReuseCounters, Store,
    },
    occurrence_files, seed, Result,
};

/// Subtree-size floor the fixture is scanned at. Every authored clone
/// measures 35–52 nodes, so 20 keeps all six clusters. It must sit
/// *above* 13: at lower floors the C# pair renders a second `identical`
/// cluster — a 13-node sibling window over the method's signature line
/// that starts at `public` while the method view starts at `static`,
/// straddling [PIPELINE-CLUSTER-SUBSUME] containment by 7 bytes
/// (gh #389). This fixture's subject is the parse store, not
/// subsumption, so the floor keeps that edge out of every report here.
pub(crate) const MULTILANG_MIN_NODES: u32 = 20;

/// Source files in the fixture — two per language, all byte-distinct.
pub(crate) const MULTILANG_FILE_COUNT: u64 = 12;

/// One occurrence's rendered location: `(start_line, end_line,
/// start_byte, end_byte)`. Every field is user-visible — a reader clicks
/// the line, an agent slices the bytes — so the golden pins all four
/// rather than merely proving a cluster exists.
pub(crate) type OccurrenceSpan = (u64, u64, u64, u64);

/// One language's authored clone pair, together with everything the
/// committed golden must report for it. Keeping the expectations beside
/// the file names means the golden suite and the invalidation matrix
/// read one table: a fixture edit that moves a span cannot be absorbed
/// by re-blessing while a second, stale table quietly disagrees.
pub(crate) struct LangCase {
    /// Language id, used only in assertion messages.
    pub(crate) language: &'static str,
    /// The file holding the canonical copy.
    pub(crate) alpha: &'static str,
    /// The file holding the pasted copy.
    pub(crate) beta: &'static str,
    /// Stable cluster id ([PIPELINE-DETERMINISM]). Ids travel into
    /// editor state, MCP `cluster-by-id` lookups and cross-run deltas,
    /// so a silent id change breaks every consumer holding one.
    pub(crate) cluster_id: &'static str,
    /// `canonical_node_count` of the authored clone — the subtree size
    /// ranking weight is computed from.
    pub(crate) nodes: u64,
    /// Where the canonical copy is reported.
    pub(crate) alpha_span: OccurrenceSpan,
    /// Where the pasted copy is reported.
    pub(crate) beta_span: OccurrenceSpan,
}

impl LangCase {
    /// The pair as the `&[&str]` the cluster lookups take.
    pub(crate) fn files(&self) -> [&'static str; 2] {
        [self.alpha, self.beta]
    }

    /// The `(file, span)` pairs the golden must report, in the fixture's
    /// canonical-then-pasted order.
    pub(crate) fn spans(&self) -> [(&'static str, OccurrenceSpan); 2] {
        [(self.alpha, self.alpha_span), (self.beta, self.beta_span)]
    }
}

/// Every language in the fixture. Each pair shares a byte-identical
/// `reconcile_entries` body and differs only in a leading banner comment
/// plus one structurally unique top-level item — so the pair clusters on
/// the shared body while the whole-file `__file__` nodes stay distinct
/// and no file-level cluster forms.
pub(crate) const MULTILANG_CASES: &[LangCase] = &[
    LangCase {
        language: "rust",
        alpha: "ledger_alpha.rs",
        beta: "ledger_beta.rs",
        cluster_id: "d8a38df1507e6efd",
        nodes: 45,
        alpha_span: (5, 15, 124, 381),
        beta_span: (7, 17, 131, 388),
    },
    LangCase {
        language: "python",
        alpha: "ledger_alpha.py",
        beta: "ledger_beta.py",
        cluster_id: "3b08286c43ec5193",
        nodes: 35,
        alpha_span: (6, 13, 109, 315),
        beta_span: (8, 15, 118, 324),
    },
    LangCase {
        language: "typescript",
        alpha: "ledger_alpha.ts",
        beta: "ledger_beta.ts",
        cluster_id: "75331bdf6bb59eea",
        nodes: 51,
        alpha_span: (5, 15, 127, 391),
        beta_span: (7, 17, 138, 402),
    },
    LangCase {
        language: "dart",
        alpha: "ledger_alpha.dart",
        beta: "ledger_beta.dart",
        cluster_id: "09ec87de54dfeffb",
        nodes: 50,
        alpha_span: (5, 15, 121, 350),
        beta_span: (7, 17, 123, 352),
    },
    LangCase {
        language: "csharp",
        alpha: "LedgerAlpha.cs",
        beta: "LedgerBeta.cs",
        cluster_id: "71c21f540b600f72",
        nodes: 44,
        alpha_span: (9, 24, 180, 537),
        beta_span: (9, 24, 187, 544),
    },
    LangCase {
        language: "go",
        alpha: "ledger_alpha.go",
        beta: "ledger_beta.go",
        cluster_id: "7e9099352ffa58f5",
        nodes: 52,
        alpha_span: (7, 17, 125, 345),
        beta_span: (9, 19, 135, 355),
    },
];

/// Every authored clone is a byte-identical Type-1 copy with embeddings
/// off, so all four signals are pinned to exact values — no bands, no
/// approximation. `token_jaccard` is the load-bearing one: the audit's
/// corrupted-signature regression surfaced precisely as this value
/// moving while every other field held ([PIPELINE-INCREMENTAL-INTEGRITY]).
pub(crate) const MULTILANG_SIGNALS: &[(&str, f64)] = &[
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("embedding_cos", 0.0),
    ("fused", 1.0),
];

/// `tests/fixtures/incremental-multilang`.
pub(crate) fn multilang_dir() -> PathBuf {
    fixture("incremental-multilang")
}

/// The authored corpus the golden describes.
pub(crate) fn multilang_corpus() -> PathBuf {
    multilang_dir().join("src")
}

/// The committed cold-report golden.
pub(crate) fn multilang_golden_path() -> PathBuf {
    multilang_dir().join("expected-report.json")
}

/// Copies the fixture corpus into `scan_root`. The checked-in directory
/// is never scanned in place — a store-on run writes `.deslop/cache`
/// into its scan root ([OUTPUT-DIR]), which must never land in the
/// fixture tree.
pub(crate) fn seed_multilang(scan_root: &Path) -> Result<()> {
    seed(&multilang_corpus(), scan_root)
}

/// The cluster spanning one language's pair, failing with the whole
/// report when that language went undetected.
pub(crate) fn expect_lang_clone<'a>(report: &'a Value, case: &LangCase) -> Result<&'a Value> {
    expect_cluster_spanning(report, &case.files())
}

/// Asserts the corpus really carries all six authored clones: every
/// language contributes exactly one `identical` cluster spanning exactly
/// its own two files, and the report analysed all twelve sources.
///
/// This is the recall floor every incremental assertion stands on — a
/// warm run that "reproduces" a report in which a language vanished
/// would otherwise pass every equivalence check.
pub(crate) fn assert_multilang_contract(report: &Value, label: &str) -> Result<()> {
    assert_eq!(
        field(report, "files_analysed").as_u64(),
        Some(MULTILANG_FILE_COUNT),
        "{label}: every seeded source file must be analysed: {report:#}"
    );
    for case in MULTILANG_CASES {
        let language = case.language;
        let clone = expect_lang_clone(report, case)?;
        assert_eq!(
            cluster_bucket(clone),
            "identical",
            "{label}/{language}: the authored pair is a byte-identical body \
             in two distinct files: {report:#}"
        );
        assert_eq!(
            cluster_size(clone),
            2,
            "{label}/{language}: the clone must span exactly the two authored \
             occurrences: {report:#}"
        );
        let mut files = occurrence_files(clone);
        files.sort();
        let mut expected = case.files().map(ToOwned::to_owned).to_vec();
        expected.sort();
        assert_eq!(
            files, expected,
            "{label}/{language}: the clone must span that language's own two \
             files and nothing else — a cross-language occurrence here means \
             the store served one language's tree for another: {report:#}"
        );
    }
    assert_eq!(
        lang_of_every_cluster(report)?.len(),
        MULTILANG_CASES.len(),
        "{label}: the fixture authors exactly one clone per language, so the \
         report must carry exactly {} clusters: {report:#}",
        MULTILANG_CASES.len()
    );
    Ok(())
}

/// The language each reported cluster belongs to, in report order.
/// Errors when a cluster spans files from more than one language (or
/// none) — that shape is only reachable through a corrupted store, and
/// naming it here keeps the failure legible.
pub(crate) fn lang_of_every_cluster(report: &Value) -> Result<Vec<&'static str>> {
    clusters(report)
        .iter()
        .map(|cluster| {
            let files = occurrence_files(cluster);
            let owners: Vec<&'static str> = MULTILANG_CASES
                .iter()
                .filter(|case| {
                    files
                        .iter()
                        .any(|name| case.files().contains(&name.as_str()))
                })
                .map(|case| case.language)
                .collect();
            match owners.as_slice() {
                [only] => Ok(*only),
                other => Err(anyhow::anyhow!(
                    "cluster spans {other:?} languages ({files:?}); every authored \
                     clone lives in exactly one language: {cluster:#}"
                )),
            }
        })
        .collect()
}

/// The cluster of every language *except* `excluded`, keyed by language
/// so two runs can be compared entry by entry regardless of how the
/// clusters were ranked.
///
/// Editing one language's file shifts that language's spans and the
/// repo-level metrics; it must leave every other language's cluster
/// byte-identical. Comparing this projection across the two runs is how
/// that is proven.
pub(crate) fn other_language_clusters<'a>(
    report: &'a Value,
    excluded: &LangCase,
) -> Result<Vec<(&'static str, &'a Value)>> {
    MULTILANG_CASES
        .iter()
        .filter(|case| case.language != excluded.language)
        .map(|case| Ok((case.language, expect_lang_clone(report, case)?)))
        .collect()
}

/// Asserts every language other than `excluded` renders exactly the
/// cluster it rendered in `baseline` — same bucket, size, weight,
/// signals, spans and ids, field for field.
pub(crate) fn assert_other_languages_unchanged(
    report: &Value,
    baseline: &Value,
    excluded: &LangCase,
    label: &str,
) -> Result<()> {
    let actual = other_language_clusters(report, excluded)?;
    let expected = other_language_clusters(baseline, excluded)?;
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: touching {} must not change how many other languages \
         report a clone",
        excluded.language
    );
    for ((language, actual), (baseline_language, expected)) in actual.iter().zip(&expected) {
        assert_eq!(
            language, baseline_language,
            "{label}: language walk desynced"
        );
        assert_eq!(
            actual, expected,
            "{label}: editing {} changed the {language} cluster — one \
             language's cache invalidation must never disturb another's \
             ([PIPELINE-INCREMENTAL-INVALIDATION])\nafter: {actual:#}\nbefore: {expected:#}",
            excluded.language
        );
    }
    Ok(())
}

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
