//! Same-file overlap collapse ([PIPELINE-CLUSTER-EXACT]).
//!
//! Fingerprinting emits one subtree per AST node, so a duplicated region
//! yields a whole nest of overlapping windows over the same bytes in the
//! same file. Only one of them is a location the user can act on; the
//! rest are artifacts of the walk. Publishing more than one inflates the
//! occurrence count, the cluster size, and the duplication percentage —
//! a false positive in the figures even when the cluster itself is real.
//!
//! Overlap is transitive over physical bytes: `[0,100]`, `[90,110]` and
//! `[105,200]` are one run, because the middle window bridges the outer
//! two. A sweep that only ever compares the next window with the run's
//! *chosen representative* loses that bridge the moment the bridge is
//! not the representative, and reports one region as two.

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster, ClusterBuildInputs},
    fingerprint::Fingerprint,
    pair::{FusedCluster, FusedEdge},
    state::{FileId, FileRegistry},
};

/// One structural bucket: every member carries the same digest, which is
/// what a hash-identical subtree set looks like leaving fingerprinting.
const SHARED_DIGEST: [u8; 32] = [7_u8; 32];

/// A member over `[start, end)` of `file_id`, sized so the cluster
/// clears the reportable node floor.
fn member(file_id: FileId, start: usize, end: usize) -> Fingerprint {
    Fingerprint {
        hash: SHARED_DIGEST,
        file_id,
        byte_range: ByteRange { start, end },
        node_count: 40,
    }
}

/// Runs the production clustering stage over `members` as a single fused
/// cluster carrying `edges`, and returns the ranked result. Every case in
/// this file feeds the pipeline through here: the signature stand-ins,
/// the empty embedding map and the fused-group shape are scaffolding
/// rather than the thing under test, and a divergent respelling would
/// mean two tests disagreeing about what the stage was actually fed.
fn ranked_with_edges(members: &[Fingerprint], edges: Vec<FusedEdge>) -> Vec<Cluster> {
    let fused = [FusedCluster {
        members: (0..members.len()).collect(),
        edges,
        convicted: false,
    }];
    build_ranked_fused_clusters(&ClusterBuildInputs {
        fingerprints: members,
        fused_clusters: &fused,
        trees: &[],
        file_languages: &std::collections::HashMap::new(),
        file_paths: &std::collections::HashMap::new(),
    })
}

/// Runs the clustering stage with no surviving discovery edge, which is
/// what a purely structural hash-identical bucket looks like.
fn ranked(members: &[Fingerprint]) -> Vec<Cluster> {
    ranked_with_edges(members, Vec::new())
}

/// One cluster's member byte ranges, in published order. Four call sites
/// respelled this same `members.iter().map(..).collect()` chain; Deslop
/// scored the copies `identical`/`nearly_identical` against this repo's
/// own corpus, and a divergent respelling would have meant two tests
/// disagreeing about what "the published range" is.
fn member_ranges(cluster: &Cluster) -> Vec<(usize, usize)> {
    cluster
        .members
        .iter()
        .map(|found| (found.byte_range.start, found.byte_range.end))
        .collect()
}

/// One surviving discovery edge between two fingerprint indices.
fn edge(left: usize, right: usize) -> FusedEdge {
    FusedEdge { left, right }
}

/// The occurrence byte ranges of the single published cluster.
fn occurrence_ranges(members: &[Fingerprint]) -> Vec<(usize, usize)> {
    let clusters = ranked(members);
    assert_eq!(
        clusters.len(),
        1,
        "one fused group publishes one cluster, got {clusters:#?}"
    );
    clusters.first().map(member_ranges).unwrap_or_default()
}

/// Asserts the occurrence ranges the collapse publishes for `members`.
/// Four cases respelled the same bind-then-compare pair; Deslop scored
/// the copies against this repo's own corpus. The observed ranges stay in
/// the failure message, so a red test still names what it actually got.
fn assert_occurrence_ranges(members: &[Fingerprint], expected: &[(usize, usize)], why: &str) {
    let ranges = occurrence_ranges(members);
    assert_eq!(ranges.as_slice(), expected, "{why}, got {ranges:?}");
}

/// Two files, one of which carries a transitively-overlapping run.
fn two_files() -> (FileId, FileId) {
    let mut registry = FileRegistry::new();
    (
        registry.register("alpha.ts".into()),
        registry.register("beta.ts".into()),
    )
}

