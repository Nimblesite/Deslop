//! `Result`-based twin of `parse_option.rs`. Same three parsers, same
//! semantics, failures carry a structured error instead of `None`.

#[derive(Debug)]
pub enum ParseError {
    Empty,
    NonDigit(char),
    Overflow,
    Zero,
    UnrecognisedBoolean,
}

pub fn parse_positive_int(input: &str) -> Result<u64, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut value: u64 = 0;
    for ch in trimmed.chars() {
        let digit = ch.to_digit(10).ok_or(ParseError::NonDigit(ch))?;
        let scaled = value.checked_mul(10).ok_or(ParseError::Overflow)?;
        value = scaled
            .checked_add(u64::from(digit))
            .ok_or(ParseError::Overflow)?;
    }
    if value == 0 {
        Err(ParseError::Zero)
    } else {
        Ok(value)
    }
}

pub fn parse_boolean(input: &str) -> Result<bool, ParseError> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" => Ok(true),
        "false" | "no" | "n" | "0" => Ok(false),
        _ => Err(ParseError::UnrecognisedBoolean),
    }
}

pub fn parse_first_word(input: &str) -> Result<String, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }
    let word = trimmed
        .split_whitespace()
        .next()
        .ok_or(ParseError::Empty)?;
    Ok(word.to_owned())
}
