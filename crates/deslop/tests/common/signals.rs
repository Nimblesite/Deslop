//! Fused-signal assertion vocabulary shared by the golden confidence
//! suites ([FUSED-STRATEGY-BOUNDED-MAX], [FUSED-CONTENT-GATE],
//! [FUSED-THRESHOLD]).
//!
//! The bands, the honest shape-only bucket set, and the diagnostic dump
//! are the same contract in every suite that asserts on `signals.fused`,
//! so they live here rather than being restated per binary.

use std::{collections::BTreeSet, path::Path};

use anyhow::anyhow;
use deslop_core::{
    buckets::{
        CONTENT_PROMOTE_FLOOR, CONTENT_SUPPORT_FLOOR, SATURATING_TOKEN_FLOOR,
        STRUCTURAL_ONLY_MAX_SUPPORT,
    },
    pair::EMBEDDING_SUPPORT_FLOOR,
};
use serde_json::Value;

use super::{
    approx, cluster_bucket, cluster_file_set, cluster_size, clusters, field, occurrence_is_hidden,
    occurrence_texts, occurrences, signal, Result,
};

/// The top reported confidence band ([FUSED-THRESHOLD]): at or above this
/// the report states maximal measured evidence, so nothing but real
/// duplication may reach it.
pub(crate) const ACT_NOW_FUSED: f64 = 0.85;

/// The lower band boundary ([FUSED-THRESHOLD]): below this the report
/// states weak evidence, so a genuine clone that lands here understates
/// what was measured even when the report still lists it.
pub(crate) const REUSE_FUSED: f64 = 0.6;

/// Buckets that describe a shape-only match without claiming the
/// *content* is duplicated ([RANK-STRUCTURAL-ONLY]).
pub(crate) const HONEST_SHAPE_ONLY_BUCKETS: [&str; 2] = ["structural_only", "loosely_similar"];

/// The only bucket a byte-identical Type-1 copy may honestly carry
/// ([CLONE-BUCKETS-ROUTING]). Named because the noise pins need to say
/// "exactly this", not "one of the act-now three": their controls are
/// copied byte for byte, so every other act-now label claims the copies
/// differ somewhere they do not.
pub(crate) const IDENTICAL_BUCKET: &str = "identical";

/// Buckets a cluster may carry once it has reached the act-now line.
pub(crate) const ACT_NOW_BUCKETS: [&str; 3] =
    [IDENTICAL_BUCKET, "nearly_identical", "same_behavior"];

/// Asserts the full `structural_only` contract
/// ([RANK-STRUCTURAL-ONLY], [FUSED-CONTENT-GATE]).
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
/// `route_shape_identical` and routes here.
///
/// Since #344 the measured content evidence *is* on the report wire, so
/// this helper no longer has to guess which door a cluster came through
/// — [`assert_reached_a_real_route`] checks each door by its own entry
/// condition.
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
        signal(cluster, "embedding_cos") < EMBEDDING_SUPPORT_FLOOR,
        "{label}: a cosine that *vouches* for the cluster escapes the \
         content gate entirely (`route_shape_identical`), so it can never \
         co-exist with this bucket: {dump}",
        dump = signal_dump(cluster)
    );
    assert_reached_a_real_route(cluster, label);
    assert!(
        signal(cluster, "fused") < ACT_NOW_FUSED,
        "{label}: structural_only is a demoted verdict and must stay below \
         the act-now line: {dump}",
        dump = signal_dump(cluster)
    );
}

