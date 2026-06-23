// GH #232 fixture — alpha side.
//
// `alpha_lead` / `alpha_tail` differ *structurally* (parameter arity)
// from their beta counterparts so they never merge into the duplicated
// block. The two middle functions (`render_header` + `render_footer`)
// are byte-for-byte identical to beta.rs. The clone the pipeline reports
// is the synthetic sibling **window** spanning those two functions — a
// byte range that matches no single AST node. That window must still
// report a true token-Jaccard (~1.0), not the 0.0 fallback of GH #232.

fn alpha_lead(x: i32) -> i32 {
    x + 1
}

fn render_header(buf: &mut String, title: &str, level: usize) {
    for _ in 0..level {
        buf.push('#');
    }
    buf.push(' ');
    buf.push_str(title);
    buf.push('\n');
}

fn render_footer(buf: &mut String, note: &str, count: usize) {
    for _ in 0..count {
        buf.push('-');
    }
    buf.push(' ');
    buf.push_str(note);
    buf.push('\n');
}

fn alpha_tail(p: i32, q: i32, r: i32) -> i32 {
    p + q + r
}
