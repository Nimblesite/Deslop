//! Cross-language invariants the rendered fused confidence must satisfy
//! in *every* report ([FUSION-STRATEGY-MAX-SUM], [FUSION-CONTENT-GATE],
//! [FUSED-THRESHOLD], [RANK-STRUCTURAL-ONLY]).
//!
//! `docs/root-cause-fusion.md` names the failure mode precisely: sum-then-
//! clamp fusion over two views of one normalised tree makes `fused` a
//! re-encoding of "the shapes matched", pinned at 1.0. [FUSION-CONTENT-GATE]
//! rescues the two saturating corners (`structural >= 0.99` or
//! `token_jaccard >= 0.95`) but leaves the arithmetic itself unchanged, so
//! any cluster whose mean signals sum past 1.0 without either component
//! saturating still clamps to full confidence.
//!
//! Rather than assert one number per fixture, this suite sweeps twenty
//! corpora across eight languages and holds every visible cluster to the
//! same contract, collecting *all* breaches before failing so one run
//! reports the whole picture:
//!
//! 1. every component signal stays inside `[0, 1]`;
//! 2. act-now confidence implies an act-now bucket, and shape-only
//!    evidence never reaches the act-now line;
//! 3. a `identical` cluster reports full confidence and full structure;
//! 4. **only proven duplication may saturate** — `fused == 1.0` requires
//!    either a byte-equivalence-proven `identical` bucket or at least one
//!    byte-identical occurrence pair backing the verbatim guard;
//! 5. the ranked report is genuinely ordered worst-first;
//! 6. the score discriminates — a report whose clusters all carry one
//!    fused value is carrying no information at all.

use std::{collections::BTreeSet, path::Path};

use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Buckets whose evidence can justify a saturated confidence: byte
/// equivalence proven outright, or the [FUSION-CONTENT-GATE] verbatim
/// guard vouching for a cluster dominated by byte-identical members.
const SATURATION_BUCKETS: [&str; 2] = ["identical", "nearly_identical"];

/// Smallest number of clusters the sweep must inspect before its verdict
/// means anything — a sweep that found nothing must fail loudly rather
/// than report a clean bill of health.
const MIN_INSPECTED_CLUSTERS: usize = 20;

/// Smallest number of distinct rendered `fused` values the sweep must
/// see across every corpus. Two would be satisfied by "1.0 and 0.0";
/// three forces the middle of the range to be reachable, which is the
/// property `docs/root-cause-fusion.md` says the metric lacks.
const MIN_DISTINCT_FUSED_VALUES: usize = 3;

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

/// Accumulates every contract breach across the sweep so one failure
/// message lists all of them.
#[derive(Debug, Default)]
struct Sweep {
    /// Clusters examined across every corpus.
    inspected: usize,
    /// Contract breaches, each already carrying its corpus and signals.
    violations: Vec<String>,
    /// Distinct rendered fused values, formatted so they de-duplicate
    /// without hashing floats.
    fused_values: BTreeSet<String>,
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
            let _inserted = self
                .fused_values
                .insert(format!("{fused:.4}", fused = signal(cluster, "fused")));
            self.check_ranges(dir, cluster);
            self.check_bucket_agreement(dir, cluster);
            self.check_saturation(dir, root, cluster)?;
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

    /// Every component of the signal triple, and the fusion of them, is
    /// documented as a `[0, 1]` confidence.
    fn check_ranges(&mut self, dir: &str, cluster: &Value) {
        for key in ["structural", "token_jaccard", "embedding_cos", "fused"] {
            let value = signal(cluster, key);
            if !(0.0..=1.0).contains(&value) {
                self.record(dir, cluster, &format!("signal `{key}` escaped [0, 1]"));
            }
        }
    }

