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
    ast::NormalizedNode,
    cluster::Cluster,
    fingerprint::Fingerprint,
    state::FileId,
    tokens::collapsed_leaf_ranges,
};

/// Measures and attaches the content-agreement score for every cluster.
/// Runs once per render, immediately after ranking; cost is one walk per
/// cluster member over already-normalised trees — no re-parsing.
pub fn attach_content_agreement<S: BuildHasher>(
    clusters: &mut [Cluster],
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
) {
    let tree_index: HashMap<FileId, &NormalizedNode> =
        trees.iter().map(|tree| (tree.file_id, tree)).collect();
    for cluster in clusters {
        cluster.content_agreement = cluster_content_agreement(&cluster.members, &tree_index, sources);
    }
    tracing::debug!("content agreement attached");
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
    if has_duplicated_content(&member_keys) {
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

/// True when two members carry the same non-empty content-key vector —
/// a verbatim copy hiding among same-shape lookalikes (gh #104's
/// body-equivalence guard). Transitive closure can merge a genuine
/// byte-identical pair into a cluster of same-shape neighbours; the
/// mean against one canonical member would then average the proven
/// copy below the support floor and suppress real duplication, so a
/// duplicated vector short-circuits to full agreement.
fn has_duplicated_content(member_keys: &[Option<Vec<u64>>]) -> bool {
    let mut seen: std::collections::HashSet<&[u64]> = std::collections::HashSet::new();
    member_keys
        .iter()
        .flatten()
        .filter(|keys| !keys.is_empty())
        .any(|keys| !seen.insert(keys.as_slice()))
}

/// Positional agreement between two members' collapsed-leaf content
/// keys. Shape-identical members always carry equal-length key vectors;
/// anything else (including unresolvable members) scores `0.0`. Two
/// empty vectors agree fully — a subtree with no identifiers or
/// literals has no content left to disagree on.
fn pair_agreement(canonical: Option<&[u64]>, member: Option<&[u64]>) -> f64 {
    let (Some(canonical), Some(member)) = (canonical, member) else {
        return 0.0;
    };
    if canonical.len() != member.len() {
        return 0.0;
    }
    if canonical.is_empty() {
        return 1.0;
    }
    let equal = canonical
        .iter()
        .zip(member.iter())
        .filter(|(left, right)| left == right)
        .count();
    member_count(equal) / member_count(canonical.len())
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
    let ranges = collapsed_leaf_ranges(root, member)?;
    ranges
        .iter()
        .map(|range| {
            source.get(range.start..range.end).map(truncated_hash)
        })
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
