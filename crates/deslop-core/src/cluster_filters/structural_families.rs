//! [PIPELINE-CLUSTER-ELECT] — a token bridge may not fuse two distinct
//! structural families into one cluster that then reports neither.
//!
//! Clusters are the connected components of the candidate-pair graph
//! ([`crate::pair::cluster_by_transitive_closure`]), and that graph
//! carries two kinds of edge. A structural edge joins members whose
//! normalised subtrees hash to the same value — the same code, up to the
//! identifier and literal renames normalisation erases. A token edge
//! joins members an LSH band collision found merely *similar*. Transitive
//! closure treats them alike, so one token edge is enough to weld two
//! structural families into a single component.
//!
//! The component that results is not a clone of anything. Its members do
//! not agree, so the content stage measures a low `agreement` and a low
//! `rename_consistency` over the union, the cluster buckets down to
//! `loosely_similar`, and report policy hides it. Both real families are
//! then lost *to the presence of each other*, and the loss grows with the
//! corpus: the more code a scan reaches, the more token bridges it finds.
//! `csharp-mcp` is the smallest case that shows it — a summing loop
//! copied into two files and a multiplying loop copied into two more,
//! each pair reported `nearly_identical` on its own, both gone when the
//! four files are scanned together.
//!
//! Splitting is safe in the direction that matters. Two distinct hashes
//! mean two distinct normalised subtrees: renaming every identifier and
//! changing every literal cannot produce them, because normalisation
//! collapses exactly those. A component holding two families of two is
//! therefore a merge, not a four-way clone, and reporting the families
//! separately publishes strictly more true duplication than reporting
//! their union — which is published as nothing at all.
//!
//! A component with one structural family and a fringe of near-misses is
//! left alone: that is an ordinary Type-3 cluster, and its fringe members
//! are occurrences a reader wants. Only a component with **two or more**
//! families of two or more members is a merge, and only those are split.

use crate::{fingerprint::Fingerprint, pair::FusedCluster};

use super::family::{families_by, restrict};

/// Smallest family worth reporting on its own: one lone occurrence is
/// not a duplicate of anything.
const MIN_FAMILY_MEMBERS: usize = 2;

/// Fewest reportable families that make a component a merge rather than
/// a clone with a near-miss fringe.
const MIN_FAMILIES_TO_SPLIT: usize = 2;

/// Replaces every fused component that welds two or more structural
/// families with one component per family, dropping the members that
/// belong to none.
///
/// Components with fewer than two reportable families are returned
/// untouched, so an ordinary Type-3 cluster keeps every occurrence it
/// had.
pub(crate) fn split_structural_families(
    fused_clusters: Vec<FusedCluster>,
    fingerprints: &[Fingerprint],
) -> Vec<FusedCluster> {
    fused_clusters
        .into_iter()
        .flat_map(|fused| split_one(&fused, fingerprints).unwrap_or_else(|| vec![fused]))
        .collect()
}

/// The replacement components for one cluster, or `None` to keep it as
/// it is.
fn split_one(fused: &FusedCluster, fingerprints: &[Fingerprint]) -> Option<Vec<FusedCluster>> {
    let families = families_by(&fused.members, |index| {
        fingerprints.get(index).map(|member| member.hash)
    });
    let reportable: Vec<&Vec<usize>> = families
        .iter()
        .filter(|family| family.len() >= MIN_FAMILY_MEMBERS)
        .collect();
    if reportable.len() < MIN_FAMILIES_TO_SPLIT {
        return None;
    }
    Some(
        reportable
            .into_iter()
            .map(|family| restrict(fused, family))
            .collect(),
    )
}

#[cfg(test)]
mod tests;
