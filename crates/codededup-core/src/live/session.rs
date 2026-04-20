//! Live analysis session ([LIVE-STATE]).
//!
//! Owns one [`PipelineSession`], the latest rendered [`Report`]
//! snapshot, the monotonic generation counter, and the active embedding
//! provider. The session is the single live struct in the live module
//! — nothing else holds mutable analysis state.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    delta::ReportDelta,
    embedding::{EmbeddingMode, EmbeddingProvider, OllamaModelInfo},
    fingerprint::collect_fingerprints,
    lang::LanguageParser,
    pipeline::{EmbeddingSettings, PipelineSession},
    report::{EmbeddingProvenance, Report, ReportCluster},
    sibling::collect_sibling_fingerprints,
    state::FileRegistry,
};

use super::{
    errors::LiveError,
    wire::{
        EmbeddingModelInfo, FileReport, FindSimilarInput, FindSimilarRequest, FindSimilarResult,
        SessionConfig,
    },
};

/// Live analysis session. Wraps [`PipelineSession`] with the live-only
/// metadata documented in [LIVE-STATE].
#[derive(Debug)]
pub struct AnalysisSession {
    /// Underlying analysis state.
    pipeline: PipelineSession,
    /// Atomic snapshot of the current report.
    latest_report: Arc<Report>,
    /// Monotonic generation counter.
    generation: u64,
    /// Active embedding provider.
    embedding_provider: Arc<dyn EmbeddingProvider>,
    /// Currently-active embedding mode.
    embedding_mode: EmbeddingMode,
    /// Whether the session was created with the incremental cache on.
    incremental: bool,
    /// Optional explicit exclusion config path supplied at
    /// construction.
    config_path: Option<PathBuf>,
}

impl AnalysisSession {
    /// Constructs a new session by running the first full analysis
    /// against `root`.
    ///
    /// # Errors
    ///
    /// Propagates any [`LiveError::Core`] surfaced by the underlying
    /// pipeline initialisation.
    pub fn new(
        root: PathBuf,
        min_nodes: u32,
        incremental: bool,
        config_path: Option<PathBuf>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Result<Self, LiveError> {
        let mode = EmbeddingMode::Auto;
        let (pipeline, report) = initialise_pipeline(
            root,
            min_nodes,
            incremental,
            config_path.clone(),
            mode,
            embedding_provider.as_ref(),
        )?;
        Ok(Self {
            pipeline,
            latest_report: Arc::new(report),
            generation: 1,
            embedding_provider,
            embedding_mode: mode,
            incremental,
            config_path,
        })
    }

    /// Returns an `Arc` to the current report snapshot.
    #[must_use]
    pub fn report(&self) -> Arc<Report> {
        Arc::clone(&self.latest_report)
    }

    /// Returns the current generation counter.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the workspace root pinned at construction.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.pipeline.root()
    }

