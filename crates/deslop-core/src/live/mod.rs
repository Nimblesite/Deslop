//! Live analysis module ([LIVE-PACKAGING]).
//!
//! Behind the `live` cargo feature. Owns the watcher-driven
//! [`AnalysisSession`], the single-flight [`scheduler::Scheduler`],
//! and the stable [`api::LiveApi`] surface that the LSP and MCP
//! transports forward to. Nothing in this module mutates source files
//! or reaches outside the session's pinned workspace root
//! ([MCP-SAFETY], [LIVE-LIFECYCLE]).

pub mod api;
pub mod clock;
pub mod cluster_lookup;
pub mod debouncer;
pub mod embedding_refresh;
pub mod errors;
pub mod freshness;
pub mod notifications;
pub mod scheduler;
pub mod session;
mod session_helpers;
pub mod transport;
pub mod watcher;
pub mod wire;

pub use api::{service_from_session, LiveApi, LiveService};
pub use clock::{Clock, SystemClock};
pub use debouncer::{Debouncer, CAP_MS, QUIET_MS};
pub use errors::{LiveError, LiveErrorWire};
pub use notifications::{
    broadcast_report_changed, broadcast_state, ReportChangedSender, StateSender,
};
pub use scheduler::Scheduler;
pub use session::{
    read_report_snapshot, report_for_file_in, report_for_range_in, AnalysisSession,
    EmbeddingProgressReporter,
};
pub use session_helpers::STATE_FILE_NAME as LIVE_REPORT_FILE_NAME;
pub use watcher::LiveWatcher;
pub use wire::{
    AnalysisState, ChangeSummary, EmbeddingModelInfo, EmbeddingPhase, EmbeddingProgress,
    FileReport, FindSimilarInput, FindSimilarRequest, FindSimilarResult, ReportChangedNotification,
    SessionConfig,
};
