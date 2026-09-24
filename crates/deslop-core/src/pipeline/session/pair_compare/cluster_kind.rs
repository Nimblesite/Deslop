//! [CLONE-KIND-FOLD] Groups established copies separately from informational matches.
//! Each relation uses the shared pair measurement and recorded embedding evidence.
//! Rejected comparisons do not become Similar, and unrelated members cannot hide copies.

use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, PoisonError},
};

use super::{
    same_source_bytes_and_language, AdmissionFacts, Measurements, PairAxes, ResolvedEndpoint,
    ResolvedPair,
};
use crate::{
    ast::NormalizedNode,
    buckets::ClusterKind,
    cluster::ClusterKindJudge,
    embedding::pairs::EmbeddingPair,
    fingerprint::Fingerprint,
    pair::{
        CandidatePair, CROSS_LANGUAGE_MIN_JACCARD, FUSED_THRESHOLD, SHARED_SUBTREE_MIN_JACCARD,
        SHARED_SUBTREE_MIN_OVERLAP,
    },
    pipeline::PipelineSession,
};

/// The embedding cosine of a pair the embedding pass never measured: the
/// axis contributes nothing, exactly as it did at admission.
const UNMEASURED_COSINE: f64 = 0.0;
/// Most pair classifications retained across a render's folds.
const PAIR_KIND_MEMO_MAX: usize = 16_384;
/// Reused verdicts retained when a full memo rotates.
const PAIR_KIND_MEMO_HOT_MAX: usize = 4_096;

/// Exact ordered-index verdicts, including pairs that cannot be classified.
#[derive(Default)]
struct PairKindMemo {
    /// The rendered corpus owns stable flat indices for this memo's lifetime.
    decisions: HashMap<(usize, usize), Option<ClusterKind>>,
    /// Hits since the last rotation; cold verdicts can be evicted first.
    reused: HashSet<(usize, usize)>,
}

#[cfg(test)]
#[path = "cluster_kind/tests.rs"]
mod partition_tests;

#[cfg(test)]
#[path = "cluster_kind/rescue_cache_tests.rs"]
mod rescue_cache_tests;

/// A memo miss is distinct from a measured pair without a clone kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PairKindLookup {
    /// This ordered pair has not been classified.
    Unseen,
    /// The measured pair's kind, or a recorded rejection.
    Seen(Option<ClusterKind>),
}

impl PairKindMemo {
    /// Returns a saved verdict, distinguishing a rejected pair from a miss.
    fn get(&mut self, key: (usize, usize)) -> PairKindLookup {
        let verdict = self.decisions.get(&key).copied();
        if verdict.is_some() && self.reused.len() < PAIR_KIND_MEMO_HOT_MAX {
            let _inserted = self.reused.insert(key);
        }
        verdict.map_or(PairKindLookup::Unseen, PairKindLookup::Seen)
    }

    /// Rotates at capacity, preserving bounded reused verdicts.
    fn remember(&mut self, key: (usize, usize), kind: Option<ClusterKind>) {
        if self.decisions.len() >= PAIR_KIND_MEMO_MAX {
            self.decisions
                .retain(|pair, _kind| self.reused.contains(pair));
            self.reused.clear();
        }
        let _previous = self.decisions.insert(key, kind);
    }

    /// Number of retained verdicts for the bounded-memory test.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.decisions.len()
    }
}

/// A byte-proven identical clone when its authored frontiers also match.
fn exact_text_kind(
    left: &Fingerprint,
    right: &Fingerprint,
    sources: &HashMap<crate::state::FileId, Vec<u8>>,
    languages: &HashMap<crate::state::FileId, &'static str>,
    same_frontier: impl FnOnce() -> bool,
) -> Option<ClusterKind> {
    (left.hash == right.hash
        && same_source_bytes_and_language(left, right, sources, languages)
        && same_frontier())
    .then_some(ClusterKind::Identical)
}

/// Measures cluster kinds over one render's corpus ([CLONE-KIND-FOLD]).
pub(crate) struct ClusterKindMeasurer<'corpus> {
    /// The session whose corpus, signatures, sources and policy the
    /// pair measurement reads.
    session: &'corpus PipelineSession,
    /// Every live fingerprint, flat, in corpus order — the slice the
    /// cluster build indexes members into.
    fingerprints: &'corpus [Fingerprint],
    /// The memoising measurers, shared across every cluster of the
    /// render. Locked per pair; the build hands clusters over one at a
    /// time.
    axes: Mutex<PairAxes<'corpus>>,
    /// Repeat classifications across recovery, grouping, and ranking.
    kinds: Mutex<PairKindMemo>,
    /// The cosines the embedding pass measured, keyed `(lower, higher)`
    /// flat index, so the fold reads the same embedding evidence the
    /// admission did instead of re-embedding every member.
    embedding_cosines: HashMap<(usize, usize), f64>,
}

