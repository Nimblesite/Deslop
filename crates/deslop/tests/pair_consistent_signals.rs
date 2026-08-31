//! E2E pin for gh #458 — [FUSED-CLUSTER-SIGNALS]: a rendered cluster's
//! signals are the strongest admitted pair's evidence, never a mean over
//! pairs that never cleared admission, and the displayed value is
//! connected to the pair that earned it.
//!
//! `ts-mixed-band` plus a byte-identical copy of `ledger_a.ts` yields one
//! five-member cluster (`3015b03c`) containing the copy pair and one
//! two-member cluster (`22ccedd3`) holding only the pair. Baker (1995)
//! defines duplication as a per-pair predicate — a p-match either holds
//! or it does not — so the pair is measured once at `1.0 / 1.0`, and the
//! five-member cluster must display that same pair's evidence, never an
//! average over the ten pairings of its five occurrences.

use anyhow::Result;
use serde_json::Value;

use crate::common::{signals::*, *};

/// The mixed cluster containing the byte-identical pair and three lookalikes.
const MIXED_CLUSTER_ID: &str = "3015b03cf2ead794";
/// The two-member cluster holding only the byte-identical pair.
const PAIR_CLUSTER_ID: &str = "22ccedd3ee6b95f6";
/// The byte-identical copy seeded next to `ledger_a.ts`.
const COPY_STEM: &str = "ledger_a_copy.ts";

/// Every file path the mixed cluster reports after structural-family partitioning.
const MIXED_CLUSTER_FILES: [&str; 5] = [
    "ledger_a.ts",
    COPY_STEM,
    "ledger_b.ts",
    "ledger_d.ts",
    "ledger_e.ts",
];

/// The cluster carrying exactly the given id.
fn cluster_by_id<'a>(report: &'a Value, id: &str) -> Option<&'a Value> {
    clusters(report)
        .iter()
        .find(|cluster| cluster_id(cluster) == id)
}

/// The two occurrence paths the report names as the evidence source.
fn signal_source_paths(cluster: &Value) -> Result<(String, String)> {
    let source = field(cluster, "signal_source");
    let left = source.get("left").and_then(Value::as_u64).ok_or_else(|| {
        anyhow::anyhow!("signal_source must name the left occurrence index: {source:?}")
    })?;
    let right = source.get("right").and_then(Value::as_u64).ok_or_else(|| {
        anyhow::anyhow!("signal_source must name the right occurrence index: {source:?}")
    })?;
    let occurrences = occurrences(cluster);
    let left_index = usize::try_from(left)
        .map_err(|error| anyhow::anyhow!("signal_source.left {left} is invalid: {error}"))?;
    let right_index = usize::try_from(right)
        .map_err(|error| anyhow::anyhow!("signal_source.right {right} is invalid: {error}"))?;
    let left_path = occurrence_path(
        occurrences
            .get(left_index)
            .ok_or_else(|| anyhow::anyhow!("signal_source.left {left} out of range"))?,
    )?;
    let right_path = occurrence_path(
        occurrences
            .get(right_index)
            .ok_or_else(|| anyhow::anyhow!("signal_source.right {right} out of range"))?,
    )?;
    Ok((left_path.to_owned(), right_path.to_owned()))
}

/// Seeded `ts-mixed-band` plus the byte-identical copy, run with the
/// issue's own flags (`--min-nodes 15 --embeddings off`).
fn run_pair_mean_report() -> Result<Value> {
    let tmp = tempfile::tempdir()?;
    let fixtures = fixture("ts-mixed-band");
    for entry in std::fs::read_dir(&fixtures)? {
        let entry = entry?;
        let target = tmp.path().join(entry.file_name());
        let _bytes = std::fs::copy(entry.path(), target)?;
    }
    let _bytes = std::fs::copy(fixtures.join("ledger_a.ts"), tmp.path().join(COPY_STEM))?;
    run_report_args(tmp.path(), &["--min-nodes", "15", "--embeddings", "off"])
}

