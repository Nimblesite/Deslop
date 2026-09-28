//! Explicit endpoint-to-endpoint evidence measurement ([FUSED-PAIR-SIGNALS]).

use std::collections::{hash_map::RandomState, HashMap};

use super::PipelineSession;
use crate::{
    ast::NormalizedNode,
    content::{tree_index_of, ContentContradiction, ContentMeasurer, PairScope},
    embedding::{cosine_similarity, EmbeddingProvider},
    error::CoreError,
    lsh::estimate_jaccard,
    overlap::{OverlapMeasurer, RescueContext},
    pair::{CandidatePair, PairScore},
    report::{ContentMeasurement, PairComparison, PairComparisonParams, PairEvidence},
    state::FileId,
};

/// An explicit comparison measures the whole endpoints the caller named,
/// never the interior-window scope ([FUSED-PAIR-SIGNALS]).
const WHOLE_ENDPOINTS: bool = false;

mod admission;
use admission::AdmissionFacts;
mod cluster_kind;
pub(crate) use cluster_kind::ClusterKindMeasurer;
mod core_need;
mod endpoints;
use endpoints::{ResolvedEndpoint, ResolvedPair};
mod text_identity;
pub(super) use text_identity::same_source_bytes_and_language;
use text_identity::SourceIdentity;

impl PipelineSession {
    /// Recomputes evidence for exactly the two requested occurrences.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::SamePairEndpoint`] for a repeated endpoint,
    /// [`CoreError::UnknownPairEndpoint`] when either range is absent from
    /// this generation, and [`CoreError::Embedding`] when an active provider
    /// returns invalid evidence.
    pub fn compare_pair(
        &self,
        params: &PairComparisonParams,
        provider: Option<&dyn EmbeddingProvider>,
    ) -> Result<PairComparison, CoreError> {
        if params.left == params.right {
            return Err(CoreError::SamePairEndpoint);
        }
        let pair = self.resolve_pair(params)?;
        let evidence = self.measure_pair(&pair, provider)?;
        Ok(PairComparison {
            left: params.left.clone(),
            right: params.right.clone(),
            evidence,
        })
    }

    /// Measures every pair-owned axis and applies the admission algebra.
    fn measure_pair(
        &self,
        pair: &ResolvedPair<'_>,
        provider: Option<&dyn EmbeddingProvider>,
    ) -> Result<PairEvidence, CoreError> {
        let trees = self.trees_for_pair(pair)?;
        let pairs = self.comparison_anchor_pairs();
        let mut axes = PairAxes::new(&trees, self, &pairs);
        let embedding_cos = self.embedding_cos(pair, provider)?;
        let measurements = self.measure_axes(pair, &mut axes, embedding_cos, WHOLE_ENDPOINTS);
        Ok(self.build_evidence(pair, measurements))
    }

    /// [FUSED-SHARED-SUBTREE] Explicit comparisons use discovery's exact-copy anchors.
    fn comparison_anchor_pairs(&self) -> Vec<CandidatePair> {
        let signatures = self.store.signatures();
        crate::pair::candidate_pairs_for_language_policy(
            self.store.fingerprints(),
            &signatures,
            &crate::lsh::BandCollisionSource::new(&signatures),
            &[],
            None,
            &self.file_languages,
            self.exclusion.allows_cross_language_comparison(),
        )
    }

    /// Measures structural, token, and raw-content evidence beside the
    /// caller-supplied embedding cosine. The explicit comparison asks a
    /// provider for the cosine; the cluster fold reads the one the
    /// embedding pass already measured ([CLONE-KIND-FOLD]). `interior`
    /// is the content scope of two windows strictly inside a function
    /// ([FUSED-CONTENT-GATE-INTERIOR]), which only a span admitted here
    /// rather than at discovery pays.
    fn measure_axes(
        &self,
        pair: &ResolvedPair<'_>,
        axes: &mut PairAxes<'_>,
        embedding_cos: f64,
        interior: bool,
    ) -> Measurements {
        let merkle_equal = pair.left.fingerprint.hash == pair.right.fingerprint.hash;
        let text = pair.text_identity(&self.sources);
        let structural = axes
            .overlap
            .overlap(pair.left.fingerprint, pair.right.fingerprint);
        let token_jaccard = self.token_jaccard(pair, merkle_equal, &axes.trees);
        let content_pair = axes.content.pair(
            (pair.left.fingerprint, pair.right.fingerprint),
            &axes.trees,
            &self.sources,
            &self.file_languages,
        );
        let content = content_pair.whole(
            &self.sources,
            PairScope {
                same_file: !pair.cross_file(),
                interior,
                core: false,
            },
        );
        let mut measured = Measurements {
            score: PairScore {
                structural,
                token_jaccard,
                embedding_cos,
            },
            content,
            core: CoreCopy::Absent,
            rescue_scope: axes.rescue.allows_rescue(
                pair.left.fingerprint,
                pair.right.fingerprint,
                structural,
            ),
            merkle_equal,
            text,
        };
        let smaller_nodes = pair
            .left
            .fingerprint
            .node_count
            .min(pair.right.fingerprint.node_count);
        let preliminary = AdmissionFacts::from(self, pair, measured);
        let category_without_core = preliminary.classification(measured);
        if core_need::required(
            measured,
            smaller_nodes,
            preliminary.admitted,
            category_without_core,
        ) {
            measured.core = core_need::verdict(self, pair, axes, &content_pair);
        }
        measured
    }

