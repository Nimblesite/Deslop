//! Deterministic normalised-AST dump for `--debug-ast`.
//!
//! Developer tool: emits every [`NormalizedNode`] in the tree, one
//! per line, with `kind` and byte range. Indented by depth so visual
//! structure matches the tree structure. Identifiers and literals
//! appear as their collapsed kind (`__ident__`, `__literal__`) per
//! [PIPELINE-NORMALIZE-AST] — the dump shows the tree **after**
//! normalisation, which is what the fingerprinter sees.
//!
//! Used by the CLI's `--debug-ast` flag and by the golden-AST e2e
//! test that guards the C# normalisation contract from silent
//! regressions.

use std::fmt::Write as _;

use crate::ast::NormalizedNode;

/// Renders `root` as a stable, line-oriented text dump. Two spaces
/// of indent per level; `kind` then `[start..end]`.
#[must_use]
pub fn render_ast_dump(root: &NormalizedNode) -> String {
    let mut out = String::new();
    write_node(&mut out, root, 0);
    out
}

/// Writes one node plus its subtree to `out`.
fn write_node(out: &mut String, node: &NormalizedNode, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    let _ = writeln!(
        out,
        "{kind} [{start}..{end}]",
        kind = node.kind,
        start = node.byte_range.start,
        end = node.byte_range.end,
    );
    for child in &node.children {
        write_node(out, child, depth.saturating_add(1));
    }
}
