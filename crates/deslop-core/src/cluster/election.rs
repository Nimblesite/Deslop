//! The one view each file publishes for an overlapping run of its
//! occurrences ([PIPELINE-CLUSTER-EXACT-SCOPE]).
//!
//! Split from the parent module, which assembles clusters; this module
//! only decides, file by file, which of several overlapping views of one
//! region the cluster reports.

use std::{collections::BTreeMap, hash::BuildHasher};

use super::{
    matched::{Width, WidthEvidence},
    scope::DeclarationScopes,
};
use crate::{
    ast::ByteRange,
    fingerprint::Fingerprint,
    pair::{FusedCluster, SHARED_SUBTREE_MIN_NODE_COUNT},
    state::FileId,
};

/// Collapses overlapping sibling-window occurrences that live in the
/// same file into a single canonical member per overlapping region.
///
/// Fixes ([PIPELINE-CLUSTER-EXACT] sibling-extension runaway):
/// the sibling pass at [`crate::sibling`] emits one fingerprint per
/// contiguous window of widths 2..=8. When a physical clone spans many
/// siblings, several windows cover overlapping byte ranges in the same
/// file and — without this dedup — all survive as distinct members of
/// the cluster. That inflates `members.len()` (used by
/// [`rank_weight`]), the rendered `occurrences` list, and the
/// `cluster-by-id` MCP payload.
///
/// Cross-file distinctness is preserved: two occurrences in different
/// files never collapse, no matter how their byte ranges relate. Two
/// non-overlapping occurrences inside the same file also survive —
/// only a transitively overlapping chain collapses to one canonical
/// member. Within a run the representative is selected by authored
/// scope and matched width ([PIPELINE-CLUSTER-EXACT-SCOPE]): an
/// enclosing view inside the same authored declaration stays; otherwise
/// the wider byte range wins when another copy matches its width
/// ([PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED]), with stable byte-range
/// ordering as the tie-breaker. Pair grades never choose a view — a
/// bridge that should not connect two components must fail pair
/// admission, not be hidden by the collapse.
#[must_use]
pub(super) fn collapse_overlapping_per_file(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
    scopes: &DeclarationScopes<'_, impl BuildHasher>,
) -> Vec<usize> {
    let by_file = members_by_file(fused, fingerprints);
    let mut evidence = WidthEvidence::new(fused, fingerprints, scopes);
    let mut out: Vec<usize> = Vec::new();
    for bucket in by_file.into_values() {
        out.extend(collapse_overlapping_single_file(
            bucket,
            scopes,
            &mut evidence,
        ));
    }
    // Corpus-index order, not `FileId` order: ids encode registration
    // history (a removed-and-restored file gets a fresh id), while the
    // corpus index follows the path-ordered snapshot, so rendered
    // occurrence order stays byte-identical across edit history
    // ([PIPELINE-DETERMINISM]).
    out.sort_unstable();
    out
}

/// The component's members grouped by file, each with its fingerprint.
fn members_by_file(
    fused: &FusedCluster,
    fingerprints: &[Fingerprint],
) -> BTreeMap<FileId, Vec<(usize, Fingerprint)>> {
    let mut by_file: BTreeMap<FileId, Vec<(usize, Fingerprint)>> = BTreeMap::new();
    for index in fused.members.iter().copied() {
        if let Some(member) = fingerprints.get(index) {
            by_file
                .entry(member.file_id)
                .or_default()
                .push((index, member.clone()));
        }
    }
    by_file
}

/// Greedy sweep over one file's occurrences: sort by `(start, -end)`
/// and keep one canonical member per overlapping run. The representative
/// is the enclosing view when it shares an authored declaration with
/// the candidate; otherwise the widest byte range (largest physical
/// clone) wins, and equal-width ties keep the first-encountered member
/// so the result stays deterministic across runs
/// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
///
/// The run's frontier is tracked separately from its representative
/// ([PIPELINE-CLUSTER-EXACT]). Overlap is transitive, and the window
/// that bridges two others is often narrower than both: for `[0,100]`,
/// `[90,110]`, `[105,200]` the bridge loses the width contest, so a
/// sweep that tests the next window against the representative alone
/// finds `[105,200]` disjoint and publishes one physical region as two
/// occurrences — inflating the cluster size, the occurrence list and the
/// duplication percentage.
fn collapse_overlapping_single_file<L: BuildHasher>(
    mut bucket: Vec<(usize, Fingerprint)>,
    scopes: &DeclarationScopes<'_, L>,
    evidence: &mut WidthEvidence<'_, '_, L>,
) -> Vec<usize> {
    bucket.sort_by_key(|(_, member)| {
        (
            member.byte_range.start,
            usize::MAX.saturating_sub(member.byte_range.end),
        )
    });
    let mut runs: Vec<OverlapRun> = Vec::with_capacity(bucket.len());
    for (index, member) in bucket {
        let candidate = Occurrence::of(index, &member, scopes);
        match runs.last_mut() {
            Some(run) if run.reaches(candidate.range) => run.absorb(candidate, evidence),
            _ => runs.push(OverlapRun::start(candidate)),
        }
    }
    runs.into_iter()
        .map(|run| run.representative.index)
        .collect()
}

