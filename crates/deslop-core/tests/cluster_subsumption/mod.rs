//! Cross-cluster subsumption boundary ([PIPELINE-CLUSTER-SUBSUME]).
//!
//! Subsumption deletes a whole cluster from the report, so the predicate
//! that decides "these two are one duplication" is a false-negative
//! surface: every byte it is too generous about is a finding the user
//! never sees. The two shapes it must accept are strict enclosure (a
//! duplicated method and the statement clones nested inside it) and the
//! *crossed* case (two views of one whole-file duplicate whose depth
//! difference falls on opposite sides in each file, so neither occurrence
//! set nests inside the other).
//!
//! Everything else it must refuse. Two clusters that merely intersect —
//! partially, one-sidedly, or by a single byte where one ends and the
//! next begins — describe two regions that happen to touch, and both are
//! findings. This suite pins both directions: the shapes that must
//! collapse, and the shapes that must not.

/// Same-region shapes: what collapses and what must not.
mod region;
/// Release, file sets and cycles: how verdicts combine into a report.
mod release;
/// Straddles: two padded readings of one nested view.
mod straddle;

use std::collections::BTreeMap;

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs},
    fingerprint::Fingerprint,
    pair::FusedCluster,
    state::{FileId, FileRegistry},
};

/// Normalised nodes every member holds, so equal-size views carry equal
/// mass and rank by id alone.
const MEMBER_NODES: usize = 40;

/// Normalised nodes for a view that must out-mass every other view and
/// so be scanned first ([RANK-MASS-SUM]).
const HEAVY_NODES: usize = 80;

/// Normalised nodes for a view that must trail the equal-mass views.
const LIGHT_NODES: usize = 30;

/// Normalised nodes for the lightest view, scanned last.
const LIGHTEST_NODES: usize = 20;

/// [RANK-MASS-SUM] Mass of a two-occurrence view of `HEAVY_NODES` nodes:
/// the node count times one fewer than its occurrences.
const HEAVY_MASS: u64 = 80;

/// [RANK-MASS-SUM] Mass of a two-occurrence view of `LIGHTEST_NODES`
/// nodes.
const LIGHTEST_MASS: u64 = 20;

/// A cluster member over `[start, end)` of `file_id`, digest `digest`,
/// holding `node_count` normalised nodes.
fn member(
    file_id: FileId,
    span: (usize, usize),
    digest: [u8; 32],
    node_count: usize,
) -> Fingerprint {
    Fingerprint {
        hash: digest,
        file_id,
        byte_range: ByteRange {
            start: span.0,
            end: span.1,
        },
        node_count,
    }
}

/// A digest unique to view `index`: its little-endian bytes, zero-padded.
fn digest(index: usize) -> [u8; 32] {
    let mut hash = [0_u8; 32];
    let bytes = u64::try_from(index).unwrap_or(u64::MAX).to_le_bytes();
    for (slot, byte) in hash.iter_mut().zip(bytes) {
        *slot = byte;
    }
    hash
}

/// One view: its occurrences as `(file, span)` pairs, in member order.
type View = Vec<(FileId, (usize, usize))>;

/// A view with the normalised node count each of its members holds,
/// which fixes its mass and so its place in the ranked list.
type WeightedView = (usize, View);

/// Ranks and subsumes `views`, one fused cluster each, over whatever
/// files their occurrences name; view `n` carries digest `n + 1`.
fn published_across(views: &[View]) -> Vec<Cluster> {
    let weighted: Vec<WeightedView> = views
        .iter()
        .map(|view| (MEMBER_NODES, view.clone()))
        .collect();
    published_weighted(&weighted)
}

/// [`published_across`] with each view's node count chosen by the test,
/// so mass — not id — decides the rank order. Every result is held to
/// the report contract, and to being identical when built again.
fn published_weighted(views: &[WeightedView]) -> Vec<Cluster> {
    let clusters = ranked(views);
    assert_report_contract(views, &clusters);
    let again = ranked(views);
    assert_eq!(
        ids(&again),
        ids(&clusters),
        "[PIPELINE-DETERMINISM] the same views rank to the same ids in the same order"
    );
    assert_eq!(
        occurrences(&again),
        occurrences(&clusters),
        "[PIPELINE-DETERMINISM] the same views publish the same occurrences"
    );
    clusters
}

