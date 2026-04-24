//! Syntax-only boilerplate classification.
//!
//! Implements [PIPELINE-BOILERPLATE-FILTER]. The classifier is
//! intentionally language-aware at the AST-kind level and never scans
//! raw source text. Downstream stages use it to keep import/prologue
//! scaffolding and route decorators out of clone fingerprints while
//! retaining byte ranges for optional hygiene hints.

use crate::{
    ast::{ByteRange, NormalizedNode},
    state::FileId,
};

/// One syntax-only import/prologue range suppressed from clone ranking.
#[derive(Debug, Clone)]
pub struct BoilerplateRange {
    /// File containing the boilerplate range.
    pub file_id: FileId,
    /// Language id that produced the syntax kind.
    pub language: &'static str,
    /// Original byte range in the source file.
    pub byte_range: ByteRange,
}

/// Returns true when `kind` is an import/prologue carrier for `language`.
#[must_use]
pub fn is_import_boilerplate_carrier(language: &str, kind: &str) -> bool {
    match language {
        "csharp" => csharp_carrier(kind),
        "python" => python_carrier(kind),
        "rust" => rust_carrier(kind),
        _ => false,
    }
}

/// Returns true when the whole subtree is import/prologue boilerplate.
#[must_use]
pub fn is_import_boilerplate_only_subtree(language: &str, node: &NormalizedNode) -> bool {
    if is_import_boilerplate_carrier(language, node.kind) {
        return true;
    }
    !node.children.is_empty()
        && node
            .children
            .iter()
            .all(|child| is_import_boilerplate_only_subtree(language, child))
}

/// Collects byte ranges for every import/prologue carrier in `root`.
#[must_use]
pub fn collect_import_boilerplate_ranges(
    root: &NormalizedNode,
    language: &'static str,
) -> Vec<BoilerplateRange> {
    let mut ranges = Vec::new();
    collect_ranges(root, language, &mut ranges);
    ranges
}

/// Recursive carrier scan used by [`collect_import_boilerplate_ranges`].
fn collect_ranges(
    node: &NormalizedNode,
    language: &'static str,
    ranges: &mut Vec<BoilerplateRange>,
) {
    if is_import_boilerplate_carrier(language, node.kind) {
        ranges.push(range_for(node, language));
        return;
    }
    for child in &node.children {
        collect_ranges(child, language, ranges);
    }
}

/// Converts a carrier node into its reportable suppressed range.
fn range_for(node: &NormalizedNode, language: &'static str) -> BoilerplateRange {
    BoilerplateRange {
        file_id: node.file_id,
        language,
        byte_range: node.byte_range,
    }
}

/// C# import/prologue carriers.
fn csharp_carrier(kind: &str) -> bool {
    matches!(
        kind,
        "using_directive" | "file_scoped_namespace_declaration"
    )
}

/// Python import/prologue and framework-route carriers.
fn python_carrier(kind: &str) -> bool {
    matches!(
        kind,
        "import_statement" | "import_from_statement" | "decorator"
    )
}

/// Rust import/prologue carriers.
fn rust_carrier(kind: &str) -> bool {
    matches!(kind, "use_declaration" | "extern_crate_declaration")
}
