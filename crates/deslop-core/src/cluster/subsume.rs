//! Cross-cluster subsumption ([PIPELINE-CLUSTER-EXACT], gh #50).
//!
//! Nested AST subtrees over the same physical code (e.g.
//! `attribute_list + method_declaration` vs. bare `method_declaration`)
//! form separate fused clusters covering the same bytes at different
//! depths. Only one may reach the report; publishing both shows the user
//! the same duplicate twice and double-counts it in `clusters_total` and
//! the duplication metric.
//!
//! **Sameness is occurrence overlap, not strict nesting.** Two clusters
//! describe the same physical duplication when every occurrence of one
//! overlaps some occurrence of the other in the same file — the exact
//! predicate the #50 acceptance test asserts against the rendered report
//! (`no_two_clusters_cover_the_same_physical_bytes`). Requiring strict
//! containment missed the *crossed* case, where the depth difference
//! falls on opposite sides in each file: `ledger_c[0..1238] +
//! ledger_a[0..1234]` and `ledger_c[0..1237] + ledger_a[0..1235]` are
//! two views of one whole-file duplicate, yet neither set nests inside
//! the other, so both were published (gh #343 corpus, pinned by
//! `issue_343_sum_clamp_saturation.rs`).

use crate::fingerprint::Fingerprint;

use super::{Cluster, LOW_STRUCTURAL_TYPE4_CEILING, TYPE4_EMBEDDING_FLOOR};

/// Collapses redundant clusters that cover the same physical bytes.
///
/// Runs after ranking so the weight order is available: `outer` is
/// always the heavier cluster of a pair, and [PIPELINE-CLUSTER-EXACT]
/// keeps the outer view of a duplicated region.
pub(super) fn collapse_cross_cluster_overlap(clusters: Vec<Cluster>) -> Vec<Cluster> {
    let len = clusters.len();
    let mut dropped = vec![false; len];
    for outer in 0..len {
        if !cluster_dropped(&dropped, outer) {
            scan_inner_pairs(&clusters, &mut dropped, outer, len);
        }
    }
    clusters
        .into_iter()
        .enumerate()
        .filter_map(|(index, cluster)| (!cluster_dropped(&dropped, index)).then_some(cluster))
        .collect()
}

/// Decision produced by [`evaluate_pair`] for one `(outer, inner)` pair.
enum PairDecision {
    /// Discard the inner cluster; the outer subsumes it.
    DropInner,
    /// Discard the outer cluster; the inner subsumes it.
    DropOuter,
    /// Retain both clusters.
    Keep,
}

/// Evaluates every `(outer, inner)` pair for the given `outer` index and
/// updates `dropped`. Breaks early when `outer` itself is dropped.
fn scan_inner_pairs(clusters: &[Cluster], dropped: &mut [bool], outer: usize, len: usize) {
    for inner in (outer.saturating_add(1))..len {
        if cluster_dropped(dropped, inner) {
            continue;
        }
        let Some(outer_cluster) = clusters.get(outer) else {
            continue;
        };
        let Some(inner_cluster) = clusters.get(inner) else {
            continue;
        };
        match evaluate_pair(outer_cluster, inner_cluster) {
            PairDecision::DropInner => {
                log_subsumption(outer_cluster, inner_cluster, "drop_inner");
                drop_cluster(dropped, inner);
            }
            PairDecision::DropOuter => {
                log_subsumption(inner_cluster, outer_cluster, "drop_outer");
                drop_cluster(dropped, outer);
                break;
            }
            PairDecision::Keep => {}
        }
    }
}

/// Records which cluster subsumed which, so a surprising collapse is
/// traceable without re-running the pipeline.
fn log_subsumption(survivor: &Cluster, discarded: &Cluster, decision: &'static str) {
    tracing::debug!(
        decision,
        survivor = survivor.id.as_str(),
        survivor_size = survivor.members.len(),
        survivor_structural = survivor.signals.structural,
        discarded = discarded.id.as_str(),
        discarded_size = discarded.members.len(),
        discarded_structural = discarded.signals.structural,
        "cross-cluster subsumption",
    );
}

