//! Parser that reports failure via `Option::None`. Paired with
//! `parse_result.rs` (same semantics via `Result`) and
//! `parse_sentinel.rs` (same semantics via sentinel values). Three
//! different error-handling idioms, same underlying logic — a
//! Type-4 family.

pub fn parse_positive_int(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut value: u64 = 0;
    for ch in trimmed.chars() {
        let Some(digit) = ch.to_digit(10) else {
            return None;
        };
        let scaled = value.checked_mul(10)?;
        value = scaled.checked_add(u64::from(digit))?;
    }
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

pub fn parse_boolean(input: &str) -> Option<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" => Some(true),
        "false" | "no" | "n" | "0" => Some(false),
        _ => None,
    }
}

pub fn parse_first_word(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let word = trimmed.split_whitespace().next()?;
    Some(word.to_owned())
}
