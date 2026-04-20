//! Byte offset ↔ LSP `Position` conversion ([LSP-DIAGNOSTICS]).
//!
//! The LSP protocol encodes positions as `(line, character)` with
//! `character` counting UTF-16 code units from the last newline per
//! the LSP 3.17 spec. This module centralises that arithmetic so every
//! provider (diagnostics, code-lens, hover, goto-definition) projects
//! byte ranges onto the same line grid.

use tower_lsp::lsp_types::Position;

/// Returns the zero-indexed `Position` corresponding to `byte_offset`
/// in `source`. Offsets past the end of `source` clamp to the last
/// position of the final line.
#[must_use]
pub fn position_for_byte(source: &str, byte_offset: usize) -> Position {
    let clamped = byte_offset.min(source.len());
    let prefix = source.get(..clamped).unwrap_or(source);
    let line_count =
        u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count()).unwrap_or(u32::MAX);
    let last_line_start = prefix
        .rfind('\n')
        .map_or(0, |newline_index| newline_index.saturating_add(1));
    let last_line_text = prefix.get(last_line_start..).unwrap_or("");
    let character = u32::try_from(last_line_text.encode_utf16().count()).unwrap_or(u32::MAX);
    Position {
        line: line_count,
        character,
    }
}

/// Inverse of [`position_for_byte`]. Walks `source` counting lines
/// then UTF-16 code units to re-derive the byte offset.
#[must_use]
pub fn byte_for_position(source: &str, position: Position) -> usize {
    let mut byte_cursor: usize = 0;
    let mut remaining_lines = position.line;
    for (index, byte) in source.bytes().enumerate() {
        if remaining_lines == 0 {
            byte_cursor = index;
            break;
        }
        if byte == b'\n' {
            remaining_lines = remaining_lines.saturating_sub(1);
            if remaining_lines == 0 {
                byte_cursor = index.saturating_add(1);
                break;
            }
        }
    }
    if remaining_lines > 0 {
        return source.len();
    }
    advance_utf16_units(source, byte_cursor, position.character)
}

/// Advances from `start_byte` by `character` UTF-16 code units in
/// `source`. Clamps at the end of the file or the next newline.
fn advance_utf16_units(source: &str, start_byte: usize, character: u32) -> usize {
    let tail = source.get(start_byte..).unwrap_or("");
    let mut units_consumed: u32 = 0;
    for (local_offset, character_value) in tail.char_indices() {
        if character_value == '\n' || units_consumed >= character {
            return start_byte.saturating_add(local_offset).min(source.len());
        }
        let unit_width = u32::try_from(character_value.len_utf16()).unwrap_or(1);
        units_consumed = units_consumed.saturating_add(unit_width);
    }
    source.len()
}
