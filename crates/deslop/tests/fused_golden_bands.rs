//! Golden bucket and evidence coverage for the final pair-scoped
//! contract ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE],
//! [FUSED-THRESHOLD], [FUSED-SCOPE]).
//!
//! `docs/plans/fused-score-followups.md` states the contract the report
//! has to satisfy: every rendered cluster carries the elected pair's
//! measured axes and its content evidence, with no cluster-level `fused`
//! and no fused band. Each `fused-golden-<language>` fixture directory
//! stages the same four real-world scenarios side by side so one report
//! exercises all of them:
//!
//! | files                             | scenario                                            | required verdict |
//! |-----------------------------------|-----------------------------------------------------|------------------|
//! | `verbatim_a` / `verbatim_b`       | byte-identical copy-paste (Type-1)                   | `identical`, all axes 1.0 |
//! | `rename_a` / `rename_b`           | maximal identifier rename, same logic (Type-2)       | `nearly_identical`, certified rename evidence |
//! | `rename_lean_a` / `rename_lean_b` | the same Type-2 rename with **one** ubiquitous literal | `nearly_identical`, certified rename evidence |
//! | `shape_*` (×4)                    | unrelated descriptors sharing only the AST shape     | never act-now, ranked last |
//!
//! The Type-2 rows are the load-bearing ones. A rename-only copy is the
//! textbook definition of a Type-2 clone and every clone detector must
//! report it. [FUSED-CONTENT-GATE] measures raw-content agreement over
//! *all* collapsed leaves — identifiers and literals pooled — so a
//! maximally renamed clone with few literals scores low agreement and is
//! indistinguishable from unrelated scaffolding. These fixtures keep the
//! maximal rename that the shipped rename-showcase fixtures were softened
//! away from, so the distinction is pinned rather than avoided.
//!
//! The lean pair holds the same verdict *below* the literal-anchor mass
//! the anchored pair carries: its rename evidence rests on Baker's
//! parameterized-match proof — repeated, bijectively consistent
//! identifier substitutions — with a single `0` literal contributing
//! almost nothing. Pinning both pairs keeps the verdict contract stated
//! across the anchor axis, not only at its comfortable end.
//!
//! Every scenario carries a distinct AST shape so transitive closure
//! cannot merge the three of them into one cluster.

use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::common::{signals::*, *};

/// Node floor for the golden corpora — matches the small-fixture value the
/// TypeScript/JS feature suites use so every scenario subtree qualifies.
const MIN_NODES: u32 = 12;

/// Both Type-2 scenarios, held to one identical verdict contract: the
/// anchored maximal rename (`rename`, literals on both sides of every
/// branch) and the lean maximal rename (`rename_lean`, one ubiquitous
/// literal). A contract asserted for only one of them is a contract
/// stated only above the anchor floor — the exact gap
/// `[REPAIR-RENAME-ANCHOR-MASS]` records as a shipped false negative.
const RENAME_STEMS: [&str; 2] = ["rename", "rename_lean"];

/// One per-language golden corpus staged under `tests/fixtures`.
#[derive(Debug)]
struct Corpus {
    /// Language id, used only in assertion messages.
    language: &'static str,
    /// Fixture directory name.
    dir: &'static str,
    /// Source-file extension the fixture uses.
    extension: &'static str,
}

impl Corpus {
    /// The bare fixture file name for a scenario stem.
    fn file(&self, stem: &str) -> String {
        format!("{stem}.{extension}", extension = self.extension)
    }

    /// Renders the corpus through the CLI at [`MIN_NODES`].
    fn report(&self) -> Result<Value> {
        run_report(&fixture(self.dir), MIN_NODES)
    }
}

/// Every language the golden buckets are pinned in.
const CORPORA: [Corpus; 6] = [
    Corpus {
        language: "csharp",
        dir: "fused-golden-csharp",
        extension: "cs",
    },
    Corpus {
        language: "python",
        dir: "fused-golden-python",
        extension: "py",
    },
    Corpus {
        language: "typescript",
        dir: "fused-golden-typescript",
        extension: "ts",
    },
    Corpus {
        language: "go",
        dir: "fused-golden-go",
        extension: "go",
    },
    Corpus {
        language: "rust",
        dir: "fused-golden-rust",
        extension: "rs",
    },
    Corpus {
        language: "php",
        dir: "fused-golden-php",
        extension: "php",
    },
];

