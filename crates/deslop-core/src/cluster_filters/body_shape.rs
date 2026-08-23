//! One definition of "these two function bodies are the same shape",
//! shared by every noise filter that must separate *different
//! implementations* from *renamed copies* ([CLONE-NOISE-SIGNATURE-ONLY],
//! [CLONE-NOISE-POLYMORPHIC-SIGNATURE]). Two filters carrying their own
//! comparator drift apart on exactly the cases that decide a report —
//! this file is the single place the semantics live.

use tree_sitter::Node;

use crate::lang::shared::is_behaviour_bearing_token;

/// Node kinds that reach *outside* the body: a call and a member
/// access. Which collaborator a body reaches for is behaviour, so the
/// identifier naming it joins the stream as bytes while every local and
/// parameter stays erased. Reading kinds alone made
/// `self.containers[i] … container.invoke(…)` and
/// `self.machines[i] … machine.execute(…)` one identical stream, so
/// both suppressions answered "these bodies are the same" about two
/// implementations of one abstract contract that share nothing but the
/// signature the contract forces ([CLONE-NOISE-POLYMORPHIC-SIGNATURE]).
const REACH_KINDS: &[&str] = &[
    "call",
    "call_expression",
    "invocation_expression",
    "attribute",
    "field_expression",
    "member_access_expression",
    "member_expression",
];

/// Fields of a [`REACH_KINDS`] node naming the collaborator rather than
/// the receiver: Python's `function`/`attribute`, Rust's `field`,
/// TypeScript's `property`, C#'s `name`.
const REACH_FIELDS: &[&str] = &["function", "attribute", "field", "property", "name"];

/// One element of a [`body_kind_stream`].
#[derive(PartialEq, Eq)]
pub(super) enum ShapeToken<'src> {
    /// A node's grammar kind id.
    Kind(u16),
    /// The bytes of an identifier the body reaches for.
    Symbol(&'src [u8]),
    /// Marks the end of a subtree, so the stream encodes nesting and
    /// arity, not just a flat sequence — without it `A(B, C)` and
    /// `A(B(C))` would linearise identically.
    Close,
}

/// The owned mirror of [`ShapeToken`], for memoising a body stream in
/// [`ParseCache`](super::snippets::ParseCache) beyond the source
/// borrow. Identical comparison semantics — conversions are lossless.
#[derive(PartialEq, Eq, Clone)]
pub(super) enum OwnedShapeToken {
    /// A node's grammar kind id.
    Kind(u16),
    /// The bytes of an identifier the body reaches for.
    Symbol(Vec<u8>),
    /// Marks the end of a subtree.
    Close,
}

impl From<&ShapeToken<'_>> for OwnedShapeToken {
    fn from(token: &ShapeToken<'_>) -> Self {
        match token {
            ShapeToken::Kind(kind) => Self::Kind(*kind),
            ShapeToken::Symbol(bytes) => Self::Symbol(bytes.to_vec()),
            ShapeToken::Close => Self::Close,
        }
    }
}

/// A pending step in the iterative [`body_kind_stream`] walk. Explicit
/// frames keep the walk stack-safe on adversarially deep trees, the
/// same discipline the fingerprint hasher follows.
enum Frame<'tree> {
    /// Emit this node's kind and schedule its children. The flag marks
    /// a leaf reached for through a [`REACH_FIELDS`] field, whose bytes
    /// join the stream after its kind.
    Enter(Node<'tree>, bool),
    /// Emit [`ShapeToken::Close`] for the subtree just finished.
    Exit,
}

/// Depth-first stream of `body`'s node kinds with a
/// [`ShapeToken::Close`] marker per subtree, carrying the bytes of every
/// collaborator the body reaches for. Grammar extras (comments) are
/// skipped and no other source text is emitted, so two bodies compare
/// equal exactly when they are the same construct tree calling the same
/// collaborators — regardless of what their locals and parameters are
/// named, what their comments say, or which values their literals carry.
///
/// Named children *and* behaviour-bearing anonymous tokens
/// ([PIPELINE-NORMALIZE-AST-OPERATOR]). Reading named children alone
/// made `self.total = base + fee` and `self.total = base - fee`
/// identical streams, so every filter that asks "do these two bodies
/// differ?" — the signature-only suppression and the polymorphic
/// suppression both — answered *no* about implementations that compute
/// different answers, and suppressed accordingly. Framing punctuation
/// stays out: brackets and commas are already implied by the parent
/// kind and the [`ShapeToken::Close`] markers.
pub(super) fn body_kind_stream<'src>(body: Node<'_>, source: &'src [u8]) -> Vec<ShapeToken<'src>> {
    let mut stream = Vec::new();
    let mut stack = vec![Frame::Enter(body, false)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Exit => stream.push(ShapeToken::Close),
            Frame::Enter(node, _) if node.is_extra() => {}
            Frame::Enter(node, collaborator) => {
                stream.push(ShapeToken::Kind(node.kind_id()));
                if collaborator {
                    stream.extend(source.get(node.byte_range()).map(ShapeToken::Symbol));
                }
                stack.push(Frame::Exit);
                push_child_frames(node, &mut stack);
            }
        }
    }
    stream
}

/// Pushes `node`'s comparable children as [`Frame::Enter`] entries in
/// reverse so the stack pops them in source order, flagging the ones
/// that name a collaborator.
fn push_child_frames<'tree>(node: Node<'tree>, stack: &mut Vec<Frame<'tree>>) {
    let reaches = REACH_KINDS.contains(&node.kind());
    let mut cursor = node.walk();
    let mut children: Vec<Frame<'tree>> = Vec::new();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named() || is_behaviour_bearing_token(child.kind()) {
                children.push(Frame::Enter(child, reaches && names_collaborator(&cursor)));
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    stack.extend(children.into_iter().rev());
}

/// True when the cursor sits on a leaf identifier held by a
/// [`REACH_FIELDS`] field. A non-leaf there — `container.invoke` as a
/// call's `function` — carries the collaborator deeper in, where the
/// walk reaches it under its own field.
fn names_collaborator(cursor: &tree_sitter::TreeCursor<'_>) -> bool {
    cursor.node().named_child_count() == 0
        && cursor
            .field_name()
            .is_some_and(|field| REACH_FIELDS.contains(&field))
}
