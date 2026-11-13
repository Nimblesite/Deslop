//! [FUSED-CONTENT-GATE-LITERAL-BOUND] One string is not a code structure.
//!
//! A long string parses into a run of text and escape fragments. Two
//! unrelated strings with similar runs — a usage message and an embedded
//! test program, sharing only `\n` and `\"` — then look like two copies
//! of one shape, and a near-miss pair is admitted on shape alone. When
//! one authored literal's fragments outnumber everything else in an
//! endpoint, that shape is the literal's own text layout. And when one
//! literal is most of an endpoint's source text — a multi-line SQL query
//! assigned and given two parameters — the few statements around it are
//! all two endpoints can share. Either way the pair is a copy only if
//! the two literals hold the same text.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lang::shared::LITERAL_KIND,
    state::FileId,
    tokens::resolve_fingerprint_nodes,
};

/// A literal carries its endpoint when it is a strict majority of the
/// endpoint's nodes or of its source bytes: more than one in every this-many.
const MAJORITY_DENOMINATOR: usize = 2;
/// The literal node itself, counted beside its fragments.
const LITERAL_NODE: usize = 1;

/// An endpoint's heaviest authored literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaviestLiteral {
    /// File the literal lives in.
    file: FileId,
    /// Source bytes of the whole literal.
    range: ByteRange,
    /// Whether the literal is most of the endpoint, by nodes or by bytes.
    carries_endpoint: bool,
}

/// The heaviest literal under `fingerprint`, `None` when the endpoint
/// does not resolve or holds no literal. Ties keep the first in source order.
pub(crate) fn heaviest_literal(
    tree_index: &HashMap<FileId, &NormalizedNode>,
    fingerprint: &Fingerprint,
) -> Option<HeaviestLiteral> {
    let root = tree_index.get(&fingerprint.file_id)?;
    let mut heaviest = None;
    for member in resolve_fingerprint_nodes(root, fingerprint)? {
        weigh_literals(member, &mut heaviest);
    }
    heaviest.map(|literal: Weighed| HeaviestLiteral {
        file: fingerprint.file_id,
        range: literal.range,
        carries_endpoint: is_majority(literal.nodes, fingerprint.node_count)
            || is_majority(span(literal.range), span(fingerprint.byte_range)),
    })
}

/// One literal's weight inside an endpoint.
#[derive(Clone, Copy)]
struct Weighed {
    /// The literal node plus its fragments.
    nodes: usize,
    /// Source bytes of the whole literal.
    range: ByteRange,
}

impl Weighed {
    /// Heavier by fragments first, then by text: a raw string is one
    /// fragment however long it runs.
    fn outweighs(self, other: Self) -> bool {
        (self.nodes, span(self.range)) > (other.nodes, span(other.range))
    }
}

/// Byte length of a range.
const fn span(range: ByteRange) -> usize {
    range.end.saturating_sub(range.start)
}

/// Whether `part` is more than half of `whole`.
const fn is_majority(part: usize, whole: usize) -> bool {
    part.saturating_mul(MAJORITY_DENOMINATOR) > whole
}

/// Pre-order walk keeping the literal with the most fragments. A fragment
/// is a leaf child: code inside an interpolation is structure, not text.
fn weigh_literals(node: &NormalizedNode, heaviest: &mut Option<Weighed>) {
    if node.kind == LITERAL_KIND {
        let fragments = node
            .children
            .iter()
            .filter(|child| child.children.is_empty())
            .count();
        let literal = Weighed {
            nodes: fragments.saturating_add(LITERAL_NODE),
            range: node.byte_range,
        };
        if heaviest.map_or(true, |best| literal.outweighs(best)) {
            *heaviest = Some(literal);
        }
    }
    for child in &node.children {
        weigh_literals(child, heaviest);
    }
}

/// Whether a literal-carried endpoint is paired with different text: one
/// side is carried by its literal and the other side's heaviest literal
/// does not hold the same bytes.
pub(crate) fn carried_text_differs<S: BuildHasher>(
    left: Option<HeaviestLiteral>,
    right: Option<HeaviestLiteral>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> bool {
    let carried = [left, right]
        .into_iter()
        .flatten()
        .any(|literal| literal.carries_endpoint);
    let same_text = left.zip(right).is_some_and(|(left, right)| {
        match (bytes(left, sources), bytes(right, sources)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    });
    carried && !same_text
}

/// Raw source bytes of one literal.
fn bytes<S: BuildHasher>(
    literal: HeaviestLiteral,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<&[u8]> {
    sources
        .get(&literal.file)?
        .get(literal.range.start..literal.range.end)
}
