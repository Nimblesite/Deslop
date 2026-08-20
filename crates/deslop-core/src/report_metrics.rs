//! Repo-wide duplication metrics and fail-over threshold
//! ([METRICS-REPO], [EXIT-CODES]).
//!
//! Computed deterministically from the same cluster set the rendered
//! [`crate::report::Report`] carries. Hidden occurrences
//! ([EXCLUSION-CONFIG] `report_hide`) are excluded from the numerator so
//! a noisy generated-code tier cannot inflate the metric.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    hash::BuildHasher,
    path::{Path, PathBuf},
};

use crate::{
    cluster::Cluster,
    config::ExclusionConfig,
    diff_scope::DiffScope,
    report_render::{relative_to_scan_root, LineIndex, LineIndices},
    state::{FileId, FileRegistry},
};

// `RepoMetrics`, `DiffMetrics`, `ThresholdSummary`, and
// `ThresholdSource` are generated from `docs/models/live-ipc.td` by
// `scripts/typediagram-gen.mjs`. The data shapes live in
// `crate::wire_generated`; the constructors and `Default` impl below
// stay here.
pub use crate::wire_generated::{
    DiffMetrics, FileMetric, RepoMetrics, ThresholdSource, ThresholdSummary,
};

impl RepoMetrics {
    /// Returns an empty metrics block (all counters zero, threshold
    /// source `"none"`). Used by `--from-report` when the input lacks
    /// the field and by render paths that never loaded a threshold.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            analysed_loc: 0,
            duplicated_loc: 0,
            duplication_percent: 0.0,
            clusters_total: 0,
            duplicated_files: 0,
            threshold: ThresholdSummary::none(),
            per_file: Vec::new(),
            folders: Vec::new(),
            diff: None,
        }
    }

    /// True when the configured threshold was exceeded.
    #[must_use]
    pub const fn breached(&self) -> bool {
        self.threshold.breached
    }
}

impl Default for RepoMetrics {
    fn default() -> Self {
        Self::empty()
    }
}

impl ThresholdSummary {
    /// Build the verdict from a resolved threshold and the measured
    /// duplication percentage.
    #[must_use]
    pub fn resolve(percent: f64, source: ThresholdSource, measured: f64) -> Self {
        let breached = !matches!(source, ThresholdSource::None) && measured > percent;
        Self {
            percent,
            breached,
            source,
        }
    }

    /// "No threshold" variant.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            percent: 0.0,
            breached: false,
            source: ThresholdSource::None,
        }
    }
}

/// Per-file analysed-line counts captured at file-read time on the
/// pipeline's only I/O pass. Keyed by [`FileId`] so updates from
/// [`crate::pipeline::PipelineSession`] can replace a single file
/// without re-scanning the corpus.
pub type AnalysedLines = HashMap<FileId, u64>;

/// Inputs to [`compute_repo_metrics`]. Bundled because the call site
/// already threads every one of them through the renderer; adding a
/// sibling function with positional args would push the render entry
/// point past the 7-argument budget.
#[derive(Debug)]
pub struct MetricsInputs<'a, S: BuildHasher> {
    /// The clusters that survive into the rendered report — the visible
    /// set after [`crate::report::render_report`] drops report-hidden and
    /// noise / structural-only clusters. The metric counts the same
    /// clusters the report carries, never one the report dropped, so
    /// per-file and repo percentages stay consistent with the cluster list
    /// every surface renders ([METRICS-REPO]).
    pub clusters: &'a [&'a Cluster],
    /// Per-file source bytes keyed by [`FileId`]. Used to convert
    /// `byte_range` to line numbers; only read, never mutated.
    pub sources: &'a HashMap<FileId, Vec<u8>>,
    /// Shared per-file line indexes built once for report rendering and metrics.
    pub line_indices: &'a LineIndices,
    /// Per-file language id. Required to evaluate per-language
    /// `report_hide` patterns.
    pub file_languages: &'a HashMap<FileId, &'static str, S>,
    /// File registry used to resolve `FileId → absolute path`.
    pub registry: &'a FileRegistry,
    /// `.deslop.toml` policy. Occurrences whose file matches a
    /// `report_hide` pattern are excluded from the numerator.
    pub exclusion: &'a ExclusionConfig,
    /// Per-file analysed-line counts accumulated during the corpus
    /// read-pass.
    pub analysed_lines: &'a AnalysedLines,
    /// Scan root every `per_file` path is rendered relative to, so the
    /// metrics rows carry the same path form as occurrence rows
    ///.
    pub scan_root: &'a Path,
    /// Verified diff scope when the run carried `--diff`. Drives the
    /// [METRICS-DIFF-SCOPE] added-line block; `None` leaves
    /// `RepoMetrics.diff` absent and every mechanical field untouched.
    pub diff: Option<&'a DiffScope>,
}

