//! [LIVE-OPEN-BUFFERS] Open-buffer entry points on [`AnalysisSession`].
//!
//! The editor tells the session what it holds open, how that text
//! changes, and when it lets go. Each call re-analyses the file from the
//! text the user sees, so a clone deleted in an unsaved buffer leaves
//! the report before the file is ever saved — and comes back if the
//! buffer is closed without saving.

use std::path::Path;

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
        self.apply_changes(&[path.to_path_buf()])
    }
}
