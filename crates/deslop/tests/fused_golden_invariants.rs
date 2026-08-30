//! Cross-language invariants the final pair-scoped report must satisfy in
//! *every* report ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE],
//! [FUSED-THRESHOLD], [FUSED-SCOPE], [RANK-STRUCTURAL-ONLY]).
//!
//! There is no cluster-level `fused` ([FUSED-SCOPE]): the report renders
//! the elected pair's measured axes and its content evidence, and the
//! bucket is the engine's verdict. The old cluster-confidence sweep
//! (band membership, fused saturation, "score must discriminate") is
//! gone with the field it measured; what survives is the contract the
//! report still owes: bounded axes, honest bucket/evidence agreement,
//! worst-first ranking, and no surviving wire `fused`.
//!
//! Rather than assert one number per fixture, this suite sweeps twenty
//! corpora across eight languages and holds every visible cluster to the
//! same contract, collecting *all* breaches before failing so one run
//! reports the whole picture:
//!
//! 1. every rendered signal stays inside `[0, 1]`;
//! 2. no cluster carries a wire `fused` field;
//! 3. an `identical` bucket implies full structure and full pair
//!    agreement;
//! 4. a demoted `structural_only` / `loosely_similar` bucket never
//!    carries certified rename evidence (a clone wearing a demoted label
//!    is a false negative);
//! 5. the ranked report is genuinely ordered worst-first.

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::common::{signals::*, *};

/// Buckets whose evidence can justify a saturated shape and a full
/// pair-agreement reading: byte equivalence proven outright, or the
/// [FUSED-CONTENT-GATE] verbatim guard vouching for a cluster dominated
/// by byte-identical members.
const SATURATION_BUCKETS: [&str; 2] = ["identical", "nearly_identical"];

/// Smallest number of clusters the sweep must inspect before its verdict
/// means anything — a sweep that found nothing must fail loudly rather
/// than report a clean bill of health.
const MIN_INSPECTED_CLUSTERS: usize = 20;

/// Fixture corpora swept, with the node floor each is sized for.
const SWEEP: [(&str, u32); 21] = [
    ("ts-mixed-band", 12),
    ("fused-golden-csharp", 12),
    ("fused-golden-python", 12),
    ("fused-golden-typescript", 12),
    ("fused-golden-go", 12),
    ("fused-golden-rust", 12),
    ("fused-golden-php", 12),
    ("csharp-type1", 10),
    ("csharp-type3", 10),
    ("csharp-issue-134-structural-only", 10),
    ("dart-small", 10),
    ("dart-type3", 10),
    ("fsharp-type3", 10),
    ("go-type3", 10),
    ("js-type1-identical", 10),
    ("js-type2-loop", 10),
    ("typescript-small", 10),
    ("ts-type2-loop", 10),
    ("python-type3", 10),
    ("php-small", 10),
    ("rust-issue-232-token-jaccard", 10),
];

/// The corpora that stage several distinct degrees of duplication in
/// one tree, and therefore cannot render one bucket for all of them.
/// Listed explicitly: a positional slice of [`SWEEP`] silently changes
/// which corpora are asserted whenever the sweep gains an entry, and
/// prepending `ts-mixed-band` had already dropped PHP from this contract.
const GOLDEN_CORPORA: [(&str, u32); 7] = [
    ("ts-mixed-band", 12),
    ("fused-golden-csharp", 12),
    ("fused-golden-python", 12),
    ("fused-golden-typescript", 12),
    ("fused-golden-go", 12),
    ("fused-golden-rust", 12),
    ("fused-golden-php", 12),
];

/// Accumulates every contract breach across the sweep so one failure
/// message lists all of them.
#[derive(Debug, Default)]
struct Sweep {
    /// Clusters examined across every corpus.
    inspected: usize,
    /// Contract breaches, each already carrying its corpus and signals.
    violations: Vec<String>,
}

impl Sweep {
    /// Records one breach against the corpus and cluster that caused it.
    fn record(&mut self, dir: &str, cluster: &Value, rule: &str) {
        self.violations.push(format!(
            "[{dir}] {rule} — {dump}",
            dump = signal_dump(cluster)
        ));
    }

    /// Folds one rendered report into the sweep.
    fn absorb(&mut self, dir: &str, root: &Path, report: &Value) -> Result<()> {
        self.check_ranking(dir, report);
        for cluster in clusters(report) {
            self.inspected = self.inspected.saturating_add(1);
            self.check_ranges(dir, cluster);
            self.check_no_wire_fused(dir, cluster);
            self.check_bucket_agreement(dir, cluster);
            self.check_proven_saturation(dir, root, cluster)?;
        }
        Ok(())
    }

    /// [PIPELINE-RANK-WORST-FIRST] The rendered order is the ranking, so
    /// weights must never increase as the reader walks down the report.
    fn check_ranking(&mut self, dir: &str, report: &Value) {
        let weights: Vec<f64> = clusters(report)
            .iter()
            .map(|cluster| field(cluster, "weight").as_f64().unwrap_or_default())
            .collect();
        let unsorted = weights.windows(2).any(|pair| {
            pair.first().copied().unwrap_or_default() + 1e-9
                < pair.last().copied().unwrap_or_default()
        });
        if unsorted {
            self.violations.push(format!(
                "[{dir}] ranked report is not worst-first: {weights:?}"
            ));
        }
    }

