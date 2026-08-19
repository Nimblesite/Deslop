//! Header-line classification and path extraction for the diff parser
//! ([PIPELINE-DIFF-INGEST]): the metadata lines `git diff` interleaves
//! between file and hunk headers, the no-newline annotation, and the
//! `+++ ` new-side target payload.

use crate::error::CoreError;

use super::{parse_error, quoting::unquote_c_path};

/// True for the metadata lines `git diff` interleaves between file and
/// hunk headers. Blank lines separate sections in some producers.
/// `copy from` / `copy to` are deliberately absent: they are payload
/// ([`super::FileCopy`]), not inert metadata — swallowing them here
/// hid every metadata-only copy from the diff scope.
pub(super) fn is_metadata_line(line: &str) -> bool {
    const METADATA_PREFIXES: &[&str] = &[
        "--- ",
        "index ",
        "new file mode",
        "deleted file mode",
        "old mode",
        "new mode",
        "similarity index",
        "dissimilarity index",
        "rename from",
        "rename to",
        "Binary files ",
        "GIT binary patch",
    ];
    line.is_empty()
        || METADATA_PREFIXES
            .iter()
            .any(|prefix| line.starts_with(prefix))
}

/// Which half of a git copy pair a metadata line names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CopySide {
    /// `copy from ` — the old-side source path.
    From,
    /// `copy to ` — the new-side destination path.
    To,
}

/// Splits a `copy from ` / `copy to ` metadata line into its side and
/// raw path payload, or `None` for every other line.
pub(super) fn copy_line(line: &str) -> Option<(CopySide, &str)> {
    if let Some(rest) = line.strip_prefix("copy from ") {
        return Some((CopySide::From, rest));
    }
    line.strip_prefix("copy to ")
        .map(|rest| (CopySide::To, rest))
}

/// Resolves a `copy from ` / `copy to ` payload to the path it names.
/// Git writes these without `a/`/`b/` prefixes and without trailing
/// timestamps, C-quoting them exactly as it does `+++` targets.
///
/// # Errors
///
/// Returns [`CoreError::DiffParse`] when the path is C-quoted but
/// malformed, or names nothing at all — a guessed copy path would
/// resolve the wholesale duplication against the wrong file.
pub(super) fn copy_path(line_no: usize, raw: &str) -> Result<String, CoreError> {
    let path = match raw.strip_prefix('"') {
        Some(quoted) => unquote_c_path(line_no, quoted)?,
        None => raw.to_owned(),
    };
    if path.is_empty() {
        return Err(parse_error(line_no, "copy metadata names no path"));
    }
    Ok(path)
}

/// True for `git`'s "\ No newline at end of file" annotation, with or
/// without a CRLF terminator.
pub(super) fn is_no_newline_marker(line: &str) -> bool {
    line.strip_suffix('\r').unwrap_or(line) == "\\ No newline at end of file"
}

/// Resolves a `+++ ` payload to the new-side path
/// ([PIPELINE-DIFF-INGEST]): trailing tab-separated timestamps (plain
/// `diff -u` output) are dropped, C-quoted paths are unquoted, the `b/`
/// prefix is stripped, and `/dev/null` means the file was deleted.
/// A path left quoted would match nothing in the corpus and silently
/// drop that file's added lines. Pinned by
/// `c_quoted_new_side_path_is_unquoted`.
///
/// # Errors
///
/// Returns [`CoreError::DiffParse`] when a C-quoted path is malformed
/// or decodes to invalid UTF-8 — guessing at a filename would silently
/// drop that file from the scope rather than fail loudly.
pub(super) fn new_side_path(line_no: usize, raw: &str) -> Result<Option<String>, CoreError> {
    let target = raw.split('\t').next().unwrap_or(raw);
    let path = match target.strip_prefix('"') {
        Some(quoted) => unquote_c_path(line_no, quoted)?,
        None => target.to_owned(),
    };
    if path == "/dev/null" {
        return Ok(None);
    }
    let unprefixed = path.strip_prefix("b/").unwrap_or(&path);
    Ok(Some(unprefixed.to_owned()))
}
