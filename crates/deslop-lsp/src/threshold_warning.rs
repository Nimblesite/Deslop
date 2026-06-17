//! Startup threshold-breach warning ([CI-DESLOP], GH #194).
//!
//! `.deslop.toml [threshold] max_duplication_percent` is a CLI-only CI
//! gate ([EXIT-CODES]); the live engine never gates, hides, caps, or
//! re-ranks on it. Its live-surface footprint is display-only: the
//! resolved verdict rides `RepoMetrics.threshold` (populated once in the
//! shared render path) for the DUPLICATION panel, and this single
//! non-blocking `window/showMessage` warning fires once at startup when
//! measured duplication has smashed the configured budget. The editor
//! behaves identically whether or not a threshold is configured.

use deslop_core::live::{LiveApi, LiveService};
use tower_lsp::{lsp_types::MessageType, Client};

/// Pushes a single non-blocking warning when measured duplication has
/// smashed the `.deslop.toml` threshold. No-op when the file opts out of
/// a threshold or the budget is met. Reads the verdict the render path
/// already resolved onto `RepoMetrics.threshold`; never gates or alters
/// live state.
pub(crate) async fn push_threshold_warning(client: &Client, service: &LiveService) {
    let (breached, measured, percent) = {
        let report = service.report_get().await;
        (
            report.metrics.threshold.breached,
            report.metrics.duplication_percent,
            report.metrics.threshold.percent,
        )
    };
    if !breached {
        return;
    }
    client
        .show_message(MessageType::WARNING, breach_message(measured, percent))
        .await;
}

/// Builds the informational breach message naming the measured value,
/// the smashed threshold, and its `.deslop.toml` source.
fn breach_message(measured: f64, percent: f64) -> String {
    format!(
        "Duplication {measured:.1}% has smashed your {percent}% threshold (.deslop.toml). \
         This is informational only — Deslop does not gate, hide, or re-rank anything."
    )
}
