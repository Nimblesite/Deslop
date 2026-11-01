//! [LIVE-STATE] Read-only report queries and `find_similar` on [`AnalysisSession`].

use std::{collections::HashSet, path::Path};

use super::{
    super::{
        cluster_lookup::resolve_cluster_by_id_prefix,
        errors::LiveError,
        session_helpers::{
            cluster_overlaps_range, cluster_touches_path, collapse_overlapping_clusters_for_range,
            earliest_byte_for_path, not_listed_by, parse_and_hash_snippet, truncate,
        },
        wire::{FileReport, FindSimilarInput, FindSimilarRequest, FindSimilarResult},
    },
    AnalysisSession,
};
use crate::{
    lang::LanguageParser,
    report::{Report, ReportCluster},
};

impl AnalysisSession {
    /// Resolves a `find_similar` request.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnsupportedLanguage`] for unknown languages
    /// and [`LiveError::UnparseableInput`] / [`LiveError::Core`] for
    /// parse failures.
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

    /// Returns clusters whose occurrences overlap `path`. Thin wrapper
    /// over [`report_for_file_in`] so the live read and the LSP backend's
    /// lock-free snapshot read share one implementation.
    #[must_use]
    pub fn report_for_file(&self, path: &Path) -> FileReport {
        report_for_file_in(&self.latest_report, path)
    }

    /// Returns clusters overlapping a byte range in `path`. Thin wrapper
    /// over [`report_for_range_in`].
    #[must_use]
    pub fn report_for_range(
        &self,
        path: &Path,
        start_byte: usize,
        end_byte: usize,
    ) -> Vec<ReportCluster> {
        report_for_range_in(&self.latest_report, path, start_byte, end_byte)
    }

    /// Looks up a cluster by its stable id.
    ///
    /// Accepts either the full 16-hex canonical id or any prefix of at
    /// least 7 hex characters — the same slug the VSIX shows in hover
    /// bubbles and tree rows via `clusterSlug()`. Falls back to the
    /// shared [`resolve_cluster_by_id_prefix`] helper so the truncated-id
    /// behaviour stays consistent across the live API and the MCP wire.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::UnknownCluster`] when no cluster matches, the
    /// prefix is shorter than the slug floor, or the prefix is ambiguous.
    pub fn cluster_by_id(&self, id: &str) -> Result<ReportCluster, LiveError> {
        resolve_cluster_by_id_prefix(&self.latest_report.clusters, id).cloned()
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
        let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
        Ok(FindSimilarResult {
            clusters,
            existing: Vec::new(),
            below_min_nodes: false,
            total_occurrences,
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
        let snippet_hashes = parse_and_hash_snippet(parser, snippet, self.min_nodes)?;
        if snippet_hashes.is_empty() {
            return Ok(FindSimilarResult {
                clusters: Vec::new(),
                existing: Vec::new(),
                below_min_nodes: true,
                total_occurrences: 0,
            });
        }
        let mut clusters = self.clusters_at_subtree_hashes(&snippet_hashes);
        truncate(&mut clusters, max_results);
        let existing = self.pipeline.as_ref().map_or_else(Vec::new, |pipeline| {
            not_listed_by(pipeline.existing_occurrences(&snippet_hashes), &clusters)
        });
        let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
        Ok(FindSimilarResult {
            clusters,
            existing,
            below_min_nodes: false,
            total_occurrences,
        })
    }

    /// Returns the report clusters whose occurrences carry any of
    /// `snippet_hashes` — the content join behind snippet `find_similar`.
    ///
    /// A snippet has no workspace identity, so it can only be located
    /// by content: each hash resolves to the live fingerprints that
    /// carry it, their ranges are joined to the report through
    /// [`report_for_range_in`] (the same lookup the open-range variant
    /// uses), and the surviving clusters keep report order with no
    /// duplicates. The public cluster id cannot take part: since gh
    /// #430 it mixes the members' paths, so it is not derivable from a
    /// pathless snippet's digests at all.
    fn clusters_at_subtree_hashes(&self, snippet_hashes: &[[u8; 32]]) -> Vec<ReportCluster> {
        let Some(pipeline) = self.pipeline.as_ref() else {
            return Vec::new();
        };
        let matched_ids: HashSet<String> = pipeline
            .subtree_occurrences(snippet_hashes)
            .iter()
            .flat_map(|(path, range)| {
                report_for_range_in(&self.latest_report, path, range.start, range.end)
            })
            .map(|cluster| cluster.id)
            .collect();
        self.latest_report
            .clusters
            .iter()
            .filter(|cluster| matched_ids.contains(&cluster.id))
            .cloned()
            .collect()
    }

    /// Returns the registered parser for `language`. Returns
    /// [`LiveError::AnalysisNotReady`] in the cache-seed window.
    fn parser_for_language(&self, language: &str) -> Result<&dyn LanguageParser, LiveError> {
        let pipeline = self.pipeline.as_ref().ok_or(LiveError::AnalysisNotReady)?;
        let parsers = pipeline.parsers();
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
}

/// Builds a [`FileReport`] for `path` from an arbitrary report snapshot.
/// Shared by [`AnalysisSession::report_for_file`] and the LSP backend's
/// lock-free editor reads so both surfaces filter and sort identically
/// ([LSP-NON-INTERFERENCE]).
#[must_use]
pub fn report_for_file_in(report: &Report, path: &Path) -> FileReport {
    let mut clusters: Vec<ReportCluster> = report
        .clusters
        .iter()
        .filter(|cluster| cluster_touches_path(cluster, path))
        .cloned()
        .collect();
    clusters.sort_by_key(|cluster| earliest_byte_for_path(cluster, path));
    let total_occurrences: usize = clusters.iter().map(crate::report::occurrence_count).sum();
    FileReport {
        path: path.to_path_buf(),
        clusters,
        total_occurrences,
    }
}

/// Returns clusters overlapping a byte range in `path` from an arbitrary
/// report snapshot. Shared by [`AnalysisSession::report_for_range`] and
/// the LSP backend's lock-free editor reads.
#[must_use]
pub fn report_for_range_in(
    report: &Report,
    path: &Path,
    start_byte: usize,
    end_byte: usize,
) -> Vec<ReportCluster> {
    let clusters = report
        .clusters
        .iter()
        .filter(|cluster| cluster_overlaps_range(cluster, path, start_byte, end_byte))
        .cloned()
        .collect();
    collapse_overlapping_clusters_for_range(clusters, path, start_byte, end_byte)
}