impl<'corpus> ClusterKindMeasurer<'corpus> {
    /// Prepares the fold over the render's trees and embedding pairs.
    pub(crate) fn new(
        session: &'corpus PipelineSession,
        fingerprints: &'corpus [Fingerprint],
        trees: &'corpus [NormalizedNode],
        embedding_pairs: &[EmbeddingPair],
        pairs: &[CandidatePair],
    ) -> Self {
        Self {
            session,
            fingerprints,
            axes: Mutex::new(rank_axes(trees, session, fingerprints, pairs)),
            kinds: Mutex::new(PairKindMemo::default()),
            embedding_cosines: embedding_pairs
                .iter()
                .map(|pair| {
                    (
                        (pair.left.min(pair.right), pair.left.max(pair.right)),
                        pair.cosine,
                    )
                })
                .collect(),
        }
    }

    /// The flat-corpus endpoint at `index`, or `None` when the build
    /// handed an index outside the corpus — a defect, logged rather than
    /// hidden inside a wrong kind.
    fn endpoint(&self, index: usize) -> Option<ResolvedEndpoint<'corpus>> {
        let fingerprint = self.fingerprints.get(index);
        if fingerprint.is_none() {
            tracing::error!(index, "cluster member index outside the corpus");
        }
        fingerprint.map(|fingerprint| ResolvedEndpoint { index, fingerprint })
    }

    /// Classifies one `(canonical, member)` pair with the shared pair
    /// algebra and the embedding cosine recorded during the scan.
    fn pair_kind(
        &self,
        canonical: ResolvedEndpoint<'corpus>,
        member: ResolvedEndpoint<'corpus>,
    ) -> Option<ClusterKind> {
        let key = (canonical.index, member.index);
        if let PairKindLookup::Seen(known) = self
            .kinds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
        {
            return known;
        }
        let kind = self.classify_pair(canonical, member);
        self.kinds
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remember(key, kind);
        kind
    }

    /// Classifies a cache miss through the shared pair algebra.
    fn classify_pair(
        &self,
        canonical: ResolvedEndpoint<'corpus>,
        member: ResolvedEndpoint<'corpus>,
    ) -> Option<ClusterKind> {
        let pair = ResolvedPair {
            left: canonical,
            right: member,
        };
        if let Some(kind) = self.exact_kind(&pair) {
            return Some(kind);
        }
        let embedding_cos = self.embedding_cos(canonical.index, member.index);
        let measured = self.measured_kind_pair(&pair, embedding_cos)?;
        let facts = AdmissionFacts::from(self.session, &pair, measured);
        ClusterKind::from_pair(facts.classification(measured))
    }

    /// Equal whole bytes still require equal authored leaf boundaries.
    fn exact_kind(&self, pair: &ResolvedPair<'corpus>) -> Option<ClusterKind> {
        exact_text_kind(
            pair.left.fingerprint,
            pair.right.fingerprint,
            &self.session.sources,
            &self.session.file_languages,
            || {
                let mut axes = self.axes.lock().unwrap_or_else(PoisonError::into_inner);
                let PairAxes { content, trees, .. } = &mut *axes;
                content
                    .pair(
                        (pair.left.fingerprint, pair.right.fingerprint),
                        trees,
                        &self.session.sources,
                        &self.session.file_languages,
                    )
                    .exact_frontier_match(&self.session.sources)
            },
        )
    }

    /// Reads the cosine already measured by the embedding pass.
    fn embedding_cos(&self, left: usize, right: usize) -> f64 {
        self.embedding_cosines
            .get(&(left.min(right), left.max(right)))
            .copied()
            .unwrap_or(UNMEASURED_COSINE)
    }

    /// [FUSED-SHARED-SUBTREE-BOUND] Skip an exact alignment only when a
    /// sound upper bound rules out both clone admission and information.
    fn measured_kind_pair(
        &self,
        pair: &ResolvedPair<'corpus>,
        cosine: f64,
    ) -> Option<Measurements> {
        let mut axes = self.axes.lock().unwrap_or_else(PoisonError::into_inner);
        if self.ruled_out_by_overlap_bound(pair, &mut axes, cosine) {
            return None;
        }
        Some(self.session.measure_axes(pair, &mut axes, cosine))
    }

    /// A below-floor bound cannot rescue the pair or meet the configured
    /// shape-only floor. Only a token or embedding axis could still admit it.
    fn ruled_out_by_overlap_bound(
        &self,
        pair: &ResolvedPair<'corpus>,
        axes: &mut PairAxes<'corpus>,
        cosine: f64,
    ) -> bool {
        if pair.left.fingerprint.hash == pair.right.fingerprint.hash {
            return false;
        }
        let token = self.session.token_jaccard(pair, false, &axes.trees);
        let threshold = self.admission_threshold(pair);
        if token >= threshold || cosine >= threshold {
            return false;
        }
        let floor = self.needed_overlap_floor(token);
        axes.overlap
            .bounded_overlap(pair.left.fingerprint, pair.right.fingerprint, floor)
            < floor
    }

    /// Rescue needs 0.75 only with corroborating tokens; otherwise the
    /// configured shape-only category is the sole remaining route.
    fn needed_overlap_floor(&self, token: f64) -> f64 {
        let shape_floor = self.session.exclusion.routing().nearly_identical_min_shape;
        if token >= SHARED_SUBTREE_MIN_JACCARD {
            SHARED_SUBTREE_MIN_OVERLAP.min(shape_floor)
        } else {
            shape_floor
        }
    }

    /// Explicit cross-language audits use a different admission floor.
    fn admission_threshold(&self, pair: &ResolvedPair<'_>) -> f64 {
        if pair.cross_language(&self.session.file_languages)
            && self.session.exclusion.allows_cross_language_comparison()
        {
            CROSS_LANGUAGE_MIN_JACCARD
        } else {
            FUSED_THRESHOLD
        }
    }
}

