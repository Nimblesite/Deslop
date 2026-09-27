//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] An authored exact extent keeps
//! its original identity when two narrower exact windows span it.

use std::collections::HashMap;

use super::coalesce_exact_runs;
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::ClusterKind,
    cluster::{duplicate_mass, identity::Unnamed},
    fingerprint::Fingerprint,
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
const SHIFTED_FIRST: [ByteRange; COPIES] = [FIRST, SECOND];
const SHIFTED_SECOND: [ByteRange; COPIES] = [SECOND, FIRST];
const SHIFTED_SOURCES: [&[u8]; COPIES] = [b"ABCDABxx", b"CDABCDxx"];
const SHIFTED_FIRST_TEXT: &[u8] = b"ABCD";
const SHIFTED_SECOND_TEXT: &[u8] = b"CDAB";
const CHILD_SPANS: [ByteRange; 4] = [
    ByteRange { start: 0, end: 2 },
    ByteRange { start: 2, end: 4 },
    ByteRange { start: 4, end: 6 },
    ByteRange { start: 6, end: 8 },
];
const WINDOW_NODES: usize = 30;
const AUTHORED_NODES: usize = 45;
const COPIES: usize = 2;
const AUTHORED_VIEW_COUNT: usize = 1;
const AUTHORED_HASH: [u8; 32] = [7; 32];
const WINDOW_HASH: [u8; 32] = [3; 32];
const MISSING_COPY: &str = "the copied run must survive";
const FILE_NAMES: [&str; COPIES] = ["alpha.ts", "beta.ts"];

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
    let members = files
        .into_iter()
        .zip(ranges)
        .map(|(file_id, byte_range)| test_member(file_id, byte_range, nodes, hash))
        .collect();
    Unnamed {
        members,
        kind: ClusterKind::Identical,
        mass: duplicate_mass(ClusterKind::Identical, nodes, COPIES),
        shape_family: None,
    }
}

fn files() -> [FileId; COPIES] {
    let mut registry = FileRegistry::new();
    FILE_NAMES.map(|name| registry.register(name.into()))
}

fn coalesced(drafts: Vec<Unnamed>, files: [FileId; COPIES]) -> Vec<Unnamed> {
    let trees = files.map(tree);
    let sources: HashMap<_, _> = files
        .into_iter()
        .map(|file| (file, SOURCE.to_vec()))
        .collect();
    coalesce_exact_runs(drafts, &trees, &sources)
}

fn assert_authored_view(authored: &Unnamed) {
    assert_eq!(authored.members.len(), COPIES);
    assert_eq!(member_ranges(authored), vec![COMPLETE; COPIES]);
    let counts: Vec<_> = authored
        .members
        .iter()
        .map(|member| member.node_count)
        .collect();
    let hashes: Vec<_> = authored.members.iter().map(|member| member.hash).collect();
    assert_eq!(counts, vec![AUTHORED_NODES; COPIES]);
    assert_eq!(hashes, vec![AUTHORED_HASH; COPIES]);
}

fn member_ranges(view: &Unnamed) -> Vec<ByteRange> {
    view.members
        .iter()
        .map(|member| member.byte_range)
        .collect()
}

fn assert_one_view(result: &[Unnamed]) -> Result<&Unnamed, &'static str> {
    assert_eq!(
        result.len(),
        AUTHORED_VIEW_COUNT,
        "one copied run is one finding"
    );
    result.first().ok_or(MISSING_COPY)
}

fn extended_drafts(files: [FileId; COPIES]) -> Vec<Unnamed> {
    vec![
        split_draft(files, CROSSED_FIRST, WINDOW_NODES, WINDOW_HASH),
        split_draft(files, CROSSED_SECOND, WINDOW_NODES, WINDOW_HASH),
        draft(files, CROSSED_AUTHORED, AUTHORED_NODES, AUTHORED_HASH),
        draft(files, CROSSED_EXTENSION, WINDOW_NODES, WINDOW_HASH),
    ]
}

fn assert_full_view(full: &Unnamed) {
    assert_eq!(full.kind, ClusterKind::Identical);
    assert_eq!(full.members.len(), COPIES);
    assert_eq!(member_ranges(full), vec![MAXIMAL; COPIES]);
    let hashes: Vec<_> = full.members.iter().map(|member| member.hash).collect();
    assert_eq!(hashes, vec![*blake3::hash(SOURCE).as_bytes(); COPIES]);
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

fn assert_shifted_views(result: &[Unnamed]) {
    assert_eq!(result.len(), COPIES);
    assert_eq!(
        result.iter().map(member_ranges).collect::<Vec<_>>(),
        vec![SHIFTED_FIRST.to_vec(), SHIFTED_SECOND.to_vec()]
    );
    let sizes: Vec<_> = result.iter().map(|view| view.members.len()).collect();
    let kinds: Vec<_> = result.iter().map(|view| view.kind).collect();
    assert_eq!(sizes, vec![COPIES; COPIES]);
    assert_eq!(kinds, vec![ClusterKind::Identical; COPIES]);
}

#[test]
fn equal_windows_with_different_complete_unions_stay_separate() {
    let files = files();
    let trees = files.map(tree);
    let sources: HashMap<_, _> = files
        .into_iter()
        .zip(SHIFTED_SOURCES.map(<[u8]>::to_vec))
        .collect();
    let result = coalesce_exact_runs(shifted_drafts(files), &trees, &sources);
    assert_shifted_views(&result);
}

#[test]
fn an_existing_exact_extent_keeps_its_merkle_identity_and_count() -> Result<(), &'static str> {
    let files = files();
    let drafts = vec![
        draft(files, FIRST, WINDOW_NODES, WINDOW_HASH),
        draft(files, SECOND, WINDOW_NODES, WINDOW_HASH),
        draft(files, COMPLETE, AUTHORED_NODES, AUTHORED_HASH),
    ];
    let result = coalesced(drafts, files);
    let authored = assert_one_view(&result)?;
    assert_authored_view(authored);
    Ok(())
}

#[test]
fn an_existing_exact_extent_can_extend_to_the_full_copied_run() -> Result<(), &'static str> {
    let files = files();
    let result = coalesced(extended_drafts(files), files);
    let full = assert_one_view(&result)?;
    assert_full_view(full);
    Ok(())
}
