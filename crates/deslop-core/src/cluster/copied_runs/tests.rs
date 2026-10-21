//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] Overlapping windows add the run
//! the judge admits their union as, and every window the scan already
//! read stays in place for the kernel.

use std::collections::HashMap;

use super::coalesce_copied_runs;
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::ClusterKind,
    cluster::{duplicate_mass, identity::Unnamed, ClusterKindJudge},
    fingerprint::{subtree_hash, Fingerprint, HashScratch},
    sibling::window_hash_for_nodes,
    state::{FileId, FileRegistry},
};

const SOURCE: &[u8] = b"xxxxxxxx";
const FIRST: ByteRange = ByteRange { start: 0, end: 4 };
const SECOND: ByteRange = ByteRange { start: 2, end: 6 };
const COMPLETE: ByteRange = ByteRange { start: 0, end: 6 };
const MAXIMAL: ByteRange = ByteRange { start: 0, end: 8 };
const CROSSED_FIRST: [ByteRange; COPIES] = [
    ByteRange { start: 2, end: 6 },
    ByteRange { start: 4, end: 8 },
];
const CROSSED_SECOND: [ByteRange; COPIES] = [
    ByteRange { start: 4, end: 8 },
    ByteRange { start: 2, end: 6 },
];
const CROSSED_AUTHORED: ByteRange = ByteRange { start: 2, end: 8 };
const CROSSED_EXTENSION: ByteRange = ByteRange { start: 0, end: 4 };
/// The second statement alone: strictly inside `FIRST`, `SECOND` and
/// `COMPLETE`, as a block sits inside the windows padded around it.
const NESTED: ByteRange = ByteRange { start: 2, end: 4 };
const SHIFTED_FIRST: [ByteRange; COPIES] = [FIRST, SECOND];
const SHIFTED_SECOND: [ByteRange; COPIES] = [SECOND, FIRST];
const SHIFTED_SOURCES: [&[u8]; COPIES] = [b"ABCDABxx", b"CDABCDxx"];
const SHIFTED_FIRST_TEXT: &[u8] = b"ABCD";
const SHIFTED_SECOND_TEXT: &[u8] = b"CDAB";
/// The same shape in both files, with every name changed.
const RENAMED_SOURCES: [&[u8]; COPIES] = [b"ABCDEFGH", b"abcdefgh"];
const CHILD_SPANS: [ByteRange; 4] = [
    ByteRange { start: 0, end: 2 },
    ByteRange { start: 2, end: 4 },
    ByteRange { start: 4, end: 6 },
    ByteRange { start: 6, end: 8 },
];
/// Children of the tree that `COMPLETE` spans: the first three statements.
const COMPLETE_CHILDREN: (usize, usize) = (0, 2);
/// Nodes in the three-statement run `COMPLETE` resolves to.
const COMPLETE_NODES: usize = 3;
/// Nodes in the whole tree `MAXIMAL` resolves to: the file and its four statements.
const MAXIMAL_NODES: usize = 5;
const WINDOW_NODES: usize = 30;
const AUTHORED_NODES: usize = 45;
const NESTED_NODES: usize = 12;
const COPIES: usize = 2;
const AUTHORED_HASH: [u8; 32] = [7; 32];
const WINDOW_HASH: [u8; 32] = [3; 32];
const NESTED_HASH: [u8; 32] = [5; 32];
const MISSING_RUN: &str = "the copied run must be added";
const MISSING_WINDOW: &str = "the complete statement window lies inside every tree";
const FILE_NAMES: [&str; COPIES] = ["alpha.ts", "beta.ts"];

/// The unit stand-in for the session's pair measurement: identical bytes
/// are an identical copy, and differing bytes are whatever `renamed`
/// says — a nearly identical copy, or nothing.
struct SourceJudge {
    sources: HashMap<FileId, Vec<u8>>,
    renamed: Option<ClusterKind>,
}

impl SourceJudge {
    fn new(sources: HashMap<FileId, Vec<u8>>, renamed: Option<ClusterKind>) -> Self {
        Self { sources, renamed }
    }

    fn bytes(&self, member: &Fingerprint) -> Option<&[u8]> {
        self.sources
            .get(&member.file_id)?
            .get(member.byte_range.start..member.byte_range.end)
    }
}

impl ClusterKindJudge for SourceJudge {
    /// The join measures spans, never indexed members.
    fn kind(&self, _members: &[usize]) -> Option<ClusterKind> {
        None
    }

    fn span_kind(&self, left: &Fingerprint, right: &Fingerprint) -> Option<ClusterKind> {
        self.bytes(left)
            .zip(self.bytes(right))
            .is_some_and(|(first, second)| first == second)
            .then_some(ClusterKind::Identical)
            .or(self.renamed)
    }
}

