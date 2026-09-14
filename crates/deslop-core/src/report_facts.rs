//! Figures every surface derives from a finished [`Report`]
//! ([METRICS-REPO], [METRICS-DIFF-SCOPE]).
//!
//! The HTML page, the text dump and the CLI's stderr summary all show
//! the same handful of numbers about a run. Each used to re-derive them
//! from the report in its own words, so a change to one surface left
//! the others quietly reporting something else — and one copy sat in
//! the `deslop` binary, outside the crate that owns every calculation.
//! They are computed here, once. Renderers format what this module
//! returns; they never recompute it.

use crate::{
    report::Report,
    report_metrics::{ThresholdSource, ThresholdSummary},
};

/// Verdict word shown beside a threshold the run exceeded.
const VERDICT_BREACHED: &str = "breached";
/// Verdict word shown when the run stayed within its threshold.
const VERDICT_OK: &str = "ok";

/// The verdict word for `threshold`, or `None` when no threshold
/// governs the run — in which case every surface omits the segment
/// rather than printing a verdict about a threshold nobody set.
#[must_use]
pub fn threshold_verdict(threshold: &ThresholdSummary) -> Option<&'static str> {
    match threshold.source {
        ThresholdSource::None => None,
        ThresholdSource::Cli | ThresholdSource::Config => Some(if threshold.breached {
            VERDICT_BREACHED
        } else {
            VERDICT_OK
        }),
    }
}

/// The repo-wide cluster count. Under `--only-changed`,
/// `metrics.clusters_total` follows the filtered body ([METRICS-REPO]),
/// so the repo-wide figure is body + omitted ([METRICS-DIFF-SCOPE]).
#[must_use]
pub fn repo_cluster_count(report: &Report) -> usize {
    report
        .metrics
        .clusters_total
        .saturating_add(report.clusters_outside_diff.unwrap_or(0))
}

/// The `--only-changed` cluster split ([METRICS-DIFF-SCOPE]).
///
/// Every surviving cluster intersects the diff by construction, so the
/// visible body divides exactly into [`Self::newly`] and
/// [`Self::cross_file`], and [`Self::outside`] accounts for what the
/// filter dropped. The three reconcile with the unfiltered total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffDelta {
    /// Clusters whose every occurrence lies inside the diff.
    pub newly: usize,
    /// Clusters that intersect the diff but also cover untouched code.
    pub cross_file: usize,
    /// Clusters `--only-changed` removed from the rendered report.
    pub outside: usize,
}

impl DiffDelta {
    /// Reads the split off `report`, or `None` when `--only-changed`
    /// never ran and no surface shows a delta.
    #[must_use]
    pub fn of(report: &Report) -> Option<Self> {
        let outside = report.clusters_outside_diff?;
        let newly = report
            .clusters
            .iter()
            .filter(|cluster| cluster.is_newly_introduced == Some(true))
            .count();
        Some(Self {
            newly,
            cross_file: report.clusters.len().saturating_sub(newly),
            outside,
        })
    }

    /// Clusters that intersect the diff — the rendered report body.
    #[must_use]
    pub const fn touched(&self) -> usize {
        self.newly.saturating_add(self.cross_file)
    }
}