    /// The bucket label and the confidence are two renderings of one
    /// verdict; they may never contradict each other.
    fn check_bucket_agreement(&mut self, dir: &str, cluster: &Value) {
        let bucket = cluster_bucket(cluster);
        let fused = signal(cluster, "fused");
        if fused >= ACT_NOW_FUSED && !ACT_NOW_BUCKETS.contains(&bucket) {
            self.record(
                dir,
                cluster,
                "act-now confidence under a non-act-now bucket",
            );
        }
        if HONEST_SHAPE_ONLY_BUCKETS.contains(&bucket) && fused >= ACT_NOW_FUSED {
            self.record(dir, cluster, "shape-only evidence reached the act-now line");
        }
        if bucket == "identical" && !approx(fused, 1.0) {
            self.record(
                dir,
                cluster,
                "proven-identical cluster below full confidence",
            );
        }
        if bucket == "identical" && !approx(signal(cluster, "structural"), 1.0) {
            self.record(
                dir,
                cluster,
                "proven-identical cluster without full structure",
            );
        }
    }

    /// The anti-saturation contract: a perfect confidence is a claim of
    /// proven duplication and must be backed by bytes, not by two
    /// correlated shape signals summing past the clamp.
    fn check_saturation(&mut self, dir: &str, root: &Path, cluster: &Value) -> Result<()> {
        if !approx(signal(cluster, "fused"), 1.0) {
            return Ok(());
        }
        let bucket = cluster_bucket(cluster);
        if !SATURATION_BUCKETS.contains(&bucket) {
            self.record(dir, cluster, "saturated confidence outside a proven bucket");
        }
        if bucket != "identical" && !has_verbatim_pair(root, cluster)? {
            self.record(
                dir,
                cluster,
                "saturated confidence with no byte-identical occurrence pair to back it",
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

// [FUSION-STRATEGY-MAX-SUM] / [FUSION-CONTENT-GATE]: one contract, twenty
// corpora, eight languages. Every breach is collected before the failure
// so a regression shows its full blast radius in one run.
#[test]
fn fused_confidence_obeys_one_contract_in_every_language() -> Result<()> {
    let sweep = sweep_every_corpus()?;
    assert!(
        sweep.inspected >= MIN_INSPECTED_CLUSTERS,
        "the sweep inspected only {inspected} clusters — a clean verdict over an empty \
         report set proves nothing; the fixture corpora have stopped producing clusters",
        inspected = sweep.inspected,
    );
    assert!(
        sweep.fused_values.len() >= MIN_DISTINCT_FUSED_VALUES,
        "fused rendered only {count} distinct value(s) across {inspected} clusters in \
         eight languages ({values:?}) — a score with no spread is a re-encoding of \
         `the shapes matched`, not a confidence",
        count = sweep.fused_values.len(),
        inspected = sweep.inspected,
        values = sweep.fused_values,
    );
    assert_eq!(
        sweep.violations,
        Vec::<String>::new(),
        "the rendered fused confidence broke its contract in {count} place(s) across \
         {inspected} clusters",
        count = sweep.violations.len(),
        inspected = sweep.inspected,
    );
    Ok(())
}

// [FUSED-THRESHOLD] Per-report discrimination: each golden corpus stages
// three deliberately different degrees of duplication, so a report that
// renders one fused value for all of them has erased the distinction the
// agent recipe branches on.
#[test]
fn no_golden_report_renders_a_constant_fused_score() -> Result<()> {
    let mut verdicts: Vec<String> = Vec::new();
    for (dir, min_nodes) in SWEEP.iter().take(6) {
        let report = run_report(&fixture(dir), *min_nodes)?;
        let values: BTreeSet<String> = clusters(&report)
            .iter()
            .map(|cluster| format!("{fused:.4}", fused = signal(cluster, "fused")))
            .collect();
        verdicts.push(format!("{dir}: {values:?}"));
        assert!(
            values.len() >= 2,
            "{dir} stages a byte-identical copy, a renamed copy and an unrelated \
             same-shape family — they cannot all deserve the same confidence: {verdicts:#?}"
        );
    }
    assert_eq!(
        verdicts.len(),
        6,
        "every golden corpus must be exercised: {verdicts:#?}"
    );
    Ok(())
}
