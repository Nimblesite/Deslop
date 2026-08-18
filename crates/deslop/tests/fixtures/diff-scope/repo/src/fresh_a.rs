pub fn fresh_render(rows: &[(String, i64)]) -> String {
    let mut out = String::new();
    for (name, count) in rows {
        if *count > 0 {
            out.push_str(name);
            out.push(':');
            out.push_str(&count.to_string());
            out.push('\n');
        }
    }
    out
}