/// The cluster covering both files of a two-file scenario.
fn scenario_cluster<'a>(report: &'a Value, corpus: &Corpus, stem: &str) -> Result<&'a Value> {
    let first = corpus.file(&format!("{stem}_a"));
    let second = corpus.file(&format!("{stem}_b"));
    expect_cluster_spanning(report, &[first.as_str(), second.as_str()])
}

/// Every visible cluster built exclusively from `shape_*` fixture files.
fn shape_only_clusters(report: &Value) -> Vec<&Value> {
    clusters(report)
        .iter()
        .filter(|cluster| {
            let files = cluster_file_set(cluster);
            !files.is_empty() && files.iter().all(|name| name.starts_with("shape_"))
        })
        .collect()
}

/// Scenario 1 — a byte-identical copy-paste must bucket `identical` with
/// every rendered axis and the pair's content evidence at 1.0.
fn assert_verbatim_contract(report: &Value, corpus: &Corpus, root: &Path) -> Result<()> {
    let cluster = scenario_cluster(report, corpus, "verbatim")?;
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "{language}: a byte-identical copy-paste must bucket `identical` — {dump}"
    );
    assert_verbatim_components(cluster, corpus);
    assert_eq!(
        distinct_texts(root, cluster)?.len(),
        1,
        "{language}: an `identical` cluster's occurrences must be byte-for-byte equal — {dump}"
    );
    Ok(())
}

/// The per-signal half of scenario 1.
fn assert_verbatim_components(cluster: &Value, corpus: &Corpus) {
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "{language}: identical sources share one Merkle hash — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "{language}: identical sources share one normalised k-gram set — {dump}"
    );
    assert!(
        approx(signal(cluster, "pair_agreement"), 1.0),
        "{language}: byte-identical sources share every collapsed leaf — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{language}: the verbatim scenario has exactly two occurrences — {dump}"
    );
    assert_eq!(
        field(cluster, "category").as_str().unwrap_or("?"),
        "logic",
        "{language}: a duplicated function body is logic, not a data table — {dump}"
    );
    assert_eq!(
        cluster_file_set(cluster).into_iter().collect::<Vec<_>>(),
        vec![corpus.file("verbatim_a"), corpus.file("verbatim_b")],
        "{language}: the verbatim cluster must span exactly its own two files — {dump}"
    );
}

/// Scenario 2 — a maximal identifier rename over identical logic is a
/// Type-2 clone, with or without literal-anchor mass ([`RENAME_STEMS`]).
/// It must stay actionable: a real bucket and certified rename evidence
/// ([FUSED-CONTENT-GATE], gh #410).
fn assert_rename_contract(report: &Value, corpus: &Corpus, root: &Path, stem: &str) -> Result<()> {
    let cluster = scenario_cluster(report, corpus, stem)?;
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert!(
        !HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
        "{language}/{stem}: a Type-2 rename of real logic is duplication, not \
         shape-only evidence — demoting it is a false negative — {dump}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "{language}/{stem}: same shape, same logic, renamed identifiers is the \
         textbook `nearly_identical` clone — {dump}"
    );
    assert_rename_components(cluster, corpus, stem);
    assert_certified_rename_evidence(cluster, corpus, stem);
    assert_eq!(
        distinct_texts(root, cluster)?.len(),
        2,
        "{language}/{stem}: the rename scenario's two occurrences must differ in \
         raw bytes — {dump}"
    );
    Ok(())
}

/// The per-signal half of scenario 2: rename-invariant deterministic
/// signals and the exact two-occurrence span.
fn assert_rename_components(cluster: &Value, corpus: &Corpus, stem: &str) {
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "{language}/{stem}: identifier normalisation makes a rename structurally \
         identical — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "{language}/{stem}: the normalised k-gram stream is rename-invariant by \
         construction — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{language}/{stem}: the rename scenario has exactly two occurrences — {dump}"
    );
}

