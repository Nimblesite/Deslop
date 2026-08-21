//! The collapsed-leaf content frontier: the raw-byte evidence
//! normalisation erased.
//!
//! [FUSION-CONTENT-GATE] measures what `structural` and `token_jaccard`
//! cannot see, and every one of those measurements reads the same
//! artefact — one key per collapsed leaf, in frontier order, tagged with
//! the population it belongs to ([PIPELINE-NORMALIZE-AST-OPERATOR]).
//! Resolving that artefact, hashing it, and pairing two members'
//! positions are mechanical concerns with no opinion about clone
//! quality, so they live here rather than beside the judgements in
//! [`super`].

use std::collections::{BTreeSet, HashMap};
use std::hash::BuildHasher;

use crate::{
    ast::{ByteRange, NormalizedNode},
    fingerprint::Fingerprint,
    lang::shared::{LITERAL_KIND, OPERATOR_KIND},
    state::FileId,
    tokens::collapsed_leaves,
};

/// Which evidence population a collapsed frontier leaf belongs to.
///
/// Identifiers and literals are the two populations the rename and
/// literal-preservation measurements are defined over. Operators are a
/// third ([PIPELINE-NORMALIZE-AST-OPERATOR]) and deliberately belong to
/// neither: there is no substitution that turns `+` into `-`, so
/// counting an operator as an identifier would report a broken rename,
/// and counting it as a literal would report a data table. It is
/// evidence of its own kind — positional agreement, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Population {
    /// A collapsed identifier position.
    Identifier,
    /// A collapsed literal position.
    Literal,
    /// A behaviour-bearing operator token.
    Operator,
}

impl Population {
    /// The population a normalised leaf kind belongs to. Anything that
    /// is neither a literal nor an operator reached the frontier as a
    /// collapsed identifier.
    pub(super) fn of(kind: &str) -> Self {
        match kind {
            LITERAL_KIND => Self::Literal,
            OPERATOR_KIND => Self::Operator,
            _ => Self::Identifier,
        }
    }
}

/// One collapsed-leaf content key: the population flag plus a truncated
/// hash of the leaf's raw source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) struct LeafKey {
    /// Which evidence population the leaf belongs to.
    pub(super) population: Population,
    /// Truncated blake3 hash of the leaf's raw source bytes.
    pub(super) key: u64,
}

/// One member's resolved content frontier: the per-leaf keys plus the
/// byte range each key hashed, so rename measurement can read a leaf's
/// raw bytes back without re-walking the tree
/// ([`rename::literal_echoes`], #409).
pub(super) struct MemberContent {
    /// File every range below indexes into.
    pub(super) file: FileId,
    /// Normalised-subtree digest of the member ([`Fingerprint::hash`])
    /// — the shape half of token identity in
    /// [`dominant_verbatim_share`].
    pub(super) shape: [u8; 32],
    /// One key per collapsed leaf, in frontier order.
    pub(super) keys: Vec<LeafKey>,
    /// The source byte range each key was hashed from, 1:1 with `keys`.
    pub(super) ranges: Vec<ByteRange>,
}

/// The key slice of a resolved member, `None` when unresolvable.
pub(super) fn keys_of(content: Option<&MemberContent>) -> Option<&[LeafKey]> {
    content.map(|content| content.keys.as_slice())
}

/// Raw source bytes of one collapsed leaf, by frontier index.
pub(super) fn leaf_bytes<'src, S: BuildHasher>(
    content: &MemberContent,
    index: usize,
    sources: &'src HashMap<FileId, Vec<u8>, S>,
) -> Option<&'src [u8]> {
    let range = content.ranges.get(index)?;
    sources.get(&content.file)?.get(range.start..range.end)
}

/// Paired keys at the positions where both members carry `wanted`.
/// Shape-aligned members disagree on a position's population only at
/// parse-artifact boundaries; such positions belong to no population.
pub(super) fn population(
    canonical: &[LeafKey],
    member: &[LeafKey],
    wanted: Population,
) -> Vec<(u64, u64)> {
    canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left.population == wanted && right.population == wanted)
        .map(|(left, right)| (left.key, right.key))
        .collect()
}

/// Jaccard similarity of two members' content-key sets.
pub(super) fn key_set_jaccard(left: &[LeafKey], right: &[LeafKey]) -> f64 {
    let left: BTreeSet<LeafKey> = left.iter().copied().collect();
    let right: BTreeSet<LeafKey> = right.iter().copied().collect();
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    if union == 0 {
        return 1.0;
    }
    member_count(intersection) / member_count(union)
}

/// One member's resolved content frontier: a key and its source range
/// per collapsed leaf. `None` when the member's tree, source, or byte
/// range cannot be resolved.
pub(super) fn member_content<S: BuildHasher>(
    member: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<MemberContent> {
    let root = tree_index.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    let leaves = collapsed_leaves(root, member)?;
    let keys = leaves
        .iter()
        .map(|(kind, range)| {
            source.get(range.start..range.end).map(|bytes| LeafKey {
                population: Population::of(kind),
                key: truncated_hash(bytes),
            })
        })
        .collect::<Option<Vec<LeafKey>>>()?;
    let ranges = leaves.iter().map(|(_, range)| *range).collect();
    Some(MemberContent {
        file: member.file_id,
        shape: member.hash,
        keys,
        ranges,
    })
}

/// First eight little-endian bytes of the blake3 hash of `bytes`.
fn truncated_hash(bytes: &[u8]) -> u64 {
    let digest = blake3::hash(bytes);
    let mut prefix = [0_u8; 8];
    for (slot, byte) in prefix.iter_mut().zip(digest.as_bytes().iter()) {
        *slot = *byte;
    }
    u64::from_le_bytes(prefix)
}

/// Lossless small-count conversion for agreement divisors.
pub(super) fn member_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
