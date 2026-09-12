//! Pair-owned raw-content evidence ([FUSED-CONTENT-GATE]).

use std::{collections::HashMap, hash::BuildHasher};

mod call_targets;
mod core_frontier;
mod frontier;
mod rename;

use core_frontier::joined_content;
use frontier::{
    frontiers_aligned, key_set_jaccard, member_content, member_count, operator_contradiction,
    positional_agreement, MemberContent, Population,
};

use crate::{ast::NormalizedNode, fingerprint::Fingerprint, state::FileId};

/// Minimum combined literal count before literal share is meaningful.
const LITERAL_TABLE_MIN_LITERALS: usize = 8;

/// Semantic contradiction that blocks clone admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentContradiction {
    /// The endpoints carry no known contradiction.
    None,
    /// A behaviour-bearing operator changed.
    OperatorSubstitution,
    /// An external member-call selector changed ([FUSED-CONTENT-GATE-CALL-TARGET]).
    CallTargetSubstitution,
}

/// Raw-content evidence measured on exactly two endpoints.
#[derive(Debug, Clone, Copy)]
pub struct ContentEvidence {
    /// Fraction of authored collapsed positions whose raw bytes agree.
    pub agreement: f64,
    /// Pair-specific Type-2 rename evidence.
    pub rename_consistency: f64,
    /// Whether the pair is a rename of the same code, whatever its
    /// literals do: every substituted identifier position is explained
    /// by one bijection, at least one substitution is corroborated, and
    /// the copy keeps at least as many names as it renames
    /// ([FUSED-CONTENT-GATE-RENAME]).
    pub consistent_rename: bool,
    /// Symmetric literal share across both endpoint frontiers.
    pub literal_fraction: f64,
    /// Whether authored content similarity was measured.
    pub measured: bool,
    /// Semantic contradiction found on these endpoints.
    pub contradiction: ContentContradiction,
}

impl ContentEvidence {
    /// Returns `max(agreement, rename_consistency)` for pair admission.
    #[must_use]
    pub fn support(self) -> f64 {
        crate::buckets::content_support(self.agreement, self.rename_consistency)
    }

    /// Whether the evidence admits the pair at `floor`
    /// ([FUSED-CONTENT-GATE]): it was measured, and either the pair is a
    /// contradiction-free rename ([FUSED-CONTENT-GATE-RENAME]) or its
    /// pooled support clears the floor. One verdict, read by the
    /// pre-closure gate and by the rescue's core measurement alike.
    #[must_use]
    pub fn clears(self, floor: f64) -> bool {
        self.measured
            && self.contradiction == ContentContradiction::None
            && (self.consistent_rename || self.support() >= floor)
    }

    /// Returns explicit evidence for an unresolved pair.
    #[must_use]
    pub const fn unmeasured() -> Self {
        Self {
            agreement: 0.0,
            rename_consistency: 0.0,
            consistent_rename: false,
            literal_fraction: 0.0,
            measured: false,
            contradiction: ContentContradiction::None,
        }
    }
}

/// Indexes normalised trees by file for frontier resolution.
pub(crate) fn tree_index_of(trees: &[NormalizedNode]) -> HashMap<FileId, &NormalizedNode> {
    trees.iter().map(|tree| (tree.file_id, tree)).collect()
}

/// Measures all content axes on the two supplied endpoints.
pub fn measure_pair_content<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    trees: &[NormalizedNode],
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> ContentEvidence {
    let tree_index = tree_index_of(trees);
    measure_pair_content_indexed(left, right, &tree_index, sources, languages, false)
}

/// Measures both content axes using a caller-owned tree index.
///
/// The pre-closure admission gate evaluates many candidate pairs against one
/// tree population. Building the same file-id index per edge would turn a
/// pairwise measurement into an accidental corpus walk.
pub(crate) fn measure_pair_content_indexed<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
    interior: bool,
) -> ContentEvidence {
    let scope = PairScope {
        same_file: left.file_id == right.file_id,
        interior,
        core: false,
    };
    let left = member_content(left, tree_index, sources, languages);
    let right = member_content(right, tree_index, sources, languages);
    pair_evidence(left.as_ref().zip(right.as_ref()), sources, scope)
}