/// Computes [`RepoMetrics`] for a finished analysis pass. The returned
/// struct carries `threshold = ThresholdSummary::none()` — the CLI
/// resolves the threshold layer afterwards and overwrites
/// `metrics.threshold` in place.
#[must_use]
pub fn compute_repo_metrics<S: BuildHasher>(inputs: &MetricsInputs<'_, S>) -> RepoMetrics {
    let analysed_loc: u64 = inputs.analysed_lines.values().copied().sum();
    let mut per_file_lines: HashMap<FileId, BTreeSet<u64>> = HashMap::new();
    for &cluster in inputs.clusters {
        fold_cluster_lines(cluster, inputs, &mut per_file_lines);
    }
    let duplicated_loc: u64 = per_file_lines
        .values()
        .map(|lines| u64::try_from(lines.len()).unwrap_or(u64::MAX))
        .sum();
    let duplicated_files = per_file_lines
        .values()
        .filter(|set| !set.is_empty())
        .count();
    let duplication_percent = percent(duplicated_loc, analysed_loc);
    let per_file = per_file_metrics(&per_file_lines, inputs);
    let folders = folder_metrics(&per_file);
    RepoMetrics {
        analysed_loc,
        duplicated_loc,
        duplication_percent,
        // [METRICS-REPO] The banner equals the body by construction:
        // `inputs.clusters` is the exact post-hide list the report
        // carries, and a mixed cluster (one visible occurrence beside
        // hidden ones) is kept in it per [EXCLUSION-CONFIG], so it must
        // be counted here too. The old `>= 2 visible members` gate said
        // "0 clusters" above a body listing one.
        clusters_total: inputs.clusters.len(),
        duplicated_files,
        threshold: ThresholdSummary::none(),
        per_file,
        folders,
        diff: inputs
            .diff
            .map(|scope| diff_metrics(&per_file_lines, inputs, scope)),
    }
}

/// Builds the per-folder breakdown ([METRICS-REPO] `RepoMetrics.folders`)
/// by summing the already-computed `per_file` rows under every folder
/// prefix — clean files stay in the denominator exactly as they do per
/// file — and dividing with the same [`percent`] every other figure uses.
/// This is the **only** place folder percentages are computed; consumers
/// render these rows verbatim. Folders with no duplicated lines are
/// dropped; rows sort worst-first, path tiebreaker, like `per_file`.
fn folder_metrics(per_file: &[FileMetric]) -> Vec<FileMetric> {
    let mut rows: Vec<FileMetric> = folder_sums(per_file)
        .into_iter()
        .filter(|(_, (_, duplicated_loc))| *duplicated_loc > 0)
        .map(|(path, (analysed_loc, duplicated_loc))| FileMetric {
            path: PathBuf::from(path),
            analysed_loc,
            duplicated_loc,
            duplication_percent: percent(duplicated_loc, analysed_loc),
        })
        .collect();
    sort_worst_first(&mut rows);
    rows
}

/// Sums `(analysed_loc, duplicated_loc)` for every folder prefix of
/// every `per_file` row, clean files included.
fn folder_sums(per_file: &[FileMetric]) -> HashMap<String, (u64, u64)> {
    let mut sums: HashMap<String, (u64, u64)> = HashMap::new();
    for row in per_file {
        for prefix in folder_prefixes(&row.path) {
            let entry = sums.entry(prefix).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(row.analysed_loc);
            entry.1 = entry.1.saturating_add(row.duplicated_loc);
        }
    }
    sums
}

