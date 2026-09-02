//! Explicit endpoint-to-endpoint evidence measurement ([FUSED-PAIR-SIGNALS]).

use std::path::{Path, PathBuf};

use crate::{
    ast::NormalizedNode,
    content::measure_pair_content,
    embedding::{cosine_similarity, EmbeddingProvider},
    error::CoreError,
    fingerprint::Fingerprint,
    lsh::{estimate_jaccard, SignatureLookup},
    overlap::OverlapMeasurer,
    pair::PairScore,
    report::{PairComparison, PairComparisonParams, PairEndpoint, PairEvidence},
};

use super::PipelineSession;

mod admission;
use admission::AdmissionFacts;

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

    /// Resolves both endpoint identities against the current flat corpus.
    fn resolve_pair<'corpus>(
        &'corpus self,
        params: &PairComparisonParams,
    ) -> Result<ResolvedPair<'corpus>, CoreError> {
        let left = self.resolve_endpoint(&params.left)?;
        let right = self.resolve_endpoint(&params.right)?;
        Ok(ResolvedPair { left, right })
    }

    /// Resolves one exact path/range to its fingerprint and signature index.
    fn resolve_endpoint(&self, endpoint: &PairEndpoint) -> Result<ResolvedEndpoint<'_>, CoreError> {
        let requested = canonical_endpoint_path(&self.root, &endpoint.path);
        self.store
            .fingerprints()
            .iter()
            .enumerate()
            .find(|(_, fingerprint)| self.endpoint_matches(fingerprint, endpoint, &requested))
            .map(|(index, fingerprint)| ResolvedEndpoint { index, fingerprint })
            .ok_or_else(|| unknown_endpoint(endpoint))
    }

    /// Tests exact file identity and byte range for one fingerprint.
    fn endpoint_matches(
        &self,
        fingerprint: &Fingerprint,
        endpoint: &PairEndpoint,
        requested: &Path,
    ) -> bool {
        fingerprint.byte_range.start == endpoint.start_byte
            && fingerprint.byte_range.end == endpoint.end_byte
            && self.registry.path(fingerprint.file_id) == Some(requested)
    }

    /// Measures every pair-owned axis and applies the admission algebra.
    fn measure_pair(
        &self,
        pair: &ResolvedPair<'_>,
        provider: Option<&dyn EmbeddingProvider>,
    ) -> Result<PairEvidence, CoreError> {
        let trees = self.trees_for_pair(pair)?;
        let measurements = self.measure_axes(pair, &trees, provider)?;
        Ok(self.build_evidence(pair, measurements))
    }

    /// Measures structural, token, embedding, and raw-content evidence.
    fn measure_axes(
        &self,
        pair: &ResolvedPair<'_>,
        trees: &[NormalizedNode],
        provider: Option<&dyn EmbeddingProvider>,
    ) -> Result<Measurements, CoreError> {
        let merkle_equal = pair.left.fingerprint.hash == pair.right.fingerprint.hash;
        let structural =
            OverlapMeasurer::new(trees).overlap(pair.left.fingerprint, pair.right.fingerprint);
        let token_jaccard = self.token_jaccard(pair, merkle_equal);
        let embedding_cos = self.embedding_cos(pair, provider)?;
        let content = measure_pair_content(
            pair.left.fingerprint,
            pair.right.fingerprint,
            trees,
            &self.sources,
            &self.file_languages,
        );
        Ok(Measurements {
            score: PairScore {
                structural,
                token_jaccard,
                embedding_cos,
            },
            agreement: content.agreement,
            rename_consistency: content.rename_consistency,
            literal_fraction: content.literal_fraction,
            merkle_equal,
            byte_identical: pair.byte_identical(&self.sources),
        })
    }

    /// Estimates token Jaccard, applying the pair-local Merkle correction.
    fn token_jaccard(&self, pair: &ResolvedPair<'_>, merkle_equal: bool) -> f64 {
        if merkle_equal {
            return 1.0;
        }
        let signatures = self.store.signatures();
        pair.left
            .signature(&signatures)
            .zip(pair.right.signature(&signatures))
            .map_or(0.0, |(left, right)| estimate_jaccard(left, right))
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
            agreement: measured.agreement,
            rename_consistency: measured.rename_consistency,
            literal_fraction: measured.literal_fraction,
            fused_score: measured.score.bounded_fused(),
            content_required: facts.content_required,
            content_ok: facts.content_ok,
            admitted: facts.admitted,
            classification,
            explanation: facts.explanation(classification),
        }
    }
}

/// One resolved endpoint and its positional signature index.
#[derive(Clone, Copy)]
struct ResolvedEndpoint<'corpus> {
    /// Flat-corpus index.
    index: usize,
    /// Exact fingerprint occurrence.
    fingerprint: &'corpus Fingerprint,
}

impl ResolvedEndpoint<'_> {
    /// Signature aligned with this endpoint's flat-corpus index.
    fn signature(self, signatures: &dyn SignatureLookup) -> Option<&crate::lsh::Signature> {
        signatures.signature(self.index)
    }
}

/// Two exact endpoint occurrences.
struct ResolvedPair<'corpus> {
    /// Caller-selected left endpoint.
    left: ResolvedEndpoint<'corpus>,
    /// Caller-selected right endpoint.
    right: ResolvedEndpoint<'corpus>,
}

impl ResolvedPair<'_> {
    /// Source snippets for the pair, preserving request order.
    fn snippets(
        &self,
        sources: &std::collections::HashMap<crate::state::FileId, Vec<u8>>,
    ) -> Vec<String> {
        [self.left.fingerprint, self.right.fingerprint]
            .into_iter()
            .map(|fingerprint| super::super::embedding_batch::snippet_for(fingerprint, sources))
            .collect()
    }

    /// Whether the pair spans two source files.
    fn cross_file(&self) -> bool {
        self.left.fingerprint.file_id != self.right.fingerprint.file_id
    }

    /// Whether the two raw endpoint snippets are byte-identical.
    fn byte_identical(
        &self,
        sources: &std::collections::HashMap<crate::state::FileId, Vec<u8>>,
    ) -> bool {
        let snippets = self.snippets(sources);
        snippets.first() == snippets.get(1)
    }
}

/// Pair axes and raw-content populations before admission gates.
#[derive(Clone, Copy)]
struct Measurements {
    /// Three bounded shape/semantic axes.
    score: PairScore,
    /// Raw authored-content agreement.
    agreement: f64,
    /// Consistent rename evidence.
    rename_consistency: f64,
    /// Literal share.
    literal_fraction: f64,
    /// Exact Merkle identity.
    merkle_equal: bool,
    /// Exact raw source-slice identity.
    byte_identical: bool,
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

/// Canonical absolute identity of a wire endpoint path.
fn canonical_endpoint_path(root: &Path, path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}

/// Constructs the structured unknown-endpoint error.
fn unknown_endpoint(endpoint: &PairEndpoint) -> CoreError {
    CoreError::UnknownPairEndpoint {
        path: endpoint.path.clone(),
        start_byte: endpoint.start_byte,
        end_byte: endpoint.end_byte,
    }
}