/// Seeds only rescue alignments that cleared the floor; lower scores may
/// be bounds and are never valid answers for ranked classification.
fn rank_axes<'corpus>(
    trees: &'corpus [NormalizedNode],
    session: &'corpus PipelineSession,
    fingerprints: &[Fingerprint],
    pairs: &[CandidatePair],
) -> PairAxes<'corpus> {
    let mut axes = PairAxes::new(trees, session, pairs);
    for pair in pairs {
        if let Some((left, right)) = fingerprints
            .get(pair.left)
            .zip(fingerprints.get(pair.right))
        {
            axes.overlap
                .remember_rescued_exact(left, right, pair.shared_subtree_overlap);
        }
    }
    axes
}

impl ClusterKindJudge for ClusterKindMeasurer<'_> {
    /// The weakest relation between the canonical member and any other.
    /// The fold starts from `Identical` — a member is identical to
    /// itself — and every other member can only weaken it.
    fn kind(&self, members: &[usize]) -> Option<ClusterKind> {
        let (canonical, rest) = members.split_first()?;
        let canonical = self.endpoint(*canonical)?;
        rest.iter().try_fold(ClusterKind::Identical, |kind, index| {
            let relation = self.pair_kind(canonical, self.endpoint(*index)?)?;
            Some(kind.weaker(relation))
        })
    }

    fn groups(&self, members: &[usize]) -> Vec<(Vec<usize>, ClusterKind)> {
        crate::buckets::grouping::group_members(members, |left, right| {
            self.pair_kind(self.endpoint(left)?, self.endpoint(right)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::ByteRange,
        state::{FileId, FileRegistry},
    };

    const LEFT: usize = 7;
    const RIGHT: usize = 11;
    const COLD_LEFT: usize = 8;
    const RETAINED_AFTER_ROTATION: usize = 2;
    const EXACT_SOURCE: &[u8] = b"fn total() { accept(); }";
    const CHANGED_SOURCE: &[u8] = b"fn total() { reject(); }";
    const LEFT_PATH: &str = "left.rs";
    const RIGHT_PATH: &str = "right.rs";
    const LANGUAGE: &str = "rust";
    const OTHER_LANGUAGE: &str = "typescript";
    const EXACT_HASH: [u8; 32] = [1; 32];
    const DIFFERENT_HASH: [u8; 32] = [2; 32];
    const RANGE_START: usize = 0;
    const NODE_COUNT: usize = 40;

    struct ExactFixture {
        left: Fingerprint,
        right: Fingerprint,
        sources: HashMap<FileId, Vec<u8>>,
        languages: HashMap<FileId, &'static str>,
    }

    fn exact_fingerprint(file_id: FileId) -> Fingerprint {
        Fingerprint {
            hash: EXACT_HASH,
            file_id,
            byte_range: ByteRange {
                start: RANGE_START,
                end: EXACT_SOURCE.len(),
            },
            node_count: NODE_COUNT,
        }
    }

    impl ExactFixture {
        fn new() -> Self {
            let mut registry = FileRegistry::new();
            let left_id = registry.register(LEFT_PATH.into());
            let right_id = registry.register(RIGHT_PATH.into());
            Self {
                left: exact_fingerprint(left_id),
                right: exact_fingerprint(right_id),
                sources: HashMap::from([
                    (left_id, EXACT_SOURCE.to_vec()),
                    (right_id, EXACT_SOURCE.to_vec()),
                ]),
                languages: HashMap::from([(left_id, LANGUAGE), (right_id, LANGUAGE)]),
            }
        }
    }

    /// [CLONE-KIND-FOLD] Matching bytes and Merkle shape need no pair algebra.
    #[test]
    fn exact_text_verdict_requires_resolved_content() {
        let fixture = ExactFixture::new();
        let classify = |resolved| {
            exact_text_kind(
                &fixture.left,
                &fixture.right,
                &fixture.sources,
                &fixture.languages,
                || resolved,
            )
        };
        assert_eq!(classify(true), Some(ClusterKind::Identical));
        assert_eq!(classify(false), None);
    }

    #[test]
    fn exact_text_verdict_ignores_changed_bytes_shape_and_language() {
        let mut fixture = ExactFixture::new();
        let classify = |fixture: &ExactFixture| {
            exact_text_kind(
                &fixture.left,
                &fixture.right,
                &fixture.sources,
                &fixture.languages,
                || true,
            )
        };
        let _previous = fixture
            .sources
            .insert(fixture.right.file_id, CHANGED_SOURCE.to_vec());
        assert_eq!(classify(&fixture), None);
        let _previous = fixture
            .sources
            .insert(fixture.right.file_id, EXACT_SOURCE.to_vec());
        fixture.right.hash = DIFFERENT_HASH;
        assert_eq!(classify(&fixture), None);
        fixture.right.hash = EXACT_HASH;
        let _previous = fixture
            .languages
            .insert(fixture.right.file_id, OTHER_LANGUAGE);
        assert_eq!(classify(&fixture), None);
    }

    /// [CLONE-KIND-FOLD] Repeated pair grading reuses the exact ordered verdict.
    #[test]
    fn ordered_pair_verdicts_include_rejection_and_do_not_flip_direction() {
        let mut cache = PairKindMemo::default();
        assert_eq!(cache.get((LEFT, RIGHT)), PairKindLookup::Unseen);
        cache.remember((LEFT, RIGHT), Some(ClusterKind::NearlyIdentical));
        assert_eq!(
            cache.get((LEFT, RIGHT)),
            PairKindLookup::Seen(Some(ClusterKind::NearlyIdentical))
        );
        assert_eq!(cache.get((RIGHT, LEFT)), PairKindLookup::Unseen);
        cache.remember((RIGHT, LEFT), None);
        assert_eq!(cache.get((RIGHT, LEFT)), PairKindLookup::Seen(None));
    }

    /// [CLONE-KIND-FOLD] A full memo admits later keys without growing forever.
    #[test]
    fn pair_kind_memo_rotates_after_capacity() {
        let mut cache = PairKindMemo::default();
        for left in 0..PAIR_KIND_MEMO_MAX {
            cache.remember((left, RIGHT), Some(ClusterKind::Identical));
        }
        cache.remember(
            (PAIR_KIND_MEMO_MAX, RIGHT),
            Some(ClusterKind::NearlyIdentical),
        );
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get((LEFT, RIGHT)), PairKindLookup::Unseen);
        assert_eq!(
            cache.get((PAIR_KIND_MEMO_MAX, RIGHT)),
            PairKindLookup::Seen(Some(ClusterKind::NearlyIdentical))
        );
    }

    /// [CLONE-KIND-FOLD] A reused verdict survives rotation without retaining cold pairs.
    #[test]
    fn pair_kind_memo_keeps_reused_verdicts_across_rotation() {
        let mut cache = PairKindMemo::default();
        for left in 0..PAIR_KIND_MEMO_MAX {
            cache.remember((left, RIGHT), Some(ClusterKind::Identical));
        }
        assert_eq!(
            cache.get((LEFT, RIGHT)),
            PairKindLookup::Seen(Some(ClusterKind::Identical))
        );
        cache.remember(
            (PAIR_KIND_MEMO_MAX, RIGHT),
            Some(ClusterKind::NearlyIdentical),
        );
        assert_eq!(
            cache.get((LEFT, RIGHT)),
            PairKindLookup::Seen(Some(ClusterKind::Identical))
        );
        assert_eq!(cache.get((COLD_LEFT, RIGHT)), PairKindLookup::Unseen);
        assert_eq!(cache.len(), RETAINED_AFTER_ROTATION);
    }
}
