//! [PIPELINE-CLUSTER-SUBSUME] How verdicts combine into a report: a view
//! whose absorber leaves the report is judged again against the views
//! that remain, each file set is resolved on its own, and a cycle in the
//! survivor order is decided by the coverage-mass-id order.

use deslop_core::state::{FileId, FileRegistry};

use super::{
    in_rank_order, occurrences, published_across, published_weighted, View, HEAVY_MASS,
    HEAVY_NODES, LIGHTEST_MASS, LIGHTEST_NODES, LIGHT_NODES, MEMBER_NODES,
};

/// [PIPELINE-CLUSTER-SUBSUME-FILESET] Every verdict needs both views to name
/// exactly the same files, so each file set reaches the verdict it would
/// reach alone, however the ranked list interleaves them — and the
/// verdict inside `{alpha, beta}` must not depend on where the ranked
/// list happens to put its equal-mass views. Four file sets ride one
/// list here: a straddle over
/// `{alpha, beta}` whose nested core must survive, with a decoy nested in
/// one straddler only that must come back when both straddlers die; an
/// enclosure over `{gamma, delta}`; and two half-overlapping views over
/// `{alpha, beta, gamma}` that are both findings — the decoy sits inside
/// the wider one's alpha and beta occurrences, and names no file it does
/// not, yet a view over a different file set is never its re-description.
#[test]
fn each_file_set_is_judged_on_its_own() {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let gamma = registry.register("gamma.ts".into());
    let delta = registry.register("delta.ts".into());
    let pair = |left: FileId, right: FileId, span: (usize, usize)| vec![(left, span), (right, span)];
    let trio = |span: (usize, usize)| vec![(alpha, span), (beta, span), (gamma, span)];
    let left_padded = pair(alpha, beta, (0, 200));
    let right_padded = pair(alpha, beta, (50, 250));
    let nested = pair(alpha, beta, (50, 200));
    let decoy = pair(alpha, beta, (0, 100));
    let frame = pair(gamma, delta, (0, 200));
    let inset = pair(gamma, delta, (10, 50));
    let wide = trio((0, 300));
    let half_overlapping = trio((150, 450));
    let clusters = published_across(&[
        left_padded,
        inset,
        wide,
        right_padded,
        decoy,
        half_overlapping,
        frame,
        nested,
    ]);
    assert!(in_rank_order(&clusters), "survivors stay in rank order");
    let mut actual = occurrences(&clusters);
    actual.sort_unstable();
    let mut expected = vec![
        pair(alpha, beta, (50, 200)),
        pair(alpha, beta, (0, 100)),
        pair(gamma, delta, (0, 200)),
        trio((0, 300)),
        trio((150, 450)),
    ];
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "the straddle publishes its core and releases the decoy, the \
         enclosure publishes its encloser, and the three-file views \
         stay two findings that never absorb the two-file decoy"
    );
}

/// How many disjoint views the scale pin ranks. Under the all-pairs
/// scan this replaced, 60,000 views cost 1.8 × 10⁹ pair evaluations and
/// the test ran for minutes without finishing.
const DISJOINT_VIEWS: usize = 60_000;

/// [PIPELINE-CLUSTER-SUBSUME-FILESET] The scan is a sum over file sets,
/// not a square over the ranked list. Sixty thousand views over sixty
/// thousand disjoint file pairs share nothing: every one is published
/// unchanged, in rank order, and the run finishes in well under a
/// second — where the all-pairs scan evaluated 1.8 × 10⁹ pairs and ran
/// for minutes here, and on the Flutter corpus (217,045 views) sat on
/// one core for half an hour without emitting a record.
#[test]
fn disjoint_file_sets_are_never_compared() {
    let mut registry = FileRegistry::new();
    let views: Vec<View> = (0..DISJOINT_VIEWS)
        .map(|index| {
            let left = registry.register(format!("left-{index}.ts").into());
            let right = registry.register(format!("right-{index}.ts").into());
            vec![(left, (0, 100)), (right, (0, 100))]
        })
        .collect();
    let clusters = published_across(&views);
    assert_eq!(
        clusters.len(),
        DISJOINT_VIEWS,
        "views over disjoint file sets never re-describe one another"
    );
    assert!(in_rank_order(&clusters), "equal-mass views rank by id");
    let mut published = occurrences(&clusters);
    published.sort_unstable();
    let mut expected = views;
    expected.sort_unstable();
    assert_eq!(
        published, expected,
        "every view is published exactly as it was ranked"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME-KERNEL] A view whose absorber is later
/// outranked is judged again against the views that remain. The small
/// view is enclosed by the medium one and absorbed; the heavy view then
/// outranks the medium one on the crossed shape, and its beta occurrence
/// never reaches the small view's bytes. The small view is a finding the
/// heavy survivor does not describe, so it is published with the mass
/// [RANK-MASS-SUM] gives it, after the heavier survivor.
#[test]
fn a_view_released_by_its_absorber_is_judged_against_the_views_that_remain() {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let small = (LIGHTEST_NODES, vec![(alpha, (20, 40)), (beta, (20, 40))]);
    let medium = (LIGHT_NODES, vec![(alpha, (0, 100)), (beta, (0, 100))]);
    let heavy = (HEAVY_NODES, vec![(alpha, (0, 200)), (beta, (50, 80))]);
    let clusters = published_weighted(&[small, medium, heavy]);
    assert_eq!(
        occurrences(&clusters),
        vec![
            vec![(alpha, (0, 200)), (beta, (50, 80))],
            vec![(alpha, (20, 40)), (beta, (20, 40))],
        ],
        "the heavy view outranks the medium one it crosses, and the small view \
         the medium one had absorbed comes back because the heavy view does \
         not reach its bytes in beta"
    );
    assert_eq!(
        clusters.iter().map(|cluster| cluster.mass).collect::<Vec<u64>>(),
        vec![HEAVY_MASS, LIGHTEST_MASS],
        "[RANK-MASS-SUM] two-occurrence views carry their node count as mass"
    );
}

/// [PIPELINE-CLUSTER-SUBSUME-CYCLE] Three views that each outrank the
/// next: the enclosing view beats the leader by enclosure, the leader
/// beats the crossed view on mass, and the crossed view beats the
/// enclosing view on mass. No view is free of a rival, so the leader on
/// occurrence coverage, mass and id is the finding and the other two are
/// absorbed by it. The region is reported exactly once.
#[test]
fn three_views_that_outrank_each_other_in_a_cycle_publish_the_leader() {
    let mut registry = FileRegistry::new();
    let alpha = registry.register("alpha.ts".into());
    let beta = registry.register("beta.ts".into());
    let enclosing = (LIGHTEST_NODES, vec![(alpha, (10, 90)), (beta, (0, 100))]);
    let leader = (HEAVY_NODES, vec![(alpha, (20, 80)), (beta, (10, 90))]);
    let crossed = (MEMBER_NODES, vec![(alpha, (0, 100)), (beta, (20, 80))]);
    let clusters = published_weighted(&[enclosing, leader, crossed]);
    assert_eq!(
        occurrences(&clusters),
        vec![vec![(alpha, (20, 80)), (beta, (10, 90))]],
        "the view leading on coverage, mass and id survives the cycle"
    );
    assert_eq!(
        clusters.iter().map(|cluster| cluster.mass).collect::<Vec<u64>>(),
        vec![HEAVY_MASS],
        "[RANK-MASS-SUM] the leader keeps its own mass"
    );
}