/// One same-file occurrence competing to represent an overlapping run.
#[derive(Clone, Copy)]
struct Occurrence {
    /// Fingerprint index, which is what the run finally publishes.
    index: usize,
    /// Byte range this occurrence claims.
    range: ByteRange,
    /// Normalised nodes the occurrence holds.
    nodes: usize,
    /// The authored declaration it sits strictly inside, when the
    /// grammar names one ([`DeclarationScopes::enclosing`]).
    declaration: Option<ByteRange>,
    /// The occurrence is an authored function — its range equals a
    /// function-like declaration's ([`DeclarationScopes::aligned_function`]).
    aligned: bool,
    /// The occurrence is a node the author wrote rather than a window
    /// cut over a run of siblings ([`DeclarationScopes::is_authored_node`]).
    authored: bool,
}

impl Occurrence {
    /// Reads one member's view facts off the declaration scopes.
    fn of(
        index: usize,
        member: &Fingerprint,
        scopes: &DeclarationScopes<'_, impl BuildHasher>,
    ) -> Self {
        Self {
            index,
            range: member.byte_range,
            nodes: member.node_count,
            declaration: scopes.enclosing(member),
            aligned: scopes.aligned_function(member).is_some(),
            authored: scopes.is_authored_node(member),
        }
    }

    /// Whether one authored declaration covers both views, including a file around a function.
    fn shares_declaration_with(&self, other: &Self) -> bool {
        match (self.declaration, other.declaration) {
            (Some(mine), Some(theirs)) => mine == theirs,
            (None, Some(_)) => true,
            (_, None) => false,
        }
    }

    /// True when this occurrence covers `other` and is wider on at
    /// least one side.
    fn encloses(&self, other: &Self) -> bool {
        self.range.strictly_encloses(other.range)
    }

    /// True when this occurrence is a window — not a node the author
    /// wrote — that encloses the authored function `function` with
    /// fewer than the rescue node floor of sibling nodes around it: the
    /// function plus scraps ([PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS]).
    fn is_scraps_around(&self, function: &Self) -> bool {
        !self.authored
            && function.aligned
            && self.encloses(function)
            && self.nodes.saturating_sub(function.nodes) < SHARED_SUBTREE_MIN_NODE_COUNT
    }

    /// True when the two share bytes but neither covers the other, so
    /// each starts or ends inside the other's region.
    fn straddles(&self, other: &Self) -> bool {
        self.range.partially_overlaps(other.range)
    }
}

/// One transitively-overlapping run of same-file occurrences, reduced to
/// the reported location plus the frontier the next window is tested
/// against.
struct OverlapRun {
    /// The best occurrence so far — the one the report publishes for
    /// this run.
    representative: Occurrence,
    /// Highest end byte anywhere in the run, which is not always the
    /// representative's end.
    end: usize,
}

impl OverlapRun {
    /// Opens a run at `first`.
    fn start(first: Occurrence) -> Self {
        Self {
            end: first.range.end,
            representative: first,
        }
    }

    /// Returns `true` when `candidate` overlaps the run. Members arrive
    /// in ascending start order, so reaching past the frontier is the
    /// whole half-open overlap test.
    fn reaches(&self, candidate: ByteRange) -> bool {
        candidate.start < self.end
    }

    /// Extends the run, promoting `candidate` to representative when it
    /// outranks the incumbent ([`Self::displaces`]).
    fn absorb<L: BuildHasher>(
        &mut self,
        candidate: Occurrence,
        evidence: &mut WidthEvidence<'_, '_, L>,
    ) {
        self.end = self.end.max(candidate.range.end);
        if self.displaces(&candidate, evidence) {
            self.representative = candidate;
        }
    }

