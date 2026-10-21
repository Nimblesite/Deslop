//! [LIVE-OPEN-BUFFERS] Editor buffers that shadow their file on disk.
//!
//! An editor shows the user its own copy of a file — the open buffer —
//! and that copy drifts from the bytes on disk with every unsaved
//! keystroke. A report built from the disk bytes then describes code the
//! user can no longer see: a clone deleted a moment ago still reads as a
//! duplicate. This module keeps the text of every open buffer so a change
//! pass analyses what the editor shows, and falls back to the disk only
//! for files no editor holds.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use super::corpus::read_source;
use crate::{
    error::CoreError,
    refactor::position::{byte_for_line_col, LineCol},
};

/// One edit to an open buffer, in the shape the LSP delivers it: a
/// replacement of `range` (end exclusive), or of the whole text when no
/// range is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferEdit {
    /// Replaced span as zero-indexed line / UTF-16 column pairs. `None`
    /// replaces the whole buffer.
    pub range: Option<(LineCol, LineCol)>,
    /// Text that replaces the span.
    pub text: String,
}

/// Source bytes for one path together with where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBytes {
    /// The bytes to analyse.
    pub bytes: Vec<u8>,
    /// `true` when the bytes are an open buffer's text rather than the
    /// file on disk. Unsaved text never enters the fingerprint cache.
    pub unsaved: bool,
}

/// The text of every buffer an editor currently holds open, keyed by
/// canonical path so a symlinked or relative spelling of the same file
/// resolves to the same buffer.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OpenBuffers {
    /// Current buffer text per canonical path.
    buffers: HashMap<PathBuf, String>,
}

impl OpenBuffers {
    /// Records `text` as the open buffer for `path`, replacing any
    /// earlier text for the same file.
    pub fn open(&mut self, path: &Path, text: String) {
        let _previous = self.buffers.insert(canonical_key(path), text);
    }

    /// Applies `edits` in order to the open buffer for `path`. Returns
    /// `false` — and changes nothing — when no buffer is open for it.
    #[must_use]
    pub fn edit(&mut self, path: &Path, edits: &[BufferEdit]) -> bool {
        let Some(text) = self.buffers.get_mut(&canonical_key(path)) else {
            return false;
        };
        for edit in edits {
            splice(text, edit);
        }
        true
    }

    /// Forgets the buffer for `path`. Returns `true` when one was open.
    pub fn close(&mut self, path: &Path) -> bool {
        self.buffers.remove(&canonical_key(path)).is_some()
    }

    /// Canonical paths of every open buffer.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.buffers.keys().map(PathBuf::as_path)
    }

    /// [LIVE-OPEN-BUFFERS] Whether editor text remains authoritative for `path`.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        self.buffers.contains_key(&canonical_key(path))
    }

    /// The bytes a change pass must analyse for `path`: the open buffer
    /// when the editor holds one, otherwise the file on disk.
    ///
    /// # Errors
    ///
    /// Propagates [`CoreError::Io`] when no buffer is open for `path`
    /// and the file cannot be read from disk.
    pub fn read(&self, path: &Path) -> Result<SourceBytes, CoreError> {
        if let Some(text) = self.buffers.get(&canonical_key(path)) {
            return Ok(SourceBytes {
                bytes: text.as_bytes().to_vec(),
                unsaved: true,
            });
        }
        Ok(SourceBytes {
            bytes: read_source(path)?,
            unsaved: false,
        })
    }
}

/// Resolves `path` to the key the buffer map is indexed by. A path that
/// cannot be canonicalised (e.g. removed from disk while open) resolves
/// against its surviving parent so aliases (like `/var` on macOS) match.
pub(crate) fn canonical_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_error| canonicalise_parent(path))
}

/// Canonicalises the nearest surviving ancestor of a missing source path.
fn canonicalise_parent(path: &Path) -> PathBuf {
    path.parent().and_then(|parent| {
        parent.ancestors().find_map(|ancestor| {
            let canonical = fs::canonicalize(ancestor).ok()?;
            let suffix = path.strip_prefix(ancestor).ok()?;
            Some(canonical.join(suffix))
        })
    }).unwrap_or_else(|| path.to_path_buf())
}

/// Replaces the span `edit` names in `text` — the whole text when it
/// names none. Columns past a line end clamp to that line end, and an
/// end before the start is treated as an insertion at the start, so a
/// malformed range can never split a character or panic.
fn splice(text: &mut String, edit: &BufferEdit) {
    let Some((start, end)) = edit.range else {
        text.clone_from(&edit.text);
        return;
    };
    let start_byte = byte_for_line_col(text, start);
    let end_byte = byte_for_line_col(text, end).max(start_byte);
    text.replace_range(start_byte..end_byte, &edit.text);
}
