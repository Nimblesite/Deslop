//! Content-agreement measurement for shape-identical clone clusters.
//!
//! Implements [FUSION-CONTENT-GATE] (#331/#336): normalisation collapses
//! identifiers and literals, so `structural` and `token_jaccard` agree by
//! construction on any shape match and cannot tell a renamed copy of real
//! logic from mandatory scaffolding or a data table. This pass measures
//! what normalisation erased — the fraction of collapsed leaf positions
//! whose **raw source bytes** still agree across members — and stores it
//! on each [`Cluster`] so bucket routing and the rendered fused
//! confidence can separate the two cases.

use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    ast::NormalizedNode, cluster::Cluster, fingerprint::Fingerprint, lang::shared::LITERAL_KIND,
    state::FileId, tokens::collapsed_leaves,
};

/// Minimum literal-leaf count before a subtree's literal dominance is
/// reported at all ([CLONE-NOISE-LITERAL-TABLE]). A data table is a run
/// of values; a tiny subtree that happens to be mostly literals (a
/// two-element tuple return, a short argument list) is not a table and
/// must not reach the data-category classifier.
const LITERAL_TABLE_MIN_LITERALS: usize = 8;

/// Measures and attaches the content-agreement and literal-dominance
/// scores for every cluster. Runs once per render, immediately after
/// ranking; cost is one walk per cluster member over already-normalised
/// trees — no re-parsing.
pub fn attach_content_agreement<S: BuildHasher>(
    clusters: &mut [Cluster],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
) {
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    for cluster in clusters {
        cluster.content_agreement =
            cluster_content_agreement(&cluster.members, &tree_index, sources);
        cluster.literal_fraction = cluster_literal_fraction(&cluster.members, &tree_index);
    }
    tracing::debug!("content agreement attached");
}

/// Fraction of the canonical member's collapsed leaves that are literal
/// positions, in `[0, 1]` — the language-agnostic "is this a data
/// literal?" measurement ([CLONE-NOISE-LITERAL-TABLE]). `0.0` when the
/// member cannot be resolved or carries fewer than
/// [`LITERAL_TABLE_MIN_LITERALS`] literals, so tiny literal-heavy
/// subtrees never register as tables.
fn cluster_literal_fraction(
    members: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
) -> f64 {
    let leaves = members
        .first()
        .and_then(|canonical| {
            tree_index
                .get(&canonical.file_id)
                .map(|root| (root, canonical))
        })
        .and_then(|(root, canonical)| collapsed_leaves(root, canonical));
    let Some(leaves) = leaves else {
        return 0.0;
    };
    let literals = leaves
        .iter()
        .filter(|(kind, _)| *kind == LITERAL_KIND)
        .count();
    if literals < LITERAL_TABLE_MIN_LITERALS || leaves.is_empty() {
        return 0.0;
    }
    member_count(literals) / member_count(leaves.len())
}

/// Mean agreement of every non-canonical member against the canonical
/// (first) member. `1.0` for degenerate single-member clusters; a member
/// whose leaves cannot be resolved contributes `0.0` — unresolvable
/// content is no evidence of agreement.
fn cluster_content_agreement<S: BuildHasher>(
    members: &[Fingerprint],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> f64 {
    if members.len() < 2 {
        return 1.0;
    }
    let member_keys: Vec<Option<Vec<u64>>> = members
        .iter()
        .map(|member| member_content_keys(member, tree_index, sources))
        .collect();
    if duplicated_member_share(&member_keys) >= VERBATIM_MEMBER_SHARE_FLOOR {
        return 1.0;
    }
    let canonical_keys = member_keys.first().and_then(Option::as_deref);
    let total: f64 = member_keys
        .iter()
        .skip(1)
        .map(|keys| pair_agreement(canonical_keys, keys.as_deref()))
        .sum();
    total / member_count(members.len().saturating_sub(1))
}

/// Minimum share of members that must participate in verbatim
/// duplicates before the guard vouches for the whole cluster. The #104
/// mixed cluster is a verbatim pair among a couple of lookalikes
/// (share ≥ 2/3) and must stay visible; the real-corpus failure mode is
/// the opposite — two byte-identical example widgets hiding inside 453
/// framework-mandated declarations (share ≈ 0.004), where full
/// agreement would resurrect the exact #331 mega-cluster the content
/// gate exists to demote.
const VERBATIM_MEMBER_SHARE_FLOOR: f64 = 0.5;

/// Share of members whose non-empty content-key vector also appears on
/// another member — verbatim copies hiding among same-shape lookalikes
/// (gh #104's body-equivalence guard). Transitive closure can merge a
/// genuine byte-identical pair into a cluster of same-shape neighbours;
/// the mean against one canonical member would average the proven copy
/// below the support floor, so a cluster *dominated* by verbatim copies
/// short-circuits to full agreement instead.
fn duplicated_member_share(member_keys: &[Option<Vec<u64>>]) -> f64 {
    let mut counts: HashMap<&[u64], usize> = HashMap::new();
    for keys in member_keys.iter().flatten() {
        if !keys.is_empty() {
            let entry = counts.entry(keys.as_slice()).or_insert(0_usize);
            *entry = entry.saturating_add(1);
        }
    }
    let duplicated: usize = counts.values().filter(|count| **count >= 2).copied().sum();
    if member_keys.is_empty() {
        return 0.0;
    }
    member_count(duplicated) / member_count(member_keys.len())
}

/// Agreement between two members' collapsed-leaf content keys.
/// Shape-identical members carry equal-length key vectors and score the
/// positional match fraction. Shape-*mismatched* members (an LSH-paired
/// near-miss whose trees differ) have no positional alignment, so they
/// score the key-set Jaccard instead — a genuine Type-3 near-miss
/// shares nearly all its keys, while renamed scaffolding shares few.
/// Unresolvable members score `0.0`; two empty vectors agree fully — a
/// subtree with no identifiers or literals has nothing to disagree on.
fn pair_agreement(canonical: Option<&[u64]>, member: Option<&[u64]>) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if canonical.is_empty() && member.is_empty() {
        return 1.0;
    }
    if canonical.len() != member.len() {
        return key_set_jaccard(canonical, member);
    }
    let equal = canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left == right)
        .count();
    member_count(equal) / member_count(canonical.len())
}

/// Jaccard similarity of two members' content-key sets.
fn key_set_jaccard(left: &[u64], right: &[u64]) -> f64 {
    let left: std::collections::BTreeSet<u64> = left.iter().copied().collect();
    let right: std::collections::BTreeSet<u64> = right.iter().copied().collect();
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    if union == 0 {
        return 1.0;
    }
    member_count(intersection) / member_count(union)
}

/// One content key per collapsed leaf: a truncated blake3 hash of the
/// leaf's raw source bytes. `None` when the member's tree, source, or
/// byte range cannot be resolved.
fn member_content_keys<S: BuildHasher>(
    member: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
) -> Option<Vec<u64>> {
    let root = tree_index.get(&member.file_id)?;
    let source = sources.get(&member.file_id)?;
    let leaves = collapsed_leaves(root, member)?;
    leaves
        .iter()
        .map(|(_, range)| source.get(range.start..range.end).map(truncated_hash))
        .collect()
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
fn member_count(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}