fn tree(file_id: FileId) -> NormalizedNode {
    let children = CHILD_SPANS
        .into_iter()
        .map(|byte_range| NormalizedNode {
            kind: "statement",
            children: Vec::new(),
            byte_range,
            file_id,
        })
        .collect();
    NormalizedNode {
        kind: "file",
        children,
        byte_range: MAXIMAL,
        file_id,
    }
}

fn draft(files: [FileId; COPIES], range: ByteRange, nodes: usize, hash: [u8; 32]) -> Unnamed {
    split_draft(files, [range; COPIES], nodes, hash)
}

fn test_member(
    file_id: FileId,
    byte_range: ByteRange,
    nodes: usize,
    hash: [u8; 32],
) -> Fingerprint {
    Fingerprint {
        hash,
        file_id,
        byte_range,
        node_count: nodes,
    }
}

fn split_draft(
    files: [FileId; COPIES],
    ranges: [ByteRange; COPIES],
    nodes: usize,
    hash: [u8; 32],
) -> Unnamed {
    kind_draft(files, ranges, nodes, hash, ClusterKind::Identical)
}

fn kind_draft(
    files: [FileId; COPIES],
    ranges: [ByteRange; COPIES],
    nodes: usize,
    hash: [u8; 32],
    kind: ClusterKind,
) -> Unnamed {
    let members = files
        .into_iter()
        .zip(ranges)
        .map(|(file_id, byte_range)| test_member(file_id, byte_range, nodes, hash))
        .collect();
    Unnamed {
        members,
        kind,
        mass: duplicate_mass(kind, nodes, COPIES),
        shape_family: None,
    }
}

fn files() -> [FileId; COPIES] {
    let mut registry = FileRegistry::new();
    FILE_NAMES.map(|name| registry.register(name.into()))
}

fn sources(files: [FileId; COPIES], texts: [&[u8]; COPIES]) -> HashMap<FileId, Vec<u8>> {
    files.into_iter().zip(texts.map(<[u8]>::to_vec)).collect()
}

/// Joins `drafts` over byte-identical sources, so every admitted union is
/// an identical copy.
fn coalesced(drafts: Vec<Unnamed>, files: [FileId; COPIES]) -> Vec<Unnamed> {
    let judge = SourceJudge::new(sources(files, [SOURCE; COPIES]), None);
    coalesce_copied_runs(drafts, &files.map(tree), &judge)
}

/// Joins `drafts` over renamed sources, with `renamed` as the judge's
/// verdict on every union whose bytes differ.
fn coalesced_renamed(
    drafts: Vec<Unnamed>,
    files: [FileId; COPIES],
    renamed: Option<ClusterKind>,
) -> Vec<Unnamed> {
    let judge = SourceJudge::new(sources(files, RENAMED_SOURCES), renamed);
    coalesce_copied_runs(drafts, &files.map(tree), &judge)
}

/// The digest the scan emits for the statements `first..=last` of a file,
/// or nothing when the window lies outside the tree.
fn window_digest(file: FileId, (first, last): (usize, usize)) -> Option<[u8; 32]> {
    let tree = tree(file);
    let run: Vec<&NormalizedNode> = tree.children.get(first..=last)?.iter().collect();
    Some(window_hash_for_nodes(&run))
}

fn member_ranges(view: &Unnamed) -> Vec<ByteRange> {
    view.members
        .iter()
        .map(|member| member.byte_range)
        .collect()
}

fn ranges(views: &[Unnamed]) -> Vec<Vec<ByteRange>> {
    views.iter().map(member_ranges).collect()
}

fn node_counts(view: &Unnamed) -> Vec<usize> {
    view.members
        .iter()
        .map(|member| member.node_count)
        .collect()
}

fn hashes(view: &Unnamed) -> Vec<[u8; 32]> {
    view.members.iter().map(|member| member.hash).collect()
}

/// A view is exactly the draft it came in as: extent, digest, count and kind.
fn assert_untouched(view: &Unnamed, range: ByteRange, nodes: usize, hash: [u8; 32]) {
    assert_eq!(member_ranges(view), vec![range; COPIES]);
    assert_eq!(node_counts(view), vec![nodes; COPIES]);
    assert_eq!(hashes(view), vec![hash; COPIES]);
    assert_eq!(view.kind, ClusterKind::Identical);
}

fn extended_drafts(files: [FileId; COPIES]) -> Vec<Unnamed> {
    vec![
        split_draft(files, CROSSED_FIRST, WINDOW_NODES, WINDOW_HASH),
        split_draft(files, CROSSED_SECOND, WINDOW_NODES, WINDOW_HASH),
        draft(files, CROSSED_AUTHORED, AUTHORED_NODES, AUTHORED_HASH),
        draft(files, CROSSED_EXTENSION, WINDOW_NODES, WINDOW_HASH),
    ]
}