    /// Every rendered signal axis is documented as a `[0, 1]` measurement
    /// ([FUSED-CLUSTER-SIGNALS], [FUSED-CONTENT-GATE]).
    fn check_ranges(&mut self, dir: &str, cluster: &Value) {
        for key in [
            "structural",
            "token_jaccard",
            "embedding_cos",
            "pair_agreement",
            "pair_rename_consistency",
            "literal_fraction",
        ] {
            let value = signal(cluster, key);
            if !(0.0..=1.0).contains(&value) {
                self.record(dir, cluster, &format!("signal `{key}` escaped [0, 1]"));
            }
        }
    }

    /// [FUSED-SCOPE] The cluster-level fused field is gone. A report that
    /// still renders one has not completed the cutover.
    fn check_no_wire_fused(&mut self, dir: &str, cluster: &Value) {
        if cluster.pointer("/signals/fused").is_some() {
            self.record(dir, cluster, "cluster-level `fused` survived on the wire");
        }
    }

    /// The bucket label and the measured evidence are two renderings of
    /// one verdict; they may never contradict each other.
    fn check_bucket_agreement(&mut self, dir: &str, cluster: &Value) {
        let bucket = cluster_bucket(cluster);
        if bucket == "identical" && !approx(signal(cluster, "structural"), 1.0) {
            self.record(
                dir,
                cluster,
                "proven-identical cluster without full structure",
            );
        }
        if bucket == "identical" && !approx(signal(cluster, "pair_agreement"), 1.0) {
            self.record(
                dir,
                cluster,
                "proven-identical cluster without full pair agreement",
            );
        }
        if HONEST_SHAPE_ONLY_BUCKETS.contains(&bucket)
            && approx(signal(cluster, "pair_rename_consistency"), 1.0)
        {
            self.record(
                dir,
                cluster,
                "certified rename evidence under a demoted shape-only bucket — a \
                 clone wearing a demoted label is a false negative",
            );
        }
    }

    /// The anti-saturation contract: a byte-proven bucket must carry byte
    /// proof in the raw source, not just in the signals.
    fn check_proven_saturation(&mut self, dir: &str, root: &Path, cluster: &Value) -> Result<()> {
        if !SATURATION_BUCKETS.contains(&cluster_bucket(cluster)) {
            return Ok(());
        }
        if !approx(signal(cluster, "structural"), 1.0) {
            return Ok(());
        }
        let bucket = cluster_bucket(cluster);
        if bucket != "identical" && !has_verbatim_pair(root, cluster)? {
            self.record(
                dir,
                cluster,
                "saturated structure with no byte-identical occurrence pair to back it",
            );
        }
        Ok(())
    }
}

/// Runs every corpus in [`SWEEP`] through the accumulator.
fn sweep_every_corpus() -> Result<Sweep> {
    let mut sweep = Sweep::default();
    for (dir, min_nodes) in SWEEP {
        let root = fixture(dir);
        let report = run_report(&root, min_nodes)?;
        sweep.absorb(dir, &root, &report)?;
    }
    Ok(sweep)
}

// [FUSED-CLUSTER-SIGNALS] / [FUSED-CONTENT-GATE] / [FUSED-SCOPE]: one
// contract, twenty corpora, eight languages. Every breach is collected
// before the failure so a regression shows its full blast radius in one
// run.
#[test]
fn the_report_obeys_one_contract_in_every_language() -> Result<()> {
    let sweep = sweep_every_corpus()?;
    assert!(
        sweep.inspected >= MIN_INSPECTED_CLUSTERS,
        "the sweep inspected only {inspected} clusters — a clean verdict over an empty \
         report set proves nothing; the fixture corpora have stopped producing clusters",
        inspected = sweep.inspected,
    );
    assert_eq!(
        sweep.violations,
        Vec::<String>::new(),
        "the rendered report broke its contract in {count} place(s) across \
         {inspected} clusters",
        count = sweep.violations.len(),
        inspected = sweep.inspected,
    );
    Ok(())
}

// [FUSED-CLUSTER-SIGNALS] Per-report discrimination: each golden corpus
// stages a byte-identical copy, a renamed copy and an unrelated
// same-shape family, so a report that renders one bucket for all of them
// has erased the distinction the buckets state. Pinned as a sanity floor:
// at least two distinct buckets must render per corpus.
#[test]
fn no_golden_report_renders_a_single_bucket() -> Result<()> {
    let mut verdicts: Vec<String> = Vec::new();
    for (dir, min_nodes) in GOLDEN_CORPORA {
        let report = run_report(&fixture(dir), min_nodes)?;
        let values: BTreeSet<String> = clusters(&report)
            .iter()
            .map(cluster_bucket)
            .map(ToOwned::to_owned)
            .collect();
        verdicts.push(format!("{dir}: {values:?}"));
        assert!(
            values.len() >= 2,
            "{dir} stages a byte-identical copy, a renamed copy and an unrelated \
             same-shape family — they cannot all deserve the same bucket: {verdicts:#?}"
        );
    }
    assert_eq!(
        verdicts.len(),
        GOLDEN_CORPORA.len(),
        "every golden corpus must be exercised: {verdicts:#?}"
    );
    Ok(())
}