/// Scenario 2's content-evidence clause (gh #410). Both golden rename
/// stems are a **certified** Type-2 proof: identical logic, every
/// identifier substituted consistently and corroborated by repetition,
/// every aligned literal preserved, and anchor mass well past the point
/// where the mass term alone vouches for the pair. The elected pair's
/// `pair_rename_consistency` therefore reads 1.0 — the report states the
/// strongest measured evidence, and the routing support it grants is
/// what keeps the clone in its act-now bucket.
fn assert_certified_rename_evidence(cluster: &Value, corpus: &Corpus, stem: &str) {
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert!(
        approx(signal(cluster, "pair_rename_consistency"), 1.0),
        "{language}/{stem}: every literal is preserved and every constrained \
         identifier position is explained, so the elected pair's rename \
         evidence must certify at 1.0 — {dump}"
    );
    assert!(
        signal(cluster, "pair_rename_consistency") >= deslop_core::buckets::CONTENT_SUPPORT_FLOOR,
        "{language}/{stem}: the certified rename evidence must clear the \
         cross-file support floor so routing keeps the act-now bucket — {dump}"
    );
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(cluster)),
        "{language}/{stem}: certified rename evidence must carry an act-now \
         bucket — {dump}"
    );
}

/// Scenario 3 — four unrelated descriptors that share nothing but the AST
/// shape. Either the renderer suppresses them, or they surface honestly:
/// never an act-now bucket.
fn assert_shape_only_contract(report: &Value, corpus: &Corpus) {
    let language = corpus.language;
    let families = shape_only_clusters(report);
    if families.is_empty() {
        assert!(
            clusters_hidden(report) >= 1,
            "{language}: the shape-only family vanished without being counted in \
             `clusters_hidden` — suppression must stay observable: {report:#}"
        );
        return;
    }
    for cluster in families {
        assert_shape_only_cluster(cluster, language);
    }
}

/// The per-cluster half of scenario 3.
fn assert_shape_only_cluster(cluster: &Value, language: &str) {
    let dump = signal_dump(cluster);
    assert!(
        HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
        "{language}: shape-only evidence must be labelled as such — {dump}"
    );
    assert_ne!(
        cluster_bucket(cluster),
        "identical",
        "{language}: descriptors with different names and different literals are not \
         identical code — {dump}"
    );
}

/// The whole point of the report: the scenarios must be strictly
/// separated in rank. A weight that cannot order copy-paste above rename
/// above coincidence is not a duplication ranking
/// ([RANK-MASS-SUM], [PIPELINE-RANK-WORST-FIRST]).
fn assert_band_separation(report: &Value, corpus: &Corpus) -> Result<()> {
    for stem in RENAME_STEMS {
        let rename = scenario_cluster(report, corpus, stem)?;
        assert_shape_ranks_below(report, corpus, rename)?;
    }
    Ok(())
}

/// Ranking half of the separation contract, applied per rename scenario:
/// every surviving shape-only family sits below both real clones in rank
/// — the duplicated-mass weighting must shape the report itself, not
/// just the label ([RANK-STRUCTURAL-ONLY], [PIPELINE-RANK-WORST-FIRST]).
fn assert_shape_ranks_below(report: &Value, corpus: &Corpus, rename: &Value) -> Result<()> {
    let verbatim_rank = rank_of(report, scenario_cluster(report, corpus, "verbatim")?)?;
    let rename_rank = rank_of(report, rename)?;
    for cluster in shape_only_clusters(report) {
        let shape_rank = rank_of(report, cluster)?;
        let language = corpus.language;
        assert!(
            verbatim_rank < shape_rank && rename_rank < shape_rank,
            "{language}: both genuine clones (verbatim #{verbatim_rank}, rename \
             #{rename_rank}) must outrank the shape-only family (#{shape_rank}) in \
             the weight — {dump}",
            dump = signal_dump(cluster),
        );
    }
    Ok(())
}

/// Drives one language's corpus through every contract assertion.
fn assert_golden_corpus(corpus: &Corpus) -> Result<()> {
    let root = fixture(corpus.dir);
    let report = corpus.report()?;
    assert!(
        cluster_count(&report) >= 3,
        "{language}: the corpus stages three real clones — a report with fewer \
         clusters has lost recall: {report:#}",
        language = corpus.language,
    );
    assert_verbatim_contract(&report, corpus, &root)?;
    for stem in RENAME_STEMS {
        assert_rename_contract(&report, corpus, &root, stem)?;
    }
    assert_shape_only_contract(&report, corpus);
    assert_band_separation(&report, corpus)
}

