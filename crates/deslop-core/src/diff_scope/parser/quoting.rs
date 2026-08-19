//! C-quoted path decoding for the diff parser
//! ([PIPELINE-DIFF-INGEST]).
//!
//! `git` C-quotes any path carrying non-ASCII bytes, a double quote,
//! or a backslash, escaping the bytes in octal — so one accented or
//! CJK filename produces the quoted form on every diff of that repo,
//! and a path left quoted would match nothing in the corpus and
//! silently drop that file's added lines.

use std::{iter::Peekable, str::Bytes};

use crate::error::CoreError;

use super::parse_error;

/// Decodes the body of a C-quoted path (the leading `"` already
/// stripped) into the bytes `git` escaped, then validates them as
/// UTF-8. The closing quote is required — an unterminated quote means
/// the line is not the grammar this parser accepts.
pub(super) fn unquote_c_path(line_no: usize, quoted: &str) -> Result<String, CoreError> {
    let inner = quoted
        .strip_suffix('"')
        .ok_or_else(|| parse_error(line_no, "C-quoted path is missing its closing quote"))?;
    let mut decoded: Vec<u8> = Vec::with_capacity(inner.len());
    let mut source = inner.bytes().peekable();
    while let Some(byte) = source.next() {
        if byte == b'\\' {
            decoded.push(unescape(line_no, &mut source)?);
            continue;
        }
        decoded.push(byte);
    }
    String::from_utf8(decoded)
        .map_err(|_| parse_error(line_no, "C-quoted path decodes to invalid UTF-8"))
}

/// Decodes one escape sequence, positioned just after its backslash.
fn unescape(line_no: usize, source: &mut Peekable<Bytes<'_>>) -> Result<u8, CoreError> {
    let Some(first) = source.next() else {
        return Err(parse_error(line_no, "C-quoted path ends inside an escape"));
    };
    if is_octal_digit(first) {
        return octal_escape(line_no, first, source);
    }
    simple_escape(first).ok_or_else(|| {
        parse_error(
            line_no,
            &format!(
                "unrecognised escape '\\{}' in C-quoted path",
                char::from(first)
            ),
        )
    })
}

/// Accumulates up to three octal digits into one byte; `git` writes
/// every non-ASCII byte this way. A value past `0o377` names no byte,
/// so it is refused rather than truncated into a different filename.
fn octal_escape(
    line_no: usize,
    first: u8,
    source: &mut Peekable<Bytes<'_>>,
) -> Result<u8, CoreError> {
    let mut value = u32::from(first.saturating_sub(b'0'));
    for _digit_slot in 0..2 {
        let Some(digit) = source.next_if(|byte| is_octal_digit(*byte)) else {
            break;
        };
        value = value
            .saturating_mul(8)
            .saturating_add(u32::from(digit.saturating_sub(b'0')));
    }
    u8::try_from(value)
        .map_err(|_| parse_error(line_no, "octal escape in C-quoted path exceeds one byte"))
}

/// Maps one non-octal escape character to the byte it denotes, per
/// `git`'s C-style quoting table.
fn simple_escape(escape: u8) -> Option<u8> {
    match escape {
        b'\\' => Some(b'\\'),
        b'"' => Some(b'"'),
        b'a' => Some(0x07),
        b'b' => Some(0x08),
        b'f' => Some(0x0C),
        b'n' => Some(b'\n'),
        b'r' => Some(b'\r'),
        b't' => Some(b'\t'),
        b'v' => Some(0x0B),
        _ => None,
    }
}

/// True for the octal digits `0`–`7`.
fn is_octal_digit(byte: u8) -> bool {
    (b'0'..=b'7').contains(&byte)
}