/// The whole tree is the run: its members carry the file's own Merkle
/// digest and node count, as the scan would have emitted them.
fn assert_full_view(full: &Unnamed, files: [FileId; COPIES]) {
    assert_eq!(full.kind, ClusterKind::Identical);
    assert_eq!(full.members.len(), COPIES);
    assert_eq!(member_ranges(full), vec![MAXIMAL; COPIES]);
    assert_eq!(node_counts(full), vec![MAXIMAL_NODES; COPIES]);
    let expected: Vec<[u8; 32]> = files
        .map(|file| subtree_hash(&tree(file), &mut HashScratch::default()))
        .to_vec();
    assert_eq!(hashes(full), expected);
    assert_eq!(
        full.mass,
        duplicate_mass(ClusterKind::Identical, MAXIMAL_NODES, COPIES)
    );
}

fn shifted_drafts(files: [FileId; COPIES]) -> Vec<Unnamed> {
    vec![
        split_draft(
            files,
            SHIFTED_FIRST,
            WINDOW_NODES,
            *blake3::hash(SHIFTED_FIRST_TEXT).as_bytes(),
        ),
        split_draft(
            files,
            SHIFTED_SECOND,
            WINDOW_NODES,
            *blake3::hash(SHIFTED_SECOND_TEXT).as_bytes(),
        ),
    ]
}

/// Two overlapping windows of one renamed copy: the first verbatim, the
/// second renamed.
fn renamed_drafts(files: [FileId; COPIES]) -> Vec<Unnamed> {
    vec![
        draft(files, FIRST, WINDOW_NODES, WINDOW_HASH),
        kind_draft(
            files,
            [SECOND; COPIES],
            WINDOW_NODES,
            WINDOW_HASH,
            ClusterKind::NearlyIdentical,
        ),
    ]
}

fn assert_two_windows(
    result: &[Unnamed],
    expected: [[ByteRange; COPIES]; COPIES],
    kinds: [ClusterKind; COPIES],
) {
    assert_eq!(result.len(), COPIES);
    assert_eq!(
        ranges(result),
        expected.map(|range| range.to_vec()).to_vec()
    );
    let sizes: Vec<_> = result.iter().map(|view| view.members.len()).collect();
    assert_eq!(sizes, vec![COPIES; COPIES]);
    assert_eq!(
        result.iter().map(|view| view.kind).collect::<Vec<_>>(),
        kinds.to_vec()
    );
}

#[test]
fn equal_windows_with_different_complete_unions_stay_separate() {
    let files = files();
    let judge = SourceJudge::new(sources(files, SHIFTED_SOURCES), None);
    let result = coalesce_copied_runs(shifted_drafts(files), &files.map(tree), &judge);
    assert_two_windows(
        &result,
        [SHIFTED_FIRST, SHIFTED_SECOND],
        [ClusterKind::Identical; COPIES],
    );
}

/// An extent the scan already fingerprinted is not derived again: the
/// authored view keeps its own digest and count, and the windows it
/// encloses are left for the kernel to absorb.
#[test]
fn an_existing_exact_extent_keeps_its_merkle_identity_and_count() {
    let files = files();
    let drafts = vec![
        draft(files, FIRST, WINDOW_NODES, WINDOW_HASH),
        draft(files, SECOND, WINDOW_NODES, WINDOW_HASH),
        draft(files, COMPLETE, AUTHORED_NODES, AUTHORED_HASH),
    ];
    let result = coalesced(drafts, files);
    assert_eq!(
        ranges(&result),
        vec![
            vec![FIRST; COPIES],
            vec![SECOND; COPIES],
            vec![COMPLETE; COPIES]
        ],
        "nothing is added and nothing is removed"
    );
    for (view, (range, nodes, hash)) in result.iter().zip([
        (FIRST, WINDOW_NODES, WINDOW_HASH),
        (SECOND, WINDOW_NODES, WINDOW_HASH),
        (COMPLETE, AUTHORED_NODES, AUTHORED_HASH),
    ]) {
        assert_untouched(view, range, nodes, hash);
    }
}

