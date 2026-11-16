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
//! all two endpoints can share. When both endpoints are carried so, the
//! pair is a copy only if the two endpoints' texts — each literal with the
//! code around it — are near-copies: a test whose embedded program differs
//! from its neighbour's by one name is a copy, and so are two tests whose
//! expected tables differ by colour escapes inside identical code; two
//! different queries behind the same setup are not.

use std::{collections::HashMap, hash::BuildHasher};

use super::frontier::{member_count, LiteralText};
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
/// Bytes per shingle when two literals' texts are compared: one `u64`.
/// Long enough that indentation and punctuation alone do not make two
/// texts alike.
const SHINGLE_BYTES: usize = size_of::<u64>();
/// The byte that makes a literal an embedded document: it spans lines.
const NEWLINE: u8 = b'\n';

/// An endpoint's heaviest authored literal, and the endpoint it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaviestLiteral {
    /// File the literal lives in.
    file: FileId,
    /// Whether the literal is most of the endpoint, by nodes or by bytes.
    carries_endpoint: bool,
    /// Source bytes of the whole endpoint: the literal and the code around it.
    endpoint: ByteRange,
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
        carries_endpoint: is_majority(literal.nodes, fingerprint.node_count)
            || is_majority(span(literal.range), span(fingerprint.byte_range)),
        endpoint: fingerprint.byte_range,
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
        if heaviest.is_none_or(|best| literal.outweighs(best)) {
            *heaviest = Some(literal);
        }
    }
    for child in &node.children {
        weigh_literals(child, heaviest);
    }
}