/// Asserts the cluster satisfies the entry condition of **one of the two
/// real doors** into `structural_only`, rather than merely wearing the
/// label ([CLONE-BUCKETS-ROUTING]).
///
/// This is the assertion the helper could not make until #344. Its doc
/// used to record that "content evidence is not on the report wire, so
/// no helper reading three signals can reconstruct which route ran", and
/// it stood in a single blanket bound — `embedding_cos <
/// STRUCTURAL_ONLY_MAX_SUPPORT` — for both. That bound is only route
/// 1's. Route 2 demotes on *content*, and `route_shape_identical` lets
/// it hold any cosine short of [`EMBEDDING_SUPPORT_FLOOR`], so the
/// blanket bound asserted a property the engine never promised: a
/// content-gated cluster at cosine 0.61 is a correct `structural_only`
/// and the old assertion called it a defect. It never fired only because
/// its one caller runs with embeddings off. `agreement` and
/// `rename_consistency` are now on the wire, so each door is checked by
/// its own entry condition instead.
fn assert_reached_a_real_route(cluster: &Value, label: &str) {
    let evidence_free = signal(cluster, "token_jaccard") < STRUCTURAL_ONLY_MAX_SUPPORT
        && signal(cluster, "embedding_cos") < STRUCTURAL_ONLY_MAX_SUPPORT;
    // `route_shape_identical` promotes back out of the demoted tier at
    // the Type-3 overlap cutoff when the cluster spans files, and at the
    // near-total-agreement bar when it does not — the #197 in-file
    // sibling families measure 0.72–0.80 and are API surface, not
    // extractable duplication.
    let promote_floor = if cluster_file_set(cluster).len() > 1 {
        CONTENT_SUPPORT_FLOOR
    } else {
        CONTENT_PROMOTE_FLOOR
    };
    let support = signal(cluster, "agreement").max(signal(cluster, "rename_consistency"));
    assert!(
        evidence_free || support < promote_floor,
        "{label}: structural_only requires one of the two documented \
         routes — evidence-free (token and embedding both below \
         {STRUCTURAL_ONLY_MAX_SUPPORT}), or content-gated (measured \
         support below {promote_floor}). This cluster satisfies neither, \
         so the bucket claims a demotion the evidence does not support \
         — a promoted clone wearing a demoted label is a false negative: \
         support={support:.4} {dump}",
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
/// — the proof [FUSED-CONTENT-GATE]'s verbatim guard relies on when it
/// vouches full agreement for a cluster that is not wholly identical.
pub(crate) fn has_verbatim_pair(scan_root: &Path, cluster: &Value) -> Result<bool> {
    let texts = occurrence_texts(scan_root, cluster)?;
    Ok(distinct_texts(scan_root, cluster)?.len() < texts.len())
}

/// Asserts the full **proven-rename** contract ([FUSED-CONTENT-GATE],
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

/// The proven-rename contract for a fixture that is a rename **plus an
/// inserted statement**. Verdict and not-a-copy are identical; only the
/// shape half differs, because the reported view spans the whole
/// declaration and therefore includes the insertion
/// ([FUSED-SHARED-SUBTREE], gh #408). Demanding Merkle exactness there
/// demands the fragment view — the shared sub-range either side of the
/// insertion — which is the recall hole #408 is filed against.
pub(crate) fn assert_near_miss_rename_contract(
    scan_root: &Path,
    cluster: &Value,
    label: &str,
) -> Result<()> {
    assert_near_miss_rename_shape(cluster, label);
    assert_rename_verdict(cluster, label);
    assert_rename_is_not_a_copy(scan_root, cluster, label)
}

/// Shape half for a rename carrying an inserted statement: real shape
/// evidence, bounded away from the Merkle exactness a near-miss cannot
/// have, with the rename-invariant token stream still corroborating.
fn assert_near_miss_rename_shape(cluster: &Value, label: &str) {
    let dump = signal_dump(cluster);
    let structural = signal(cluster, "structural");
    assert!(
        structural >= deslop_core::pair::SHARED_SUBTREE_MIN_OVERLAP,
        "{label}: identifier normalisation makes the shared body structurally \
         identical, so the enclosing view must clear the shared-subtree floor \
         — {dump}"
    );
    assert!(
        structural < 1.0,
        "{label}: the inserted statement is inside the reported view, so the \
         pair cannot be Merkle-exact; exactness here means the report fell back \
         to the fragment view (gh #408) — {dump}"
    );
    assert!(
        signal(cluster, "token_jaccard") >= deslop_core::pair::SHARED_SUBTREE_MIN_JACCARD,
        "{label}: the normalised k-gram stream is rename-invariant, so tokens \
         must still corroborate — {dump}"
    );
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
         lower band boundary ({REUSE_FUSED}) — below it the report states \
         weak evidence — {dump}"
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
        !occurrences(cluster).iter().any(occurrence_is_hidden),
        "{label}: a proven Type-2 clone may not have a hidden occurrence \
         — {dump}"
    );
    Ok(())
}