    /// Estimates token Jaccard, applying the pair-local Merkle correction.
    fn token_jaccard(
        &self,
        pair: &ResolvedPair<'_>,
        merkle_equal: bool,
        trees: &HashMap<FileId, &NormalizedNode>,
    ) -> f64 {
        if merkle_equal {
            return 1.0;
        }
        if pair.cross_language(&self.file_languages) {
            return self.cross_language_jaccard(pair, trees);
        }
        let signatures = self.store.signatures();
        pair.left
            .signature(&signatures)
            .zip(pair.right.signature(&signatures))
            .map_or(0.0, |(left, right)| estimate_jaccard(left, right))
    }

    /// [CONFIG-CROSS-LANGUAGE] Uses discovery's alias signatures for the same endpoints.
    fn cross_language_jaccard(
        &self,
        pair: &ResolvedPair<'_>,
        trees: &HashMap<FileId, &NormalizedNode>,
    ) -> f64 {
        let signature = |endpoint: ResolvedEndpoint<'_>| {
            super::super::signatures::cross_language_signature(
                endpoint.fingerprint,
                trees,
                self.file_languages
                    .get(&endpoint.fingerprint.file_id)
                    .copied(),
            )
        };
        estimate_jaccard(&signature(pair.left), &signature(pair.right))
    }

    /// Measures cosine for the two exact source slices when embeddings are active.
    fn embedding_cos(
        &self,
        pair: &ResolvedPair<'_>,
        provider: Option<&dyn EmbeddingProvider>,
    ) -> Result<f64, CoreError> {
        let Some(provider) = provider else {
            return Ok(0.0);
        };
        let snippets = pair.snippets(&self.sources);
        if snippets
            .iter()
            .any(|snippet| snippet.chars().count() > provider.max_input_chars())
        {
            return Ok(0.0);
        }
        let vectors = provider
            .embed_batch(&snippets)
            .map_err(|error| CoreError::Embedding {
                message: error.to_string(),
            })?;
        valid_cosine(provider, &vectors)
    }

    /// Parses each distinct endpoint file once for overlap and content evidence.
    fn trees_for_pair(&self, pair: &ResolvedPair<'_>) -> Result<Vec<NormalizedNode>, CoreError> {
        let mut file_ids = vec![pair.left.fingerprint.file_id];
        if pair.right.fingerprint.file_id != pair.left.fingerprint.file_id {
            file_ids.push(pair.right.fingerprint.file_id);
        }
        file_ids
            .into_iter()
            .map(|file_id| self.parse_tree(file_id))
            .collect()
    }

    /// Re-parses one held source using its registered language parser.
    fn parse_tree(&self, file_id: crate::state::FileId) -> Result<NormalizedNode, CoreError> {
        let source = self
            .sources
            .get(&file_id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let language = self
            .file_languages
            .get(&file_id)
            .copied()
            .unwrap_or("unknown");
        let parser = self.parsers.iter().find(|parser| parser.id() == language);
        parser
            .ok_or(CoreError::ParseFailed { language })?
            .parse_and_normalize(source, file_id)
    }

    /// Converts measured axes into the public admission response.
    fn build_evidence(&self, pair: &ResolvedPair<'_>, measured: Measurements) -> PairEvidence {
        let facts = AdmissionFacts::from(self, pair, measured);
        let classification = facts.classification(measured);
        PairEvidence {
            structural: measured.score.structural,
            token_jaccard: measured.score.token_jaccard,
            embedding_cos: measured.score.embedding_cos,
            agreement: measured.content.agreement,
            content_measurement: if measured.content.measured {
                ContentMeasurement::Measured
            } else {
                ContentMeasurement::Unmeasured
            },
            rename_consistency: measured.content.rename_consistency,
            literal_fraction: measured.content.literal_fraction,
            text_identity: measured.text.raw,
            fused_score: measured.score.bounded_fused(),
            content_required: facts.content_required,
            content_ok: facts.content_ok,
            admitted: facts.admitted,
            classification,
            explanation: facts.explanation(classification),
        }
    }
}

