//! [FUSED-CANDIDATE-BUCKET-RECOVERY] The session's copy recovery: one
//! shared gate corpus, one weld per worker.

use crate::{
    ast::NormalizedNode,
    cluster_filters::ParseCache,
    fingerprint::Fingerprint,
    pair::{CandidatePair, CopyProbe, FusedCluster, GateShared, GatedCopyWeld, PairGate},
    pipeline::PipelineSession,
};

impl PipelineSession {
    /// Adds the same-hash pairs the content gate welds inside each
    /// incomplete shape family, so an unrelated star centre cannot pull
    /// the copies behind it apart ([FUSED-CANDIDATE-BUCKET-STAR]).
    pub(super) fn recover_copy_pairs(
        &self,
        pairs: &mut Vec<CandidatePair>,
        families: &[FusedCluster],
        fingerprints: &[Fingerprint],
        trees: &[NormalizedNode],
        parse_cache: &ParseCache,
    ) {
        let shared = GateShared::new(
            pairs,
            fingerprints,
            trees,
            &self.sources,
            &self.file_languages,
            parse_cache,
        );
        let signatures = self.store.signatures();
        let allow_cross_language = self.exclusion.allows_cross_language_comparison();
        crate::pair::recover_copy_pairs(pairs, families, fingerprints, || {
            GatedCopyWeld::new(
                CopyProbe::new(
                    fingerprints,
                    &signatures,
                    &self.file_languages,
                    allow_cross_language,
                ),
                PairGate::new(&shared, fingerprints.len()),
                fingerprints,
            )
        });
    }
}
