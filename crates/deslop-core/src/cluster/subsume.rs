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
//!
//! *How the verdicts combine.* A view is published when no published
//! view re-describes its region and outranks it; every other view is
//! absorbed by one that does. That is a property of the published set,
//! not of the order the views were met in, so when a survivor leaves the
//! set — outranked, or dropped as a straddler — whatever it absorbed is
//! judged again against the views that remain ([`kernel`]). Only views
//! over exactly the same files can re-describe or straddle one another,
//! so each file set is resolved on its own
//! ([PIPELINE-CLUSTER-SUBSUME-FILESET]).

use std::collections::BTreeMap;

use crate::{fingerprint::Fingerprint, state::FileId};

use super::Cluster;

/// Survivor selection inside one file set.
mod kernel;
/// Deterministic survivor order ([PIPELINE-CLUSTER-SUBSUME]).
mod survivor;
/// Stage records ([PIPELINE-OBSERVABILITY-STAGES]).
mod tally;
use kernel::{resolve, Region};
use tally::SubsumeTally;

/// Collapses redundant clusters that cover the same physical bytes.
///
/// Runs after mass ranking; [PIPELINE-CLUSTER-SUBSUME] decides which
/// physical view survives, one file set at a time.
pub(super) fn collapse_cross_cluster_overlap(clusters: Vec<Cluster>) -> Vec<Cluster> {
    let groups = file_set_groups(&clusters);
    let mut tally = SubsumeTally::new(clusters.len(), groups.len());
    let mut published = vec![false; clusters.len()];
    for group in groups.values() {
        publish_group(&clusters, group, &mut published, &mut tally);
    }
    let survivors: Vec<Cluster> = clusters
        .into_iter()
        .zip(published)
        .filter_map(|(cluster, keep)| keep.then_some(cluster))
        .collect();
    tally.complete(survivors.len());
    survivors
}

/// [PIPELINE-CLUSTER-SUBSUME-FILESET] Ranked positions grouped by the
/// exact set of files their views name, rank order kept inside each
/// group.
fn file_set_groups(clusters: &[Cluster]) -> BTreeMap<Vec<FileId>, Vec<usize>> {
    clusters
        .iter()
        .enumerate()
        .fold(BTreeMap::new(), |mut groups, (index, cluster)| {
            groups
                .entry(file_set(&cluster.members))
                .or_default()
                .push(index);
            groups
        })
}

/// The distinct files a view names, in a canonical order.
fn file_set(members: &[Fingerprint]) -> Vec<FileId> {
    let mut files: Vec<FileId> = members.iter().map(|member| member.file_id).collect();
    files.sort_unstable();
    files.dedup();
    files
}

/// Resolves one file set and marks its survivors in `published`.
fn publish_group(
    clusters: &[Cluster],
    group: &[usize],
    published: &mut [bool],
    tally: &mut SubsumeTally,
) {
    let members: Vec<(usize, &Cluster)> = group
        .iter()
        .filter_map(|index| clusters.get(*index).map(|cluster| (*index, cluster)))
        .collect();
    let region = Region::new(members.iter().map(|(_, cluster)| *cluster).collect(), tally);
    for local in resolve(&region, tally) {
        let slot = members
            .get(local)
            .and_then(|(index, _)| published.get_mut(*index));
        if let Some(slot) = slot {
            *slot = true;
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
