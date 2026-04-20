//! Naive `split`-based CSV parser. Does not honour quoted commas —
//! incomplete compared to `csv_hand.rs`, but sold as "CSV parsing" in
//! a lot of real code. A Type-3 clone of the hand-written parser
//! (same intent, missing edge cases). The embedding pass should pair
//! it with `csv_hand.rs` with non-zero cosine even though token
//! Jaccard is low.

pub fn parse_row(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|field| {
            let trimmed = field.trim();
            if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
                trimmed[1..trimmed.len().saturating_sub(1)].replace("\"\"", "\"")
            } else {
                trimmed.to_owned()
            }
        })
        .collect()
}
