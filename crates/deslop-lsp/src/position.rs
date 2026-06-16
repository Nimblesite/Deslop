//! Byte offset ↔ LSP `Position` conversion ([LSP-DIAGNOSTICS]).
//!
//! The LSP protocol encodes positions as `(line, character)` with
//! `character` counting UTF-16 code units from the last newline per
//! the LSP 3.17 spec. This module centralises that arithmetic so the
//! additive editor surfaces (diagnostics, code-lens) project byte
//! ranges onto the same line grid.

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

#[cfg(test)]
#[allow(clippy::missing_docs_in_private_items)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result};

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
