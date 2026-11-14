//! [CONFIG-EXCLUDE-TREE-SITTER] `tree-sitter generate` output is not source.
//!
//! The generator writes `src/parser.c` with no banner on older releases,
//! and copies its runtime headers to `src/tree_sitter/` beside every
//! grammar. A 164k-line parser table then reads as hand-written C and its
//! headers as copies of each other (gh #621). The layout gives it away:
//! a `src/` directory holding the parser, the `grammar.json` it was
//! generated from, and `tree_sitter/parser.h`.
//!
//! A file that cannot be inspected is **not** excluded: the rule can only
//! remove code from the analysis, so when it cannot tell, it declines to.

use std::path::Path;

/// The directory `tree-sitter generate` writes into.
const GRAMMAR_SOURCE_DIR: &str = "src";
/// The generated parser table.
const GENERATED_PARSER: &str = "parser.c";
/// The compiled grammar the parser was generated from.
const GRAMMAR_JSON: &str = "grammar.json";
/// The generated node-type catalogue.
const NODE_TYPES: &str = "node-types.json";
/// The directory of runtime headers copied beside each grammar.
const RUNTIME_HEADER_DIR: &str = "tree_sitter";
/// The runtime header every generated parser includes.
const RUNTIME_HEADER: &str = "parser.h";
/// Extension of the copied runtime headers.
const HEADER_EXTENSION: &str = "h";
/// Files `tree-sitter generate` writes straight into `src/`. A hand-written
/// `scanner.c` sits beside them and is not listed.
const GENERATED_FILES: [&str; 3] = [GENERATED_PARSER, GRAMMAR_JSON, NODE_TYPES];

/// True when `path` is a file `tree-sitter generate` wrote: one of
/// [`GENERATED_FILES`] in a grammar's `src/`, or a header under its
/// `src/tree_sitter/`.
pub(super) fn is_tree_sitter_output(path: &Path) -> bool {
    let (Some(name), Some(dir)) = (file_name(path), path.parent()) else {
        return false;
    };
    if GENERATED_FILES.contains(&name) {
        return is_grammar_source(dir);
    }
    let header = path.extension().and_then(|ext| ext.to_str()) == Some(HEADER_EXTENSION);
    header
        && file_name(dir) == Some(RUNTIME_HEADER_DIR)
        && dir.parent().is_some_and(is_grammar_source)
}

/// True when `dir` is a grammar's generated `src/`: it holds the parser,
/// the grammar it was generated from, and the runtime header beside them.
fn is_grammar_source(dir: &Path) -> bool {
    file_name(dir) == Some(GRAMMAR_SOURCE_DIR)
        && dir.join(GENERATED_PARSER).is_file()
        && dir.join(GRAMMAR_JSON).is_file()
        && dir.join(RUNTIME_HEADER_DIR).join(RUNTIME_HEADER).is_file()
}

/// The final path component as UTF-8.
fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}
