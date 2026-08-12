//! Wire-shaped `WorkspaceEdit` serialisation shared by the merge and
//! consolidation plans ([AUTOFIX-MERGE-MCP],
//! [AUTOFIX-CONSOLIDATE-SURFACE]): `documentChanges` form with LSP
//! line/UTF-16 positions and one change annotation per plan. The LSP
//! surface deserialises this back into a native edit at resolve time.

use std::path::{Path, PathBuf};

use crate::refactor::{emit::PlannedEdit, position::line_col_for_byte};

/// One file's byte-addressed edits, already in descending start order
/// so the client can apply them against original offsets.
pub(crate) struct FileEdits<'a> {
    /// Absolute path of the edited file.
    pub absolute_path: PathBuf,
    /// Full source bytes of that file.
    pub source: &'a [u8],
    /// Byte-addressed edits, descending start order.
    pub edits: Vec<PlannedEdit>,
}

/// Serialises the per-file edits as an LSP `WorkspaceEdit` JSON value,
/// or `None` when any file is not valid UTF-8.
pub(crate) fn workspace_edit_json(
    files: &[FileEdits<'_>],
    annotation_id: &str,
    label: &str,
) -> Option<serde_json::Value> {
    let changes: Option<Vec<serde_json::Value>> = files
        .iter()
        .map(|file| document_change(file, annotation_id))
        .collect();
    Some(serde_json::json!({
        "documentChanges": changes?,
        "changeAnnotations": {
            annotation_id: { "label": label, "needsConfirmation": false }
        }
    }))
}

/// One `TextDocumentEdit` for a single file.
fn document_change(file: &FileEdits<'_>, annotation_id: &str) -> Option<serde_json::Value> {
    let text = std::str::from_utf8(file.source).ok()?;
    let edits: Vec<serde_json::Value> = file
        .edits
        .iter()
        .map(|edit| lsp_edit(text, edit, annotation_id))
        .collect();
    Some(serde_json::json!({
        "textDocument": { "uri": file_uri(&file.absolute_path), "version": null },
        "edits": edits,
    }))
}

/// One annotated LSP `TextEdit` in line/UTF-16 coordinates.
fn lsp_edit(text: &str, edit: &PlannedEdit, annotation_id: &str) -> serde_json::Value {
    let start = line_col_for_byte(text, edit.start_byte);
    let end = line_col_for_byte(text, edit.end_byte);
    serde_json::json!({
        "range": {
            "start": { "line": start.line, "character": start.character },
            "end": { "line": end.line, "character": end.character }
        },
        "newText": edit.new_text,
        "annotationId": annotation_id,
    })
}

/// RFC 8089 `file://` URI for an absolute path. Backslash
/// separators become `/`, a Windows drive letter keeps its unencoded
/// colon behind the empty authority (`file:///C:/dir/file.rs`), and
/// every remaining byte outside the unreserved set is percent-encoded.
fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let absolute = rooted_path(&text);
    let drive_colon = drive_colon_index(absolute);
    let mut uri = String::from("file:///");
    for (index, byte) in absolute.bytes().enumerate() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(char::from(byte));
            }
            b':' if Some(index) == drive_colon => uri.push(':'),
            other => {
                use std::fmt::Write as _;
                let _formatted = write!(uri, "%{other:02X}");
            }
        }
    }
    uri
}

/// The portion of a slash-normalised absolute path that follows
/// `file:///`. Windows verbatim prefixes (`\\?\C:\…`, which
/// `fs::canonicalize` always returns) are stripped so canonical and
/// plain paths render identically. UNC shares are out of scope: they
/// need a URI authority rather than a rooted path, and no deterministic
/// test can drive one without a live network share.
fn rooted_path(text: &str) -> &str {
    text.strip_prefix("//?/")
        .unwrap_or(text)
        .trim_start_matches('/')
}

/// Byte index of the Windows drive-letter colon at the head of the path
/// (`C:` in `C:/dir/file.rs`), which RFC 8089 leaves unencoded.
fn drive_colon_index(absolute: &str) -> Option<usize> {
    let mut chars = absolute.chars();
    (chars.next()?.is_ascii_alphabetic() && chars.next()? == ':').then_some(1)
}
