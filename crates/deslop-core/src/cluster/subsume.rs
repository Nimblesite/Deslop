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
    // A straddle releases views into a scan that may already have passed
    // them ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]); every pass that releases
    // drops two straddlers for good, so the passes run out.
    while scan_all_pairs(&clusters, &mut dropped, len) {}
    clusters
        .into_iter()
        .enumerate()
        .filter_map(|(index, cluster)| (!cluster_dropped(&dropped, index)).then_some(cluster))
        .collect()
}

/// Decision produced by [`evaluate_pair`] for one `(outer, inner)` pair.
#[derive(Clone, Copy)]
enum PairDecision {
    /// Discard the inner cluster; the outer subsumes it.
    DropInner,
    /// Discard the outer cluster; the inner subsumes it.
    DropOuter,
    /// Discard both: they straddle a view nested in each, and that view
    /// is the finding ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]).
    DropBoth,
    /// Retain both clusters.
    Keep,
}

/// One pass of `(outer, inner)` scans over every surviving `outer`.
/// Returns `true` when a straddle released views that need a pass of
/// their own.
fn scan_all_pairs(clusters: &[Cluster], dropped: &mut [bool], len: usize) -> bool {
    (0..len).fold(false, |released, outer| {
        if cluster_dropped(dropped, outer) {
            return released;
        }
        let released_now = scan_inner_pairs(clusters, dropped, outer, len);
        released_now || released
    })
}

/// Evaluates every `(outer, inner)` pair for the given `outer` index and
/// updates `dropped`. Stops when `outer` itself is dropped, returning
/// `true` when that drop released views the scan had already passed.
fn scan_inner_pairs(clusters: &[Cluster], dropped: &mut [bool], outer: usize, len: usize) -> bool {
    let mut absorbed: Vec<usize> = Vec::new();
    for inner in (outer.saturating_add(1))..len {
        if cluster_dropped(dropped, inner) {
            continue;
        }
        let (Some(outer_cluster), Some(inner_cluster)) = (clusters.get(outer), clusters.get(inner))
        else {
            continue;
        };
        let decision = evaluate_pair(clusters, outer_cluster, inner_cluster);
        let scan = Scan {
            clusters,
            outer: (outer, outer_cluster),
            inner: (inner, inner_cluster),
        };
        match apply_decision(decision, &scan, dropped, &mut absorbed) {
            Flow::Continue => {}
            Flow::Stop => return false,
            Flow::StopReleased => return true,
        }
    }
    false
}

