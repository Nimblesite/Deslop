//! Sentinel-value twin of `parse_option.rs` / `parse_result.rs`. Same
//! three parsers, same semantics, failures collapse to sentinels.
//! This is the C-idiom variant — classic Type-4 with respect to the
//! Option / Result implementations.

pub const INVALID_INT: u64 = 0;
pub const INVALID_BOOL: i8 = -1;
pub const EMPTY_WORD: &str = "";

pub fn parse_positive_int(input: &str) -> u64 {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return INVALID_INT;
    }
    let mut value: u64 = 0;
    for ch in trimmed.chars() {
        if !ch.is_ascii_digit() {
            return INVALID_INT;
        }
        let digit = u64::from(ch as u8 - b'0');
        match value.checked_mul(10).and_then(|scaled| scaled.checked_add(digit)) {
            Some(next) => value = next,
            None => return INVALID_INT,
        }
    }
    value
}

pub fn parse_boolean(input: &str) -> i8 {
    let lower = input.trim().to_ascii_lowercase();
    if lower == "true" || lower == "yes" || lower == "y" || lower == "1" {
        1
    } else if lower == "false" || lower == "no" || lower == "n" || lower == "0" {
        0
    } else {
        INVALID_BOOL
    }
}

pub fn parse_first_word(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return EMPTY_WORD.to_owned();
    }
    let mut out = String::new();
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            break;
        }
        out.push(ch);
    }
    out
}
