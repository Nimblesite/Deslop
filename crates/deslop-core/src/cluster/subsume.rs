//! Cross-cluster subsumption ([PIPELINE-CLUSTER-EXACT]).
//!
//! Nested AST subtrees over the same physical code (e.g.
//! `attribute_list + method_declaration` vs. bare `method_declaration`)
//! form separate fused clusters covering the same bytes at different
//! depths. Only one may reach the report; publishing both shows the user
//! the same duplicate twice and double-counts it in `clusters_total` and
//! the duplication metric.
//!
//! **Two questions, two predicates.**
//!
//! *Are these one duplication?* Occurrence overlap, not strict nesting:
//! every occurrence of one cluster overlaps some occurrence of the other
//! in the same file. Requiring containment here misses the *crossed*
//! case, where the depth difference falls on opposite sides in each
//! file: `ledger_c[0..1238] + ledger_a[0..1234]` and `ledger_c[0..1237]
//! + ledger_a[0..1235]` are two views of one whole-file duplicate, yet
//! neither set nests inside the other.
//!
//! *Which view survives?* Physical enclosure, not ranking weight. A
//! whole-method clone and the run of single-statement clones inside it
//! cover the same bytes in both directions, and the fine-grained view
//! always ranks heavier because it contributes one occurrence per
//! statement. Choosing by weight therefore rendered a duplicated
//! 60-statement method as 120 one-line occurrences and dropped the
//! method itself — the only extractable duplicate in the corpus,
//! reported as unactionable line noise. The enclosing view is the
//! duplication; the nested view re-describes it.

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
/// bytes. Returns [`PairDecision::Keep`] when they are separate regions.
fn evaluate_pair(outer: &Cluster, inner: &Cluster) -> PairDecision {
    if !covers_same_region(outer, inner) {
        return PairDecision::Keep;
    }
    if strictly_encloses(&inner.members, &outer.members) {
        return match preferred_view(inner, outer) {
            Preference::First => PairDecision::DropOuter,
            Preference::Second => PairDecision::DropInner,
        };
    }
    match preferred_view(outer, inner) {
        Preference::First => PairDecision::DropInner,
        Preference::Second => PairDecision::DropOuter,
    }
}

/// Which of two clusters covering one region reaches the report.
enum Preference {
    /// The first (proposed) view survives.
    First,
    /// The second view overturns it.
    Second,
}

/// Returns `true` when the two clusters are views of one duplicated
/// region: every occurrence of at least one lies over an occurrence of
/// the other.
fn covers_same_region(outer: &Cluster, inner: &Cluster) -> bool {
    all_occurrences_overlap_some(&inner.members, &outer.members)
        || all_occurrences_overlap_some(&outer.members, &inner.members)
}

/// Chooses between two views of one region. `proposed` is the view the
/// caller nominated — the enclosing one where nesting decides, the
/// heavier one where neither set nests.
///
/// The nomination stands unless `other` is strictly more precise and
/// names every file `proposed` names. That guard preserves
/// cross-language clusters: a `cs + rs + py` view must survive a
/// `cs`-only rival with higher structural, because it conveys
/// duplication the rival cannot express. An embedding-dominant
/// nomination also stands — it carries semantic evidence over the same
/// bytes that a structural rival does not.
fn preferred_view(proposed: &Cluster, other: &Cluster) -> Preference {
    let other_is_more_precise = other.signals.structural > proposed.signals.structural;
    if other_is_more_precise
        && !is_embedding_dominant(proposed.signals)
        && covers_every_file(&other.members, &proposed.members)
    {
        Preference::Second
    } else {
        Preference::First
    }
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
/// one occurrence in `cover` — the "same physical bytes" test.
fn all_occurrences_overlap_some(covered: &[Fingerprint], cover: &[Fingerprint]) -> bool {
    !covered.is_empty()
        && covered.iter().all(|candidate| {
            cover
                .iter()
                .any(|other| occurrences_overlap(other, candidate))
        })
}

/// Returns `true` when one occurrence wholly contains another in the
/// same file.
fn occurrence_contains(outer: &Fingerprint, inner: &Fingerprint) -> bool {
    outer.file_id == inner.file_id
        && outer.byte_range.start <= inner.byte_range.start
        && inner.byte_range.end <= outer.byte_range.end
}

/// Returns `true` when every occurrence in `nested` lies wholly inside
/// an occurrence in `enclosing`, and `enclosing` reaches beyond it.
///
/// The second half is what makes the relation strict. Identical
/// occurrence sets nest both ways, and treating those as enclosure would
/// make the survivor depend on which cluster the scan reached first.
fn strictly_encloses(enclosing: &[Fingerprint], nested: &[Fingerprint]) -> bool {
    !enclosing.is_empty()
        && !nested.is_empty()
        && nested
            .iter()
            .all(|inner| enclosing.iter().any(|outer| occurrence_contains(outer, inner)))
        && enclosing
            .iter()
            .any(|outer| !nested.iter().any(|inner| occurrence_contains(inner, outer)))
}

/// Returns `true` when every file mentioned in `required` is also
/// mentioned in `candidate`. When this is false the cluster under threat
/// covers files (e.g. cross-language) the survivor does not, so dropping
/// it would erase duplication no other cluster reports.
fn covers_every_file(candidate: &[Fingerprint], required: &[Fingerprint]) -> bool {
    required.iter().all(|needed| {
        candidate
            .iter()
            .any(|present| present.file_id == needed.file_id)
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
