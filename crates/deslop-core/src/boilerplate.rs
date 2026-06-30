//! Syntax-only boilerplate classification.
//!
//! Implements [PIPELINE-BOILERPLATE-FILTER]. The classifier is
//! intentionally language-aware at the AST-kind level and never scans
//! raw source text. Downstream stages use it to keep import/prologue
//! scaffolding and route decorators out of clone fingerprints while
//! retaining byte ranges for optional hygiene hints.

use crate::{
    ast::{ByteRange, NormalizedNode},
    lang::shared::{IDENTIFIER_KIND, LITERAL_KIND},
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
        "dart" => dart_carrier(kind),
        "javascript" | "typescript" | "tsx" => ecmascript_carrier(kind),
        _ => false,
    }
}

/// Returns true when the whole subtree is import/prologue boilerplate.
#[must_use]
pub fn is_import_boilerplate_only_subtree(language: &str, node: &NormalizedNode) -> bool {
    if is_import_boilerplate_carrier(language, node.kind) {
        return true;
    }
    if is_language_specific_boilerplate_subtree(language, node) {
        return true;
    }
    !node.children.is_empty()
        && node
            .children
            .iter()
            .all(|child| is_import_boilerplate_only_subtree(language, child))
}

/// Returns true when `node` should be suppressed from clone fingerprints —
/// it carries, or consists entirely of, import/prologue boilerplate.
#[must_use]
pub fn is_boilerplate(language: Option<&str>, node: &NormalizedNode) -> bool {
    language.is_some_and(|lang| {
        is_import_boilerplate_carrier(lang, node.kind)
            || is_import_boilerplate_only_subtree(lang, node)
    })
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
        "future_import_statement" | "import_statement" | "import_from_statement" | "decorator"
    )
}

/// Rust import/prologue carriers.
fn rust_carrier(kind: &str) -> bool {
    matches!(kind, "use_declaration" | "extern_crate_declaration")
}

/// Dart import/export/part directives. `import_or_export` is the umbrella
/// node wrapping `library_import`, `library_export`, and `part`/`part of`
/// directives — all top-level scaffolding, never duplicate logic
/// (issues #96 / #150 / #155 carried over to Dart export-barrel files).
fn dart_carrier(kind: &str) -> bool {
    matches!(kind, "import_or_export")
}

/// JavaScript / TypeScript / TSX import carriers. An ES `import_statement`
/// is pure module prologue. Re-export barrels are handled as a subtree in
/// [`is_ecmascript_reexport`] rather than here because `export_statement`
/// also wraps real exported declarations (`export function`, `export
/// const`) that must never be suppressed.
fn ecmascript_carrier(kind: &str) -> bool {
    matches!(kind, "import_statement")
}

/// Language-specific wrappers whose children encode prologue-only syntax.
fn is_language_specific_boilerplate_subtree(language: &str, node: &NormalizedNode) -> bool {
    match language {
        "python" => is_python_prologue_subtree(node),
        "javascript" | "typescript" | "tsx" => is_ecmascript_reexport(node),
        _ => false,
    }
}

/// A re-export barrel (`export { a } from "./m"`, `export { a, b }`,
/// `export * as ns from "./m"`) is module scaffolding, not duplicate logic
/// — the JS/TS analogue of Dart's `library_export`. It must carry a named
/// export clause or namespace re-export; a bare-literal `export_statement`
/// is NOT a barrel, because after normalisation `export default "x"`, a
/// default-exported template literal, and `export default /re/` all reduce
/// to an `export_statement` over a single `__literal__` — indistinguishable from
/// `export * from "m"` — and suppressing those would erase real exported
/// logic. Requiring the export clause keeps every value export (and any
/// `function`/`class`/`const` declaration export) in the report; the only
/// barrel left unsuppressed is the few-node `export * from "m"`, which sits
/// far below any `min-nodes` and so never clusters anyway.
fn is_ecmascript_reexport(node: &NormalizedNode) -> bool {
    node.kind == "export_statement"
        && node.children.iter().any(is_ecmascript_export_list)
        && node.children.iter().all(is_ecmascript_reexport_child)
}

/// The named export clause or namespace re-export that distinguishes a
/// re-export barrel from a value export.
fn is_ecmascript_export_list(node: &NormalizedNode) -> bool {
    matches!(node.kind, "export_clause" | "namespace_export")
}

/// Direct children allowed inside a re-export barrel: the named export
/// clause, a namespace re-export, and the `from "..."` module-source
/// literal. Anything else marks a real exported declaration.
fn is_ecmascript_reexport_child(node: &NormalizedNode) -> bool {
    is_ecmascript_export_list(node) || node.kind == LITERAL_KIND
}

/// Python module docstrings and guarded import blocks are prologue syntax.
fn is_python_prologue_subtree(node: &NormalizedNode) -> bool {
    is_python_docstring_statement(node) || is_python_import_guard(node)
}

/// Python docstrings parse as expression statements containing a literal.
fn is_python_docstring_statement(node: &NormalizedNode) -> bool {
    node.kind == "expression_statement"
        && !node.children.is_empty()
        && node.children.iter().all(|child| child.kind == LITERAL_KIND)
}

/// `if TYPE_CHECKING:` blocks contain an identifier guard plus imports.
fn is_python_import_guard(node: &NormalizedNode) -> bool {
    matches!(node.kind, "if_statement" | "block")
        && !node.children.is_empty()
        && node.children.iter().all(is_python_import_guard_child)
}

/// Children allowed inside a Python import-only guard.
fn is_python_import_guard_child(node: &NormalizedNode) -> bool {
    matches!(node.kind, IDENTIFIER_KIND | LITERAL_KIND)
        || is_import_boilerplate_only_subtree("python", node)
}
