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

use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    /// Authored-literal group for a composite literal's fragments
    /// ([`crate::tokens::CollapsedLeaf::literal_group`]).
    pub(super) literal_group: Option<u32>,
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

/// Whether two members' frontiers align position-for-position: equal
/// normalised shape and equal key counts, so index `i` names the same
/// authored slot in both members.
pub(super) fn frontiers_aligned(left: &MemberContent, right: &MemberContent) -> bool {
    left.shape == right.shape && left.keys.len() == right.keys.len()
}

/// Whether the endpoints contradict each other on a behaviour-bearing
/// operator, in either agreement branch ([FUSED-CONTENT-GATE]).
///
/// Aligned frontiers compare positionally ([`operators_disagree`]).
/// Misaligned frontiers compare operator multisets
/// ([`operators_substitute`]): a positional zip across shifted indices
/// pairs unrelated slots, so it misses a real substitution — the
/// operator-drift ledger pair measured `agreement = 0.94` against a
/// body/whole-function view shift and was admitted through rescue —
/// and convicts a legitimate Type-3 reorder. Pinned by
/// `crates/deslop/tests/operator_drift_is_not_duplication.rs`.
pub(super) fn operator_contradiction(left: &MemberContent, right: &MemberContent) -> bool {
    if frontiers_aligned(left, right) {
        operators_disagree(&left.keys, &right.keys)
    } else {
        operators_substitute(&left.keys, &right.keys)
    }
}

/// Whether aligned behaviour-bearing operator positions disagree.
///
/// Unlike an identifier rename or literal edit, an operator change is
/// a different computation. [FUSED-CONTENT-GATE] therefore treats one
/// as a hard contradiction instead of allowing surrounding matches to
/// dilute it into a high agreement ratio (#432).
fn operators_disagree(left: &[LeafKey], right: &[LeafKey]) -> bool {
    left.iter().zip(right).any(|(left, right)| {
        left.population == Population::Operator
            && right.population == Population::Operator
            && left.key != right.key
    })
}

/// Whether the two operator populations prove a changed computation.
///
/// Ordering alone is not a contradiction: statement reordering is a
/// Type-3 edit and keeps the multisets equal. A one-sided surplus is an
/// inserted or deleted computation — also Type-3. But when *each* side
/// carries an operator the other lacks, no reorder-only or
/// insertion-only edit explains the pair: an operation changed, so
/// surrounding authored bytes may not dilute it into content support
/// (#432). At equal cardinality this is exactly "the multisets differ";
/// at unequal cardinality it caught the operator-drift ledger pair
/// `credit[2..10]`/`debit[2..11]`, whose extra trailing `-` hid the
/// `+`/`-` substitution from the equal-cardinality rule.
fn operators_substitute(left: &[LeafKey], right: &[LeafKey]) -> bool {
    let left = operator_counts(left);
    let right = operator_counts(right);
    let one_sided = |ours: &BTreeMap<u64, usize>, theirs: &BTreeMap<u64, usize>| {
        ours.iter()
            .any(|(key, count)| theirs.get(key).map_or(true, |have| have < count))
    };
    one_sided(&left, &right) && one_sided(&right, &left)
}

/// Raw operator identities carried by one content frontier, counted.
fn operator_counts(keys: &[LeafKey]) -> BTreeMap<u64, usize> {
    let mut counts = BTreeMap::new();
    for key in keys {
        if key.population == Population::Operator {
            let slot = counts.entry(key.key).or_insert(0_usize);
            *slot = slot.saturating_add(1);
        }
    }
    counts
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
        .map(|leaf| {
            source
                .get(leaf.range.start..leaf.range.end)
                .map(|bytes| LeafKey {
                    population: Population::of(leaf.kind),
                    key: truncated_hash(bytes),
                    literal_group: leaf.literal_group,
                })
        })
        .collect::<Option<Vec<LeafKey>>>()?;
    let ranges = leaves.iter().map(|leaf| leaf.range).collect();
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
