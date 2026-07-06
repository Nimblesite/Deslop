pub fn normalise_labels(labels: &mut Vec<String>) {
    let mut cleaned: Vec<String> = Vec::new();
    for label in labels.iter() {
        let trimmed = label.trim();
        cleaned.push(trimmed.to_lowercase());
        cleaned.push(trimmed.to_uppercase());
    }
    cleaned.sort();
    cleaned.dedup();
    labels.clear();
    labels.extend(cleaned);
}

pub fn describe_ledger(labels: &mut Vec<String>) -> usize {
    normalise_labels(labels);
    labels.first().map_or(0, String::len)
}
