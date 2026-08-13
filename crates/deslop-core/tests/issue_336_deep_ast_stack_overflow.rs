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
            byte_range: ByteRange { start: 0, end: 5000 - i },
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

    handle.join().expect("dropping deep NormalizedNode tree overflowed stack");
}

#[test]
fn test_max_ast_depth_is_safe_for_limited_stacks() {
    // MAX_AST_DEPTH must be <= 200 so AST normalization depth bounds stay
    // well within 1 MB thread stack budgets (e.g. Windows main thread or small worker threads).
    assert!(
        MAX_AST_DEPTH <= 200,
        "MAX_AST_DEPTH ({MAX_AST_DEPTH}) is too large for 1MB thread stacks; must be <= 200"
    );
}
