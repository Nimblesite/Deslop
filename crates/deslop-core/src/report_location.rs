//! Shared occurrence display locations for rendered reports and live editor surfaces.

use std::path::Path;

/// Formats an occurrence location for human-readable surfaces.
#[must_use]
pub fn format_occurrence(path: &Path, start_byte: usize, source: Option<&[u8]>) -> String {
    match source {
        Some(bytes) => {
            let position = position_for_byte(bytes, start_byte);
            format!("{}:{}:{}", path.display(), position.line, position.column)
        }
        None => format!("{}:line unavailable", path.display()),
    }
}

/// Badge text for an occurrence's diff membership
/// ([OUTPUT-SCHEMA-DIFF-TAGS]). `[in diff]` marks an occurrence whose
/// lines the verified diff added; `[existing]` marks one that predates
/// it. `None` on a run without `--diff`, so every no-diff surface
/// stays byte-identical. The one shared badge source — text, HTML,
/// and any future surface must call this, never restate the strings.
#[must_use]
pub fn diff_badge(in_diff: Option<bool>) -> Option<&'static str> {
    in_diff.map(|added| if added { "[in diff]" } else { "[existing]" })
}

/// One-indexed line/column pair.
struct DisplayPosition {
    /// One-indexed line.
    line: u64,
    /// One-indexed column.
    column: u64,
}

/// Converts a UTF-8 byte offset into a human editor position.
fn position_for_byte(source: &[u8], byte: usize) -> DisplayPosition {
    let safe = byte.min(source.len());
    let prefix = source.get(..safe).unwrap_or(&[]);
    let line = count_newlines(prefix).saturating_add(1);
    let column = column_after_last_newline(prefix).saturating_add(1);
    DisplayPosition { line, column }
}

/// Counts newline bytes in `bytes`.
fn count_newlines(bytes: &[u8]) -> u64 {
    let mut count = 0_u64;
    for byte in bytes {
        if *byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Counts bytes since the last newline.
fn column_after_last_newline(bytes: &[u8]) -> u64 {
    let count = bytes
        .iter()
        .rev()
        .take_while(|byte| **byte != b'\n')
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}
