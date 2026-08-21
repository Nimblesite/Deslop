//! One definition of "these two function bodies are the same shape",
//! shared by every noise filter that must separate *different
//! implementations* from *renamed copies* ([CLONE-NOISE-SIGNATURE-ONLY],
//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE]). Two filters carrying their own
//! comparator drift apart on exactly the cases that decide a report —
//! this file is the single place the semantics live.

use tree_sitter::Node;

/// Marks the end of a subtree in a [`body_kind_stream`], so the stream
/// encodes nesting and arity, not just a flat kind sequence — without
/// it `A(B, C)` and `A(B(C))` would linearise identically. Negative,
/// so it can never collide with a `u16` grammar kind id.
const SUBTREE_CLOSE: i32 = -1;

/// A pending step in the iterative [`body_kind_stream`] walk. Explicit
/// frames keep the walk stack-safe on adversarially deep trees, the
/// same discipline the fingerprint hasher follows.
enum Frame<'tree> {
    /// Emit this node's kind and schedule its named children.
    Enter(Node<'tree>),
    /// Emit [`SUBTREE_CLOSE`] for the subtree just finished.
    Exit,
}

/// Depth-first stream of `body`'s named node kinds with a
/// [`SUBTREE_CLOSE`] marker per subtree. Only node *kinds* are emitted
/// — never source text — and grammar extras (comments) are skipped, so
/// two bodies compare equal exactly when they are the same construct
/// tree regardless of what their identifiers and literals are named,
/// what their comments say, or which values their literals carry.
pub(super) fn body_kind_stream(body: Node<'_>) -> Vec<i32> {
    let mut stream = Vec::new();
    let mut stack = vec![Frame::Enter(body)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Exit => stream.push(SUBTREE_CLOSE),
            Frame::Enter(node) if node.is_extra() => {}
            Frame::Enter(node) => {
                stream.push(i32::from(node.kind_id()));
                stack.push(Frame::Exit);
                push_named_child_frames(node, &mut stack);
            }
        }
    }
    stream
}

/// Pushes `node`'s named children as [`Frame::Enter`] entries in
/// reverse so the stack pops them in source order.
fn push_named_child_frames<'tree>(node: Node<'tree>, stack: &mut Vec<Frame<'tree>>) {
    let mut cursor = node.walk();
    let children: Vec<Node<'tree>> = node.named_children(&mut cursor).collect();
    for child in children.into_iter().rev() {
        stack.push(Frame::Enter(child));
    }
}
