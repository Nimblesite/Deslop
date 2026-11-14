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
//! the two literals are near-copies of each other: a test whose embedded
//! program differs from its neighbour's by one name is a copy, two
//! different queries are not.

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};

use super::frontier::member_count;
use crate::{
    ast::{ByteRange, NormalizedNode},
    buckets::CONTENT_SUPPORT_FLOOR,
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
/// Bytes per shingle when two literals' texts are compared. Long enough
/// that indentation and punctuation alone do not make two texts alike.
const SHINGLE_BYTES: usize = 8;

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
/// is not a near-copy of it.
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
            (Some(left), Some(right)) => near_copies(left, right),
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

/// Whether two literal texts are near-copies: the same bytes, or enough
/// shared shingles to clear the pair-content floor every other content
/// measure uses ([FUSED-CONTENT-GATE]).
fn near_copies(left: &[u8], right: &[u8]) -> bool {
    left == right || shingle_overlap(left, right) >= CONTENT_SUPPORT_FLOOR
}

/// Jaccard overlap of the two texts' distinct byte shingles. A text
/// shorter than one shingle has none, so it overlaps only an equal text.
fn shingle_overlap(left: &[u8], right: &[u8]) -> f64 {
    let left: HashSet<&[u8]> = left.windows(SHINGLE_BYTES).collect();
    let right: HashSet<&[u8]> = right.windows(SHINGLE_BYTES).collect();
    let shared = left.intersection(&right).count();
    let union = left
        .len()
        .saturating_add(right.len())
        .saturating_sub(shared);
    if union == 0 {
        return 0.0;
    }
    member_count(shared) / member_count(union)
}

#[cfg(test)]
mod tests {
    use super::{near_copies, shingle_overlap};

    const PROGRAM: &[u8] = b"using Attributes;\npublic class BenchmarkClass\n{\n    [GlobalCleanup]\n    public async IAsyncEnumerable<int> Cleanup()\n    {\n        await Task.Yield();\n        yield return 1;\n    }\n}\n";
    const ONE_NAME_CHANGED: &[u8] = b"using Attributes;\npublic class BenchmarkClass\n{\n    [IterationCleanup]\n    public async IAsyncEnumerable<int> Cleanup()\n    {\n        await Task.Yield();\n        yield return 1;\n    }\n}\n";
    const OTHER_PROGRAM: &[u8] = b"SELECT i.relname AS index_name, ix.indisunique AS is_unique\nFROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid\nWHERE n.nspname = @schema ORDER BY i.relname\n";
    const SHORT: &[u8] = b"id";
    const OTHER_SHORT: &[u8] = b"ok";

    #[test]
    fn the_same_text_and_a_one_name_edit_are_near_copies() {
        assert!(near_copies(PROGRAM, PROGRAM));
        assert!(near_copies(PROGRAM, ONE_NAME_CHANGED));
        assert!(
            near_copies(ONE_NAME_CHANGED, PROGRAM),
            "the measure is symmetric"
        );
        assert!(shingle_overlap(PROGRAM, ONE_NAME_CHANGED) < 1.0);
    }

    #[test]
    fn unrelated_texts_are_not_near_copies() {
        assert!(!near_copies(PROGRAM, OTHER_PROGRAM));
        assert!(shingle_overlap(PROGRAM, OTHER_PROGRAM) < f64::EPSILON);
    }

    #[test]
    fn texts_shorter_than_a_shingle_match_only_when_equal() {
        assert!(near_copies(SHORT, SHORT));
        assert!(!near_copies(SHORT, OTHER_SHORT));
        assert!(shingle_overlap(SHORT, OTHER_SHORT).abs() < f64::EPSILON);
    }
}
