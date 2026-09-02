//! Unit tests for [`super`] — the [CLONE-NOISE-VERBATIM-SUBGROUP]
//! partition that keeps a byte-identical copy out of the suppression
//! its cluster-mates earned.

use std::collections::HashMap;

use super::*;
use crate::{ast::ByteRange, pair::FusedEdge, state::FileRegistry};

/// A constant table. Two files holding this share every byte.
const RETRY_DEFAULTS: &str = "API_TIMEOUT = 30\nMAX_RETRIES = 5\nRETRY_BACKOFF = 2\n";

/// A different constant table with the same normalised shape — the
/// stranger [CLONE-NOISE-CONSTANT-TABLE] exists to suppress.
const THEME_TOKENS: &str = "PRIMARY_COLOR = 41\nFONT_SIZE = 12\nBORDER_WIDTH = 3\n";

/// A third distinct table, so a component can hold three strangers.
const GRID_METRICS: &str = "COLUMN_GAP = 8\nROW_GAP = 6\nGUTTER = 4\n";

/// A function body: real logic, which no noise filter suppresses.
const SETTLE: &str = "def settle(order):\n    total = 0\n    for line in order:\n        total = total + line\n    return total\n";

/// The same logic under a consistent rename — a genuine Type-2 clone.
const RECONCILE: &str = "def reconcile(batch):\n    running = 0\n    for entry in batch:\n        running = running + entry\n    return running\n";

/// A corpus of Python files, each fingerprinted over its whole extent.
struct Corpus {
    /// Whole-file source bytes by registered id.
    sources: HashMap<FileId, Vec<u8>>,
    /// Every file's language, always `python` here.
    languages: HashMap<FileId, &'static str>,
    /// One whole-file fingerprint per source, in registration order.
    fingerprints: Vec<Fingerprint>,
}

impl Corpus {
    /// Registers one file per entry of `bodies` and fingerprints each
    /// over its whole extent, so member index == position in `bodies`.
    fn new(bodies: &[&str]) -> Self {
        let one_each: Vec<&[&str]> = bodies.iter().map(std::slice::from_ref).collect();
        Self::across_files(&one_each)
    }

    /// Registers one file per entry of `files`, holding that entry's
    /// bodies concatenated, and fingerprints every body over its own
    /// byte range. Member indices run in reading order — file by file,
    /// body by body — so geometry-neutral families read as positions.
    fn across_files(files: &[&[&str]]) -> Self {
        let mut registry = FileRegistry::new();
        let mut corpus = Self {
            sources: HashMap::new(),
            languages: HashMap::new(),
            fingerprints: Vec::new(),
        };
        for (position, bodies) in files.iter().enumerate() {
            let file_id = registry.register(format!("case{position}.py").into());
            let _language = corpus.languages.insert(file_id, "python");
            let _previous = corpus.sources.insert(file_id, bodies.concat().into_bytes());
            corpus.fingerprint_each(file_id, bodies);
        }
        corpus
    }

    /// Fingerprints each of one file's bodies over its own byte range.
    fn fingerprint_each(&mut self, file_id: FileId, bodies: &[&str]) {
        let mut start: usize = 0;
        for body in bodies {
            let end = start.saturating_add(body.len());
            self.fingerprints.push(Fingerprint {
                hash: [0_u8; 32],
                file_id,
                byte_range: ByteRange { start, end },
                node_count: 16,
            });
            start = end;
        }
    }

    /// Adds a second fingerprint over the exact bytes member `index`
    /// already covers. The collector emits both a block node and the
    /// full run of that block's own children, which span one range and
    /// hash apart, so a real corpus hands this pass two views of one
    /// location — byte-identical to each other by construction and a
    /// copy of nothing ([CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES]).
    fn duplicate_view_of(&mut self, index: usize) {
        let second_view: Vec<Fingerprint> = self
            .fingerprints
            .iter()
            .skip(index)
            .take(1)
            .map(|seen| Fingerprint {
                node_count: seen.node_count.saturating_sub(1),
                ..seen.clone()
            })
            .collect();
        self.fingerprints.extend(second_view);
    }