/// Ranks and subsumes `views` once, one fused cluster each.
fn ranked(views: &[WeightedView]) -> Vec<Cluster> {
    let members: Vec<Fingerprint> = views
        .iter()
        .zip(1_usize..)
        .flat_map(|((node_count, view), index)| {
            view.iter()
                .map(move |(file_id, span)| member(*file_id, *span, digest(index), *node_count))
        })
        .collect();
    let mut next = 0_usize;
    let fused: Vec<FusedCluster> = views
        .iter()
        .map(|(_, view)| {
            let first = next;
            next = next.saturating_add(view.len());
            FusedCluster {
                members: (first..next).collect(),
                edges: Vec::new(),
                shape_family: None,
            }
        })
        .collect();
    build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: &members,
        fused_clusters: &fused,
        trees: &[],
        file_languages: &std::collections::HashMap::new(),
        file_paths: &std::collections::HashMap::new(),
    })
}

/// The published clusters' occurrences as `(file, span)` pairs, in rank
/// order.
fn occurrences(clusters: &[Cluster]) -> Vec<View> {
    clusters
        .iter()
        .map(|cluster| {
            cluster
                .members
                .iter()
                .map(|found| {
                    (
                        found.file_id,
                        (found.byte_range.start, found.byte_range.end),
                    )
                })
                .collect()
        })
        .collect()
}

/// True when `clusters` are in rank order: mass descending, then id
/// ascending ([RANK-MASS-SUM]).
fn in_rank_order(clusters: &[Cluster]) -> bool {
    clusters.windows(2).all(|pair| match pair {
        [left, right] => (right.mass, &left.id) <= (left.mass, &right.id),
        _ => true,
    })
}

/// The published cluster ids, in rank order.
fn ids(clusters: &[Cluster]) -> Vec<&str> {
    clusters.iter().map(|cluster| cluster.id.as_str()).collect()
}

/// [PIPELINE-CLUSTER-SUBSUME] The report contract every published set
/// is held to, stated from the spec rather than read off the code:
/// survivors are in rank order with unique ids and the mass
/// [RANK-MASS-SUM] gives them; no two survivors describe one
/// duplication; and every view that is not published is re-described
/// by a survivor over its own files, or straddles a survivor — or a view
/// a survivor re-describes — nested strictly inside it
/// ([PIPELINE-CLUSTER-SUBSUME-KERNEL], [PIPELINE-CLUSTER-SUBSUME-STRADDLE]).
fn assert_report_contract(views: &[WeightedView], clusters: &[Cluster]) {
    assert!(in_rank_order(clusters), "survivors stay in rank order");
    assert_unique_ids(clusters);
    assert_mass_follows_the_formula(clusters);
    let published = occurrences(clusters);
    assert_no_survivor_re_describes_another(&published);
    assert_every_unpublished_view_is_accounted_for(views, &published);
}

/// Every published id appears once.
fn assert_unique_ids(clusters: &[Cluster]) {
    let mut seen = std::collections::BTreeSet::new();
    for cluster in clusters {
        assert!(
            seen.insert(cluster.id.as_str()),
            "cluster ids are unique, but {} was published twice",
            cluster.id
        );
    }
}

/// [RANK-MASS-SUM] Mass is the smallest member's node count times one
/// fewer than the number of occurrences.
fn assert_mass_follows_the_formula(clusters: &[Cluster]) {
    for cluster in clusters {
        let smallest = cluster
            .members
            .iter()
            .map(|member| member.node_count)
            .min()
            .unwrap_or(0);
        let occurrences = cluster.members.len().saturating_sub(1);
        let expected = u64::try_from(smallest.saturating_mul(occurrences)).unwrap_or(u64::MAX);
        assert_eq!(
            cluster.mass, expected,
            "[RANK-MASS-SUM] mass of {} is its smallest member's nodes times one \
             fewer than its occurrences",
            cluster.id
        );
    }
}

/// One occurrence wholly contains another in the same file.
fn contains(outer: &(FileId, (usize, usize)), inner: &(FileId, (usize, usize))) -> bool {
    outer.0 == inner.0 && outer.1 .0 <= inner.1 .0 && inner.1 .1 <= outer.1 .1
}

