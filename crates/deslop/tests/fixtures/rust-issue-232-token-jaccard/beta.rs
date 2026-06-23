// GH #232 fixture — beta side.
//
// `beta_lead` / `beta_tail` differ *structurally* (parameter arity)
// from their alpha counterparts so they never merge into the duplicated
// block. The two middle functions (`render_header` + `render_footer`)
// are byte-for-byte identical to alpha.rs.

fn beta_lead(a: i32, b: i32) -> i32 {
    a * b
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

fn beta_tail(z: i32) -> i32 {
    z - 7
}