/// Looks up one corpus by language id.
fn corpus(language: &str) -> Result<&'static Corpus> {
    CORPORA
        .iter()
        .find(|corpus| corpus.language == language)
        .ok_or_else(|| anyhow::anyhow!("no golden corpus registered for {language}"))
}

/// No cluster-level fused field may survive on the wire ([FUSED-SCOPE]):
/// every rendered cluster in every golden corpus carries the elected
/// pair's axes and content evidence, never a cluster confidence.
#[test]
fn no_golden_report_renders_a_cluster_fused_field() -> Result<()> {
    for corpus in &CORPORA {
        let report = corpus.report()?;
        for cluster in clusters(&report) {
            assert!(
                cluster.pointer("/signals/fused").is_none(),
                "{language}: cluster-level fused must not exist on the wire \
                 ([FUSED-SCOPE]): {dump}",
                language = corpus.language,
                dump = signal_dump(cluster),
            );
        }
    }
    Ok(())
}

// [FUSED-CONTENT-GATE] C#: verbatim / Type-2 rename / unrelated
// descriptor family, all three in one report.
#[test]
fn csharp_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("csharp")?)
}

// [FUSED-CONTENT-GATE] Python: same three scenarios, `self`-bearing
// descriptor family (shared receiver names are the hardest content case).
#[test]
fn python_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("python")?)
}

// [FUSED-CONTENT-GATE] TypeScript: type annotations give the shape-only
// family extra shared identifier positions, so content agreement must
// still fall below the support floor.
#[test]
fn typescript_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("typescript")?)
}

// [FUSED-CONTENT-GATE] Go: the descriptor family carries no literals at
// all, so content agreement is measured purely over identifiers.
#[test]
fn go_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("go")?)
}

// [FUSED-CONTENT-GATE] Rust: `format!` puts the varying text inside a
// single literal leaf, the sparsest content evidence of any language here.
#[test]
fn rust_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("rust")?)
}

// [FUSED-CONTENT-GATE] PHP: sigil-prefixed variables are distinct
// identifier leaves, so a rename touches every one of them.
#[test]
fn php_golden_buckets_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("php")?)
}

// [FUSED-CLUSTER-SIGNALS] Cross-language: the same four scenarios must
// land in the same buckets and evidence in *every* language. A metric
// whose meaning shifts per language cannot back a documented contract.
#[test]
fn golden_buckets_mean_the_same_thing_in_every_language() -> Result<()> {
    let mut verdicts: Vec<String> = Vec::new();
    for corpus in &CORPORA {
        let report = corpus.report()?;
        let verbatim = scenario_cluster(&report, corpus, "verbatim")?;
        let rename = scenario_cluster(&report, corpus, "rename")?;
        let lean = scenario_cluster(&report, corpus, "rename_lean")?;
        verdicts.push(format!(
            "{language}: verbatim={verbatim_bucket} rename={rename_bucket} \
             rename_lean={lean_bucket}",
            language = corpus.language,
            verbatim_bucket = cluster_bucket(verbatim),
            rename_bucket = cluster_bucket(rename),
            lean_bucket = cluster_bucket(lean),
        ));
        assert_eq!(
            cluster_bucket(verbatim),
            "identical",
            "verbatim must bucket `identical` in every language: {verdicts:#?}"
        );
        assert_eq!(
            cluster_bucket(rename),
            "nearly_identical",
            "the anchored rename must bucket `nearly_identical` in every language \
             (gh #410): {verdicts:#?}"
        );
        assert_eq!(
            cluster_bucket(lean),
            "nearly_identical",
            "the lean rename must bucket `nearly_identical` in every language \
             (gh #410): {verdicts:#?}"
        );
        assert!(
            approx(signal(rename, "pair_rename_consistency"), 1.0)
                && approx(signal(lean, "pair_rename_consistency"), 1.0),
            "certified Type-2 renames must carry pair_rename_consistency = 1.0 \
             in every language (gh #410): {verdicts:#?}"
        );
    }
    assert_eq!(
        verdicts.len(),
        CORPORA.len(),
        "every registered golden corpus must be exercised: {verdicts:#?}"
    );
    Ok(())
}
