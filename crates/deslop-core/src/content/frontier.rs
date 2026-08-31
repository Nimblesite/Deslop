//! The collapsed-leaf content frontier: the raw-byte evidence
//! normalisation erased.
//!
//! [FUSED-CONTENT-GATE] measures what `structural` and `token_jaccard`
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
    lang::shared::{is_operator_kind, LITERAL_KIND},
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
            other if is_operator_kind(other) => Self::Operator,
            _ => Self::Identifier,
        }
    }

    /// Whether a leaf in this population is *authored content* — bytes
    /// normalisation erased, which is the only thing
    /// [FUSED-CONTENT-GATE] exists to measure.
    ///
    /// An operator is not erased. [PIPELINE-NORMALIZE-AST-OPERATOR]
    /// keeps it in the normalised kind, so `structural` and
    /// `token_jaccard` already carry it, and two members that align
    /// positionally have equal digests — every operator position
    /// matches *by construction*. Counting those matches as content
    /// agreement is the shape voting a second time under a name that is
    /// supposed to mean independent proof: measured on the
    /// structural-only fixture it lifted same-file agreement from 0.18
    /// to 0.383 and cross-file from 0.17 to 0.371 with the rename proof
    /// still 0.000, promoting scaffolding that shares no authored byte.
    ///
    /// Only *agreement* is withheld, never disagreement: an operator
    /// that differs is a different computation and still counts against
    /// the members, in [`positional_agreement`], [`key_set_jaccard`]
    /// and [`super::pair_substance_varies`] alike. The literal-table
    /// measurement already draws this same line
    /// ([`super::canonical_literal_fraction`]).
    pub(super) const fn is_authored_content(self) -> bool {
        matches!(self, Self::Identifier | Self::Literal)
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
///
/// A key both members carry that is not authored content
/// ([`Population::is_authored_content`]) is struck from *both* halves
/// of the ratio: it would raise the score while proving nothing the
/// shape signals had not already claimed. A one-sided operator stays in
/// the union, so a member that computes a different answer still scores
/// lower than one that does not.
pub(super) fn key_set_jaccard(left: &[LeafKey], right: &[LeafKey]) -> f64 {
    let left: BTreeSet<LeafKey> = left.iter().copied().collect();
    let right: BTreeSet<LeafKey> = right.iter().copied().collect();
    let free = left
        .intersection(&right)
        .filter(|key| !key.population.is_authored_content())
        .count();
    let intersection = left.intersection(&right).count().saturating_sub(free);
    let union = left.union(&right).count().saturating_sub(free);
    if union == 0 {
        return 1.0;
    }
    member_count(intersection) / member_count(union)
}

/// Whether aligned behaviour-bearing operator positions disagree.
///
/// Unlike an identifier rename or literal edit, an operator change is
/// a different computation. [FUSED-CONTENT-GATE] therefore treats one
/// as a hard contradiction instead of allowing surrounding matches to
/// dilute it into a high agreement ratio (#432).
pub(super) fn operators_disagree(left: &[LeafKey], right: &[LeafKey]) -> bool {
    left.iter().zip(right).any(|(left, right)| {
        left.population == Population::Operator
            && right.population == Population::Operator
            && left.key != right.key
    })
}

/// Whether two equally-sized operator populations contain different
/// tokens. Ordering alone is not a contradiction: statement reordering
/// is a Type-3 edit. A changed multiset at equal cardinality is an
/// operator substitution, so surrounding authored bytes may not dilute
/// it into content support (#432).
pub(super) fn operators_substitute(left: &[LeafKey], right: &[LeafKey]) -> bool {
    let mut left = operator_keys(left);
    let mut right = operator_keys(right);
    if left.is_empty() || left.len() != right.len() {
        return false;
    }
    left.sort_unstable();
    right.sort_unstable();
    left != right
}

/// Raw operator identities carried by one content frontier.
fn operator_keys(keys: &[LeafKey]) -> Vec<u64> {
    keys.iter()
        .filter(|key| key.population == Population::Operator)
        .map(|key| key.key)
        .collect()
}

/// Positional agreement between two shape-aligned members' frontiers:
/// the share of measured positions whose raw bytes match.
///
/// A position is measured when it carries authored content or when it
/// disagrees. A *matching* non-content position — in practice a shared
/// operator — is measured by neither numerator nor denominator, because
/// shape-aligned members carry identical operator kinds by construction
/// ([`Population::is_authored_content`]).
///
/// `1.0` when nothing was measured, the same answer two empty
/// frontiers get and for the same reason: a subtree with no authored
/// content has nothing to disagree on.
pub(super) fn positional_agreement(canonical: &[LeafKey], member: &[LeafKey]) -> f64 {
    let (matched, measured) = canonical.iter().zip(member.iter()).fold(
        (0_usize, 0_usize),
        |(matched, measured), (left, right)| {
            let agrees = left == right;
            if agrees && !left.population.is_authored_content() {
                return (matched, measured);
            }
            (
                matched.saturating_add(usize::from(agrees)),
                measured.saturating_add(1),
            )
        },
    );
    if measured == 0 {
        return 1.0;
    }
    member_count(matched) / member_count(measured)
}

/// One member's resolved content frontier: a key and its source range
/// per collapsed leaf, with the member's import/prologue boilerplate
/// excluded exactly as every other measurement axis excludes it
/// ([PIPELINE-BOILERPLATE-FILTER]). `None` when the member's tree,
/// source, or byte range cannot be resolved.
pub(super) fn member_content<S: BuildHasher, L: BuildHasher>(
    member: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> Option<MemberContent> {
    let root = tree_index.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    let language = languages.get(&member.file_id).map(|id| &**id);
    let leaves = collapsed_leaves(root, member, language)?;
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
