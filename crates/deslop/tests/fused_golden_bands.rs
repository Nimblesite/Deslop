//! Golden band coverage for the fused confidence
//! ([FUSION-STRATEGY-MAX-SUM], [FUSION-CONTENT-GATE], [FUSED-THRESHOLD]).
//!
//! `docs/root-cause-fusion.md` states the contract the fused score has to
//! satisfy: it must carry information, so that the three documented agent
//! bands are all reachable and mean what the docs say they mean. Each
//! `fused-golden-<language>` fixture directory stages the same three
//! real-world scenarios side by side so one report exercises all of them:
//!
//! | files                       | scenario                                        | required verdict |
//! |-----------------------------|-------------------------------------------------|------------------|
//! | `verbatim_a` / `verbatim_b` | byte-identical copy-paste (Type-1)               | `identical`, `fused == 1.0` |
//! | `rename_a` / `rename_b`     | maximal identifier rename, same logic (Type-2)   | `nearly_identical`, `fused >= 0.6` and `< 1.0` |
//! | `shape_*` (×4)              | unrelated descriptors sharing only the AST shape | never act-now, ranked last |
//!
//! The Type-2 row is the load-bearing one. A rename-only copy is the
//! textbook definition of a Type-2 clone and every clone detector must
//! report it. [FUSION-CONTENT-GATE] measures raw-content agreement over
//! *all* collapsed leaves — identifiers and literals pooled — so a
//! maximally renamed clone with few literals scores low agreement and is
//! indistinguishable from unrelated scaffolding. These fixtures keep the
//! maximal rename that the shipped rename-showcase fixtures were softened
//! away from, so the distinction is pinned rather than avoided.
//!
//! Every scenario carries a distinct AST shape so transitive closure
//! cannot merge the three of them into one cluster.

use std::path::Path;

use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Node floor for the golden corpora — matches the small-fixture value the
/// TypeScript/JS feature suites use so every scenario subtree qualifies.
const MIN_NODES: u32 = 12;

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

/// Every language the golden bands are pinned in.
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

/// Band 1 — a byte-identical copy-paste is the only thing allowed to
/// reach a saturated fused confidence, and it must reach it.
fn assert_verbatim_band(report: &Value, corpus: &Corpus, root: &Path) -> Result<()> {
    let cluster = scenario_cluster(report, corpus, "verbatim")?;
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "{language}: a byte-identical copy-paste must bucket `identical` — {dump}"
    );
    assert!(
        approx(signal(cluster, "fused"), 1.0),
        "{language}: a byte-identical copy-paste is the definition of full confidence — {dump}"
    );
    assert_verbatim_components(cluster, corpus);
    assert_eq!(
        distinct_texts(root, cluster)?.len(),
        1,
        "{language}: an `identical` cluster's occurrences must be byte-for-byte equal — {dump}"
    );
    Ok(())
}

/// The per-signal and shape half of band 1, split out to keep each
/// assertion body inside the function-length budget.
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

/// Band 2 — a maximal identifier rename over identical logic and
/// identical literals is a Type-2 clone. It must stay actionable: a real
/// bucket, a confidence inside the reuse band, and strictly below the
/// byte-identical band so the score still discriminates.
fn assert_rename_band(report: &Value, corpus: &Corpus, root: &Path) -> Result<()> {
    let cluster = scenario_cluster(report, corpus, "rename")?;
    let dump = signal_dump(cluster);
    let language = corpus.language;
    assert!(
        !HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
        "{language}: a Type-2 rename of real logic is duplication, not shape-only \
         evidence — demoting it is a false negative — {dump}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "{language}: same shape, same literals, renamed identifiers is the textbook \
         `nearly_identical` clone — {dump}"
    );
    assert_rename_components(cluster, corpus);
    assert_eq!(
        distinct_texts(root, cluster)?.len(),
        2,
        "{language}: the rename scenario's two occurrences must differ in raw bytes — {dump}"
    );
    Ok(())
}