/// The corpus-backed measurers one pair measurement reads: the memoising
/// structural overlap measurer and the per-file tree index the content
/// axes walk. Built once per explicit comparison over its two trees, and
/// once per render over the whole population for the cluster fold.
struct PairAxes<'corpus> {
    /// Structural overlap, memoised per structural pair.
    overlap: OverlapMeasurer<'corpus>,
    /// Exact-fingerprint frontiers reused across pair comparisons.
    content: ContentMeasurer,
    /// `FileId → normalised root` for the content axes.
    trees: HashMap<FileId, &'corpus NormalizedNode>,
    /// Original candidate anchors preserve rescue scope and container checks.
    rescue: RescueContext<'corpus, RandomState, RandomState>,
}

impl<'corpus> PairAxes<'corpus> {
    /// Indexes `trees` for every measurement that follows.
    fn new(
        trees: &'corpus [NormalizedNode],
        session: &'corpus PipelineSession,
        pairs: &[CandidatePair],
    ) -> Self {
        Self {
            overlap: OverlapMeasurer::new(trees),
            content: ContentMeasurer::default(),
            trees: tree_index_of(trees),
            rescue: RescueContext::new(
                pairs,
                session.store.fingerprints(),
                trees,
                &session.sources,
                &session.file_languages,
                usize::try_from(session.min_nodes).unwrap_or(usize::MAX),
            ),
        }
    }
}

/// Pair axes and raw-content populations before admission gates.
#[derive(Clone, Copy)]
struct Measurements {
    /// Three bounded shape/semantic axes.
    score: PairScore,
    /// Keep the complete measured content, including rename proof and missing evidence.
    content: crate::content::ContentEvidence,
    /// [FUSED-SHARED-SUBTREE-CORE] Whether the aligned core is a copy,
    /// including the bounded Async-suffix call-edit case.
    core: CoreCopy,
    /// Scope and container checks measured by the original rescue implementation.
    rescue_scope: bool,
    /// Exact Merkle identity.
    merkle_equal: bool,
    /// How far the two raw source ranges are the same text.
    text: SourceIdentity,
}

/// A copied core may additionally prove a bounded async call-selector edit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CoreCopy {
    /// No copied aligned core was measured.
    Absent,
    /// The core is a copy without an async selector edit.
    Copied,
    /// The copied core contains a measured async selector edit.
    AsyncEdited,
}

impl CoreCopy {
    /// Builds only valid states from the shared core verdict.
    fn from(copy: bool, contradiction: ContentContradiction) -> Self {
        match (copy, contradiction) {
            (false, _) => Self::Absent,
            (true, ContentContradiction::CallTargetAsyncEdit) => Self::AsyncEdited,
            (true, _) => Self::Copied,
        }
    }

    /// Whether the aligned core passed the shared content rule.
    fn is_copy(self) -> bool {
        self != Self::Absent
    }

    /// Whether that passing core also proved the bounded async edit.
    fn has_async_edit(self) -> bool {
        self == Self::AsyncEdited
    }
}

/// Validates two provider vectors and measures their canonical cosine.
fn valid_cosine(provider: &dyn EmbeddingProvider, vectors: &[Vec<f32>]) -> Result<f64, CoreError> {
    let dimensions = provider.spec().dimensions;
    let [left, right] = vectors else {
        return Err(CoreError::Embedding {
            message: "pair comparison provider returned invalid vectors".to_owned(),
        });
    };
    let valid = [left, right].into_iter().all(|vector| {
        vector.len() == dimensions && vector.iter().all(|component| component.is_finite())
    });
    if valid {
        return Ok(cosine_similarity(left, right));
    }
    Err(CoreError::Embedding {
        message: "pair comparison provider returned invalid vectors".to_owned(),
    })
}