/// [PIPELINE-CLUSTER-EXACT] Overlap is transitive. `[0,100]`, `[90,110]`
/// and `[105,200]` are one physical run in one file: the middle window
/// overlaps both neighbours, so all three describe one location and the
/// cluster must publish one occurrence for that file.
///
/// The bridge is deliberately the *narrowest* of the three, because the
/// representative is chosen by width. A sweep that compares only against
/// the representative discards the bridge, then finds `[105,200]`
/// disjoint from `[0,100]` and publishes it as a second occurrence — one
/// region counted twice, in the cluster size, the occurrence list, and
/// the duplication metric.
#[test]
fn a_transitively_overlapping_run_collapses_to_one_occurrence() {
    let (alpha, beta) = two_files();
    let ranges = occurrence_ranges(&[
        member(alpha, 0, 100),
        member(alpha, 90, 110),
        member(alpha, 105, 200),
        member(beta, 0, 100),
    ]);
    assert_eq!(
        ranges.len(),
        2,
        "one run in alpha plus one occurrence in beta is two locations, got \
         {ranges:?}"
    );
    assert_eq!(
        ranges,
        vec![(0, 100), (0, 100)],
        "the widest window of the run represents it, and beta is untouched, \
         got {ranges:?}"
    );
}

/// Same run, widest window last: the representative must still be the
/// widest, and the run must still collapse to one occurrence whichever
/// order the walk emitted the windows in.
#[test]
fn the_widest_window_of_a_run_represents_it_regardless_of_emission_order() {
    let (alpha, beta) = two_files();
    assert_occurrence_ranges(
        &[
            member(alpha, 0, 40),
            member(alpha, 30, 60),
            member(alpha, 20, 200),
            member(beta, 0, 200),
        ],
        &[(20, 200), (0, 200)],
        "the 180-byte window represents the run",
    );
}

/// Control: disjoint windows in one file are two real locations and must
/// both survive. A collapse rule that merged them would erase a genuine
/// duplicate — the false negative on the other side of the same code.
#[test]
fn disjoint_windows_in_one_file_stay_separate_occurrences() {
    let (alpha, beta) = two_files();
    assert_occurrence_ranges(
        &[
            member(alpha, 0, 100),
            member(alpha, 200, 300),
            member(beta, 0, 100),
        ],
        &[(0, 100), (200, 300), (0, 100)],
        "two disjoint alpha windows and one beta window are three locations,",
    );
}

/// [PIPELINE-CLUSTER-EXACT-SCOPE]: within an overlapping run the wider
/// authored view represents it — pair edge strength never enters the
/// selection. A whole-file root `[0,200]` and a window `[10,150]`
/// nested in it overlap in the same file, so the collapse publishes
/// one canonical member per file: the root, because no pair grade may
/// choose a view ([PIPELINE-CLUSTER-CLOSURE]). Reversing which edge is
/// stronger — or deleting the edges entirely — must not change the
/// published view.
#[test]
fn the_wider_member_represents_a_run_regardless_of_edge_strength() {
    let (alpha, beta) = two_files();
    let members = [
        member(alpha, 0, 200),
        member(alpha, 10, 150),
        member(beta, 10, 150),
    ];
    for edges in [
        vec![edge(0, 2), edge(1, 2)],
        vec![edge(1, 2), edge(0, 2)],
        Vec::new(),
    ] {
        let clusters = ranked_with_edges(&members, edges);
        let ranges: Vec<Vec<(usize, usize)>> = clusters.iter().map(member_ranges).collect();
        assert_eq!(
            ranges,
            vec![vec![(0, 200), (10, 150)]],
            "the wider root represents the alpha run whatever the edges say"
        );
    }
}

/// Control: a run that collapses to a single location is not a
/// duplicate at all, and the cluster is dropped rather than published as
/// a one-occurrence finding.
#[test]
fn a_run_collapsing_to_one_location_publishes_no_cluster() {
    let (alpha, _beta) = two_files();
    let clusters = ranked(&[
        member(alpha, 0, 100),
        member(alpha, 90, 110),
        member(alpha, 105, 200),
    ]);
    assert!(
        clusters.is_empty(),
        "one physical location is not a duplicate, got {clusters:#?}"
    );
}
