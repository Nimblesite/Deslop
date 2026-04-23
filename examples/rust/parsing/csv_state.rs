//! Explicit-state-machine CSV parser. Semantically equivalent to the
//! hand-written parser in `csv_hand.rs` but driven by an enum rather
//! than ad-hoc booleans. Same behavior, different code [Type-4]
//! cluster.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    FieldStart,
    InField,
    InQuoted,
    QuoteInQuoted,
}

pub fn parse_row(input: &str) -> Vec<String> {
    let mut fields: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut state = State::FieldStart;
    for ch in input.chars() {
        state = advance(state, ch, &mut current, &mut fields);
    }
    if state == State::QuoteInQuoted {
        // Trailing quote closed the last field.
    }
    fields.push(current);
    fields
}

fn advance(state: State, ch: char, current: &mut String, fields: &mut Vec<String>) -> State {
    match (state, ch) {
        (State::FieldStart, '"') => State::InQuoted,
        (State::FieldStart, ',') => {
            fields.push(std::mem::take(current));
            State::FieldStart
        }
        (State::FieldStart, other) => {
            current.push(other);
            State::InField
        }
        (State::InField, ',') => {
            fields.push(std::mem::take(current));
            State::FieldStart
        }
        (State::InField, other) => {
            current.push(other);
            State::InField
        }
        (State::InQuoted, '"') => State::QuoteInQuoted,
        (State::InQuoted, other) => {
            current.push(other);
            State::InQuoted
        }
        (State::QuoteInQuoted, '"') => {
            current.push('"');
            State::InQuoted
        }
        (State::QuoteInQuoted, ',') => {
            fields.push(std::mem::take(current));
            State::FieldStart
        }
        (State::QuoteInQuoted, other) => {
            current.push(other);
            State::InField
        }
    }
}
