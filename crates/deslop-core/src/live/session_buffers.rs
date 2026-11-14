//! [LIVE-OPEN-BUFFERS] Open-buffer entry points on [`AnalysisSession`].
//!
//! The editor tells the session what it holds open, how that text
//! changes, and when it lets go. Each call re-analyses the file from the
//! text the user sees, so a clone deleted in an unsaved buffer leaves
//! the report before the file is ever saved — and comes back if the
//! buffer is closed without saving.

use std::path::Path;

use crate::wire_generated::AnalysedBuffer;

use super::{errors::LiveError, session::AnalysisSession};
use crate::{delta::ReportDelta, pipeline::BufferEdit};

impl AnalysisSession {
    /// Records `text` as the editor's copy of `path` and re-analyses the
    /// file from it. Opening a file whose buffer matches the disk is a
    /// no-op pass ([LIVE-SCHEDULER-NOOP]).
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`].
    pub fn open_buffer(&mut self, path: &Path, text: String) -> Result<ReportDelta, LiveError> {
        // [LIVE-WORKSPACE-SCOPE] Validate before retaining editor text or scheduling analysis.
        self.guard_path(path)?;
        self.buffers.open(path, text);
        self.apply_changes(&[path.to_path_buf()])
    }

    /// Applies `edits` to the open buffer for `path` and re-analyses the
    /// file from the edited text.
    ///
    /// # Errors
    ///
    /// Returns [`LiveError::BufferNotOpen`] when the editor never opened
    /// `path`, and propagates pipeline errors via [`LiveError::Core`].
    pub fn edit_buffer(
        &mut self,
        path: &Path,
        edits: &[BufferEdit],
    ) -> Result<ReportDelta, LiveError> {
        self.guard_path(path)?;
        if !self.buffers.edit(path, edits) {
            return Err(LiveError::BufferNotOpen {
                path: path.to_path_buf(),
            });
        }
        self.apply_changes(&[path.to_path_buf()])
    }

    /// The acknowledgement a buffer change has earned, recorded against the
    /// report it produced. In the seed window the change is only queued, so
    /// the report still describes the older text: the version is held back
    /// for [`Self::take_deferred_acknowledgements`].
    pub fn acknowledgement(&mut self, buffer: AnalysedBuffer) -> Option<AnalysedBuffer> {
        if self.is_seed_only() {
            let _previous = self
                .deferred_acknowledgements
                .insert(buffer.path, buffer.version);
            return None;
        }
        let _previous = self
            .analysed_versions
            .insert(buffer.path.clone(), buffer.version);
        Some(buffer)
    }

    /// Buffers whose seed-window edits the cold pass has now replayed.
    pub fn take_deferred_acknowledgements(&mut self) -> Vec<AnalysedBuffer> {
        let replayed = std::mem::take(&mut self.deferred_acknowledgements);
        self.analysed_versions.extend(
            replayed
                .iter()
                .map(|(path, version)| (path.clone(), *version)),
        );
        replayed
            .into_iter()
            .map(|(path, version)| AnalysedBuffer { path, version })
            .collect()
    }

    /// [LIVE-OPEN-BUFFERS] The editor version of each open buffer the
    /// current report analysed. Read under the same lock as the report, so
    /// a client can release exactly the versions the report it holds
    /// describes.
    #[must_use]
    pub fn analysed_versions(&self) -> Vec<AnalysedBuffer> {
        self.analysed_versions
            .iter()
            .map(|(path, version)| AnalysedBuffer {
                path: path.clone(),
                version: *version,
            })
            .collect()
    }

    /// Forgets the editor's copy of `path` and re-analyses the file from
    /// disk, so a buffer discarded without saving stops shadowing the
    /// saved bytes. Closing a path that was never open re-reads the disk
    /// and changes nothing.
    ///
    /// # Errors
    ///
    /// Propagates pipeline errors via [`LiveError::Core`].
    pub fn close_buffer(&mut self, path: &Path) -> Result<ReportDelta, LiveError> {
        self.guard_path(path)?;
        let _was_open = self.buffers.close(path);
        let _analysed = self.analysed_versions.remove(path);
        self.apply_changes(&[path.to_path_buf()])
    }
}
