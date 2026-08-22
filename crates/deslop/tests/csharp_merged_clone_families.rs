//! E2E regression for [PIPELINE-CLUSTER-ELECT] — one cluster may not
//! swallow two unrelated clone families and take both down with it.
//!
//! The `csharp-mcp` corpus holds two independent Type-2 pairs. Alpha's
//! `Compute` and Beta's `Run` are one summing loop written twice with
//! every identifier renamed. Delta's `Times` and Gamma's `Times` are one
//! multiplying loop that differs by a single literal. Scan either pair on
//! its own and the detector reports it `nearly_identical` at the shipped
//! `--min-nodes` default.
//!
//! Scan all four together and both vanish. The band collision puts the
//! sum and the product in the *same* cluster; the merged four-member
//! cluster then has `agreement = 0.31` and `rename_consistency = 0.33`,
//! because half its members multiply where the other half add, so it
//! buckets `loosely_similar` and the report policy hides it. Two real
//! duplicates are lost to the presence of each other — a false negative
//! that grows with the size of the corpus, which is the direction that
//! matters.
//!
//! The disagreement itself is correct: `+` and `*` are behaviour, and
//! [PIPELINE-NORMALIZE-AST-OPERATOR] exists so the content frontier can
//! see it. The defect is the response to it. A cluster whose members
//! split cleanly into subgroups that *do* agree must be reported as
//! those subgroups, exactly as [CLONE-NOISE-VERBATIM-SUBGROUP] already
//! partitions a proven copy out of a noise family. Dropping the union is
//! the one outcome that loses information the pipeline had.
//!
//! Both families live in ONE scan on purpose: a detector that reported
//! them only in isolation would pass a two-file test and still ship this
//! bug.

mod common;

use crate::common::*;
use serde_json::Value;

/// The four-file C# corpus, shared with `deslop-mcp`'s transport suite.
const FIXTURE: &str = "csharp-mcp";

/// The shipped `--min-nodes` default. The defect must be pinned at the
/// setting users actually run: a lower floor changes which subtrees are
/// eligible and would let the test pass against unfixed code.
const MIN_NODES: u32 = 30;

/// Every `.cs` file in the corpus.
const FILES_ANALYSED: u64 = 4;

/// Canonical half of the summing pair.
const ALPHA: &str = "Alpha.cs";

/// The summing pair's copy, with every identifier renamed.
const BETA: &str = "Beta.cs";

/// Canonical half of the multiplying pair.
const DELTA: &str = "Delta.cs";

/// The multiplying pair's copy, differing by one loop-start literal.
const GAMMA: &str = "Gamma.cs";

/// The bucket a consistent rename of the same statement shape lands in.
const NEARLY_IDENTICAL: &str = "nearly_identical";

/// One occurrence per file in each family.
const FAMILY_OCCURRENCES: u64 = 2;

/// Two families, two clusters. Anything else means one absorbed the other.
const EXPECTED_CLUSTERS: usize = 2;

/// Both families are near-identical clones, so both must reach the band
/// that tells a reader to act rather than merely to look.
const ACT_NOW_FUSED: f64 = 0.85;

/// The summing pair renames nothing away: with every identifier mapped
/// one-to-one the substance the two bodies share is only what survives
/// the rename, so its agreement sits well under the multiplying pair's.
const RENAMED_PAIR_MAX_AGREEMENT: f64 = 0.75;

/// The multiplying pair renames nothing at all, so almost every byte of
/// substance is shared.
const LITERAL_PAIR_MIN_AGREEMENT: f64 = 0.9;

/// Nothing is hidden: every view of this corpus is either published or
/// elected away by a view of the same region.
///
/// This was one. A four-occurrence `for`-loop family spanned all four
/// files, because `total = total + index` and `product = product *
/// factor` normalised to the same subtree while operators collapsed to
/// a shared placeholder ([PIPELINE-NORMALIZE-AST-OPERATOR]). Spanning
/// both families, it nested inside neither method cluster and could
/// not be elected against either, so it survived to be hidden. With
/// each operator leaf carrying its own token the four-way view does not
/// exist: it is two two-occurrence views, each nested inside its own
/// method cluster and elected away there.
///
/// Zero is the stronger pin, and the assertion's purpose is unchanged —
/// anything hidden here is a family suppressed rather than elected.
const HIDDEN_CLUSTERS: u64 = 0;

