//! Fused-signal assertion vocabulary shared by the golden confidence
//! suites ([FUSION-STRATEGY-BOUNDED-MAX], [FUSION-CONTENT-GATE],
//! [FUSED-THRESHOLD]).
//!
//! The bands, the honest shape-only bucket set, and the diagnostic dump
//! are the same contract in every suite that asserts on `signals.fused`,
//! so they live here rather than being restated per binary.

use std::{collections::BTreeSet, path::Path};

use anyhow::anyhow;
use deslop_core::buckets::{SATURATING_TOKEN_FLOOR, STRUCTURAL_ONLY_MAX_SUPPORT};
use serde_json::Value;

use super::{
    approx, cluster_bucket, cluster_file_set, cluster_size, clusters, field, occurrence_texts,
    occurrences, signal, Result,
};

/// The agent-facing act-now line ([FUSED-THRESHOLD]): at or above this a
/// `find-similar` consumer must refuse to write the copy, so nothing but
/// real duplication may reach it.
pub(crate) const ACT_NOW_FUSED: f64 = 0.85;

/// The agent-facing reuse-bias line (`docs/snippets/agents-md-recipe.md`):
/// below this the recipe tells the agent to author the code outright, so
/// a genuine clone that lands here is a false negative at the agent
/// surface even when the report still lists it.
pub(crate) const REUSE_FUSED: f64 = 0.6;

/// Buckets that describe a shape-only match without claiming the
/// *content* is duplicated ([RANK-STRUCTURAL-ONLY]).
pub(crate) const HONEST_SHAPE_ONLY_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// Buckets a cluster may carry once it has reached the act-now line.
pub(crate) const ACT_NOW_BUCKETS: [&str; 3] = ["identical", "nearly_identical", "same_behavior"];

/// Asserts the full `structural_only` contract
/// ([RANK-STRUCTURAL-ONLY], [FUSION-CONTENT-GATE]).
///
/// A cluster reaches this bucket by one of **two** routes, and a test
/// that admits only one pins an implementation instead of the contract:
///
/// - **evidence-free** — token and embedding support below
///   `STRUCTURAL_ONLY_MAX_SUPPORT`. Shape is the only signal, which is
///   the #197 REST-family shape.
/// - **content-gated** — shape evidence saturates while the measured
///   content evidence refuses to vouch for it. The token layer may read
///   anywhere in that case: it saturates when it echoes the structural
///   pass's own normalised representation, and it can sit mid-band when
///   the fingerprint-scoped fallback cost it part of its signature.
///
/// So `token_jaccard` alone identifies neither route. What both routes
/// *do* share is the precondition that admits them:
/// `has_saturating_shape_evidence` — a saturated structural fingerprint
/// or a saturated token echo. The evidence-free route additionally
/// requires `structural >= 0.99`, and the content-gated route is only
/// reachable through that same predicate, so it holds universally and
/// catches a mislabelled low-shape cluster the old band admitted.
///
/// An earlier revision asserted that `token_jaccard` could not land
/// between the two constants. Production emits exactly that: a
/// `structural = 1.00`, mid-token, low-content cluster is demoted by
/// `route_shape_identical` and routes here. Content evidence is not on
/// the report wire, so no helper reading three signals can reconstruct
/// which route ran — scenario tests pin the bucket and metrics instead.
pub(crate) fn assert_structural_only_contract(cluster: &Value, label: &str) {
    let structural = signal(cluster, "structural");
    let token = signal(cluster, "token_jaccard");
    assert!(
        structural >= 0.99 || token >= SATURATING_TOKEN_FLOOR,
        "{label}: both routes into structural_only require saturating shape \
         evidence — structural >= 0.99, or a token echo >= \
         {SATURATING_TOKEN_FLOOR}. Without it the bucket claims a shape match \
         the signals do not show: {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        signal(cluster, "embedding_cos") < STRUCTURAL_ONLY_MAX_SUPPORT,
        "{label}: semantic support disqualifies structural_only on both \
         routes: {dump}",
        dump = signal_dump(cluster)
    );
    assert!(
        signal(cluster, "fused") < ACT_NOW_FUSED,
        "{label}: structural_only is a demoted verdict and must stay below \
         the act-now line: {dump}",
        dump = signal_dump(cluster)
    );
}

/// One-line dump of everything an accuracy assertion needs to explain
/// itself, so a failure names the actual numbers instead of restating
/// the expectation.
pub(crate) fn signal_dump(cluster: &Value) -> String {
    format!(
        "id={id} bucket={bucket} category={category} size={size} weight={weight:.3} \
         structural={structural:.4} token_jaccard={token:.4} embedding_cos={embedding:.4} \
         fused={fused:.4} files={files:?}",
        id = field(cluster, "id").as_str().unwrap_or("?"),
        bucket = cluster_bucket(cluster),
        category = field(cluster, "category").as_str().unwrap_or("?"),
        size = cluster_size(cluster),
        weight = field(cluster, "weight").as_f64().unwrap_or_default(),
        structural = signal(cluster, "structural"),
        token = signal(cluster, "token_jaccard"),
        embedding = signal(cluster, "embedding_cos"),
        fused = signal(cluster, "fused"),
        files = cluster_file_set(cluster),
    )
}

