//! Cross-cluster subsumption ([PIPELINE-CLUSTER-SUBSUME]).
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
//! *Are these one duplication?* Bidirectional coverage by
//! per-occurrence containment: every occurrence of each cluster
//! contains — or is contained by — an occurrence of the other, in the
//! same file. Three weaker predicates were each wrong in a different
//! direction.
//!
//! Requiring the whole occurrence *set* to nest misses the *crossed*
//! case, where the depth difference falls on opposite sides in each
//! file: `ledger_c[0..1238] + ledger_a[0..1234]` and `ledger_c[0..1237]
//! + ledger_a[0..1235]` are two views of one whole-file duplicate, yet
//! neither set nests inside the other.
//!
//! Accepting bare *intersection* goes wrong the other way: two
//! duplicated regions that share a single byte, where one ends and the
//! next begins, are two findings, and treating them as one deletes a
//! duplicate nothing else reports.
//!
//! Accepting coverage in *either* direction is wrong a third way. A wide
//! cluster whose occurrences each happen to contain one member of a much
//! larger, differently-scoped cluster satisfies it — and then a pair of
//! byte-identical generated functions is deleted in favour of the
//! one-line statement family nested inside them, which also reaches a
//! file the functions never mention.
//!
//! *Which view survives?* File coverage, physical enclosure,
//! occurrence coverage, duplicated mass, then stable cluster id, in
//! that order. Pair evidence is forbidden because the component owns
//! none. A nested fragment cannot displace an enclosing authored view
//! merely because the fragment's pair happened to score more highly.
//!
//! *Before either question, file coverage.* A view that names a file
//! the survivor does not name is never dropped, however deeply it nests
//! and however imprecise it is: no other cluster reports that file, so
//! the finding does not move to the survivor — it disappears. Enclosure
//! makes this easy to get wrong, because the enclosing view can be the
//! narrower one.

use crate::fingerprint::Fingerprint;

use super::Cluster;

/// Deterministic survivor selection ([PIPELINE-CLUSTER-SUBSUME]).
mod survivor;
use survivor::{covers_same_region, preferred_view, Preference};

/// Collapses redundant clusters that cover the same physical bytes.
///
/// Runs after mass ranking; [PIPELINE-CLUSTER-SUBSUME] decides which
/// physical view survives.
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
    let mut absorbed: Vec<usize> = Vec::new();
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
                absorbed.push(inner);
            }
            PairDecision::DropOuter => {
                log_subsumption(inner_cluster, outer_cluster, "drop_outer");
                drop_cluster(dropped, outer);
                restore_absorbed(dropped, &absorbed, inner);
                break;
            }
            PairDecision::Keep => {}
        }
    }
}

/// Un-drops every view `outer` had absorbed before it was itself
/// overturned, so they are judged against the view that survived
/// instead of vanishing with the one that did not.
///
/// A view absorbs its nested rivals as the scan walks past them, and
/// only later meets the rival that overturns it. Without this, those
/// absorbed views die with their absorber and *nothing* reports their
/// bytes — the "orphan" this module's history already records
/// (`issue_343_sum_clamp_saturation` counted one). Measuring
/// A later survivor replacement makes this routine rather than rare:
/// a whole-file view can absorb the genuine method-level view and then
/// be replaced by another covering component.
///
/// The survivor is exempt: it is not an orphan, it is the reason the
/// absorber died. Restored views are re-judged because each is scanned
/// again in its own turn as an `outer`, so a genuinely redundant one is
/// re-absorbed by whichever view legitimately covers it.
fn restore_absorbed(dropped: &mut [bool], absorbed: &[usize], survivor: usize) {
    for index in absorbed.iter().copied().filter(|index| *index != survivor) {
        if let Some(slot) = dropped.get_mut(index) {
            *slot = false;
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
        survivor_mass = survivor.mass,
        discarded = discarded.id.as_str(),
        discarded_size = discarded.members.len(),
        discarded_mass = discarded.mass,
        survivor_spans = span_summary(&survivor.members).as_str(),
        discarded_spans = span_summary(&discarded.members).as_str(),
        "cross-cluster subsumption",
    );
}

/// Compact `file:start..end` list for the subsumption trace. Byte
/// offsets only — never source text ([PRINCIPLES-LOGGING]).
fn span_summary(members: &[Fingerprint]) -> String {
    members
        .iter()
        .map(|member| {
            format!(
                "{:?}:{}..{}",
                member.file_id, member.byte_range.start, member.byte_range.end
            )
        })
        .collect::<Vec<String>>()
        .join(",")
}

/// Decides which cluster survives when their occurrences cover the same
/// bytes. Returns [`PairDecision::Keep`] when they are separate regions.
fn evaluate_pair(outer: &Cluster, inner: &Cluster) -> PairDecision {
    if !covers_same_region(outer, inner) {
        return PairDecision::Keep;
    }
    // Enclosure is nominated in both directions. `outer` and `inner`
    // are mass-order positions, not nesting roles, so either may be the
    // physical encloser.
    if strictly_encloses(&inner.members, &outer.members) {
        return match preferred_view(inner, outer, Nesting::ProposedEncloses) {
            Preference::First => PairDecision::DropOuter,
            Preference::Second => PairDecision::DropInner,
            Preference::Neither => PairDecision::Keep,
        };
    }
    let nesting = if strictly_encloses(&outer.members, &inner.members) {
        Nesting::ProposedEncloses
    } else {
        Nesting::Neither
    };
    match preferred_view(outer, inner, nesting) {
        Preference::First => PairDecision::DropInner,
        Preference::Second => PairDecision::DropOuter,
        Preference::Neither => PairDecision::Keep,
    }
}

/// Whether the nominated view physically encloses its rival.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Nesting {
    /// The nominated view strictly encloses the other.
    ProposedEncloses,
    /// Neither occurrence set strictly encloses the other.
    Neither,
}

/// Returns `true` when two occurrences describe one location: same
/// file, and one byte range wholly contains the other.
///
/// Containment, never bare intersection. Two duplicated regions that
/// merely touch — partially, one-sidedly, or at the single byte where
/// one ends and the next begins — are two findings, and an intersection
/// test cannot tell them from a re-description. Containment can, and it
/// still admits the crossed case, where each occurrence contains or is
/// contained by its counterpart even though neither *set* nests.
fn occurrences_describe_one_location(left: &Fingerprint, right: &Fingerprint) -> bool {
    occurrence_contains(left, right) || occurrence_contains(right, left)
}

/// Returns `true` when every occurrence in `covered` is paired by
/// containment with an occurrence in `cover` — the "same physical
/// bytes" test.
pub(super) fn all_occurrences_paired(covered: &[Fingerprint], cover: &[Fingerprint]) -> bool {
    !covered.is_empty()
        && covered.iter().all(|candidate| {
            cover
                .iter()
                .any(|other| occurrences_describe_one_location(other, candidate))
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
        && nested.iter().all(|inner| {
            enclosing
                .iter()
                .any(|outer| occurrence_contains(outer, inner))
        })
        && enclosing
            .iter()
            .any(|outer| !nested.iter().any(|inner| occurrence_contains(inner, outer)))
}

/// Returns `true` when every file mentioned in `required` is also
/// mentioned in `candidate`. When this is false the cluster under threat
/// covers files (e.g. cross-language) the survivor does not, so dropping
/// it would erase duplication no other cluster reports.
pub(super) fn covers_every_file(candidate: &[Fingerprint], required: &[Fingerprint]) -> bool {
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