    /// Re-analyses `changed`, swaps the report, bumps the generation,
    /// and returns a [`ReportDelta`] versus the previous snapshot.
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`].
    pub fn apply_changes(&mut self, changed: &[PathBuf]) -> Result<ReportDelta, LiveError> {
        let previous = Arc::clone(&self.latest_report);
        let prev_generation = self.generation;
        let next = self.run_pipeline(changed)?;
        self.generation = self.generation.saturating_add(1);
        let next_arc = Arc::new(next);
        self.latest_report = Arc::clone(&next_arc);
        Ok(ReportDelta::between(
            Some((prev_generation, &previous)),
            self.generation,
            &next_arc,
        ))
    }

    /// Resolves a `find_similar` request against the live corpus.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnsupportedLanguage`] for snippet inputs
    /// whose language has no registered parser, and
    /// [`LiveError::UnparseableInput`] / [`LiveError::Core`] when
    /// parsing fails.
    pub fn find_similar(
        &self,
        request: &FindSimilarRequest,
    ) -> Result<FindSimilarResult, LiveError> {
        match &request.input {
            FindSimilarInput::OpenRange {
                path,
                start_byte,
                end_byte,
            } => self.find_similar_for_range(path, *start_byte, *end_byte, request.max_results),
            FindSimilarInput::Snippet { snippet, language } => self.find_similar_for_snippet(
                snippet.as_str(),
                language.as_str(),
                request.max_results,
            ),
        }
    }

    /// Returns the resolved session configuration.
    #[must_use]
    pub fn session_config(&self) -> SessionConfig {
        SessionConfig {
            workspace_root: self.pipeline.root().to_path_buf(),
            min_nodes: self.pipeline.min_nodes(),
            languages: self.parser_ids(),
            embedding_provenance: self.latest_report.embedding_provenance.clone(),
            exclusion_config_path: self.config_path.clone(),
            cache_root: self
                .pipeline
                .root()
                .join(crate::embedding::cache::DEFAULT_CACHE_DIR_NAME),
            incremental: self.incremental,
        }
    }

    /// Returns clusters whose occurrences overlap `path`.
    #[must_use]
    pub fn report_for_file(&self, path: &Path) -> FileReport {
        let mut clusters: Vec<ReportCluster> = self
            .latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_touches_path(cluster, path))
            .cloned()
            .collect();
        clusters.sort_by_key(|cluster| earliest_byte_for_path(cluster, path));
        FileReport {
            path: path.to_path_buf(),
            clusters,
        }
    }

    /// Returns clusters overlapping a byte range in `path`.
    #[must_use]
    pub fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        self.latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_overlaps_range(cluster, path, start_byte, end_byte))
            .cloned()
            .collect()
    }

    /// Looks up a cluster by its stable id.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches.
    pub fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        self.latest_report
            .clusters
            .iter()
            .find(|cluster| cluster.id == id)
            .cloned()
            .ok_or_else(|| LiveError::UnknownCluster { id: id.to_owned() })
    }

    /// Swaps the embedding provider and re-runs the pipeline.
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`].
    ///
    /// TRADEOFF: re-runs `update_files` over every currently-live path
    /// because [`PipelineSession`] has no narrower "refresh embeddings
    /// only" entry point. The warm fingerprint cache short-circuits
    /// parsing for unchanged content so the cost is dominated by the
    /// embedding pass itself.
    pub fn set_embedding_model(
        &mut self,
        provider: Arc<dyn EmbeddingProvider>,
    ) -> Result<Option<EmbeddingProvenance>, LiveError> {
        self.embedding_provider = provider;
        let live_paths: Vec<PathBuf> = self.live_paths_snapshot();
        let report = self.run_pipeline(&live_paths)?;
        self.generation = self.generation.saturating_add(1);
        let provenance = report.embedding_provenance.clone();
        self.latest_report = Arc::new(report);
        Ok(provenance)
    }

    /// Lists embedding models available to the session — built-in
    /// stub plus any Ollama models reachable at `endpoint`. Falls back
    /// to stub-only when Ollama is unreachable ([LIVE-QUERY-API]).
    #[must_use]
    pub fn list_embedding_models(endpoint: &str) -> Vec<EmbeddingModelInfo> {
        let mut out = vec![stub_model_info()];
        match crate::embedding::list_ollama_models(endpoint) {
            Ok(models) => append_ollama_models(&mut out, models),
            Err(error) => {
                tracing::info!(%error, "ollama unreachable; returning stub-only model list");
            }
        }
        out
    }

    /// Snapshot of the absolute paths currently part of the corpus.
    fn live_paths_snapshot(&self) -> Vec<PathBuf> {
        let registry: &FileRegistry = self.pipeline.registry();
        self.pipeline
            .file_languages()
            .keys()
            .filter_map(|file_id| registry.path(*file_id).map(Path::to_path_buf))
            .collect()
    }

    /// Stable list of registered parser ids.
    fn parser_ids(&self) -> Vec<String> {
        self.pipeline
            .parsers()
            .iter()
            .map(|parser| parser.id().to_owned())
            .collect()
    }

    /// Runs the underlying pipeline with the active provider/mode
    /// settings.
    fn run_pipeline(&mut self, changed: &[PathBuf]) -> Result<Report, LiveError> {
        let embedding = EmbeddingSettings {
            mode: self.embedding_mode,
            provider: Some(self.embedding_provider.as_ref()),
        };
        Ok(self.pipeline.update_files(changed, embedding)?)
    }

    /// Resolves the open-buffer-range variant of `find_similar`.
    fn find_similar_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
        max_results: Option<usize>,
    ) -> Result<FindSimilarResult, LiveError> {
        self.guard_path(path)?;
        let mut clusters = self.report_for_range(path, start_byte, end_byte);
        truncate(&mut clusters, max_results);
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
        })
    }

    /// Resolves the snippet variant of `find_similar`.
    fn find_similar_for_snippet(
        &self,
        snippet: &str,
        language: &str,
        max_results: Option<usize>,
    ) -> Result<FindSimilarResult, LiveError> {
        let parser = self.parser_for_language(language)?;
        let snippet_hashes = parse_and_hash_snippet(parser, snippet, self.pipeline.min_nodes())?;
        if snippet_hashes.is_empty() {
            return Ok(FindSimilarResult {
                clusters: Vec::new(),
                below_min_nodes: true,
            });
        }
        let mut clusters = self.clusters_matching_hashes(&snippet_hashes);
        truncate(&mut clusters, max_results);
        Ok(FindSimilarResult {
            clusters,
            below_min_nodes: false,
        })
    }

    /// Returns the cluster snapshots whose stable id matches one of
    /// the supplied snippet hashes.
    fn clusters_matching_hashes(&self, snippet_hashes: &[[u8; 32]]) -> Vec<ReportCluster> {
        self.latest_report
            .clusters
            .iter()
            .filter(|cluster| cluster_matches_any_hash(cluster, snippet_hashes))
            .cloned()
            .collect()
    }

    /// Returns the registered parser for `language` or an error.
    fn parser_for_language(&self, language: &str) -> Result<&dyn LanguageParser, LiveError> {
        let parsers = self.pipeline.parsers();
        if let Some(found) = parsers.iter().find(|parser| parser.id() == language) {
            return Ok(found.as_ref());
        }
        Err(LiveError::UnsupportedLanguage {
            requested: language.to_owned(),
            registered: parsers
                .iter()
                .map(|parser| parser.id().to_owned())
                .collect(),
        })
    }

    /// Asserts `path` is under the workspace root.
    fn guard_path(&self, path: &Path) -> Result<(), LiveError> {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.pipeline.root().join(path)
        };
        if resolved.starts_with(self.pipeline.root()) {
            Ok(())
        } else {
            Err(LiveError::PathOutsideWorkspace {
                path: resolved,
                workspace_root: self.pipeline.root().to_path_buf(),
            })
        }
    }
}

/// Calls [`PipelineSession::initialise`] with the supplied embedding
/// provider. Extracted so [`AnalysisSession::new`] stays under the
/// 20-line function budget.
fn initialise_pipeline(
    root: PathBuf,
    min_nodes: u32,
    incremental: bool,
    config_path: Option<PathBuf>,
    mode: EmbeddingMode,
    provider: &dyn EmbeddingProvider,
) -> Result<(PipelineSession, Report), LiveError> {
    let embedding = EmbeddingSettings {
        mode,
        provider: Some(provider),
    };
    Ok(PipelineSession::initialise(
        root,
        min_nodes,
        incremental,
        config_path,
        embedding,
    )?)
}

/// Truncates `clusters` to `max_results` when supplied.
fn truncate(clusters: &mut Vec<ReportCluster>, max_results: Option<usize>) {
    if let Some(cap) = max_results {
        clusters.truncate(cap);
    }
}

/// Returns `true` when at least one occurrence in `cluster` lives in
/// `path`.
fn cluster_touches_path(cluster: &ReportCluster, path: &Path) -> bool {
    cluster
        .occurrences
        .iter()
        .any(|occurrence| occurrence_path_matches(occurrence.path.as_path(), path))
}

/// Returns `true` when at least one occurrence overlaps the given
/// byte range in `path`.
fn cluster_overlaps_range(
    cluster: &ReportCluster,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    cluster.occurrences.iter().any(|occurrence| {
        occurrence_path_matches(occurrence.path.as_path(), path)
            && ranges_overlap(
                occurrence.start_byte,
                occurrence.end_byte,
                start_byte,
                end_byte,
            )
    })
}

/// Returns the smallest start byte across the occurrences of
/// `cluster` that live in `path`.
fn earliest_byte_for_path(cluster: &ReportCluster, path: &Path) -> usize {
    cluster
        .occurrences
        .iter()
        .filter(|occurrence| occurrence_path_matches(occurrence.path.as_path(), path))
        .map(|occurrence| occurrence.start_byte)
        .min()
        .unwrap_or(usize::MAX)
}

/// Compares an occurrence path against a query path, accepting either
/// a full match or a suffix match (so workspace-relative queries find
/// absolute occurrence paths).
fn occurrence_path_matches(occurrence: &Path, query: &Path) -> bool {
    occurrence == query || occurrence.ends_with(query) || query.ends_with(occurrence)
}

/// Inclusive/exclusive byte-range overlap test.
fn ranges_overlap(
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    left_start < right_end && right_start < left_end
}

/// Parses `snippet` and returns the structural hashes for every
/// subtree that meets `min_nodes`.
fn parse_and_hash_snippet(
    parser: &dyn LanguageParser,
    snippet: &str,
    min_nodes: u32,
) -> Result<Vec<[u8; 32]>, LiveError> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("__snippet__"));
    let normalised = parser
        .parse_and_normalize(snippet.as_bytes(), file_id)
        .map_err(|source| LiveError::UnparseableInput {
            path: None,
            start_byte: 0,
            end_byte: snippet.len(),
            message: source.to_string(),
        })?;
    let min_usize = usize::try_from(min_nodes).unwrap_or(usize::MAX);
    let mut hashes: Vec<[u8; 32]> = collect_fingerprints(&normalised, min_usize)
        .into_iter()
        .map(|fingerprint| fingerprint.hash)
        .collect();
    hashes.extend(
        collect_sibling_fingerprints(&normalised, min_usize)
            .into_iter()
            .map(|fingerprint| fingerprint.hash),
    );
    Ok(hashes)
}

/// Returns `true` when `cluster.id` matches any of the provided
/// snippet hashes when projected through the same short-id encoding
/// used by [`crate::cluster`].
fn cluster_matches_any_hash(cluster: &ReportCluster, snippet_hashes: &[[u8; 32]]) -> bool {
    snippet_hashes
        .iter()
        .any(|hash| short_id_for(*hash) == cluster.id)
}

/// Mirrors `cluster::encode_short_id` — first 8 bytes lowercase hex.
fn short_id_for(hash: [u8; 32]) -> String {
    let mut out = String::with_capacity(16);
    for byte in hash.iter().take(8) {
        let high = (*byte >> 4) & 0x0F;
        let low = *byte & 0x0F;
        out.push(hex_nibble(high));
        out.push(hex_nibble(low));
    }
    out
}

/// Lower-nibble lookup mirroring `cluster::hex_nibble`.
const fn hex_nibble(nibble: u8) -> char {
    match nibble {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'a',
        11 => 'b',
        12 => 'c',
        13 => 'd',
        14 => 'e',
        _ => 'f',
    }
}

/// Returns the hard-coded model info for the built-in stub provider.
fn stub_model_info() -> EmbeddingModelInfo {
    let dimensions = crate::embedding::StubProvider::new()
        .embed("")
        .map(|vector| vector.len())
        .ok();
    EmbeddingModelInfo {
        provider_id: crate::embedding::STUB_PROVIDER_ID.to_owned(),
        model_id: "blake3-stub".to_owned(),
        model_version: Some("v1".to_owned()),
        dimensions,
        recommended: false,
        reachable: true,
    }
}

/// Translates the Ollama tag list into [`EmbeddingModelInfo`] entries.
fn append_ollama_models(out: &mut Vec<EmbeddingModelInfo>, models: Vec<OllamaModelInfo>) {
    for entry in models {
        out.push(EmbeddingModelInfo {
            provider_id: crate::embedding::DEFAULT_PROVIDER_ID.to_owned(),
            model_id: entry.bare_id,
            model_version: Some(entry.digest),
            dimensions: None,
            recommended: entry.is_embedding_model,
            reachable: true,
        });
    }
}