/// The per-signal half of band 2: rename-invariant deterministic signals,
/// a confidence that stays inside the reuse band, and no saturation.
fn assert_rename_components(cluster: &Value, corpus: &Corpus) {
    let dump = signal_dump(cluster);
    let language = corpus.language;
    let fused = signal(cluster, "fused");
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "{language}: identifier normalisation makes a rename structurally identical — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "{language}: the normalised k-gram stream is rename-invariant by construction — {dump}"
    );
    assert!(
        fused >= REUSE_FUSED,
        "{language}: a renamed copy of real logic must stay at or above the reuse-bias \
         line ({REUSE_FUSED}) — below it the agent recipe tells the agent to write the \
         copy anyway — {dump}"
    );
    assert!(
        fused < 1.0,
        "{language}: only a byte-identical copy may saturate the confidence; a rename \
         has measurably less evidence — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{language}: the rename scenario has exactly two occurrences — {dump}"
    );
}

/// Band 3 — four unrelated descriptors that share nothing but the AST
/// shape. Either the renderer suppresses them, or they surface honestly:
/// never act-now, never an act-now bucket.
fn assert_shape_only_band(report: &Value, corpus: &Corpus) {
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

/// The per-cluster half of band 3.
fn assert_shape_only_cluster(cluster: &Value, language: &str) {
    let dump = signal_dump(cluster);
    assert!(
        signal(cluster, "fused") < ACT_NOW_FUSED,
        "{language}: unrelated same-shape descriptors must not reach the act-now line \
         ({ACT_NOW_FUSED}) — that is the #331/#336 saturation bug — {dump}"
    );
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

/// The whole point of the metric: the three scenarios must be strictly
/// separated, in both confidence and rank. A score that cannot order
/// copy-paste above rename above coincidence is not a confidence score.
fn assert_band_separation(report: &Value, corpus: &Corpus) -> Result<()> {
    let language = corpus.language;
    let verbatim = scenario_cluster(report, corpus, "verbatim")?;
    let rename = scenario_cluster(report, corpus, "rename")?;
    assert!(
        signal(verbatim, "fused") > signal(rename, "fused"),
        "{language}: a byte-identical copy must be strictly more confident than a \
         renamed one — verbatim[{verbatim_dump}] rename[{rename_dump}]",
        verbatim_dump = signal_dump(verbatim),
        rename_dump = signal_dump(rename),
    );
    assert_shape_ranks_below(report, corpus, rename)
}

/// Ranking half of the separation contract: every surviving shape-only
/// family sits below both real clones in confidence and in rank — the
/// fused confidence must shape the weighting itself, not just the label
/// ([RANK-STRUCTURAL-ONLY], [PIPELINE-RANK-WORST-FIRST]).
fn assert_shape_ranks_below(report: &Value, corpus: &Corpus, rename: &Value) -> Result<()> {
    let verbatim_rank = rank_of(report, scenario_cluster(report, corpus, "verbatim")?)?;
    let rename_rank = rank_of(report, rename)?;
    for cluster in shape_only_clusters(report) {
        assert_outscored_and_outranked(report, corpus, [verbatim_rank, rename_rank], rename, cluster)?;
    }
    Ok(())
}

/// One shape-only family against both real clones: outscored in fused
/// confidence by the rename, and outranked in the weighting by the
/// verbatim copy and the rename alike — a four-member coincidence must
/// not outrank a two-member proven clone on geometry.
fn assert_outscored_and_outranked(
    report: &Value,
    corpus: &Corpus,
    clone_ranks: [usize; 2],
    rename: &Value,
    cluster: &Value,
) -> Result<()> {
    let language = corpus.language;
    let shape_rank = rank_of(report, cluster)?;
    assert!(
        signal(rename, "fused") > signal(cluster, "fused"),
        "{language}: a renamed real clone must outscore coincidental shape \
         agreement — rename[{rename_dump}] shape[{shape_dump}]",
        rename_dump = signal_dump(rename),
        shape_dump = signal_dump(cluster),
    );
    let [verbatim_rank, rename_rank] = clone_ranks;
    assert!(
        verbatim_rank < shape_rank && rename_rank < shape_rank,
        "{language}: both genuine clones (verbatim #{verbatim_rank}, rename \
         #{rename_rank}) must outrank the shape-only family (#{shape_rank}) in the \
         weighting — {dump}",
        dump = signal_dump(cluster),
    );
    Ok(())
}

/// Drives one language's corpus through every band assertion.
fn assert_golden_corpus(corpus: &Corpus) -> Result<()> {
    let root = fixture(corpus.dir);
    let report = corpus.report()?;
    assert!(
        cluster_count(&report) >= 2,
        "{language}: the corpus stages two real clones — a report with fewer clusters \
         has lost recall: {report:#}",
        language = corpus.language,
    );
    assert_verbatim_band(&report, corpus, &root)?;
    assert_rename_band(&report, corpus, &root)?;
    assert_shape_only_band(&report, corpus);
    assert_band_separation(&report, corpus)
}

/// Looks up one corpus by language id.
fn corpus(language: &str) -> Result<&'static Corpus> {
    CORPORA
        .iter()
        .find(|corpus| corpus.language == language)
        .ok_or_else(|| anyhow::anyhow!("no golden corpus registered for {language}"))
}

// [FUSION-CONTENT-GATE] C#: verbatim / Type-2 rename / unrelated
// descriptor family, all three in one report.
#[test]
fn csharp_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("csharp")?)
}

