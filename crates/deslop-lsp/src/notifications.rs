//! LSP notification marker types for Deslop custom methods.

use deslop_core::live::{EmbeddingProgress, ReportChangedNotification};

/// Method name for model-swap progress ([VSIX-SESSION-PROGRESS]).
pub const EMBEDDING_PROGRESS: &str = "deslop/embeddingProgress";

/// Method name for generation-change notifications.
pub const REPORT_CHANGED: &str = "deslop/reportChanged";

/// Method name for `deslop/analysisState` pushed by the scheduler
/// whenever a watcher-driven pass starts, finishes, or errors.
pub const ANALYSIS_STATE: &str = "deslop/analysisState";

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch our custom method.
#[derive(Debug)]
pub enum EmbeddingProgressNotification {}

impl tower_lsp::lsp_types::notification::Notification for EmbeddingProgressNotification {
    type Params = EmbeddingProgress;
    const METHOD: &'static str = EMBEDDING_PROGRESS;
}

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch `deslop/reportChanged`.
#[derive(Debug)]
pub enum ReportChangedLspNotification {}

impl tower_lsp::lsp_types::notification::Notification for ReportChangedLspNotification {
    type Params = ReportChangedNotification;
    const METHOD: &'static str = REPORT_CHANGED;
}

/// Type-only marker so `tower_lsp::Client::send_notification` can
/// dispatch `deslop/analysisState`.
#[derive(Debug)]
pub enum AnalysisStateLspNotification {}

impl tower_lsp::lsp_types::notification::Notification for AnalysisStateLspNotification {
    /// Plain string — VSIX checks `state === "running"` etc.
    type Params = String;
    const METHOD: &'static str = ANALYSIS_STATE;
}