/// The two endpoint texts a refusal is judged on: `Some` only when both
/// sides are carried by their heaviest literal and both texts read back.
/// A side its literal does not carry has code of its own, and the content
/// gate has compared it; a text that cannot be read proves nothing, so it
/// refuses nothing.
pub(crate) fn carried_texts<S: BuildHasher>(
    left: Option<HeaviestLiteral>,
    right: Option<HeaviestLiteral>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<(&[u8], &[u8])> {
    let (left, right) = left
        .zip(right)
        .filter(|(left, right)| left.carries_endpoint && right.carries_endpoint)?;
    endpoint_text(left, sources).zip(endpoint_text(right, sources))
}

/// Whether two literal-carried endpoints hold different text: the texts
/// [`carried_texts`] returns are not near-copies. Identical code around
/// two differing literals is shared text like any other.
pub(crate) fn carried_text_differs<S: BuildHasher>(
    left: Option<HeaviestLiteral>,
    right: Option<HeaviestLiteral>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> bool {
    carried_texts(left, right, sources).is_some_and(|(left, right)| !near_copies(left, right))
}

/// Raw source bytes of one carried endpoint.
fn endpoint_text<S: BuildHasher>(
    literal: HeaviestLiteral,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<&[u8]> {
    sources
        .get(&literal.file)?
        .get(literal.endpoint.start..literal.endpoint.end)
}

/// How two differing literal texts compare in positional agreement:
/// near-copies, two embedded documents (both span lines), or different.
pub(super) fn literal_text(left: &[u8], right: &[u8]) -> LiteralText {
    if near_copies(left, right) {
        LiteralText::NearCopies
    } else if [left, right].iter().all(|text| text.contains(&NEWLINE)) {
        LiteralText::Documents
    } else {
        LiteralText::Different
    }
}

/// Whether two literal texts are near-copies: the same bytes, or enough
/// shared shingles to clear the pair-content floor every other content
/// measure uses ([FUSED-CONTENT-GATE]).
fn near_copies(left: &[u8], right: &[u8]) -> bool {
    left == right || shingles_clear_floor(&shingles(left), &shingles(right))
}

/// A text's distinct byte shingles, sorted: each window read as one
/// integer, so nothing is hashed and nothing collides. A text shorter
/// than one shingle has none, so it is a near-copy only of an equal text.
pub(crate) fn shingles(text: &[u8]) -> Vec<u64> {
    let mut distinct: Vec<u64> = text
        .windows(SHINGLE_BYTES)
        .filter_map(|window| window.try_into().ok().map(u64::from_le_bytes))
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    distinct
}

/// Whether two shingle sets overlap enough for their texts to be
/// near-copies ([FUSED-CONTENT-GATE]).
pub(crate) fn shingles_clear_floor(left: &[u64], right: &[u64]) -> bool {
    shingle_overlap(left, right) >= CONTENT_SUPPORT_FLOOR
}

/// Jaccard overlap of two sorted, distinct shingle sets.
fn shingle_overlap(left: &[u64], right: &[u64]) -> f64 {
    let shared = left
        .iter()
        .filter(|shingle| right.binary_search(shingle).is_ok())
        .count();
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
    use std::collections::HashMap;

    use super::{carried_text_differs, near_copies, shingles, HeaviestLiteral};

    fn shingle_overlap(left: &[u8], right: &[u8]) -> f64 {
        super::shingle_overlap(&shingles(left), &shingles(right))
    }

    use crate::{
        ast::ByteRange,
        state::{FileId, FileRegistry},
    };

    const PROGRAM: &[u8] = b"using Attributes;\npublic class BenchmarkClass\n{\n    [GlobalCleanup]\n    public async IAsyncEnumerable<int> Cleanup()\n    {\n        await Task.Yield();\n        yield return 1;\n    }\n}\n";
    const ONE_NAME_CHANGED: &[u8] = b"using Attributes;\npublic class BenchmarkClass\n{\n    [IterationCleanup]\n    public async IAsyncEnumerable<int> Cleanup()\n    {\n        await Task.Yield();\n        yield return 1;\n    }\n}\n";
    const OTHER_PROGRAM: &[u8] = b"SELECT i.relname AS index_name, ix.indisunique AS is_unique\nFROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid\nWHERE n.nspname = @schema ORDER BY i.relname\n";
    const SHORT: &[u8] = b"id";
    const OTHER_SHORT: &[u8] = b"ok";
    const LEFT_PATH: &str = "left.cs";
    const RIGHT_PATH: &str = "right.cs";

    fn two_files() -> (FileId, FileId) {
        let mut registry = FileRegistry::new();
        (
            registry.register(LEFT_PATH.into()),
            registry.register(RIGHT_PATH.into()),
        )
    }

    fn carried(file: FileId, text: &[u8]) -> HeaviestLiteral {
        HeaviestLiteral {
            file,
            carries_endpoint: true,
            endpoint: ByteRange {
                start: 0,
                end: text.len(),
            },
        }
    }

    fn sources(texts: &[(FileId, &[u8])]) -> HashMap<FileId, Vec<u8>> {
        texts
            .iter()
            .map(|(file, text)| (*file, text.to_vec()))
            .collect()
    }

    // [FUSED-CONTENT-GATE-LITERAL-BOUND] A refusal needs both texts read:
    // a missing or shortened source proves nothing and refuses nothing.
    #[test]
    fn only_two_read_and_different_carried_texts_are_refused() {
        let (left_file, right_file) = two_files();
        let left = Some(carried(left_file, PROGRAM));
        let right = Some(carried(right_file, OTHER_PROGRAM));
        let both = sources(&[(left_file, PROGRAM), (right_file, OTHER_PROGRAM)]);
        assert!(carried_text_differs(left, right, &both), "different texts");
        let edited = sources(&[(left_file, PROGRAM), (right_file, ONE_NAME_CHANGED)]);
        let near = Some(carried(right_file, ONE_NAME_CHANGED));
        assert!(!carried_text_differs(left, near, &edited), "a near-copy");
        let missing = sources(&[(left_file, PROGRAM)]);
        assert!(
            !carried_text_differs(left, right, &missing),
            "missing source"
        );
        assert!(!carried_text_differs(right, left, &missing), "either side");
        let shortened = sources(&[(left_file, PROGRAM), (right_file, SHORT)]);
        assert!(
            !carried_text_differs(left, right, &shortened),
            "stale range"
        );
        assert!(!carried_text_differs(left, None, &both), "no literal");
        let uncarried = right.map(|literal| HeaviestLiteral {
            carries_endpoint: false,
            ..literal
        });
        assert!(
            !carried_text_differs(left, uncarried, &both),
            "one side carried"
        );
    }

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