/// Joining adds the widest admitted union as one new view and leaves
/// every window in place — enclosure is the kernel's verdict
/// ([PIPELINE-CLUSTER-SUBSUME]) — with their identities untouched.
#[test]
fn an_existing_exact_extent_can_extend_to_the_full_copied_run() -> Result<(), &'static str> {
    let files = files();
    let result = coalesced(extended_drafts(files), files);
    assert_eq!(
        ranges(&result),
        vec![
            CROSSED_FIRST.to_vec(),
            CROSSED_SECOND.to_vec(),
            vec![CROSSED_AUTHORED; COPIES],
            vec![CROSSED_EXTENSION; COPIES],
            vec![MAXIMAL; COPIES],
        ],
        "the four windows, then the run they read"
    );
    assert_full_view(result.last().ok_or(MISSING_RUN)?, files);
    let authored = result.get(2).ok_or(MISSING_RUN)?;
    assert_untouched(authored, CROSSED_AUTHORED, AUTHORED_NODES, AUTHORED_HASH);
    Ok(())
}

/// A view nested inside an admitted extent stays: it is the core the
/// straddle rule reads when the extent's padded neighbours reach the
/// kernel ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]).
#[test]
fn a_view_nested_in_an_admitted_extent_is_kept() {
    let files = files();
    let drafts = vec![
        draft(files, FIRST, WINDOW_NODES, WINDOW_HASH),
        draft(files, SECOND, WINDOW_NODES, WINDOW_HASH),
        draft(files, COMPLETE, AUTHORED_NODES, AUTHORED_HASH),
        draft(files, NESTED, NESTED_NODES, NESTED_HASH),
    ];
    let result = coalesced(drafts, files);
    assert_eq!(
        ranges(&result),
        vec![
            vec![FIRST; COPIES],
            vec![SECOND; COPIES],
            vec![COMPLETE; COPIES],
            vec![NESTED; COPIES]
        ],
        "the windows, the authored extent and the nested view all stay"
    );
    for (view, (range, nodes, hash)) in result.iter().zip([
        (FIRST, WINDOW_NODES, WINDOW_HASH),
        (SECOND, WINDOW_NODES, WINDOW_HASH),
        (COMPLETE, AUTHORED_NODES, AUTHORED_HASH),
        (NESTED, NESTED_NODES, NESTED_HASH),
    ]) {
        assert_untouched(view, range, nodes, hash);
    }
}

/// A renamed copy joins exactly as a verbatim one does: the union the
/// judge admits as nearly identical is added as one nearly identical run,
/// at the scan's own digest and node count for those statements, beside
/// the two windows it was read from.
#[test]
fn renamed_windows_join_into_one_nearly_identical_run() -> Result<(), &'static str> {
    let files = files();
    let result = coalesced_renamed(
        renamed_drafts(files),
        files,
        Some(ClusterKind::NearlyIdentical),
    );
    assert_eq!(
        ranges(&result),
        vec![
            vec![FIRST; COPIES],
            vec![SECOND; COPIES],
            vec![COMPLETE; COPIES]
        ],
        "both windows stay, and the run they read is added"
    );
    let run = result.last().ok_or(MISSING_RUN)?;
    assert_eq!(run.kind, ClusterKind::NearlyIdentical);
    assert_eq!(node_counts(run), vec![COMPLETE_NODES; COPIES]);
    let complete_digests: Vec<[u8; 32]> = files
        .iter()
        .map(|file| window_digest(*file, COMPLETE_CHILDREN))
        .collect::<Option<_>>()
        .ok_or(MISSING_WINDOW)?;
    assert_eq!(hashes(run), complete_digests);
    assert_eq!(
        run.mass,
        duplicate_mass(ClusterKind::NearlyIdentical, COMPLETE_NODES, COPIES)
    );
    Ok(())
}

/// A shape-only verdict on the union is not a copied run: two padded
/// windows around a shared block stay two windows for the straddle rule.
#[test]
fn windows_whose_union_is_shape_only_stay_separate() {
    let files = files();
    let result = coalesced_renamed(
        renamed_drafts(files),
        files,
        Some(ClusterKind::StructuralOnly),
    );
    assert_two_windows(
        &result,
        [[FIRST; COPIES], [SECOND; COPIES]],
        [ClusterKind::Identical, ClusterKind::NearlyIdentical],
    );
}

/// Only identical and nearly identical windows read a copied run; a
/// similar pair is never joined, whatever the judge would say of the union.
#[test]
fn similar_windows_are_never_joined() {
    let files = files();
    let drafts = vec![
        kind_draft(
            files,
            [FIRST; COPIES],
            WINDOW_NODES,
            WINDOW_HASH,
            ClusterKind::LooselySimilar,
        ),
        kind_draft(
            files,
            [SECOND; COPIES],
            WINDOW_NODES,
            WINDOW_HASH,
            ClusterKind::LooselySimilar,
        ),
    ];
    let result = coalesced(drafts, files);
    assert_two_windows(
        &result,
        [[FIRST; COPIES], [SECOND; COPIES]],
        [ClusterKind::LooselySimilar; COPIES],
    );
}