/// Decides which cluster survives when their occurrences cover the same
/// bytes. Returns [`PairDecision::Keep`] when neither dominates.
fn evaluate_pair(outer: &Cluster, inner: &Cluster) -> PairDecision {
    if all_occurrences_overlap_some(&inner.members, &outer.members) {
        resolve_outer_dominant(outer, inner)
    } else if all_occurrences_overlap_some(&outer.members, &inner.members) {
        PairDecision::DropOuter
    } else {
        PairDecision::Keep
    }
}

/// Chooses the survivor when every occurrence of `inner` lies over an
/// occurrence of `outer`.
///
/// - Equal or better structural on the outer → drop inner (outer is
///   heavier and at least as precise).
/// - Outer is embedding-dominant → drop inner. The outer carries
///   semantic evidence the inner does not, and the region it covers
///   already includes the inner's, so keeping both would republish the
///   same bytes.
/// - Otherwise the inner is the higher-quality view: drop the outer,
///   but only when the outer names no file the inner leaves out. That
///   guard preserves cross-language clusters — a `cs + rs + py` outer
///   must survive a `cs`-only inner with higher structural, because it
///   conveys duplication the inner cannot express.
fn resolve_outer_dominant(outer: &Cluster, inner: &Cluster) -> PairDecision {
    if outer.signals.structural >= inner.signals.structural
        || embedding_outer_should_survive(outer, inner)
    {
        PairDecision::DropInner
    } else if outer_files_covered_by_inner(&inner.members, &outer.members) {
        PairDecision::DropOuter
    } else {
        PairDecision::Keep
    }
}

/// Returns true when a small structural cluster must not erase a larger
/// embedding-dominant cluster that carries distinct semantic evidence.
fn embedding_outer_should_survive(outer: &Cluster, inner: &Cluster) -> bool {
    is_embedding_dominant(outer.signals) && inner.signals.structural > outer.signals.structural
}

/// Returns true for low-structural clusters created by the embedding pass.
fn is_embedding_dominant(signals: crate::pair::PairScore) -> bool {
    signals.structural < LOW_STRUCTURAL_TYPE4_CEILING
        && signals.embedding_cos >= TYPE4_EMBEDDING_FLOOR
}

/// Returns `true` when two occurrences share a file and their byte
/// ranges intersect.
fn occurrences_overlap(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.file_id == right.file_id
        && left.byte_range.start < right.byte_range.end
        && right.byte_range.start < left.byte_range.end
}

/// Returns `true` when every occurrence in `covered` overlaps at least
/// one occurrence in `cover` — the "same physical bytes" test of #50.
fn all_occurrences_overlap_some(covered: &[Fingerprint], cover: &[Fingerprint]) -> bool {
    !covered.is_empty()
        && covered.iter().all(|candidate| {
            cover
                .iter()
                .any(|other| occurrences_overlap(other, candidate))
        })
}

/// Returns `true` when every file mentioned in `outer_set` is also
/// mentioned in `inner_set`. When this is false the outer cluster covers
/// additional files (e.g. cross-language) the inner does not, so the
/// outer must not be dropped.
fn outer_files_covered_by_inner(inner_set: &[Fingerprint], outer_set: &[Fingerprint]) -> bool {
    outer_set.iter().all(|outer_fp| {
        inner_set
            .iter()
            .any(|inner_fp| inner_fp.file_id == outer_fp.file_id)
    })
}

/// Returns `true` when `index` is already marked for removal.
fn cluster_dropped(dropped: &[bool], index: usize) -> bool {
    dropped.get(index).copied().unwrap_or(true)
}

/// Marks `index` for removal when the slot exists.
fn drop_cluster(dropped: &mut [bool], index: usize) {
    if let Some(slot) = dropped.get_mut(index) {
        *slot = true;
    }
}
