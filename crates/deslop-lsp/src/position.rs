//! Byte offset ↔ LSP `Position` adapters ([LSP-DIAGNOSTICS]).
//!
//! The arithmetic lives in `deslop_core::refactor::position` (the
//! merge engine renders wire `WorkspaceEdit`s with the same rules);
//! this module only projects [`LineCol`] onto
//! `tower_lsp::lsp_types::Position` for the editor surfaces.

use deslop_core::refactor::position::{byte_for_line_col, line_col_for_byte, LineCol};
use tower_lsp::lsp_types::Position;

/// Returns the zero-indexed `Position` corresponding to `byte_offset`
/// in `source`. Offsets past the end of `source` clamp to the last
/// position of the final line.
#[must_use]
pub fn position_for_byte(source: &str, byte_offset: usize) -> Position {
    let line_col = line_col_for_byte(source, byte_offset);
    Position {
        line: line_col.line,
        character: line_col.character,
    }
}

/// Returns the byte offset corresponding to the zero-indexed LSP
/// `position` in `source` — the inverse of [`position_for_byte`], with
/// the same UTF-16 column semantics. Positions past the end of a line
/// clamp to the line end; lines past the end of `source` clamp to
/// `source.len()`.
#[must_use]
pub fn byte_for_position(source: &str, position: Position) -> usize {
    byte_for_line_col(
        source,
        LineCol {
            line: position.line,
            character: position.character,
        },
    )
}

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};

    #[test]
    fn byte_for_position_inverts_position_for_byte() {
        let source = "ab\ncd\u{1F600}ef\ngh";
        for byte_offset in [0, 1, 3, 4, 12, source.len()] {
            let position = position_for_byte(source, byte_offset);
            assert_eq!(
                byte_for_position(source, position),
                byte_offset,
                "round-trip at byte {byte_offset}"
            );
        }
        let past_line_end = Position {
            line: 0,
            character: 99,
        };
        assert_eq!(
            byte_for_position(source, past_line_end),
            2,
            "columns past the line end clamp to the newline"
        );
        let past_last_line = Position {
            line: 9,
            character: 0,
        };
        assert_eq!(
            byte_for_position(source, past_last_line),
            source.len(),
            "lines past EOF clamp to the source length"
        );
    }

    #[test]
    fn position_for_byte_covers_newlines_utf16_clamping_and_offsets() -> Result<()> {
        let single = "hello world";
        assert_eq!(
            position_for_byte(single, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            position_for_byte(single, 5),
            Position {
                line: 0,
                character: 5
            }
        );
        assert_eq!(
            position_for_byte(single, 999),
            Position {
                line: 0,
                character: 11
            },
            "offsets past EOF clamp to the final character"
        );
        let multi = "ab\ncd\nef";
        assert_eq!(
            position_for_byte(multi, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            position_for_byte(multi, 3),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            position_for_byte(multi, 4),
            Position {
                line: 1,
                character: 1
            }
        );
        assert_eq!(
            position_for_byte(multi, 7),
            Position {
                line: 2,
                character: 1
            }
        );
        let emoji = "A\u{1F600}B";
        let after_emoji = emoji.find('B').ok_or_else(|| anyhow!("B is present"))?;
        let pos = position_for_byte(emoji, after_emoji);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3, "emoji occupies two UTF-16 code units");
        assert_eq!(
            position_for_byte("", 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            position_for_byte("", 99),
            Position {
                line: 0,
                character: 0
            }
        );
        Ok(())
    }
}
