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
    /// body by body — so a family reads as a list of positions whether
    /// or not its members share a file, which is what
    /// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] turns on.
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
                    strength: 1.0,
                })
        })
        .collect();
    FusedCluster { members, edges }
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
