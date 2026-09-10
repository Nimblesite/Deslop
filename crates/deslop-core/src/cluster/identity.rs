//! Public cluster ids ([PIPELINE-DETERMINISM]).
//!
//! An id names exactly one finding. It is what `cluster-by-id` resolves,
//! what the editor stores, and what breaks the ranking tie so the order
//! is total. Every input to it is a function of workspace state — never
//! of registration history — so two runs over one tree agree on every id,
//! and two findings in one tree never share one.

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use super::Cluster;
use crate::{buckets::ClusterKind, fingerprint::Fingerprint, state::FileId};

/// A reportable cluster before it is named.
#[derive(Debug)]
pub(super) struct Unnamed {
    /// Members of the cluster, in corpus order.
    pub(super) members: Vec<Fingerprint>,
    /// The clone kind ([CLONE-KIND-FOLD]).
    pub(super) kind: ClusterKind,
    /// Duplicated mass ([RANK-MASS-SUM]).
    pub(super) mass: u64,
    /// The shape family the cluster was admitted out of.
    pub(super) shape_family: Option<usize>,
}

/// What every cluster of one shape over one file set shares: the smallest
/// member's digest and the members' workspace-relative paths in sorted
/// order. Clusters with one source are told apart by where they sit.
type Source<'paths> = ([u8; 32], Vec<&'paths Path>);

/// Where a cluster sits: each member's path and byte range, in member
/// order. Ranks the clusters of one source, so the ordinal an id carries
/// is read from position *rank* rather than raw offsets — an edit above a
/// cluster moves its bytes but not its rank, and does not rename it.
type Position<'paths> = Vec<(&'paths Path, usize, usize)>;

/// Names every cluster ([PIPELINE-DETERMINISM]).
///
/// The id is `blake3` over the smallest member's digest and every
/// member's workspace-relative path in sorted order — and, for every
/// cluster after the first sharing both, its ordinal among them, ordered
/// by their members' positions. The digest alone stamped the three
/// unrelated same-shape findings of the #107 fixture with one id; the
/// paths told those apart but not two copies of one shape inside one
/// file, which fed the digest the same shape and the same path
/// (`two_same_shape_copies_in_one_file_get_two_ids`). The ordinal is the
/// one input that still separates them. The first cluster of a source is
/// named by the source alone, so a finding keeps its id when a second
/// copy of its shape appears after it, and every id published before
/// this rule is unchanged by it. `file_paths` must cover every member's
/// file; an uncovered file degrades that member's path to empty and
/// weakens the id to what the ordinal alone can distinguish.
pub(super) fn name_clusters(
    drafts: Vec<Unnamed>,
    file_paths: &HashMap<FileId, PathBuf>,
) -> Vec<Cluster> {
    let sources: Vec<Source<'_>> = drafts
        .iter()
        .map(|draft| source(&draft.members, file_paths))
        .collect();
    let ordinals = ordinals(&drafts, &sources, file_paths);
    drafts
        .into_iter()
        .zip(sources)
        .zip(ordinals)
        .map(|((draft, source), ordinal)| Cluster {
            id: cluster_id(&source, ordinal),
            members: draft.members,
            mass: draft.mass,
            kind: draft.kind,
            shape_family: draft.shape_family,
        })
        .collect()
}

/// Each draft's rank among the drafts that share its source, by position.
fn ordinals(
    drafts: &[Unnamed],
    sources: &[Source<'_>],
    file_paths: &HashMap<FileId, PathBuf>,
) -> Vec<u64> {
    let mut by_source: BTreeMap<&Source<'_>, Vec<(Position<'_>, usize)>> = BTreeMap::new();
    for (index, (draft, source)) in drafts.iter().zip(sources).enumerate() {
        by_source
            .entry(source)
            .or_default()
            .push((position(&draft.members, file_paths), index));
    }
    let mut ordinals = vec![0_u64; drafts.len()];
    for mut group in by_source.into_values() {
        group.sort_unstable();
        for (ordinal, (_, index)) in group.into_iter().enumerate() {
            if let Some(slot) = ordinals.get_mut(index) {
                *slot = u64::try_from(ordinal).unwrap_or(u64::MAX);
            }
        }
    }
    ordinals
}

/// The smallest member's digest with the sorted member paths.
fn source<'paths>(
    members: &[Fingerprint],
    file_paths: &'paths HashMap<FileId, PathBuf>,
) -> Source<'paths> {
    let digest = members
        .iter()
        .map(|member| member.hash)
        .min()
        .unwrap_or([0_u8; 32]);
    let mut paths: Vec<&Path> = members
        .iter()
        .map(|member| path_of(member, file_paths))
        .collect();
    paths.sort_unstable();
    (digest, paths)
}

/// Each member's path and byte range, in member order.
fn position<'paths>(
    members: &[Fingerprint],
    file_paths: &'paths HashMap<FileId, PathBuf>,
) -> Position<'paths> {
    members
        .iter()
        .map(|member| {
            (
                path_of(member, file_paths),
                member.byte_range.start,
                member.byte_range.end,
            )
        })
        .collect()
}

/// A member's workspace-relative path, empty when the map does not cover
/// its file.
fn path_of<'paths>(
    member: &Fingerprint,
    file_paths: &'paths HashMap<FileId, PathBuf>,
) -> &'paths Path {
    file_paths
        .get(&member.file_id)
        .map_or(Path::new(""), PathBuf::as_path)
}

/// The first cluster of a source: named by the source alone.
const FIRST_OF_SOURCE: u64 = 0;

/// The public id: the first eight bytes of the digest over one source —
/// and, past the first cluster of that source, one ordinal — hex-encoded.
fn cluster_id(source: &Source<'_>, ordinal: u64) -> String {
    let mut hasher = blake3::Hasher::new();
    let _ = hasher.update(&source.0);
    for path in &source.1 {
        let _ = hasher.update(path.as_os_str().as_encoded_bytes());
        let _ = hasher.update(&[0]);
    }
    if ordinal > FIRST_OF_SOURCE {
        let _ = hasher.update(&ordinal.to_le_bytes());
    }
    encode_short_id(*hasher.finalize().as_bytes())
}

/// Shortens a full 32-byte hash to an 8-byte hex stable id for reporting.
#[must_use]
pub fn encode_short_id(hash: [u8; 32]) -> String {
    blake3::Hash::from(hash)
        .to_hex()
        .chars()
        .take(SHORT_ID_HEX_LENGTH)
        .collect()
}

/// Eight digest bytes represented by two hexadecimal digits each.
const SHORT_ID_HEX_LENGTH: usize = 16;
