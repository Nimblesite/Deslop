//! [CORPUS-SCORE-RENDER] The cells a scorecard table is built from.
//!
//! Every figure reaches the document through one of these, so no two tables can
//! spell a percentage, a duration or an absent measurement differently. Nothing
//! here derives a number: each function formats one that was already computed.

/// Milliseconds in a second, so the wall-time column reads in seconds.
const MS_PER_SECOND: f64 = 1000.0;
/// What an absent or unmeasured figure prints as. Never zero: a run nobody
/// measured must not read as a run that cost nothing.
pub(super) const ABSENT: &str = "—";

/// One markdown table row.
pub(super) fn row(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// A header row and the divider under it, so a table can never disagree with
/// its own width.
pub(super) fn header(cells: &[String]) -> Vec<String> {
    let divider = format!("|{}|", vec!["---"; cells.len()].join("|"));
    vec![row(cells), divider]
}

/// A percentage, or an explicit absence. Nothing judged must never print as a
/// perfect score.
pub(super) fn score_cell(value: Option<f64>) -> String {
    value.map_or_else(
        || "not judged".to_owned(),
        |percent| format!("{percent:.1}%"),
    )
}

/// Milliseconds rendered as seconds.
pub(super) fn seconds(ms: u64) -> String {
    format!("{:.2} s", as_f64(ms) / MS_PER_SECOND)
}

/// Mebibytes, or an explicit absence.
pub(super) fn megabytes(value: Option<u64>) -> String {
    value.map_or_else(|| ABSENT.to_owned(), |mb| format!("{mb} MB"))
}

/// CPU seconds, or an explicit absence.
pub(super) fn cpu(value: Option<f64>) -> String {
    value.map_or_else(|| ABSENT.to_owned(), |secs| format!("{secs:.2} s"))
}

/// Widening that keeps the lossy cast in one reviewed place.
fn as_f64(value: u64) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

/// A signed count, so a change column reads without the reader subtracting.
pub(super) fn signed(delta: i64) -> String {
    if delta == 0 {
        ABSENT.to_owned()
    } else {
        format!("{delta:+}")
    }
}

/// A signed measurement in its unit, or an explicit absence.
pub(super) fn signed_amount(delta: Option<i64>, unit: &str) -> String {
    match delta {
        Some(delta) if delta != 0 => format!("{delta:+} {unit}"),
        _ => ABSENT.to_owned(),
    }
}

/// A signed fractional measurement in its unit, or an explicit absence.
pub(super) fn signed_fraction(delta: Option<f64>, unit: &str) -> String {
    delta.map_or_else(|| ABSENT.to_owned(), |delta| format!("{delta:+.1} {unit}"))
}

/// A count with the noun that agrees with it, so a one-repository run never
/// reads as `1 repositories`.
pub(super) fn counted(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("**{count} {noun}**")
}