#[test]
fn two_clone_families_in_one_corpus_do_not_erase_each_other() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    let visible = visible_cluster_lines(&report);

    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED),
        "every fixture file must be parsed — a scan that reached none of \
         them reports no duplication for a reason this test is not \
         about: {report:#}"
    );

    let summing = expect_cluster_spanning(&report, &[ALPHA, BETA])?;
    let multiplying = expect_cluster_spanning(&report, &[DELTA, GAMMA])?;
    assert_families_were_elected_apart(&report, &visible);
    assert_near_identical_pair(summing, ALPHA, BETA, &report);
    assert_near_identical_pair(multiplying, DELTA, GAMMA, &report);
    assert_content_axes_separate_strictly(summing, multiplying, &report);
    assert_headline_counts_both_families(&report);
    Ok(())
}

/// The election itself: two clusters, neither spanning both families.
fn assert_families_were_elected_apart(report: &Value, visible: &[String]) {
    assert!(
        cluster_spanning(report, &[ALPHA, DELTA]).is_none(),
        "a summing loop and a multiplying loop are not occurrences of \
         one another; a cluster spanning them is the merge that hides \
         both families: {visible:#?}"
    );
    assert_eq!(
        cluster_count(report),
        EXPECTED_CLUSTERS,
        "the corpus holds exactly two clone families: {visible:#?}"
    );
    assert_eq!(
        clusters_hidden(report),
        HIDDEN_CLUSTERS,
        "every view of this corpus is published or elected away by a \
         view of the same region; a hidden view means a family was \
         suppressed rather than elected: {visible:#?}"
    );
}

/// What both families must hold regardless of why they are copies.
fn assert_near_identical_pair(family: &Value, left: &str, right: &str, report: &Value) {
    assert_eq!(
        cluster_bucket(family),
        NEARLY_IDENTICAL,
        "{left}/{right} is the same statement shape with the same \
         behaviour, which is the definition of nearly-identical: \
         {report:#}"
    );
    assert_eq!(
        cluster_size(family),
        FAMILY_OCCURRENCES,
        "{left}/{right} is a two-occurrence family: {report:#}"
    );
    assert!(
        approx(signal(family, "structural"), 1.0),
        "{left}/{right} share one normalised subtree — identifier \
         renames and literal edits are invisible to it: {report:#}"
    );
    assert!(
        approx(signal(family, "token_jaccard"), 1.0),
        "the token layer is rename-invariant by design, so \
         {left}/{right} saturate it: {report:#}"
    );
    assert!(
        signal(family, "fused") >= ACT_NOW_FUSED,
        "{left}/{right} must keep the rank a near-identical clone \
         earns, not be diluted by an unrelated family sharing the \
         corpus: {report:#}"
    );
}

/// The two families are copies for opposite reasons, and the report must
/// say so in opposite directions. One floor asserted over both would pass
/// against an engine that had flattened them to a single undifferentiated
/// score — the failure mode that produced the merged cluster.
fn assert_content_axes_separate_strictly(summing: &Value, multiplying: &Value, report: &Value) {
    assert!(
        approx(signal(summing, "rename_consistency"), 1.0),
        "{ALPHA}/{BETA} map every identifier one-to-one — `input` to \
         `limit`, `total` to `accumulator`, `index` to `position` — and \
         a total consistent rename is certified, not estimated: \
         {report:#}"
    );
    assert!(
        signal(summing, "agreement") <= RENAMED_PAIR_MAX_AGREEMENT,
        "{ALPHA}/{BETA} share only the substance a total rename leaves \
         behind, so their agreement must stay well under the pair that \
         renames nothing: {report:#}"
    );
    assert!(
        signal(multiplying, "agreement") >= LITERAL_PAIR_MIN_AGREEMENT,
        "{DELTA}/{GAMMA} differ by a single loop-start literal and \
         nothing else, so nearly all their substance is shared: \
         {report:#}"
    );
    assert!(
        signal(multiplying, "rename_consistency") < 1.0,
        "[REPAIR-RENAME-LITERAL-ECHO] — {DELTA}/{GAMMA} rename no \
         identifier and change one literal, so the rename story is \
         incomplete and must not certify as total: {report:#}"
    );
    assert!(
        signal(summing, "rename_consistency") > signal(multiplying, "rename_consistency")
            && signal(multiplying, "agreement") > signal(summing, "agreement"),
        "the two families must separate strictly and in opposite \
         directions on the two content axes; equal values would mean \
         the report cannot tell a renamed copy from an edited one: \
         {report:#}"
    );
}

/// The repo-wide figures must count what the election restored.
fn assert_headline_counts_both_families(report: &Value) {
    assert!(
        visible_duplicated_loc(report) > 0,
        "four methods forming two copied pairs duplicate real lines: \
         {report:#}"
    );
    assert!(
        metric_field(report, "duplication_percent")
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "the headline figure must count both surviving families: \
         {report:#}"
    );
}