    /// The wider authored scope displaces the incumbent; equal widths
    /// keep the incumbent ([PIPELINE-CLUSTER-EXACT-SCOPE]).
    ///
    /// **Inside one declaration grades are never compared.**
    /// A window nested in the occurrence it competes with measures a
    /// higher cross-file edge exactly to the extent that it drops the
    /// statements the two copies disagree on, so a grade contest inside
    /// one authored declaration would keep whichever window omits the most.
    /// The enclosing view therefore stays when it shares the authored
    /// declaration with the candidate — `typescript-type3` pins the
    /// enclosing `accumulate`/`aggregate` view winning over the 37-node
    /// interior run that dropped the extra `running = running + 2` and
    /// reported a Merkle-equal pair
    /// (`js_ts_signatures::typescript_near_miss_produces_cross_file_structural_cluster`).
    ///
    /// Between views that do not share a declaration, width decides when
    /// a copy matches it ([PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED]); a
    /// bridge that should not connect two files must fail pair admission,
    /// never be hidden by the collapse.
    /// `fsharp_issue_339_sibling_window_rename` keeps passing because
    /// the wider whole-module view never reaches the component (its
    /// pair to the other module fails admission), so the exact sibling
    /// window remains the only view of the region.
    fn displaces<L: BuildHasher>(
        &self,
        candidate: &Occurrence,
        evidence: &mut WidthEvidence<'_, '_, L>,
    ) -> bool {
        // Pair grades cannot choose a view: a bridge that should not
        // connect must fail pair admission, not be hidden by the
        // collapse. Equal-width ties keep the incumbent, so the run
        // stays deterministic across runs.
        self.declaration_verdict(candidate)
            .or_else(|| self.scraps_verdict(candidate))
            .or_else(|| self.scope_verdict(candidate))
            .or_else(|| self.matched_verdict(candidate, evidence))
            .unwrap_or_else(|| candidate.range.len() > self.representative.range.len())
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE] Inside one authored declaration the
    /// enclosing view stays. `None` where the incumbent does not enclose
    /// the candidate within one declaration.
    fn scope_verdict(&self, candidate: &Occurrence) -> Option<bool> {
        let incumbent = &self.representative;
        (incumbent.encloses(candidate) && incumbent.shares_declaration_with(candidate))
            .then_some(false)
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE-MATCHED] Between two views of one
    /// run, a view whose width a copy matches beats one whose copies all
    /// miss it; anything else — equals, or a view with no copy of its own
    /// — leaves the width rule standing. Width no copy has is whatever the
    /// author wrote next to the duplication — a type declared above a
    /// function — not part of it. `None` where the width rule stands.
    fn matched_verdict<L: BuildHasher>(
        &self,
        candidate: &Occurrence,
        evidence: &mut WidthEvidence<'_, '_, L>,
    ) -> Option<bool> {
        match (
            evidence.width(self.representative.index),
            evidence.width(candidate.index),
        ) {
            (Width::Unmatched, Width::Matched) => Some(true),
            (Width::Matched, Width::Unmatched) => Some(false),
            _ => None,
        }
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE-SCRAPS] Between an authored function
    /// and a window that encloses it with fewer than the rescue node
    /// floor of siblings around it, the function is the finding. The
    /// window is the function plus scraps — two field declarations, a
    /// constructor line — and reporting it would publish one method of
    /// a family at a different extent from its siblings. A node the
    /// author wrote — a class body, a whole file — keeps the width rule.
    /// `None` where the rule does not decide.
    fn scraps_verdict(&self, candidate: &Occurrence) -> Option<bool> {
        if self.representative.is_scraps_around(candidate) {
            return Some(true);
        }
        if candidate.is_scraps_around(&self.representative) {
            return Some(false);
        }
        None
    }

    /// [PIPELINE-CLUSTER-EXACT-SCOPE-STRADDLE] Between two views that
    /// straddle each other, the one that *is* an authored declaration is
    /// the finding. The other starts or ends inside a function it does
    /// not contain, so it welds a cut-off body to whatever sits beside
    /// it — a namespace line, a class shell, a sibling member — and no
    /// width can make that region something the author wrote. Views
    /// where one contains the other never reach this rule: a whole file
    /// holding a method whole is still the wider authored scope.
    /// `None` where the rule does not decide.
    fn declaration_verdict(&self, candidate: &Occurrence) -> Option<bool> {
        if !self.representative.straddles(candidate) {
            return None;
        }
        match (self.representative.aligned, candidate.aligned) {
            (true, _) => Some(false),
            (false, true) => Some(true),
            (false, false) => None,
        }
    }
}
