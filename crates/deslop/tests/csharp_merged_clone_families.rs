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

use std::path::Path;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
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
/// One occurrence per file in each family.
const FAMILY_OCCURRENCES: u64 = 2;

/// Two families, two clusters. Anything else means one absorbed the other.
const EXPECTED_CLUSTERS: usize = 2;

/// The summing pair renames nothing away: with every identifier mapped
/// one-to-one the substance the two bodies share is only what survives
/// the rename, so its agreement sits well under the multiplying pair's.
/// The multiplying pair renames nothing at all, so almost every byte of
/// substance is shared.
/// Nothing is hidden: every view of this corpus is either published or
/// elected away by a view of the same region.
/// This was one. A four-occurrence `for`-loop family spanned all four
/// files, because `total = total + index` and `product = product *
/// factor` normalised to the same subtree while operators collapsed to
/// a shared placeholder ([PIPELINE-NORMALIZE-AST-OPERATOR]). Spanning
/// both families, it nested inside neither method cluster and could
/// not be elected against either, so it survived to be hidden. With
/// each operator leaf carrying its own token the four-way view does not
/// exist: it is two two-occurrence views, each nested inside its own
/// method cluster and elected away there.
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

    let scan_root = fixture(FIXTURE);
    let summing = expect_cluster_spanning(&report, &[ALPHA, BETA])?;
    let multiplying = expect_cluster_spanning(&report, &[DELTA, GAMMA])?;
    assert_families_were_elected_apart(&report, &visible);
    assert_near_identical_pair(&scan_root, summing, ALPHA, BETA, &report)?;
    assert_near_identical_pair(&scan_root, multiplying, DELTA, GAMMA, &report)?;
    assert_content_axes_separate_strictly(&scan_root, summing, multiplying, &report)?;
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
fn assert_near_identical_pair(
    scan_root: &Path,
    family: &Value,
    left: &str,
    right: &str,
    report: &Value,
) -> Result<()> {
    assert_eq!(
        cluster_size(family),
        FAMILY_OCCURRENCES,
        "{left}/{right} is a two-occurrence family: {report:#}"
    );
    // [PIPELINE-CLUSTER-CLOSURE] The bucket and content axes are pair-scoped
    // now. The wire facts that hold the acceptance: the family is admitted,
    // mass-honest, clean-surfaced, and a byte-distinct rename (identifier
    // renames change the bytes — a verbatim reading would be a fabrication).
    assert_structural_only_contract(family, "csharp merged families");
    assert_no_pair_surface_on_cluster(family, "csharp merged families");
    assert!(
        !has_verbatim_pair(scan_root, family)?,
        "{left}/{right} are renames / literal edits and must slice to \
         differing bytes: {report:#}"
    );
    Ok(())
}

/// The two families are copies for opposite reasons, and the wire must
/// keep them apart: they are two clusters with distinct ids, and the
/// report never claims either is a verbatim copy of the other.
fn assert_content_axes_separate_strictly(
    scan_root: &Path,
    summing: &Value,
    multiplying: &Value,
    report: &Value,
) -> Result<()> {
    assert_ne!(
        cluster_id(summing),
        cluster_id(multiplying),
        "the summing and multiplying families are different code and must \
         keep different cluster ids: {report:#}"
    );
    assert!(
        !has_verbatim_pair(scan_root, summing)?,
        "the summing family is a rename and must be byte-distinct: {report:#}"
    );
    assert!(
        !has_verbatim_pair(scan_root, multiplying)?,
        "the multiplying family is an edit and must be byte-distinct: {report:#}"
    );
    Ok(())
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