/// Zero-based rank of `cluster` inside the ranked report, resolved by
/// identity so two clusters with equal weights can never be confused.
pub(crate) fn rank_of(report: &Value, cluster: &Value) -> Result<usize> {
    clusters(report)
        .iter()
        .position(|candidate| std::ptr::eq(candidate, cluster))
        .ok_or_else(|| anyhow!("cluster is missing from the ranked report: {report:#}"))
}

/// The distinct source slices a cluster's occurrences point at. A
/// single-element set proves byte-identical duplication; anything larger
/// proves the raw content differs.
pub(crate) fn distinct_texts(scan_root: &Path, cluster: &Value) -> Result<BTreeSet<String>> {
    Ok(occurrence_texts(scan_root, cluster)?.into_iter().collect())
}

/// True when at least two of a cluster's occurrences are byte-identical
/// — the proof [FUSION-CONTENT-GATE]'s verbatim guard relies on when it
/// vouches full agreement for a cluster that is not wholly identical.
pub(crate) fn has_verbatim_pair(scan_root: &Path, cluster: &Value) -> Result<bool> {
    let texts = occurrence_texts(scan_root, cluster)?;
    Ok(distinct_texts(scan_root, cluster)?.len() < texts.len())
}

/// Asserts the full **proven-rename** contract ([FUSION-CONTENT-GATE],
/// [TECH-PMATCH-BAKER], `[REPAIR-RENAME-ANCHOR-MASS]`) — the mirror of
/// [`assert_structural_only_contract`], and the reason both live here.
///
/// The two contracts describe the same signal triple. A maximal Type-2
/// rename and an anchor-poor scaffolding family both render
/// `structural = 1.00, token_jaccard = 1.00`: the token LSH pass hashes
/// the normalised representation the structural pass already collapsed,
/// so neither deterministic axis can tell them apart. Only the measured
/// content evidence separates them, and it is not on the report wire
/// (#344) — so a suite asserting one verdict is really asserting *which
/// side of the content gate* the fixture falls on. Stating both
/// contracts once, here, is what stops two suites drifting into
/// asserting opposite verdicts about the same evidence, which is exactly
/// what the pre-`[REPAIR-RENAME-ANCHOR-MASS]` literal-anchor cliff
/// produced.
///
/// Demotion is the failure mode this guards: a renamed copy of real
/// logic that lands in `HONEST_SHAPE_ONLY_BUCKETS`, or below
/// [`REUSE_FUSED`], is a false negative at the agent surface — the
/// recipe tells the agent to write the copy anyway.
pub(crate) fn assert_proven_rename_contract(
    scan_root: &Path,
    cluster: &Value,
    label: &str,
) -> Result<()> {
    assert_rename_shape(cluster, label);
    assert_rename_verdict(cluster, label);
    assert_rename_is_not_a_copy(scan_root, cluster, label)
}

/// Shape half of the proven-rename contract: identifier normalisation
/// makes a rename structurally identical, and the normalised k-gram
/// stream is rename-invariant by construction.
fn assert_rename_shape(cluster: &Value, label: &str) {
    let dump = signal_dump(cluster);
    assert!(
        approx(signal(cluster, "structural"), 1.0),
        "{label}: identifier normalisation makes a rename structurally \
         identical — {dump}"
    );
    assert!(
        approx(signal(cluster, "token_jaccard"), 1.0),
        "{label}: the normalised k-gram stream is rename-invariant by \
         construction — {dump}"
    );
}

/// Verdict half: the bucket and the fused confidence a rename whose
/// identifier mapping is proven must carry.
fn assert_rename_verdict(cluster: &Value, label: &str) {
    let dump = signal_dump(cluster);
    assert!(
        !HONEST_SHAPE_ONLY_BUCKETS.contains(&cluster_bucket(cluster)),
        "{label}: a Type-2 rename of real logic is duplication, not \
         shape-only evidence — demoting it is a false negative — {dump}"
    );
    assert_eq!(
        cluster_bucket(cluster),
        "nearly_identical",
        "{label}: same shape, same logic, renamed identifiers is the \
         textbook `nearly_identical` clone — {dump}"
    );
    let fused = signal(cluster, "fused");
    assert!(
        fused >= REUSE_FUSED,
        "{label}: a renamed copy of real logic must stay at or above the \
         reuse-bias line ({REUSE_FUSED}) — below it the agent recipe tells \
         the agent to write the copy anyway — {dump}"
    );
    assert!(
        fused < 1.0,
        "{label}: only a byte-identical copy may saturate the confidence \
         — {dump}"
    );
}

/// Occurrence half: every occurrence must differ in raw bytes, or the
/// promotion proves nothing about renames, and none may be hidden — a
/// clone the report will not show is a false negative whatever its
/// bucket says.
fn assert_rename_is_not_a_copy(scan_root: &Path, cluster: &Value, label: &str) -> Result<()> {
    let dump = signal_dump(cluster);
    assert_eq!(
        distinct_texts(scan_root, cluster)?.len(),
        occurrences(cluster).len(),
        "{label}: every occurrence must differ in raw bytes, or this is a \
         Type-1 copy proving nothing about rename evidence — {dump}"
    );
    assert!(
        occurrences(cluster)
            .iter()
            .all(|occurrence| occurrence.get("hidden") != Some(&Value::Bool(true))),
        "{label}: a proven Type-2 clone may not have a hidden occurrence \
         — {dump}"
    );
    Ok(())
}
