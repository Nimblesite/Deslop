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
    path::Path,
};

use crate::{
    cluster::Cluster,
    config::ExclusionConfig,
    report_render::relative_to_scan_root,
    state::{FileId, FileRegistry},
};

// `RepoMetrics`, `ThresholdSummary`, and `ThresholdSource` are generated
// from `docs/models/live-ipc.td` by `scripts/typediagram-gen.mjs`. The
// data shapes live in `crate::wire_generated`; the constructors and
// `Default` impl below stay here.
pub use crate::wire_generated::{FileMetric, RepoMetrics, ThresholdSource, ThresholdSummary};

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
    /// ([Deslop#286]).
    pub scan_root: &'a Path,
}

/// Computes [`RepoMetrics`] for a finished analysis pass. The returned
/// struct carries `threshold = ThresholdSummary::none()` — the CLI
/// resolves the threshold layer afterwards and overwrites
/// `metrics.threshold` in place.
#[must_use]
pub fn compute_repo_metrics<S: BuildHasher>(inputs: &MetricsInputs<'_, S>) -> RepoMetrics {
    let analysed_loc: u64 = inputs.analysed_lines.values().copied().sum();
    let mut per_file_lines: HashMap<FileId, BTreeSet<u64>> = HashMap::new();
    let mut contributing_clusters: usize = 0;
    for &cluster in inputs.clusters {
        if fold_cluster_lines(cluster, inputs, &mut per_file_lines) {
            contributing_clusters = contributing_clusters.saturating_add(1);
        }
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
    RepoMetrics {
        analysed_loc,
        duplicated_loc,
        duplication_percent,
        clusters_total: contributing_clusters,
        duplicated_files,
        threshold: ThresholdSummary::none(),
        per_file: per_file_metrics(&per_file_lines, inputs),
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
    rows.sort_by(|left, right| {
        right
            .duplication_percent
            .partial_cmp(&left.duplication_percent)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
    });
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
/// sets. Returns `true` when the cluster contributed at least two
/// non-hidden occurrences and therefore counts toward
/// `clusters_total`.
fn fold_cluster_lines<S: BuildHasher>(
    cluster: &Cluster,
    inputs: &MetricsInputs<'_, S>,
    per_file_lines: &mut HashMap<FileId, BTreeSet<u64>>,
) -> bool {
    let mut visible_members: usize = 0;
    for member in &cluster.members {
        if add_member_lines(member, inputs, per_file_lines) {
            visible_members = visible_members.saturating_add(1);
        }
    }
    visible_members >= 2
}

/// Adds the line range covered by `member` to `per_file_lines` unless
/// the file is `report_hide`-suppressed or its source bytes are
/// unavailable. Returns `true` when the occurrence was counted as
/// non-hidden.
fn add_member_lines<S: BuildHasher>(
    member: &crate::fingerprint::Fingerprint,
    inputs: &MetricsInputs<'_, S>,
    per_file_lines: &mut HashMap<FileId, BTreeSet<u64>>,
) -> bool {
    if occurrence_is_hidden(member.file_id, inputs) {
        return false;
    }
    let Some(source) = inputs.sources.get(&member.file_id) else {
        return false;
    };
    let entry = per_file_lines.entry(member.file_id).or_default();
    let (start_line, end_line) =
        byte_range_to_line_range(source, member.byte_range.start, member.byte_range.end);
    for line in start_line..=end_line {
        let _inserted = entry.insert(line);
    }
    true
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
fn byte_range_to_line_range(source: &[u8], start: usize, end: usize) -> (u64, u64) {
    let safe_end = end.min(source.len());
    let safe_start = start.min(safe_end);
    let start_line = line_for_offset(source, safe_start);
    let end_offset = safe_end.saturating_sub(1).max(safe_start);
    let end_line = line_for_offset(source, end_offset);
    (start_line, end_line)
}

/// 1-indexed line number containing byte `offset` in `source`.
fn line_for_offset(source: &[u8], offset: usize) -> u64 {
    let safe = offset.min(source.len());
    let prefix = source.get(..safe).unwrap_or(&[]);
    count_newlines(prefix).saturating_add(1)
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
fn percent(num: u64, denom: u64) -> f64 {
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
