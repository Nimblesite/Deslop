//! Byte offset ↔ line/character conversion (LSP 3.17 semantics).
//!
//! Lives in `deslop-core` so the merge engine can render a real
//! `WorkspaceEdit` for the wire ([AUTOFIX-MERGE-MCP]); the LSP layer
//! adapts [`LineCol`] onto `tower_lsp::lsp_types::Position`. The
//! `character` column counts UTF-16 code units from the last newline,
//! per the LSP spec.

/// A zero-indexed line/character pair with UTF-16 column semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// Zero-indexed line.
    pub line: u32,
    /// UTF-16 code units from the line start.
    pub character: u32,
}

/// Returns the zero-indexed [`LineCol`] corresponding to `byte_offset`
/// in `source`. Offsets past the end of `source` clamp to the last
/// position of the final line.
#[must_use]
pub fn line_col_for_byte(source: &str, byte_offset: usize) -> LineCol {
    let clamped = byte_offset.min(source.len());
    let prefix = source.get(..clamped).unwrap_or(source);
    let line =
        u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count()).unwrap_or(u32::MAX);
    let last_line_start = prefix
        .rfind('\n')
        .map_or(0, |newline_index| newline_index.saturating_add(1));
    let last_line_text = prefix.get(last_line_start..).unwrap_or("");
    let character = u32::try_from(last_line_text.encode_utf16().count()).unwrap_or(u32::MAX);
    LineCol { line, character }
}

/// Returns the byte offset corresponding to the zero-indexed
/// `position` in `source` — the inverse of [`line_col_for_byte`].
/// Positions past the end of a line clamp to the line end; lines past
/// the end of `source` clamp to `source.len()`.
#[must_use]
pub fn byte_for_line_col(source: &str, position: LineCol) -> usize {
    let Some(line_start) = line_start_offset(source, position.line) else {
        return source.len();
    };
    let line = source.get(line_start..).unwrap_or("");
    let mut utf16_column = 0_u32;
    for (offset, character) in line.char_indices() {
        if utf16_column >= position.character || character == '\n' {
            return line_start.saturating_add(offset);
        }
        let width = u32::try_from(character.len_utf16()).unwrap_or(u32::MAX);
        utf16_column = utf16_column.saturating_add(width);
    }
    source.len()
}

/// Byte offset where zero-indexed `line` begins, or `None` when
/// `source` has fewer lines.
fn line_start_offset(source: &str, line: u32) -> Option<usize> {
    let mut start = 0_usize;
    for _ in 0..line {
        let rest = source.get(start..)?;
        let newline = rest.find('\n')?;
        start = start.saturating_add(newline).saturating_add(1);
    }
    Some(start)
}