/// The pair one decision applies to: mass-order indices with their
/// clusters, and the whole list a straddle searches for its core.
struct Scan<'a> {
    /// Every cluster, in mass order.
    clusters: &'a [Cluster],
    /// The `outer` index and cluster.
    outer: (usize, &'a Cluster),
    /// The `inner` index and cluster.
    inner: (usize, &'a Cluster),
}

/// What a decision does to the scan of one `outer`.
enum Flow {
    /// Keep scanning `outer` against later clusters.
    Continue,
    /// `outer` is gone; nothing was released.
    Stop,
    /// `outer` is gone and views were released into passed territory.
    StopReleased,
}

/// Applies one [`PairDecision`] to `dropped` and the running `absorbed`
/// list of `outer`.
fn apply_decision(
    decision: PairDecision,
    scan: &Scan<'_>,
    dropped: &mut [bool],
    absorbed: &mut Vec<usize>,
) -> Flow {
    let (outer, outer_cluster) = scan.outer;
    let (inner, inner_cluster) = scan.inner;
    match decision {
        PairDecision::DropInner => {
            log_subsumption(outer_cluster, inner_cluster, "drop_inner");
            drop_cluster(dropped, inner);
            absorbed.push(inner);
            Flow::Continue
        }
        PairDecision::DropOuter => {
            log_subsumption(inner_cluster, outer_cluster, "drop_outer");
            drop_cluster(dropped, outer);
            restore_absorbed(dropped, absorbed, Some(inner));
            Flow::Stop
        }
        PairDecision::DropBoth => drop_both(scan, dropped, absorbed),
        PairDecision::Keep => Flow::Continue,
    }
}

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Drops both straddlers and brings
/// back everything they stood in front of: the views `outer` absorbed and
/// every view nested in both, so the finding they were padded readings
/// of is judged on its own.
fn drop_both(scan: &Scan<'_>, dropped: &mut [bool], absorbed: &[usize]) -> Flow {
    let (outer, outer_cluster) = scan.outer;
    let (inner, inner_cluster) = scan.inner;
    log_subsumption(inner_cluster, outer_cluster, "drop_both_straddle");
    drop_cluster(dropped, inner);
    drop_cluster(dropped, outer);
    restore_absorbed(dropped, absorbed, None);
    for (index, core) in scan.clusters.iter().enumerate() {
        if is_core_of(core, outer_cluster, inner_cluster) {
            restore_cluster(dropped, index);
        }
    }
    Flow::StopReleased
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
/// The survivor, when there is one, is exempt: it is not an orphan, it
/// is the reason the absorber died. Two straddling views die without a
/// survivor between them ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]), so
/// everything either absorbed comes back. Restored views are re-judged
/// because each is scanned again in its own turn as an `outer`, so a
/// genuinely redundant one is re-absorbed by whichever view
/// legitimately covers it.
fn restore_absorbed(dropped: &mut [bool], absorbed: &[usize], survivor: Option<usize>) {
    for index in absorbed
        .iter()
        .copied()
        .filter(|index| Some(*index) != survivor)
    {
        restore_cluster(dropped, index);
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
/// bytes. Returns [`PairDecision::Keep`] when they are separate regions,
/// and [`PairDecision::DropBoth`] when they are two padded readings of a
/// third view nested in each.
fn evaluate_pair(clusters: &[Cluster], outer: &Cluster, inner: &Cluster) -> PairDecision {
    if !covers_same_region(outer, inner) {
        return if straddle_over_a_core(clusters, outer, inner) {
            PairDecision::DropBoth
        } else {
            PairDecision::Keep
        };
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

/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Whether `first` and `second` are
/// two padded readings of one view nested in both: they name the same
/// files, every occurrence of each overlaps an occurrence of the other in
/// its file, neither contains the other, and some third cluster lies
/// strictly inside both in every file. That third view is the
/// duplication; each straddler adds bytes on a side the other does not
/// share, and publishing both would report the nested view twice under
/// two different extents.
fn straddle_over_a_core(clusters: &[Cluster], first: &Cluster, second: &Cluster) -> bool {
    covers_every_file(&first.members, &second.members)
        && covers_every_file(&second.members, &first.members)
        && all_occurrences_overlap(&first.members, &second.members)
        && all_occurrences_overlap(&second.members, &first.members)
        && clusters.iter().any(|core| is_core_of(core, first, second))
}

/// Whether `core` is a distinct view strictly inside both `first` and
/// `second` that still names every file they name.
fn is_core_of(core: &Cluster, first: &Cluster, second: &Cluster) -> bool {
    core.id != first.id
        && core.id != second.id
        && covers_every_file(&core.members, &first.members)
        && strictly_encloses(&first.members, &core.members)
        && strictly_encloses(&second.members, &core.members)
}

/// Returns `true` when every occurrence in `covered` shares at least one
/// byte with an occurrence in `cover` in the same file.
fn all_occurrences_overlap(covered: &[Fingerprint], cover: &[Fingerprint]) -> bool {
    !covered.is_empty()
        && covered.iter().all(|candidate| {
            cover
                .iter()
                .any(|other| occurrences_overlap(other, candidate))
        })
}

/// Returns `true` when two occurrences in the same file share at least
/// one byte.
fn occurrences_overlap(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.file_id == right.file_id
        && left.byte_range.start < right.byte_range.end
        && right.byte_range.start < left.byte_range.end
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

/// Clears the removal mark on `index` when the slot exists.
fn restore_cluster(dropped: &mut [bool], index: usize) {
    if let Some(slot) = dropped.get_mut(index) {
        *slot = false;
    }
}
