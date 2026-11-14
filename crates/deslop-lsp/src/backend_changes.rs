//! Applies a change to the live session and pushes the outcome to the
//! editor. Shared by the disk-driven path (`didChangeWatchedFiles`) and
//! the open-buffer path (`didOpen` / `didChange` / `didClose`,
//! [LIVE-OPEN-BUFFERS]) so both publish the same `deslop/reportChanged`
//! notification and the same diagnostics refresh.
use std::path::{Path, PathBuf};

use deslop_core::{
    live::{AnalysisSession, ChangeSummary, LiveError, ReportChangedNotification},
    wire_generated::AnalysedBuffer,
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
        self.apply_session_change(|session| session.apply_changes(paths), None)
            .await;
    }

    /// Publishes the editor version together with the report that analysed it.
    pub(super) async fn apply_buffer_change(
        &self,
        path: &Path,
        version: i32,
        change: impl SessionChange,
    ) {
        let analysed_buffer = Some(AnalysedBuffer {
            path: path.to_path_buf(),
            version,
        });
        self.apply_session_change(change, analysed_buffer).await;
    }

    /// Runs `change` against the locked session and pushes the resulting
    /// report delta to the editor.
    pub(super) async fn apply_session_change(
        &self,
        change: impl SessionChange,
        analysed_buffer: Option<AnalysedBuffer>,
    ) {
        let session = self.service.session();
        let mut guard = session.lock().await;
        match self
            .commit_change(&mut guard, change, analysed_buffer)
            .await
        {
            Ok(Some(notification)) => {
                self.client
                    .send_notification::<ReportChangedLspNotification>(notification)
                    .await;
                drop(guard);
                self.diagnostics.refresh(&self.client).await;
            }
            Ok(None) => {}
            Err(error) => tracing::error!(%error, "failed to apply session change"),
        }
    }

    /// Applies `change`, remembers the delta baseline, and converts a
    /// non-empty delta into an LSP notification.
    async fn commit_change(
        &self,
        session: &mut AnalysisSession,
        change: impl SessionChange,
        analysed_buffer: Option<AnalysedBuffer>,
    ) -> Result<Option<ReportChangedNotification>, LiveError> {
        let previous_generation = session.generation();
        let previous_report = session.report();
        let delta = change(session)?;
        let analysed_buffer = analysed_buffer.and_then(|buffer| session.acknowledgement(buffer));
        self.service
            .remember_snapshot(previous_generation, previous_report)
            .await;
        if delta.is_empty() && analysed_buffer.is_none() {
            return Ok(None);
        }
        Ok(Some(ReportChangedNotification {
            generation: delta.to_generation,
            summary: ChangeSummary::from_delta(&delta),
            analysed_buffer,
        }))
    }
}