/// Deterministic wire order shared by `per_file` and `folders`: worst
/// percentage first, path tiebreaker.
fn sort_worst_first(rows: &mut [FileMetric]) {
    rows.sort_by(|left, right| {
        right
            .duplication_percent
            .partial_cmp(&left.duplication_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
    });
}

/// Every folder prefix of `path`, shallowest first, joined with `/` on
/// every platform so folder-row paths group the same segments a client
/// splits a file-row path into. Root and current-dir markers contribute
/// nothing; a Windows drive prefix is kept as its own segment.
fn folder_prefixes(path: &Path) -> Vec<String> {
    let segments: Vec<String> = path
        .components()
        .filter(|component| {
            !matches!(
                component,
                std::path::Component::RootDir | std::path::Component::CurDir
            )
        })
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut prefixes = Vec::new();
    let mut accumulated = String::new();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        if !accumulated.is_empty() {
            accumulated.push('/');
        }
        accumulated.push_str(segment);
        prefixes.push(accumulated.clone());
    }
    prefixes
}

/// Builds the [METRICS-DIFF-SCOPE] added-line block. The numerator is
/// the intersection of the *same* per-file duplicated-line projection
/// `duplicated_loc` counts with the diff's added spans — computed here,
/// beside it, from the same `per_file_lines` sets, so the two figures
/// can never diverge in projection. Threshold stays `none()`; the CLI
/// resolves it only under `--only-changed`.
fn diff_metrics<S: BuildHasher>(
    per_file_lines: &HashMap<FileId, BTreeSet<u64>>,
    inputs: &MetricsInputs<'_, S>,
    scope: &DiffScope,
) -> DiffMetrics {
    let added_loc = scope.added_line_total();
    let duplicated_added_loc: u64 = per_file_lines
        .iter()
        .filter_map(|(file_id, lines)| {
            let path = relative_to_scan_root(inputs.registry.path(*file_id)?, inputs.scan_root);
            let inside = lines.iter().filter(|line| scope.contains(&path, **line));
            u64::try_from(inside.count()).ok()
        })
        .sum();
    DiffMetrics {
        added_loc,
        duplicated_added_loc,
        duplication_percent: percent(duplicated_added_loc, added_loc),
        threshold: ThresholdSummary::none(),
    }
}

/// Builds the per-file duplication breakdown ([METRICS-REPO]
/// `RepoMetrics.per_file`). The universe is every analysed file unioned
/// with every file carrying duplicated lines, so clean files keep exact
/// percentage denominators. Sorted worst-first by percentage, path
/// tiebreaker, so the wire order is deterministic.
fn per_file_metrics<S: BuildHasher>(
    per_file_lines: &HashMap<FileId, BTreeSet<u64>>,
    inputs: &MetricsInputs<'_, S>,
) -> Vec<FileMetric> {
    let mut universe: HashSet<FileId> = inputs.analysed_lines.keys().copied().collect();
    universe.extend(per_file_lines.keys().copied());
    let mut rows: Vec<FileMetric> = universe
        .into_iter()
        .filter_map(|file_id| file_metric(file_id, per_file_lines, inputs))
        .collect();
    sort_worst_first(&mut rows);
    rows
}

/// Projects one file's analysed and duplicated line counts into a
/// [`FileMetric`]. Returns `None` when the registry cannot resolve the
/// file path — a metric row with no location is useless to consumers.
fn file_metric<S: BuildHasher>(
    file_id: FileId,
    per_file_lines: &HashMap<FileId, BTreeSet<u64>>,
    inputs: &MetricsInputs<'_, S>,
) -> Option<FileMetric> {
    let path = relative_to_scan_root(inputs.registry.path(file_id)?, inputs.scan_root);
    let analysed_loc = inputs.analysed_lines.get(&file_id).copied().unwrap_or(0);
    let duplicated_loc = per_file_lines
        .get(&file_id)
        .map_or(0, |lines| u64::try_from(lines.len()).unwrap_or(u64::MAX));
    Some(FileMetric {
        path,
        analysed_loc,
        duplicated_loc,
        duplication_percent: percent(duplicated_loc, analysed_loc),
    })
}

/// Projects every non-hidden occurrence of `cluster` onto per-file line
/// sets. Hidden occurrences contribute nothing, so a generated tier
/// never inflates `duplicated_loc` ([METRICS-REPO]).
fn fold_cluster_lines<S: BuildHasher>(
    cluster: &Cluster,
    inputs: &MetricsInputs<'_, S>,
    per_file_lines: &mut HashMap<FileId, BTreeSet<u64>>,
) {
    for member in &cluster.members {
        add_member_lines(member, inputs, per_file_lines);
    }
}

