//! Hand-written CSV row parser. Supports quoted fields and escaped
//! double-quotes. Paired with `csv_split.rs` (naive `split`-based
//! parser) and `csv_state.rs` (explicit state machine) — three same
//! behavior, different code [Type-4] variants of the same parser.

pub fn parse_row(input: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                let _ = chars.next();
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }
    fields.push(current);
    fields
}
