//! [FUSED-SHARED-SUBTREE-CORE] The joined frontier of a rescued pair's
//! aligned core: each span pair sliced from the whole endpoints' frontiers
//! and concatenated in order, so index `i` names the same authored slot
//! on both sides exactly as it does for a shape-equal pair.

use std::collections::BTreeMap;

use super::{
    call_targets::CallTarget,
    frontier::{frontiers_aligned, LeafKey, MemberContent},
};
use crate::{ast::ByteRange, fingerprint::Fingerprint, state::FileId};

impl LeafKey {
    /// The same key inside a frontier whose literal groups begin
    /// `offset` groups later — how a span's keys join a concatenated
    /// core without two spans' composite literals sharing a group
    /// ([FUSED-SHARED-SUBTREE-CORE]).
    fn regrouped(self, offset: u32) -> Self {
        Self {
            literal_group: self.literal_group.map(|group| group.saturating_add(offset)),
            ..self
        }
    }
}

impl MemberContent {
    /// A frontier holding no positions yet, for a core to be appended to.
    fn empty(file: FileId) -> Self {
        Self {
            file,
            shape: [0_u8; 32],
            keys: Vec::new(),
            ranges: Vec::new(),
            external_calls: Vec::new(),
        }
    }

    /// This frontier's positions inside `range` — one subtree's
    /// contiguous stretch of the pre-order frontier — as a frontier of
    /// its own under `shape`, with literal groups renumbered from zero
    /// in order of appearance and call-target collaborators re-indexed
    /// to the stretch, or dropped when they lie outside it. Two
    /// Merkle-equal subtrees slice to frontiers that align position for
    /// position whatever their absolute positions were
    /// ([FUSED-SHARED-SUBTREE-CORE]).
    fn slice(&self, range: ByteRange, shape: [u8; 32]) -> Self {
        let inside = self.positions_inside(range);
        let first = inside.first().copied().unwrap_or_default();
        let mut groups = GroupRenumbering::default();
        Self {
            file: self.file,
            shape,
            keys: inside
                .iter()
                .filter_map(|index| self.keys.get(*index))
                .map(|key| groups.renumber(*key))
                .collect(),
            ranges: inside
                .iter()
                .filter_map(|index| self.ranges.get(*index))
                .copied()
                .collect(),
            external_calls: inside
                .iter()
                .map(|index| self.external_call_within(*index, first, inside.len()))
                .collect(),
        }
    }

    /// The frontier positions whose leaves lie inside `range`, in order.
    fn positions_inside(&self, range: ByteRange) -> Vec<usize> {
        self.ranges
            .iter()
            .enumerate()
            .filter(|(_, leaf)| leaf.start >= range.start && leaf.end <= range.end)
            .map(|(index, _)| index)
            .collect()
    }

    /// The call target at `index`, re-indexed to a stretch of `len`
    /// positions starting at `first`.
    fn external_call_within(&self, index: usize, first: usize, len: usize) -> Option<CallTarget> {
        self.external_calls
            .get(index)
            .and_then(|target| target.as_ref())
            .map(|target| target.within(first, len))
    }

    /// Appends one span's frontier after the positions already held,
    /// with the span's literal groups renumbered past `groups` and its
    /// call-target collaborators shifted past the held positions, so
    /// every index and group stays distinct ([FUSED-SHARED-SUBTREE-CORE]).
    fn append(&mut self, part: Self, groups: u32) {
        let offset = self.keys.len();
        self.keys
            .extend(part.keys.iter().map(|key| key.regrouped(groups)));
        self.ranges.extend(part.ranges);
        self.external_calls.extend(
            part.external_calls
                .iter()
                .map(|target| target.as_ref().map(|target| target.shifted(offset))),
        );
    }
}

/// Literal groups numbered afresh in order of first appearance.
#[derive(Default)]
struct GroupRenumbering {
    /// Original group to its number in the slice.
    seen: BTreeMap<u32, u32>,
}

impl GroupRenumbering {
    /// `key` with its literal group renumbered.
    fn renumber(&mut self, key: LeafKey) -> LeafKey {
        let Some(group) = key.literal_group else {
            return key;
        };
        let next = u32::try_from(self.seen.len()).unwrap_or(u32::MAX);
        let renumbered = *self.seen.entry(group).or_insert(next);
        LeafKey {
            literal_group: Some(renumbered),
            ..key
        }
    }
}

/// [FUSED-SHARED-SUBTREE-CORE] Both endpoints' frontiers over their
/// aligned core — every span pair the alignment claims, sliced from the
/// whole endpoints' frontiers and concatenated in order — so index `i`
/// names the same authored slot on both sides exactly as it does for a
/// shape-equal pair. `None` when the core is empty and when a pair of
/// spans does not align position for position, which no aligned pair
/// should fail: an unmeasurable core is refused, never vouched for.
pub(super) fn joined_content(
    core: &[(Fingerprint, Fingerprint)],
    whole: (&MemberContent, &MemberContent),
) -> Option<(MemberContent, MemberContent)> {
    let _first = core.first()?;
    let mut joined = (
        MemberContent::empty(whole.0.file),
        MemberContent::empty(whole.1.file),
    );
    let mut groups = 0_u32;
    for (left_span, right_span) in core {
        let parts = (
            whole.0.slice(left_span.byte_range, left_span.hash),
            whole.1.slice(right_span.byte_range, right_span.hash),
        );
        groups = append_aligned(&mut joined, parts, groups)?;
    }
    let shape = core_shape(core);
    joined.0.shape = shape;
    joined.1.shape = shape;
    Some(joined)
}

/// Appends one span pair to the joined frontiers and returns the
/// literal groups held afterwards, or `None` when the two spans do not
/// align position for position.
fn append_aligned(
    joined: &mut (MemberContent, MemberContent),
    parts: (MemberContent, MemberContent),
    groups: u32,
) -> Option<u32> {
    if !frontiers_aligned(&parts.0, &parts.1) {
        return None;
    }
    let held = literal_groups(&parts.0.keys);
    joined.0.append(parts.0, groups);
    joined.1.append(parts.1, groups);
    Some(groups.saturating_add(held))
}

/// How many literal groups `keys` hold, so the next span's groups can
/// follow them.
fn literal_groups(keys: &[LeafKey]) -> u32 {
    keys.iter()
        .filter_map(|key| key.literal_group)
        .max()
        .map_or(0, |last| last.saturating_add(1))
}

/// One digest over the core's span hashes, assigned to both sides so
/// the joined frontiers read as one shape.
fn core_shape(core: &[(Fingerprint, Fingerprint)]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for (span, _partner) in core {
        let _chained = hasher.update(&span.hash);
    }
    *hasher.finalize().as_bytes()
}
