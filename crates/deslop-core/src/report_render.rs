//! Conversion from pipeline clusters to the mass-only report contract.

use std::{
    collections::HashMap,
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use crate::{
    ast::ByteRange,
    cluster::{duplicate_mass, Cluster},
    config::ExclusionConfig,
    fingerprint::Fingerprint,
    report::{ReportCluster, ReportOccurrence},
    state::{FileId, FileRegistry},
};

/// Per-file byte offsets of newline characters for line lookup.
#[derive(Debug)]
pub struct LineIndex {
    /// Source length used to clamp requested offsets.
    source_len: usize,
    /// Byte offsets of every newline in ascending order.
    newline_offsets: Vec<usize>,
}

/// Line indexes keyed by source file identity.
pub type LineIndices = HashMap<FileId, LineIndex>;

/// Source bytes and indexes built from those exact bytes.
pub(crate) struct ReportSources<'a> {
    /// Source bytes keyed by file id.
    sources: &'a HashMap<FileId, Vec<u8>>,
    /// Line index built from the same bytes.
    line_indices: LineIndices,
}

/// One source and its matching line index.
#[derive(Clone, Copy)]
struct ReportSource<'a> {
    /// Source bytes for one file.
    bytes: &'a [u8],
    /// Line index corresponding to `bytes`.
    line_index: &'a LineIndex,
}

impl<'a> ReportSources<'a> {
    /// Indexes every source once for the whole render.
    pub(crate) fn new(sources: &'a HashMap<FileId, Vec<u8>>) -> Self {
        let line_indices = sources
            .iter()
            .map(|(file_id, source)| (*file_id, LineIndex::new(source)))
            .collect();
        Self {
            sources,
            line_indices,
        }
    }

    /// Returns the indexes used by the metrics pass.
    pub(crate) const fn line_indices(&self) -> &LineIndices {
        &self.line_indices
    }

    /// Returns one source with its corresponding line index.
    fn source(&self, file_id: FileId) -> Option<ReportSource<'_>> {
        Some(ReportSource {
            bytes: self.sources.get(&file_id)?.as_slice(),
            line_index: self.line_indices.get(&file_id)?,
        })
    }
}

impl LineIndex {
    /// Builds one index with a single pass over the source bytes.
    fn new(source: &[u8]) -> Self {
        let newline_offsets = source
            .iter()
            .enumerate()
            .filter_map(|(offset, byte)| (*byte == b'\n').then_some(offset))
            .collect();
        Self {
            source_len: source.len(),
            newline_offsets,
        }
    }

    /// Returns the one-indexed line containing `offset`.
    pub(crate) fn line_for_offset(&self, offset: usize) -> usize {
        let safe_offset = offset.min(self.source_len);
        self.newline_offsets
            .partition_point(|newline| *newline < safe_offset)
            .saturating_add(1)
    }

    /// Returns the indexed source byte length.
    pub(crate) const fn source_len(&self) -> usize {
        self.source_len
    }
}

/// Converts one internal component to a mass-only report component.
pub(crate) fn cluster_to_report<S: BuildHasher>(
    cluster: &Cluster,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
    sources: &ReportSources<'_>,
) -> ReportCluster {
    let canonical_node_count = cluster
        .members
        .iter()
        .map(|member| member.node_count)
        .min()
        .unwrap_or_default();
    let occurrences: Vec<ReportOccurrence> = cluster
        .members
        .iter()
        .map(|member| {
            occurrence(
                member,
                registry,
                file_languages,
                scan_root,
                exclusion,
                sources,
            )
        })
        .collect();
    let occurrences_total = occurrences.len();
    let occurrence_count = occurrences
        .iter()
        .filter(|occurrence| !occurrence.hidden)
        .count();
    ReportCluster {
        id: cluster.id.clone(),
        rank: 0,
        rank_band: String::new(),
        mass: duplicate_mass(canonical_node_count, occurrence_count),
        canonical_node_count,
        occurrences,
        occurrences_total,
        occurrence_count,
        occurrences_truncated: false,
        intersects_diff: None,
        is_newly_introduced: None,
    }
}

/// Builds one exact report occurrence.
fn occurrence<S: BuildHasher>(
    member: &Fingerprint,
    registry: &FileRegistry,
    file_languages: &HashMap<FileId, &'static str, S>,
    scan_root: &Path,
    exclusion: &ExclusionConfig,
    sources: &ReportSources<'_>,
) -> ReportOccurrence {
    let file_id = member.file_id;
    let absolute = registry.path(file_id).map(Path::to_path_buf);
    let language = file_languages.get(&file_id).copied().unwrap_or("");
    let source = sources.source(file_id);
    let hidden = absolute
        .as_deref()
        .is_some_and(|path| exclusion.is_report_hidden(path, language))
        || source.is_some_and(|item| crate::config::has_generated_header(item.bytes));
    let (start_line, end_line) = source.map_or((0, 0), |item| {
        byte_range_to_line_range(item.line_index, member.byte_range)
    });
    let path = absolute.map_or_else(PathBuf::new, |path| relative_to_scan_root(&path, scan_root));
    ReportOccurrence {
        path,
        start_byte: member.byte_range.start,
        end_byte: member.byte_range.end,
        start_line,
        end_line,
        hidden,
        in_diff: None,
    }
}

/// Renders `absolute` relative to `scan_root` when it lies inside, and
/// spells the result for publication ([OUTPUT-SCHEMA-PATH-SEPARATOR]).
/// Every path the report carries — occurrence, per-file metric,
/// boilerplate hint — is built here, so the spelling is decided once.
pub(crate) fn relative_to_scan_root(absolute: &Path, scan_root: &Path) -> PathBuf {
    crate::paths::reported(absolute.strip_prefix(scan_root).unwrap_or(absolute))
}

/// Resolves the report path for `file_id`.
pub(crate) fn display_path(file_id: FileId, registry: &FileRegistry, scan_root: &Path) -> PathBuf {
    registry
        .path(file_id)
        .map_or_else(PathBuf::new, |path| relative_to_scan_root(path, scan_root))
}

/// Converts a byte range into an inclusive one-indexed line range.
fn byte_range_to_line_range(index: &LineIndex, range: ByteRange) -> (i64, i64) {
    let start = index.line_for_offset(range.start);
    let end = index.line_for_offset(range.end.saturating_sub(1));
    (
        i64::try_from(start).unwrap_or(i64::MAX),
        i64::try_from(end).unwrap_or(i64::MAX),
    )
}

/// Collapses ASCII whitespace runs for exact-copy comparisons.
pub(crate) fn canonicalise_whitespace(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut last_was_space = true;
    for &byte in bytes {
        if byte.is_ascii_whitespace() {
            if !last_was_space {
                out.push(b' ');
            }
            last_was_space = true;
        } else {
            out.push(byte);
            last_was_space = false;
        }
    }
    if out.last() == Some(&b' ') {
        let _ = out.pop();
    }
    out
}