    /// Splits one component holding every registered file, in order.
    fn split_all(&self) -> Vec<FusedCluster> {
        self.split(&component(self.fingerprints.len()))
    }

    /// Splits `component` against this corpus.
    fn split(&self, component: &FusedCluster) -> Vec<FusedCluster> {
        split_noise_verbatim_families(
            std::slice::from_ref(component),
            &self.fingerprints,
            &self.sources,
            &self.languages,
            &ParseCache::new(),
        )
    }
}

/// One component over member indices `0..size`, fully connected.
fn component(size: usize) -> FusedCluster {
    let members: Vec<usize> = (0..size).collect();
    let edges: Vec<FusedEdge> = members
        .iter()
        .flat_map(|left| {
            members
                .iter()
                .filter(move |right| *right > left)
                .map(move |right| FusedEdge {
                    left: *left,
                    right: *right,
                })
        })
        .collect();
    FusedCluster {
        members,
        edges,
        shape_family: None,
    }
}

/// The member index lists of `clusters`, for direct comparison.
fn member_lists(clusters: &[FusedCluster]) -> Vec<Vec<usize>> {
    clusters
        .iter()
        .map(|cluster| cluster.members.clone())
        .collect()
}

// The defect this module exists for: a proven copy sharing a component
// with one stranger must not leave with it.
#[test]
fn a_suppressed_component_holding_a_copy_is_reduced_to_the_copy() {
    let corpus = Corpus::new(&[RETRY_DEFAULTS, RETRY_DEFAULTS, THEME_TOKENS]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1]],
        "members 0 and 1 are byte-identical constant tables; member 2 shares \
         only the shape normalisation leaves behind. The copy survives the \
         suppression its cluster-mate earned, and the stranger does not ride \
         out with it",
    );
}

// Two distinct copies inside one suppressed component are two distinct
// findings, not one merged cluster and not none.
#[test]
fn every_verbatim_family_in_a_suppressed_component_survives_separately() {
    let corpus = Corpus::new(&[
        RETRY_DEFAULTS,
        THEME_TOKENS,
        RETRY_DEFAULTS,
        THEME_TOKENS,
        GRID_METRICS,
    ]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 2], vec![1, 3]],
        "both copies are real duplication and are reported apart; the \
         table that was copied nowhere (member 4) is dropped. Families \
         come out in first-member order so the pass is deterministic"
    );
}

// The whole point of gating on the filters: a component they do not
// suppress is left exactly as it was, copy or no copy.
#[test]
fn a_component_the_filters_do_not_suppress_keeps_every_member() {
    let corpus = Corpus::new(&[SETTLE, SETTLE, RECONCILE]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 2]],
        "two byte-identical function bodies plus a consistent rename of them \
         is a three-way clone, not a noise family with a copy inside it — \
         splitting it would erase the rename half"
    );
}

// A component with nothing to protect is not re-parsed and not changed.
#[test]
fn a_suppressed_component_with_no_copy_in_it_is_left_to_be_suppressed() {
    let corpus = Corpus::new(&[RETRY_DEFAULTS, THEME_TOKENS, GRID_METRICS]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 2]],
        "three unrelated tables hold no copy between them, so this pass has \
         nothing to rescue and must hand the component on untouched for the \
         report to hide"
    );
}

// A component that is *entirely* one copy is already the family.
#[test]
fn a_fully_verbatim_component_is_handed_on_unchanged() {
    let corpus = Corpus::new(&[RETRY_DEFAULTS, RETRY_DEFAULTS, RETRY_DEFAULTS]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 2]],
        "three copies of one table are one three-way duplicate; there is no \
         stranger to partition off"
    );
}

// Discovery edges follow their endpoints, so the same-file overlap
// collapse that ranks representatives by cross-file edge strength never
// sees an edge to a member that left.
#[test]
fn edges_survive_only_where_both_endpoints_did() {
    let corpus = Corpus::new(&[RETRY_DEFAULTS, THEME_TOKENS, RETRY_DEFAULTS]);
    let split = corpus.split_all();
    let edges: Vec<Vec<(usize, usize)>> = split
        .iter()
        .map(|cluster| {
            cluster
                .edges
                .iter()
                .map(|edge| (edge.left, edge.right))
                .collect()
        })
        .collect();
    assert_eq!(
        member_lists(&split),
        vec![vec![0, 2]],
        "the copy is members 0 and 2, across the stranger at 1"
    );
    assert_eq!(
        edges,
        vec![vec![(0, 2)]],
        "only the edge joining the two surviving members is kept; an edge \
         pointing at the dropped stranger would misreport the component's \
         cross-file strength"
    );
}

// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] The price the arbitration
// names, charged where a reader can see it. Members 0 and 1 share every
// byte, so the old predicate protected them; they also share a file,
// which is proof of the idiom the filter just recognised rather than
// proof of a paste.
#[test]
fn an_intra_file_verbatim_family_takes_the_suppression_with_its_component() {
    let corpus = Corpus::across_files(&[&[RETRY_DEFAULTS, RETRY_DEFAULTS, THEME_TOKENS]]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 2]],
        "a byte-identical family that never leaves its file is not a copy —          nothing is partitioned off it, and the whole component is handed on          for the report to hide"
    );
}

// The discriminating case: one suppressed component holding both kinds
// of family at once. Only the family that crossed a file boundary is a
// copy, and only it escapes.
#[test]
fn only_the_cross_file_family_escapes_a_suppressed_component() {
    let corpus = Corpus::across_files(&[
        &[RETRY_DEFAULTS],
        &[RETRY_DEFAULTS],
        &[THEME_TOKENS, THEME_TOKENS],
    ]);
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1]],
        "the retry table is byte-identical across two files, so it is a paste and \
         survives the suppression its cluster-mates earned; the theme table is \
         byte-identical twice inside one file, so it is the idiom and leaves with \
         the component — a pass that kept it would republish scaffolding as a clone"
    );
}

// [CLONE-NOISE-VERBATIM-SUBGROUP-EXACT-BYTES] A family is sized by the
// occurrences it holds, not the fingerprints. Two views of one location
// are byte-identical by construction, so counting them as two members
// made every component holding a multi-statement body look splittable:
// the noise filters re-parsed and convicted components no split could
// ever change, and their counters reported those convictions as work
// the corpus had asked for ([PERF-FLUTTER-TODO-OBSERVABILITY]).
#[test]
fn one_location_seen_twice_is_not_a_splittable_family() {
    let mut corpus = Corpus::new(&[RETRY_DEFAULTS, THEME_TOKENS]);
    corpus.duplicate_view_of(0);
    corpus.duplicate_view_of(1);
    let whole = component(corpus.fingerprints.len());
    assert_eq!(
        corpus.fingerprints.len(),
        4,
        "two tables, each fingerprinted twice over its own range"
    );
    assert!(
        splittable_families(&whole, &corpus.fingerprints, &corpus.sources).is_none(),
        "neither table was copied anywhere: each byte-identical group is one \
         location seen twice, and no split of this component is possible"
    );
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 2, 3]],
        "so the component is handed on whole, exactly as it arrived"
    );
}

// The positive control for the same rule: a family that really does
// cover two locations is still splittable, and it keeps every view of
// them. Dropping the second view here would rob the same-file overlap
// collapse of the candidate it selects a representative from
// ([PIPELINE-CLUSTER-EXACT]).
#[test]
fn a_copy_stays_splittable_and_keeps_both_views_of_its_locations() {
    let mut corpus = Corpus::across_files(&[&[RETRY_DEFAULTS], &[RETRY_DEFAULTS], &[THEME_TOKENS]]);
    corpus.duplicate_view_of(0);
    let whole = component(corpus.fingerprints.len());
    assert_eq!(
        splittable_families(&whole, &corpus.fingerprints, &corpus.sources),
        Some(vec![vec![0, 1, 3]]),
        "the retry table sits in two files, so it covers two locations and a \
         split can act; the second view of member 0 belongs to the family"
    );
    assert_eq!(
        member_lists(&corpus.split_all()),
        vec![vec![0, 1, 3]],
        "the copy survives the suppression its cluster-mate earned, carrying \
         both views of its first location; the theme table is copied nowhere \
         and leaves with the component"
    );
}
