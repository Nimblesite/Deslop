//! Push-notification helpers ([LIVE-NOTIFICATIONS]).
//!
//! Thin wrappers over [`tokio::sync::broadcast`] senders so the
//! scheduler does not have to know which subscribers exist. Slow
//! subscribers fall behind silently — broadcast channels drop the
//! oldest message rather than blocking the scheduler ([LIVE-PERF-BUDGETS]).

use tokio::sync::broadcast::Sender;

use super::wire::{AnalysisState, ReportChangedNotification};

/// Convenience alias for an `analysis/state` broadcaster.
pub type StateSender = Sender<AnalysisState>;
/// Convenience alias for a `report/changed` broadcaster.
pub type ReportChangedSender = Sender<ReportChangedNotification>;

/// Broadcasts a `report/changed` notification, swallowing the
/// "no active subscribers" error.
pub fn broadcast_report_changed(
    sender: &ReportChangedSender,
    notification: ReportChangedNotification,
) {
    let _result = sender.send(notification);
}

/// Broadcasts an `analysis/state` notification.
pub fn broadcast_state(sender: &StateSender, state: AnalysisState) {
    let _result = sender.send(state);
}
