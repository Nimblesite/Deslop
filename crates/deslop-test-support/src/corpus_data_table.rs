//! [CORPUS-PRECISION] Is this ranked cluster a table of numbers rather than
//! logic?
//!
//! A block that is almost entirely digits and separators — a font-metrics
//! table, a list of coordinates — repeats because it enumerates rows, not
//! because anybody copied logic. Extracting it changes nothing, so it must
//! not sit at the head of the report ahead of copy-paste a reader can act on.
//!
//! The judging is separated from the clone on disk deliberately. The check
//! used to live inside the corpus suite, where every test is `#[ignore]`d and
//! needs a pinned clone before it can run, so nothing asserted what it said —
//! and it spent that time reporting every breach as "0 occurrences" because it
//! read a field the report does not carry (gh #540). Its sibling in
//! [`crate::corpus_precision`] had the same defect, fixed there and left here.

use serde_json::Value;

use crate::corpus::{field_u64, Failure, OCCURRENCE_COUNT};

/// Number of top-ranked clusters subjected to the language-agnostic precision
/// checks. Ranking is the product, so the head of the report is where a false
/// positive does the most damage.
pub const RANKED_HEAD: usize = 10;

/// Fraction of non-whitespace characters that must be digits or data
/// punctuation before a snippet counts as a data table rather than logic.
/// Real logic carries identifiers and keywords, so it lands far below this.
pub const DATA_TABLE_RATIO: f64 = 0.6;

/// How much of the offending text the failure quotes, so a reader can see
/// what was ranked without opening the report.
const SNIPPET_CHARS: usize = 70;

/// The check id this module reports under.
const CHECK: &str = "data_table_rank";

/// [CORPUS-PRECISION] Judges one ranked cluster, given the text of its first
/// occurrence. `position` is the cluster's zero-based index in the report.
///
/// Returns the failure when the cluster is a data table ranked as logic, and
/// `None` when it is ordinary duplicated code.
///
/// No cluster is exempted by a category it declares. [RANK-CATEGORY] is
/// explicit that a detection-time finding kind "is not carried as
/// clone-cluster similarity metadata", and the wire model carries no such
/// field, so the exemption this check used to apply could never open: it read
/// `category`, got nothing, and told the reader the cluster was "categorised
/// `absent`" — a fact about a field that does not exist. What the check
/// actually asserts is what it now says: a table of digits is not logic, and
/// the head of the report is for logic.
#[must_use]
pub fn data_table_failure(position: usize, cluster: &Value, text: &str) -> Option<Failure> {
    let ratio = data_character_ratio(text);
    if ratio < DATA_TABLE_RATIO {
        return None;
    }
    // One-based, like the report's own `rank` field: a message that says
    // "rank 9" about the tenth cluster sends the reader to the wrong finding.
    let rank = position.saturating_add(1);
    Some(Failure::new(
        CHECK,
        format!(
            "rank {rank}: cluster of {} occurrences is {:.0}% numeric/separator \
             characters — a data table ranked at full logic weight, ahead of \
             copy-paste a reader can act on. Snippet: {}",
            field_u64(cluster, OCCURRENCE_COUNT),
            ratio * 100.0,
            text.chars()
                .take(SNIPPET_CHARS)
                .collect::<String>()
                .replace('\n', " "),
        ),
    ))
}

/// Fraction of non-whitespace characters that are digits or the punctuation
/// that separates literals in a table.
#[must_use]
pub fn data_character_ratio(text: &str) -> f64 {
    let significant = || text.chars().filter(|character| !character.is_whitespace());
    let total = significant().count();
    if total == 0 {
        return 0.0;
    }
    let data = significant()
        .filter(|character| is_data_character(*character))
        .count();
    as_f64(data) / as_f64(total)
}

/// True for digits and the punctuation that separates literals in a table.
fn is_data_character(character: char) -> bool {
    character.is_ascii_digit() || matches!(character, ';' | ',' | '|' | '[' | ']' | '.' | '-')
}

/// Widens a count to `f64`, saturating rather than wrapping.
fn as_f64(count: usize) -> f64 {
    u32::try_from(count).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests;
