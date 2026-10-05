//! Applies a change to the live session and pushes the outcome to the
//! editor. Shared by the disk-driven path (`didChangeWatchedFiles`) and
//! the open-buffer path (`didOpen` / `didChange` / `didClose`,
//! [LIVE-OPEN-BUFFERS]) so both publish the same `deslop/reportChanged`
//! notification and the same diagnostics refresh.
use std::path::PathBuf;

use deslop_core::{
    live::{AnalysisSession, ChangeSummary, LiveError, ReportChangedNotification},
    ReportDelta,
};

use super::LspBackend;
use crate::notifications::ReportChangedLspNotification;

/// One mutation of the locked session, returning the delta it produced.
pub(super) trait SessionChange:
    FnOnce(&mut AnalysisSession) -> Result<ReportDelta, LiveError> + Send
{
}

impl<F> SessionChange for F where
    F: FnOnce(&mut AnalysisSession) -> Result<ReportDelta, LiveError> + Send
{
}

impl LspBackend {
    /// Re-runs analysis for paths that changed on disk when the editor
    /// relays file events.
    pub(super) async fn apply_changed_paths(&self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }
        self.apply_session_change(|session| session.apply_changes(paths))
            .await;
    }

    /// Runs `change` against the locked session and pushes the resulting
    /// report delta to the editor.
    pub(super) async fn apply_session_change(&self, change: impl SessionChange) {
        match self.session_change_notification(change).await {
            Ok(Some(notification)) => {
                self.client
                    .send_notification::<ReportChangedLspNotification>(notification)
                    .await;
                self.diagnostics.refresh(&self.client).await;
            }
            Ok(None) => {}
            Err(error) => tracing::error!(%error, "failed to apply session change"),
        }
    }

    /// Applies `change` and converts a non-empty delta into an LSP
    /// notification.
    async fn session_change_notification(
        &self,
        change: impl SessionChange,
    ) -> Result<Option<ReportChangedNotification>, LiveError> {
        let session = self.service.session();
        let (previous_generation, previous_report, delta) = {
            let mut guard = session.lock().await;
            let previous_generation = guard.generation();
            let previous_report = guard.report();
            let delta = change(&mut guard)?;
            (previous_generation, previous_report, delta)
        };
        self.service
            .remember_snapshot(previous_generation, previous_report)
            .await;
        if delta.is_empty() {
            return Ok(None);
        }
        Ok(Some(ReportChangedNotification {
            generation: delta.to_generation,
            summary: ChangeSummary::from_delta(&delta),
        }))
    }
}