/// Adds the line range covered by `member` to `per_file_lines` unless
/// the file is `report_hide`-suppressed or its source bytes are
/// unavailable.
fn add_member_lines<S: BuildHasher>(
    member: &crate::fingerprint::Fingerprint,
    inputs: &MetricsInputs<'_, S>,
    per_file_lines: &mut HashMap<FileId, BTreeSet<u64>>,
) {
    if occurrence_is_hidden(member.file_id, inputs) {
        return;
    }
    let Some(line_index) = inputs.line_indices.get(&member.file_id) else {
        return;
    };
    let entry = per_file_lines.entry(member.file_id).or_default();
    let (start_line, end_line) =
        byte_range_to_line_range(line_index, member.byte_range.start, member.byte_range.end);
    for line in start_line..=end_line {
        let _inserted = entry.insert(line);
    }
}

/// Returns `true` when the occurrence's file is covered by a
/// `[EXCLUSION-CONFIG]` `report_hide` pattern.
fn occurrence_is_hidden<S: BuildHasher>(file_id: FileId, inputs: &MetricsInputs<'_, S>) -> bool {
    let Some(path) = inputs.registry.path(file_id) else {
        return false;
    };
    let language = inputs.file_languages.get(&file_id).copied().unwrap_or("");
    if inputs.exclusion.is_report_hidden(path, language) {
        return true;
    }
    inputs
        .sources
        .get(&file_id)
        .is_some_and(|source| crate::config::has_generated_header(source))
}

/// Converts a half-open `[start, end)` byte range into a closed
/// 1-indexed line range. Empty ranges yield `(line, line)` for the
/// starting line so they still occupy one row in the set.
fn byte_range_to_line_range(index: &LineIndex, start: usize, end: usize) -> (u64, u64) {
    let safe_end = end.min(index.source_len());
    let safe_start = start.min(safe_end);
    let start_line = u64::try_from(index.line_for_offset(safe_start)).unwrap_or(u64::MAX);
    let end_offset = safe_end.saturating_sub(1).max(safe_start);
    let end_line = u64::try_from(index.line_for_offset(end_offset)).unwrap_or(u64::MAX);
    (start_line, end_line)
}

/// Counts physical lines in `source`: one per `\n` plus one for a
/// trailing partial line. Empty input contributes zero.
#[must_use]
pub fn count_analysed_lines(source: &[u8]) -> u64 {
    if source.is_empty() {
        return 0;
    }
    let trailing: u64 = u64::from(!source.ends_with(b"\n"));
    count_newlines(source).saturating_add(trailing)
}

/// Counts the number of `\n` bytes in `bytes`. Phrased as a manual
/// loop to sidestep the `naive_bytecount` clippy lint — without
/// pulling in the `bytecount` crate for a value the analysis never
/// reads in a hot loop.
fn count_newlines(bytes: &[u8]) -> u64 {
    let mut count: u64 = 0;
    for byte in bytes {
        if *byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    count
}

/// `100 * num / denom`, clamped into `[0, 100]`. Returns `0.0` when
/// `denom == 0`. The `u64 -> u32 -> f64` reduction is safe because
/// both inputs are physical line counts — real repos never reach
/// 2^32 lines, and we clamp before casting so the `as f64` step never
/// loses precision in the reachable range.
pub(crate) fn percent(num: u64, denom: u64) -> f64 {
    if denom == 0 {
        return 0.0;
    }
    let cap = u64::from(u32::MAX);
    let num32 = u32::try_from(num.min(cap)).unwrap_or(u32::MAX);
    let denom32 = u32::try_from(denom.min(cap)).unwrap_or(u32::MAX);
    if denom32 == 0 {
        return 0.0;
    }
    let ratio = f64::from(num32) / f64::from(denom32);
    let pct = ratio * 100.0_f64;
    pct.clamp(0.0, 100.0)
}

/// Resolves a threshold percentage coming from config or CLI. Rejects
/// NaN, negatives, and values above `100.0`. The error string is
/// user-facing.
///
/// # Errors
///
/// Returns `Err` with a short message when `value` is not finite, is
/// negative, or exceeds `100.0`.
pub fn validate_threshold_percent(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err(format!("threshold must be a finite number, got {value}"));
    }
    if !(0.0_f64..=100.0_f64).contains(&value) {
        return Err(format!(
            "threshold must be within [0.0, 100.0], got {value}"
        ));
    }
    Ok(value)
}