// gh #458 acceptance: the same two files must not read `1.0000/1.0000` in
// one cluster and `0.9982/0.8313` in another within a single report. The
// mixed cluster displays the strongest admitted pair's evidence — the
// byte-identical `ledger_a.ts` ↔ `ledger_a_copy.ts` pair — so its rendered
// `structural` and `token_jaccard` are the same `1.0 / 1.0` the pair
// cluster displays, the cluster keeps its supported bucket, and the report
// names the pair that earned the displayed value.
#[test]
fn a_byte_identical_pair_reads_the_same_in_every_cluster() -> Result<()> {
    let report = run_pair_mean_report()?;

    let mixed = cluster_by_id(&report, MIXED_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("mixed cluster {MIXED_CLUSTER_ID} missing: {report:#}"))?;
    assert_eq!(
        occurrences(mixed).len(),
        MIXED_CLUSTER_FILES.len(),
        "the mixed cluster must report exactly its partitioned structural family"
    );
    assert_eq!(
        occurrence_paths(mixed),
        MIXED_CLUSTER_FILES,
        "the mixed cluster's occurrence set must contain the copy pair and three lookalikes"
    );
    assert_eq!(
        field(mixed, "occurrence_count").as_u64(),
        Some(MIXED_CLUSTER_FILES.len() as u64),
        "occurrence_count must match the reported occurrences"
    );

    let pair = cluster_by_id(&report, PAIR_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("pair cluster {PAIR_CLUSTER_ID} missing: {report:#}"))?;
    assert_eq!(
        occurrences(pair).len(),
        2,
        "the pair cluster holds one pair"
    );

    // Pair-consistency: the byte-identical pair measures 1.0 / 1.0, and
    // the mixed cluster must display that pair's evidence rather than a
    // number averaged over pairs the report does not name.
    let dump_mixed = signal_dump(mixed);
    let dump_pair = signal_dump(pair);
    assert_eq!(
        signal(pair, "structural").to_bits(),
        1.0_f64.to_bits(),
        "the pair cluster must render the pair's own structural: {dump_pair}"
    );
    assert_eq!(
        signal(pair, "token_jaccard").to_bits(),
        1.0_f64.to_bits(),
        "the pair cluster must render the pair's own token evidence: {dump_pair}"
    );
    assert_eq!(
        signal(mixed, "structural").to_bits(),
        1.0_f64.to_bits(),
        "the mixed cluster must display the strongest admitted pair's \
         structural (the byte-identical pair), not a diluted mean: {dump_mixed}"
    );
    assert_eq!(
        signal(mixed, "token_jaccard").to_bits(),
        1.0_f64.to_bits(),
        "the mixed cluster must display the strongest admitted pair's \
         token evidence, not a diluted mean: {dump_mixed}"
    );

    // Admission: the proven pair keeps the mixed cluster supported —
    // the existential gate stays open (Baker: any pair that qualifies
    // qualifies the group), and the lookalikes cannot demote it. The two
    // clusters land in BOTH buckets: the mixed cluster keeps its
    // nearly-identical bucket while the copy pair alone is identical —
    // the lookalikes do not manufacture an identical verdict, and the
    // byte-identical pair does not lose its supported bucket.
    assert_eq!(
        cluster_bucket(mixed),
        "nearly_identical",
        "a cluster containing a byte-identical pair must keep its supported \
         bucket, got {}: {report:#}",
        cluster_bucket(mixed)
    );
    assert_eq!(
        cluster_bucket(pair),
        "identical",
        "the byte-identical pair alone is identical code"
    );

    // Ranking: more duplicated mass ranks worse-first.
    let rank_mixed = field(mixed, "rank").as_u64().unwrap_or(u64::MAX);
    let rank_pair = field(pair, "rank").as_u64().unwrap_or(u64::MAX);
    assert!(
        rank_mixed < rank_pair,
        "five copies of the ledgers outrank two copies, got {rank_mixed} vs {rank_pair}"
    );

    // Display rule: the reported signals are connected to the pair that
    // earned them, not to an anonymous cluster average.
    let (left_path, right_path) = signal_source_paths(mixed)?;
    assert_eq!(
        (left_path.as_str(), right_path.as_str()),
        ("ledger_a.ts", COPY_STEM),
        "the mixed cluster's displayed signals must name the byte-identical \
         pair that earned them, got {left_path} and {right_path}"
    );
    Ok(())
}

/// The pooled byte agreement a byte-identical pair earns on its own:
/// their collapsed leaves are the same bytes at every position
/// ([FUSED-CONTENT-GATE]).
const VERBATIM_PAIR_AGREEMENT: f64 = 1.0;

