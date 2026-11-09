//! The two occurrences one pair measurement compares, resolved against the
//! flat corpus ([FUSED-PAIR-SIGNALS]).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::text_identity::SourceIdentity;
use crate::{
    ast::ByteRange,
    cluster::run_fingerprint,
    error::CoreError,
    fingerprint::Fingerprint,
    lsh::{Signature, SignatureLookup},
    pipeline::{embedding_batch::snippet_for, PipelineSession},
    report::{PairComparisonParams, PairEndpoint, PairTextIdentity},
    state::FileId,
};

impl PipelineSession {
    /// Resolves both endpoint identities against the current flat corpus,
    /// falling back to the `joined` runs [`Self::joined_runs`] built.
    pub(super) fn resolve_pair<'corpus>(
        &'corpus self,
        params: &PairComparisonParams,
        joined: &'corpus [Option<Fingerprint>; 2],
    ) -> Result<ResolvedPair<'corpus>, CoreError> {
        let [left_run, right_run] = joined;
        let left = self.resolve_endpoint(&params.left, left_run.as_ref())?;
        let right = self.resolve_endpoint(&params.right, right_run.as_ref())?;
        Ok(ResolvedPair { left, right })
    }

    /// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] For each endpoint the scan never
    /// fingerprinted, the copied run the cluster build would have joined
    /// there: a report renders a long run as one finding several windows
    /// wide, and every range a report renders must be comparable
    /// ([PAIR-COMPARE-CLI]). The file is parsed only on this fallback.
    pub(super) fn joined_runs(
        &self,
        params: &PairComparisonParams,
    ) -> Result<[Option<Fingerprint>; 2], CoreError> {
        Ok([
            self.joined_run(&params.left)?,
            self.joined_run(&params.right)?,
        ])
    }

    /// The joined run at one endpoint, or `None` when the scan already
    /// fingerprinted it or the range is not one authored run.
    fn joined_run(&self, endpoint: &PairEndpoint) -> Result<Option<Fingerprint>, CoreError> {
        let requested = canonical_endpoint_path(&self.root, &endpoint.path);
        if self.indexed_endpoint(endpoint, &requested).is_some() {
            return Ok(None);
        }
        let Some(file_id) = self.file_id_for(&requested) else {
            return Ok(None);
        };
        let tree = self.parse_tree(file_id)?;
        let range = ByteRange {
            start: endpoint.start_byte,
            end: endpoint.end_byte,
        };
        Ok(run_fingerprint(&tree, range))
    }

    /// Resolves one exact path/range to its fingerprint and signature
    /// index, or to the joined run built for it.
    fn resolve_endpoint<'corpus>(
        &'corpus self,
        endpoint: &PairEndpoint,
        joined: Option<&'corpus Fingerprint>,
    ) -> Result<ResolvedEndpoint<'corpus>, CoreError> {
        let requested = canonical_endpoint_path(&self.root, &endpoint.path);
        self.indexed_endpoint(endpoint, &requested)
            .or_else(|| joined.map(ResolvedEndpoint::unindexed))
            .ok_or_else(|| unknown_endpoint(endpoint))
    }

    /// The fingerprint the scan emitted at exactly this path and range.
    fn indexed_endpoint(
        &self,
        endpoint: &PairEndpoint,
        requested: &Path,
    ) -> Option<ResolvedEndpoint<'_>> {
        self.store
            .fingerprints()
            .iter()
            .enumerate()
            .find(|(_, fingerprint)| self.endpoint_matches(fingerprint, endpoint, requested))
            .map(|(index, fingerprint)| ResolvedEndpoint {
                index: Some(index),
                fingerprint,
            })
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
}

/// One resolved endpoint and its positional signature index.
#[derive(Clone, Copy)]
pub(super) struct ResolvedEndpoint<'corpus> {
    /// Flat-corpus index, the key to this endpoint's signature and
    /// embedding cosine; `None` for a span the corpus never fingerprinted,
    /// which the cluster build measures when it joins a copied run
    /// ([PIPELINE-CLUSTER-SUBSUME-STRADDLE]).
    pub(super) index: Option<usize>,
    /// Exact fingerprint occurrence.
    pub(super) fingerprint: &'corpus Fingerprint,
}

impl<'corpus> ResolvedEndpoint<'corpus> {
    /// A span the corpus never fingerprinted: no signature, no cosine.
    pub(super) fn unindexed(fingerprint: &'corpus Fingerprint) -> Self {
        Self {
            index: None,
            fingerprint,
        }
    }

    /// Signature aligned with this endpoint's flat-corpus index.
    pub(super) fn signature(self, signatures: &dyn SignatureLookup) -> Option<&Signature> {
        signatures.signature(self.index?)
    }
}

/// Two exact endpoint occurrences.
pub(super) struct ResolvedPair<'corpus> {
    /// Caller-selected left endpoint.
    pub(super) left: ResolvedEndpoint<'corpus>,
    /// Caller-selected right endpoint.
    pub(super) right: ResolvedEndpoint<'corpus>,
}

impl ResolvedPair<'_> {
    /// Source snippets for the pair, preserving request order.
    pub(super) fn snippets(&self, sources: &HashMap<FileId, Vec<u8>>) -> Vec<String> {
        [self.left.fingerprint, self.right.fingerprint]
            .into_iter()
            .map(|fingerprint| snippet_for(fingerprint, sources))
            .collect()
    }

    /// Whether the pair spans two source files.
    pub(super) fn cross_file(&self) -> bool {
        self.left.fingerprint.file_id != self.right.fingerprint.file_id
    }

    /// Whether endpoint languages require discovery's cross-language token space.
    pub(super) fn cross_language(&self, languages: &HashMap<FileId, &'static str>) -> bool {
        languages.get(&self.left.fingerprint.file_id)
            != languages.get(&self.right.fingerprint.file_id)
    }

    /// How far the two raw endpoint snippets are the same text, read once
    /// from the same bytes so the byte answer and the indentation answer
    /// cannot disagree ([FUSED-PAIR-SIGNALS]).
    pub(super) fn text_identity(&self, sources: &HashMap<FileId, Vec<u8>>) -> SourceIdentity {
        let snippets = self.snippets(sources);
        let [left, right] = snippets.as_slice() else {
            return SourceIdentity {
                raw: PairTextIdentity::Different,
                identical: false,
            };
        };
        SourceIdentity::measure(left, right)
    }
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
