//! Pair-owned raw-content evidence ([FUSED-CONTENT-GATE]).

use std::{collections::HashMap, hash::BuildHasher};

mod frontier;
mod rename;

use frontier::{
    frontiers_aligned, key_set_jaccard, member_content, member_count, operator_contradiction,
    positional_agreement, MemberContent, Population,
};

use crate::{ast::NormalizedNode, fingerprint::Fingerprint, state::FileId};

/// Minimum combined literal count before literal share is meaningful.
const LITERAL_TABLE_MIN_LITERALS: usize = 8;

/// Semantic contradiction that prevents pair-content support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentContradiction {
    /// The endpoints carry no known contradiction.
    None,
    /// A behaviour-bearing operator changed.
    OperatorSubstitution,
}

/// Raw-content evidence measured on exactly two endpoints.
#[derive(Debug, Clone, Copy)]
pub struct ContentEvidence {
    /// Fraction of authored collapsed positions whose raw bytes agree.
    pub agreement: f64,
    /// Pair-specific Type-2 rename evidence.
    pub rename_consistency: f64,
    /// Symmetric literal share across both endpoint frontiers.
    pub literal_fraction: f64,
    /// Whether both endpoints resolved to authored content.
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

    /// Returns explicit evidence for an unresolved pair.
    #[must_use]
    pub const fn unmeasured() -> Self {
        Self {
            agreement: 0.0,
            rename_consistency: 0.0,
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
    measure_pair_content_indexed(
        left,
        right,
        &tree_index,
        sources,
        languages,
        PairShape::default(),
    )
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
    shape: PairShape,
) -> ContentEvidence {
    let scope = PairScope {
        same_file: left.file_id == right.file_id,
        interior: shape.interior,
    };
    let left = member_content(left, tree_index, sources, languages);
    let right = member_content(right, tree_index, sources, languages);
    pair_evidence(left.as_ref().zip(right.as_ref()), sources, scope)
}

/// What the caller knows about where the two endpoints sit — the half
/// of [`PairScope`] that only a caller holding the declaration scopes
/// can answer.
#[derive(Clone, Copy, Default)]
pub(crate) struct PairShape {
    /// Both endpoints are windows strictly inside an authored function.
    pub(crate) interior: bool,
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
    if operator_contradiction(left, right) {
        return ContentEvidence {
            measured: true,
            contradiction: ContentContradiction::OperatorSubstitution,
            ..ContentEvidence::unmeasured()
        };
    }
    ContentEvidence {
        agreement: pair_agreement(Some(left), Some(right)),
        rename_consistency: rename::pair_rename_consistency(
            Some(left),
            Some(right),
            sources,
            scope,
        ),
        literal_fraction: pair_literal_fraction(left, right),
        measured: true,
        contradiction: ContentContradiction::None,
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

/// Measures pair agreement using an already-built tree index.
pub(crate) fn pair_content_agreement<S: BuildHasher, L: BuildHasher>(
    left: &Fingerprint,
    right: &Fingerprint,
    tree_index: &HashMap<FileId, &NormalizedNode>,
    sources: &HashMap<FileId, Vec<u8>, S>,
    languages: &HashMap<FileId, &'static str, L>,
) -> f64 {
    let left = member_content(left, tree_index, sources, languages);
    let right = member_content(right, tree_index, sources, languages);
    pair_agreement(left.as_ref(), right.as_ref())
}

/// Fraction of aligned authored positions whose raw bytes match.
fn pair_agreement(left: Option<&MemberContent>, right: Option<&MemberContent>) -> f64 {
    let (Some(left), Some(right)) = (left, right) else {
        return 0.0;
    };
    if operator_contradiction(left, right) {
        return 0.0;
    }
    if left.keys.is_empty() && right.keys.is_empty() {
        return 1.0;
    }
    if !frontiers_aligned(left, right) {
        return key_set_jaccard(&left.keys, &right.keys);
    }
    positional_agreement(&left.keys, &right.keys)
}

/// Returns a share, treating an empty evidence population as consistent.
fn vacuous_share(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 1.0;
    }
    member_count(numerator) / member_count(denominator)
}