/// Exact evidence earned by the elected byte-identical occurrence pair.
const ELECTED_PAIR_EXACT_EVIDENCE: f64 = 1.0;
/// Embeddings are disabled for this fixture, so the elected pair has no vector evidence.
const EMBEDDINGS_OFF_EVIDENCE: f64 = 0.0;

// gh #458 (content half) — [FUSED-CONTENT-GATE]: the pair-local contract
// covers the content axes too. Baker (1995) defines duplication per pair:
// there is no class-level average to take, and unrelated closure members
// must never vote a proven copy below CONTENT_SUPPORT_FLOOR.
#[test]
fn a_byte_identical_pairs_content_evidence_is_never_diluted_by_the_cluster() -> Result<()> {
    let report = run_pair_mean_report()?;

    let mixed = cluster_by_id(&report, MIXED_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("mixed cluster {MIXED_CLUSTER_ID} missing: {report:#}"))?;
    let pair = cluster_by_id(&report, PAIR_CLUSTER_ID)
        .ok_or_else(|| anyhow::anyhow!("pair cluster {PAIR_CLUSTER_ID} missing: {report:#}"))?;

    // The pair alone: byte-identical files agree completely on content.
    assert_eq!(
        signal(pair, "pair_agreement").to_bits(),
        VERBATIM_PAIR_AGREEMENT.to_bits(),
        "the byte-identical pair's own agreement must be {VERBATIM_PAIR_AGREEMENT}: {dump}",
        dump = signal_dump(pair)
    );

    // The mixed cluster's elected occurrence pair owns every rendered axis. The
    // separate two-member cluster covers a different AST range in the same files, so
    // equating its rename-anchor mass would compare different occurrence pairs.
    let (left_path, right_path) = signal_source_paths(mixed)?;
    assert_eq!(
        (left_path.as_str(), right_path.as_str()),
        ("ledger_a.ts", COPY_STEM),
        "the content evidence must cite the same elected pair as the shape evidence"
    );
    assert_eq!(
        signal(mixed, "structural").to_bits(),
        ELECTED_PAIR_EXACT_EVIDENCE.to_bits(),
        "the elected pair is structurally exact: {}",
        signal_dump(mixed)
    );
    assert_eq!(
        signal(mixed, "token_jaccard").to_bits(),
        ELECTED_PAIR_EXACT_EVIDENCE.to_bits(),
        "the elected pair has exact normalized token evidence: {}",
        signal_dump(mixed)
    );
    assert_eq!(
        signal(mixed, "embedding_cos").to_bits(),
        EMBEDDINGS_OFF_EVIDENCE.to_bits(),
        "an absent embedding input must render zero: {}",
        signal_dump(mixed)
    );
    assert_eq!(
        signal(mixed, "pair_agreement").to_bits(),
        ELECTED_PAIR_EXACT_EVIDENCE.to_bits(),
        "cluster members must not dilute the elected pair's byte agreement: {}",
        signal_dump(mixed)
    );
    assert_eq!(
        signal(mixed, "pair_rename_consistency").to_bits(),
        ELECTED_PAIR_EXACT_EVIDENCE.to_bits(),
        "the elected pair's certified rename evidence must remain attached to that pair: {}",
        signal_dump(mixed)
    );
    assert_eq!(
        cluster_bucket(mixed),
        "nearly_identical",
        "the elected pair's evidence must keep the mixed cluster in its supported bucket"
    );

    // The gate reads these axes, so dilution is a demotion engine: the
    // proven pair must keep the cluster's content support above the
    // floor the gate demotes below.
    let support = signal(mixed, "pair_agreement").max(signal(mixed, "pair_rename_consistency"));
    assert!(
        support >= deslop_core::buckets::CONTENT_SUPPORT_FLOOR,
        "a cluster holding a byte-identical pair must carry content support at or \
         above {floor} — below it the gate demotes a proven copy out of its supported \
         bucket, a false negative manufactured purely by averaging: support={support:.4} \
         {dump}",
        floor = deslop_core::buckets::CONTENT_SUPPORT_FLOOR,
        dump = signal_dump(mixed)
    );
    Ok(())
}
