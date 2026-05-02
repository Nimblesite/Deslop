//! HTML escaping shared by renderers that emit source snippets.

/// HTML-escapes the four characters that can break out of content
/// context. Never emits entities for anything else so the output stays
/// human-diffable.
pub(super) fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}
