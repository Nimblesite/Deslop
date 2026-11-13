//! [FUSED-CONTENT-GATE-LITERAL-BOUND] One string is not a code structure.
//!
//! A long string parses into a run of text and escape fragments. Two
//! unrelated strings with similar runs — a usage message and an embedded
//! test program, sharing only `\n` and `\"` — then look like two copies
//! of one shape, and a near-miss pair is admitted on shape alone. When
//! one authored literal's fragments outnumber everything else in an
//! endpoint, that shape is the literal's own text layout, so the pair is
//! a copy only if the two literals hold the same text.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lang::shared::LITERAL_KIND,
    state::FileId,
    tokens::resolve_fingerprint_nodes,
};

/// A literal carries its endpoint when its nodes are a strict majority:
/// more than one node in every this-many.
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
    /// Whether the literal and its fragments outnumber the rest of the endpoint.
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
    heaviest.map(|(mass, range): (usize, ByteRange)| HeaviestLiteral {
        file: fingerprint.file_id,
        range,
        carries_endpoint: mass.saturating_mul(MAJORITY_DENOMINATOR) > fingerprint.node_count,
    })
}

/// Pre-order walk keeping the literal with the most fragments. A fragment
/// is a leaf child: code inside an interpolation is structure, not text.
fn weigh_literals(node: &NormalizedNode, heaviest: &mut Option<(usize, ByteRange)>) {
    if node.kind == LITERAL_KIND {
        let fragments = node
            .children
            .iter()
            .filter(|child| child.children.is_empty())
            .count();
        let mass = fragments.saturating_add(LITERAL_NODE);
        if heaviest.map_or(true, |(best, _)| mass > best) {
            *heaviest = Some((mass, node.byte_range));
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