/// [FUSED-SHARED-SUBTREE-CORE] Measures both content axes over the code
/// two endpoints share: `core` is their Merkle-equal subtrees paired in
/// order ([`crate::overlap::OverlapMeasurer::aligned_core`]), so the
/// joined frontiers align position for position and every measure the
/// gate applies to a shape-equal pair applies here unchanged. An empty
/// or unresolvable core is unmeasured, and an unmeasured pair is never
/// admitted.
///
/// A contradiction is read over the whole endpoints first: the core
/// holds only what the two share, and a changed operator or call target
/// is exactly what they do not ([FUSED-CONTENT-GATE],
/// [FUSED-CONTENT-GATE-CALL-TARGET]).
pub(crate) fn measure_aligned_core<S: BuildHasher, L: BuildHasher>(
    endpoints: (&Fingerprint, &Fingerprint),
    core: &[(Fingerprint, Fingerprint)],
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
    scope: PairScope,
) -> ContentEvidence {
    let Some((whole_left, whole_right)) = whole_contents(endpoints, tree_index, sources, languages)
    else {
        return ContentEvidence::unmeasured();
    };
    if pair_contradiction(&whole_left, &whole_right).is_some() {
        return pair_evidence(Some((&whole_left, &whole_right)), sources, scope);
    }
    let joined = joined_content(core, (&whole_left, &whole_right));
    pair_evidence(
        joined.as_ref().map(|(left, right)| (left, right)),
        sources,
        scope,
    )
}

/// Both endpoints' whole frontiers, or `None` when either does not resolve.
fn whole_contents<S: BuildHasher, L: BuildHasher>(
    endpoints: (&Fingerprint, &Fingerprint),
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> Option<(MemberContent, MemberContent)> {
    member_content(endpoints.0, tree_index, sources, languages).zip(member_content(
        endpoints.1,
        tree_index,
        sources,
        languages,
    ))
}

/// Where the two endpoints sit, for the rename axis's scope rules
/// ([FUSED-CONTENT-GATE]).
#[derive(Clone, Copy)]
pub(crate) struct PairScope {
    /// Both endpoints are in one file, so the rename axis keeps its
    /// stricter same-file form.
    pub(crate) same_file: bool,
    /// Both endpoints are windows strictly inside an authored function,
    /// so a rename over a literal-free window cannot vouch for itself.
    pub(crate) interior: bool,
    /// The frontiers are the aligned core of a rescued pair
    /// ([FUSED-SHARED-SUBTREE-CORE]): the ordered-overlap and token
    /// floors the rescue demanded already vouch for the pair's
    /// vocabulary, so the rename test does not ask the core to keep as
    /// many names as it renames — a near-miss method that renames every
    /// local keeps nothing by that count.
    pub(crate) core: bool,
}

/// Builds pair evidence from two resolved content frontiers.
fn pair_evidence<S: BuildHasher>(
    pair: Option<(&MemberContent, &MemberContent)>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    scope: PairScope,
) -> ContentEvidence {
    let Some((left, right)) = pair else {
        return ContentEvidence::unmeasured();
    };

    ContentEvidence {
        agreement: pair_agreement(Some(left), Some(right)),
        rename_consistency: rename::pair_rename_consistency(
            Some(left),
            Some(right),
            sources,
            scope,
        ),
        consistent_rename: rename::pair_rename_is_consistent(left, right, sources, scope),
        literal_fraction: pair_literal_fraction(left, right),
        measured: true,
        contradiction: pair_contradiction(left, right).unwrap_or(ContentContradiction::None),
    }
}

/// Returns symmetric literal share across both endpoint frontiers.
fn pair_literal_fraction(left: &MemberContent, right: &MemberContent) -> f64 {
    let literals = left
        .keys
        .iter()
        .chain(&right.keys)
        .filter(|leaf| leaf.population == Population::Literal)
        .count();
    let vocabulary = left
        .keys
        .iter()
        .chain(&right.keys)
        .filter(|leaf| leaf.population != Population::Operator)
        .count();
    if literals < LITERAL_TABLE_MIN_LITERALS || vocabulary == 0 {
        return 0.0;
    }
    member_count(literals) / member_count(vocabulary)
}

/// Fraction of aligned authored positions whose raw bytes match.
fn pair_agreement(left: Option<&MemberContent>, right: Option<&MemberContent>) -> f64 {
    let (Some(left), Some(right)) = (left, right) else {
        return 0.0;
    };

    if left.keys.is_empty() && right.keys.is_empty() {
        return 1.0;
    }
    if !frontiers_aligned(left, right) {
        return key_set_jaccard(&left.keys, &right.keys);
    }
    positional_agreement(&left.keys, &right.keys)
}

/// Semantic contradictions share one verdict across pair admission and explicit comparison.
fn pair_contradiction(left: &MemberContent, right: &MemberContent) -> Option<ContentContradiction> {
    if operator_contradiction(left, right) {
        Some(ContentContradiction::OperatorSubstitution)
    } else if call_targets::contradicts(left, right) {
        Some(ContentContradiction::CallTargetSubstitution)
    } else {
        None
    }
}

/// Returns a share, treating an empty evidence population as consistent.
fn vacuous_share(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 1.0;
    }
    member_count(numerator) / member_count(denominator)
}
