//! [CLONE-KIND-FOLD] Groups established copies separately from informational matches.
//! Each relation uses the shared pair measurement and recorded embedding evidence.
//! Rejected comparisons do not become Similar, and unrelated members cannot hide copies.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use super::{AdmissionFacts, PairAxes, ResolvedEndpoint, ResolvedPair};
use crate::{
    ast::NormalizedNode, buckets::ClusterKind, cluster::ClusterKindJudge,
    embedding::pairs::EmbeddingPair, fingerprint::Fingerprint, pair::CandidatePair,
    pipeline::PipelineSession,
};

/// The embedding cosine of a pair the embedding pass never measured: the
/// axis contributes nothing, exactly as it did at admission.
const UNMEASURED_COSINE: f64 = 0.0;

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
            axes: Mutex::new(PairAxes::new(trees, session, pairs)),
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
        let pair = ResolvedPair {
            left: canonical,
            right: member,
        };
        let embedding_cos = self
            .embedding_cosines
            .get(&(
                canonical.index.min(member.index),
                canonical.index.max(member.index),
            ))
            .copied()
            .unwrap_or(UNMEASURED_COSINE);
        let measured = {
            let mut axes = self.axes.lock().unwrap_or_else(PoisonError::into_inner);
            self.session.measure_axes(&pair, &mut axes, embedding_cos)
        };
        let facts = AdmissionFacts::from(self.session, &pair, measured);
        ClusterKind::from_pair(facts.classification(measured))
    }
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