/// Every occurrence of `view` contains, or is contained by, an
/// occurrence of `other` in its file.
fn paired(view: &View, other: &View) -> bool {
    view.iter().all(|occurrence| {
        other
            .iter()
            .any(|rival| contains(rival, occurrence) || contains(occurrence, rival))
    })
}

/// Two views describe one duplication when each pairs with the other.
fn one_duplication(left: &View, right: &View) -> bool {
    paired(left, right) && paired(right, left)
}

/// Every occurrence of `inner` lies inside an occurrence of `outer`, and
/// `outer` reaches beyond it.
fn strictly_inside(inner: &View, outer: &View) -> bool {
    inner
        .iter()
        .all(|occurrence| outer.iter().any(|wide| contains(wide, occurrence)))
        && outer
            .iter()
            .any(|wide| !inner.iter().any(|occurrence| contains(occurrence, wide)))
}

/// The distinct files a view names, in a canonical order: only views
/// over the same files can describe one duplication
/// ([PIPELINE-CLUSTER-SUBSUME-FILESET]).
fn files_of(view: &View) -> Vec<FileId> {
    let mut files: Vec<FileId> = view.iter().map(|(file, _)| *file).collect();
    files.sort_unstable();
    files.dedup();
    files
}

/// Views grouped by the files they name.
fn by_file_set(views: &[View]) -> BTreeMap<Vec<FileId>, Vec<&View>> {
    views.iter().fold(BTreeMap::new(), |mut groups, view| {
        groups.entry(files_of(view)).or_default().push(view);
        groups
    })
}

/// No two survivors describe the same duplication.
fn assert_no_survivor_re_describes_another(published: &[View]) {
    for group in by_file_set(published).values() {
        for (position, left) in group.iter().enumerate() {
            for right in group.get(position.saturating_add(1)..).unwrap_or_default() {
                assert!(
                    !one_duplication(left, right),
                    "two survivors describe one duplication: {left:?} and {right:?}"
                );
            }
        }
    }
}

/// Every view that is not published is re-described by a survivor over
/// its own files, or straddles a view nested strictly inside it that a
/// survivor is or re-describes.
fn assert_every_unpublished_view_is_accounted_for(views: &[WeightedView], published: &[View]) {
    let survivors = by_file_set(published);
    let inputs: Vec<View> = views.iter().map(|(_, view)| view.clone()).collect();
    let candidates = by_file_set(&inputs);
    for view in &inputs {
        if published.contains(view) {
            continue;
        }
        let files = files_of(view);
        let cores = candidates.get(&files).map_or(&[][..], Vec::as_slice);
        let accounted = survivors
            .get(&files)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .any(|survivor| {
                one_duplication(view, survivor)
                    || cores
                        .iter()
                        .any(|core| strictly_inside(core, view) && one_duplication(core, survivor))
            });
        assert!(
            accounted,
            "an unpublished view is neither re-described by a survivor over its files \
             nor straddling a nested view a survivor reports: {view:?}"
        );
    }
}

/// Builds two same-shape clusters over two files and returns whatever
/// survives ranking and subsumption.
///
/// `left` and `right` each give the byte span used in *both* files, so
/// every cluster names the same two files. That isolates the region
/// predicate: the file-coverage guard cannot be what decides the
/// outcome, because neither view names a file the other omits.
fn published(left: [(usize, usize); 2], right: [(usize, usize); 2]) -> Vec<Cluster> {
    published_views(&[left, right])
}

/// [`published`] for any number of same-shape views over the same two
/// files; view `n` carries digest `n + 1`.
fn published_views(views: &[[(usize, usize); 2]]) -> Vec<Cluster> {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let over_both: Vec<View> = views
        .iter()
        .map(|view| vec![(alpha, view[0]), (beta, view[1])])
        .collect();
    published_across(&over_both)
}

/// The published clusters' occurrence spans, in rank order.
fn spans(clusters: &[Cluster]) -> Vec<Vec<(usize, usize)>> {
    clusters
        .iter()
        .map(|cluster| {
            cluster
                .members
                .iter()
                .map(|found| (found.byte_range.start, found.byte_range.end))
                .collect()
        })
        .collect()
}
