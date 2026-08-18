//! Regression test for GH #336 stack overflow on deep ASTs.

use deslop_core::{
    ast::{ByteRange, NormalizedNode},
    lang::shared::MAX_AST_DEPTH,
};

#[test]
fn test_deep_normalized_node_drop_does_not_overflow_stack() {
    let mut registry = deslop_core::state::FileRegistry::new();
    let file_id = registry.register(std::path::PathBuf::from("deep.fs"));
    let mut current = NormalizedNode {
        kind: "leaf",
        children: Vec::new(),
        byte_range: ByteRange { start: 0, end: 10 },
        file_id,
    };

    for i in 0..5000 {
        current = NormalizedNode {
            kind: "node",
            children: vec![current],
            byte_range: ByteRange {
                start: 0,
                end: 5000 - i,
            },
            file_id,
        };
    }

    // Run drop on a 256 KB stack thread to verify iterative stack safety
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            drop(current);
        })
        .unwrap();

    handle
        .join()
        .expect("dropping deep NormalizedNode tree overflowed stack");
}

/// Builds a `depth`-deep chain of single-child nodes.
fn deep_chain(depth: usize, file_id: deslop_core::state::FileId) -> NormalizedNode {
    let mut current = NormalizedNode {
        kind: "leaf",
        children: Vec::new(),
        byte_range: ByteRange { start: 0, end: 1 },
        file_id,
    };
    for level in 1..depth {
        current = NormalizedNode {
            kind: "node",
            children: vec![current],
            byte_range: ByteRange {
                start: 0,
                end: level + 1,
            },
            file_id,
        };
    }
    current
}

/// The depth guard is only a guard if the walks it admits actually survive
/// the smallest stack they run on.
///
/// This replaces an assertion that `MAX_AST_DEPTH <= 200`. That pinned a
/// number, not a behaviour: it passed whether or not the walks overflowed,
/// and it was satisfied by capping the guard at 150 — which bought stack
/// safety by skipping 36 real files in `dotnet/fsharp`, trading a crash for
/// silent false negatives. Hashing is iterative now, so the walks are
/// exercised at the full admitted depth on a 1 MB stack instead.
#[test]
fn walks_at_max_ast_depth_survive_a_1mb_stack() {
    let mut registry = deslop_core::state::FileRegistry::new();
    let file_id = registry.register(std::path::PathBuf::from("deep.fs"));
    let root = deep_chain(MAX_AST_DEPTH, file_id);

    let handle = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || {
            let fingerprints = deslop_core::fingerprint::collect_fingerprints(&root, 2);
            let siblings = deslop_core::sibling::collect_sibling_fingerprints(&root, 2);
            (fingerprints.len(), siblings.len())
        })
        .expect("spawning the walk thread");

    let (fingerprint_count, _sibling_count) = handle
        .join()
        .expect("walking a MAX_AST_DEPTH-deep tree overflowed a 1 MB stack");

    // Every node but the leaf roots a subtree of >= 2 nodes, so the walk must
    // have reached the bottom rather than bailing out part-way.
    assert_eq!(
        fingerprint_count,
        MAX_AST_DEPTH - 1,
        "expected one fingerprint per non-leaf node at depth {MAX_AST_DEPTH}"
    );
}
