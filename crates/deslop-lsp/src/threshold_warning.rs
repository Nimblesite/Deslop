//! Startup threshold-breach warning ([CI-DESLOP], GH #194).
//!
//! `.deslop.toml [threshold] max_duplication_percent` is a CLI-only CI
//! gate ([EXIT-CODES]); the live engine never gates, hides, caps, or
//! re-ranks on it. Its *only* live-surface involvement is this single
//! non-blocking `window/showMessage` warning, surfaced once at startup
//! when the measured duplication has smashed the configured budget. The
//! editor behaves identically whether or not a threshold is configured.

use deslop_core::{
    live::{LiveApi, LiveService},
    ExclusionConfig,
};
use tower_lsp::{lsp_types::MessageType, Client};

/// Pushes a single non-blocking warning when measured duplication has
/// smashed the `.deslop.toml` threshold. No-op when the file opts out of
/// a threshold or the budget is met. Never gates or alters live state.
pub(crate) async fn push_threshold_warning(client: &Client, service: &LiveService) {
    let Some(percent) = configured_threshold(service).await else {
        return;
    };
    let measured = service.report_get().await.metrics.duplication_percent;
    if measured <= percent {
        return;
    }
    client
        .show_message(MessageType::WARNING, breach_message(measured, percent))
        .await;
}

/// Loads the workspace `[threshold] max_duplication_percent`, re-reading
/// `.deslop.toml` next to the session root exactly as the CLI gate does
/// rather than coupling the live engine to a knob it must ignore.
async fn configured_threshold(service: &LiveService) -> Option<f64> {
    let root = service.session().lock().await.root().to_path_buf();
    ExclusionConfig::discover(&root).ok()?.fail_over_percent()
}

/// Builds the informational breach message naming the measured value,
/// the smashed threshold, and its `.deslop.toml` source.
fn breach_message(measured: f64, percent: f64) -> String {
    format!(
        "Duplication {measured:.1}% has smashed your {percent}% threshold (.deslop.toml). \
         This is informational only — Deslop does not gate, hide, or re-rank anything."
    )
}
