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

use std::collections::HashMap;

use deslop_core::{
    ast::ByteRange,
    cluster::{build_ranked_fused_clusters, Cluster},
    fingerprint::Fingerprint,
    lsh::Signature,
    pair::FusedCluster,
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
/// cluster and returns the ranked result.
fn ranked(members: Vec<Fingerprint>) -> Vec<Cluster> {
    let signatures: Vec<Signature> = members.iter().map(|_| [11_u64; 128]).collect();
    let fused = [FusedCluster {
        members: (0..members.len()).collect(),
    }];
    let vectors: HashMap<usize, Vec<f32>> = HashMap::new();
    build_ranked_fused_clusters(&members, &signatures, &vectors, &fused)
}

/// The occurrence byte ranges of the single published cluster.
fn occurrence_ranges(members: Vec<Fingerprint>) -> Vec<(usize, usize)> {
    let clusters = ranked(members);
    assert_eq!(
        clusters.len(),
        1,
        "one fused group publishes one cluster, got {clusters:#?}"
    );
    clusters
        .first()
        .map(|cluster| {
            cluster
                .members
                .iter()
                .map(|found| (found.byte_range.start, found.byte_range.end))
                .collect()
        })
        .unwrap_or_default()
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
    let ranges = occurrence_ranges(vec![
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
    let ranges = occurrence_ranges(vec![
        member(alpha, 0, 40),
        member(alpha, 30, 60),
        member(alpha, 20, 200),
        member(beta, 0, 200),
    ]);
    assert_eq!(
        ranges,
        vec![(20, 200), (0, 200)],
        "the 180-byte window represents the run, got {ranges:?}"
    );
}

/// Control: disjoint windows in one file are two real locations and must
/// both survive. A collapse rule that merged them would erase a genuine
/// duplicate — the false negative on the other side of the same code.
#[test]
fn disjoint_windows_in_one_file_stay_separate_occurrences() {
    let (alpha, beta) = two_files();
    let ranges = occurrence_ranges(vec![
        member(alpha, 0, 100),
        member(alpha, 200, 300),
        member(beta, 0, 100),
    ]);
    assert_eq!(
        ranges,
        vec![(0, 100), (200, 300), (0, 100)],
        "two disjoint alpha windows and one beta window are three locations, \
         got {ranges:?}"
    );
}

/// Control: a run that collapses to a single location is not a
/// duplicate at all, and the cluster is dropped rather than published as
/// a one-occurrence finding.
#[test]
fn a_run_collapsing_to_one_location_publishes_no_cluster() {
    let (alpha, _beta) = two_files();
    let clusters = ranked(vec![
        member(alpha, 0, 100),
        member(alpha, 90, 110),
        member(alpha, 105, 200),
    ]);
    assert!(
        clusters.is_empty(),
        "one physical location is not a duplicate, got {clusters:#?}"
    );
}
