//! [CLONE-KIND-FOLD] The clone kind of a whole cluster, folded from the
//! same pair measurement `pair/compare` answers with.
//!
//! A cluster is the transitive closure of admitted pairs, so no single
//! measurement describes it. Its kind is the weakest pair classification
//! between the canonical (first) occurrence and every other member: what
//! an explicit comparison of that member against the canonical would
//! report. A cluster whose every member is byte-identical to the
//! canonical is `Identical`; one member that only shares shape makes the
//! whole cluster `StructuralOnly`; one member the direct comparison does
//! not admit at all — welded in only through other members — makes it
//! `LooselySimilar`. The fold never averages, and it reads no cluster
//! quantity.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use crate::{
    ast::NormalizedNode, buckets::ClusterKind, cluster::ClusterKindJudge,
    embedding::pairs::EmbeddingPair, fingerprint::Fingerprint, pipeline::PipelineSession,
};

use super::{AdmissionFacts, PairAxes, ResolvedEndpoint, ResolvedPair};

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
    ) -> Self {
        Self {
            session,
            fingerprints,
            axes: Mutex::new(PairAxes::new(trees)),
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

    /// The kind one `(canonical, member)` pair folds into: exactly the
    /// classification `pair/compare` reports for the two endpoints.
    fn pair_kind(
        &self,
        canonical: ResolvedEndpoint<'corpus>,
        member: ResolvedEndpoint<'corpus>,
    ) -> ClusterKind {
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
    fn kind(&self, members: &[usize]) -> ClusterKind {
        let Some((canonical, rest)) = members.split_first() else {
            return ClusterKind::Identical;
        };
        let Some(canonical) = self.endpoint(*canonical) else {
            return ClusterKind::LooselySimilar;
        };
        rest.iter()
            .filter_map(|index| self.endpoint(*index))
            .map(|member| self.pair_kind(canonical, member))
            .fold(ClusterKind::Identical, ClusterKind::weaker)
    }
}
