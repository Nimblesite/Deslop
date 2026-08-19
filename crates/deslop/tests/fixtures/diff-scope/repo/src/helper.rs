pub fn parse_config(raw: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            pairs.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    pairs
}
