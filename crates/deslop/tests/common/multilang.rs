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

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    cluster_bucket, cluster_size, clusters, expect_cluster_spanning, field, fixture,
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
    /// One row of [`MULTILANG_CASES`], positional by design. Six
    /// languages each respelling the same seven field names cost 45
    /// redundant lines that Deslop scored `structural_only` against
    /// this repo's own corpus; the field docs above stay the single
    /// place a reader learns what each position means.
    const fn row(
        language: &'static str,
        alpha: &'static str,
        beta: &'static str,
        cluster_id: &'static str,
        nodes: u64,
        alpha_span: OccurrenceSpan,
        beta_span: OccurrenceSpan,
    ) -> Self {
        Self {
            language,
            alpha,
            beta,
            cluster_id,
            nodes,
            alpha_span,
            beta_span,
        }
    }

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
    LangCase::row(
        "rust",
        "ledger_alpha.rs",
        "ledger_beta.rs",
        "d8a38df1507e6efd",
        45,
        (5, 15, 124, 381),
        (7, 17, 131, 388),
    ),
    LangCase::row(
        "python",
        "ledger_alpha.py",
        "ledger_beta.py",
        "3b08286c43ec5193",
        35,
        (6, 13, 109, 315),
        (8, 15, 118, 324),
    ),
    LangCase::row(
        "typescript",
        "ledger_alpha.ts",
        "ledger_beta.ts",
        "75331bdf6bb59eea",
        51,
        (5, 15, 127, 391),
        (7, 17, 138, 402),
    ),
    LangCase::row(
        "dart",
        "ledger_alpha.dart",
        "ledger_beta.dart",
        "09ec87de54dfeffb",
        50,
        (5, 15, 121, 350),
        (7, 17, 123, 352),
    ),
    LangCase::row(
        "csharp",
        "LedgerAlpha.cs",
        "LedgerBeta.cs",
        "f887f991dc1f4969",
        46,
        (9, 24, 173, 537),
        (9, 24, 180, 544),
    ),
    LangCase::row(
        "go",
        "ledger_alpha.go",
        "ledger_beta.go",
        "7e9099352ffa58f5",
        52,
        (7, 17, 125, 345),
        (9, 19, 135, 355),
    ),
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
            analysis_fields(actual),
            analysis_fields(expected),
            "{label}: editing {} changed the {language} cluster — one \
             language's cache invalidation must never disturb another's \
             ([PIPELINE-INCREMENTAL-INVALIDATION])\nafter: {actual:#}\nbefore: {expected:#}",
            excluded.language
        );
        assert_rank_states_report_position(report, actual, label, language);
    }
    Ok(())
}

/// The analysis half of a rendered cluster: everything except the two
/// fields that state a position in the *report* rather than a fact about
/// the code. Deleting another language's file legitimately renumbers the
/// report ([SEVERITY-BAND]), so `rank` and `rank_band` are asserted
/// separately — against the report the cluster is actually published in,
/// which is a stronger claim than "unchanged".
fn analysis_fields(cluster: &Value) -> Value {
    let mut copy = cluster.clone();
    if let Some(object) = copy.as_object_mut() {
        let _rank = object.remove("rank");
        let _band = object.remove("rank_band");
    }
    copy
}

/// A cluster's stated rank must be its actual position in the report it
/// is published in, and it must carry a band. A rank carried over from a
/// previous generation — the failure mode a client re-numbering locally
/// would hide — shows up here as an off-by-N.
fn assert_rank_states_report_position(
    report: &Value,
    cluster: &Value,
    label: &str,
    language: &str,
) {
    let id = field(cluster, "id").as_str().unwrap_or_default();
    let position = clusters(report)
        .iter()
        .position(|entry| field(entry, "id").as_str() == Some(id));
    let expected = position
        .map(|index| index.saturating_add(1))
        .and_then(|rank| u64::try_from(rank).ok());
    assert_eq!(
        field(cluster, "rank").as_u64(),
        expected,
        "{label}: the {language} cluster's rank must be its position in the \
         report it is published in, not one carried from an earlier generation"
    );
    assert!(
        !field(cluster, "rank_band")
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "{label}: every published cluster carries a severity band ([SEVERITY-BAND])"
    );
}