// [FUSION-CONTENT-GATE] Python: same three scenarios, `self`-bearing
// descriptor family (shared receiver names are the hardest content case).
#[test]
fn python_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("python")?)
}

// [FUSION-CONTENT-GATE] TypeScript: type annotations give the shape-only
// family extra shared identifier positions, so content agreement must
// still fall below the support floor.
#[test]
fn typescript_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("typescript")?)
}

// [FUSION-CONTENT-GATE] Go: the descriptor family carries no literals at
// all, so content agreement is measured purely over identifiers.
#[test]
fn go_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("go")?)
}

// [FUSION-CONTENT-GATE] Rust: `format!` puts the varying text inside a
// single literal leaf, the sparsest content evidence of any language here.
#[test]
fn rust_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("rust")?)
}

// [FUSION-CONTENT-GATE] PHP: sigil-prefixed variables are distinct
// identifier leaves, so a rename touches every one of them.
#[test]
fn php_fused_bands_separate_copy_paste_rename_and_coincidence() -> Result<()> {
    assert_golden_corpus(corpus("php")?)
}

// [FUSED-THRESHOLD] Cross-language: the same three scenarios must land in
// the same three bands in *every* language. A metric whose meaning shifts
// per language cannot back a documented agent contract.
#[test]
fn fused_bands_mean_the_same_thing_in_every_language() -> Result<()> {
    let mut verdicts: Vec<String> = Vec::new();
    for corpus in &CORPORA {
        let report = corpus.report()?;
        let verbatim = signal(scenario_cluster(&report, corpus, "verbatim")?, "fused");
        let rename = signal(scenario_cluster(&report, corpus, "rename")?, "fused");
        verdicts.push(format!(
            "{language}: verbatim={verbatim:.4} rename={rename:.4}",
            language = corpus.language,
        ));
        assert!(
            approx(verbatim, 1.0) && (REUSE_FUSED..1.0).contains(&rename),
            "band contract broken — every language must report verbatim at 1.0 and a \
             Type-2 rename inside [{REUSE_FUSED}, 1.0): {verdicts:#?}"
        );
    }
    assert_eq!(
        verdicts.len(),
        CORPORA.len(),
        "every registered golden corpus must be exercised: {verdicts:#?}"
    );
    Ok(())
}
