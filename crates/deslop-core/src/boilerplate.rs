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
        "osprey-python" => python_import_carrier(kind),
        "rust" => rust_carrier(kind),
        "dart" => dart_carrier(kind),
        "javascript" | "typescript" | "tsx" | "osprey" | "osprey-ml" => ecmascript_carrier(kind),
        "go" => go_carrier(kind),
        "c" => c_carrier(kind),
        "php" => php_carrier(kind),
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
    kind == "decorator" || python_import_carrier(kind)
}

/// Osprey decorators declare language semantics and remain structural.
fn python_import_carrier(kind: &str) -> bool {
    matches!(
        kind,
        "future_import_statement" | "import_statement" | "import_from_statement"
    )
}

/// Rust import/prologue carriers.
fn rust_carrier(kind: &str) -> bool {
    matches!(kind, "use_declaration" | "extern_crate_declaration")
}

/// Dart import/export/part directives. `import_or_export` is the umbrella
/// node wrapping `library_import`, `library_export`, and `part`/`part of`
/// directives — all top-level scaffolding, never duplicate logic
/// (carried over to Dart export-barrel files).
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

/// PHP opening tag and `use` import carriers.
fn php_carrier(kind: &str) -> bool {
    matches!(kind, "php_tag" | "namespace_use_declaration")
}

/// A statement-form `namespace` or `declare` is PHP prologue. Its braced form
/// wraps real code, which must keep its fingerprints.
fn is_php_prologue_statement(node: &NormalizedNode) -> bool {
    matches!(node.kind, "namespace_definition" | "declare_statement")
        && !node
            .children
            .iter()
            .any(|child| child.kind == "compound_statement")
}

/// The clause every Go file opens with.
const GO_PACKAGE_CLAUSE: &str = "package_clause";

/// Go package-header and import carriers. A `package_clause` and an
/// `import_declaration` (including the grouped `import (...)` form wrapping
/// an `import_spec_list`) are file prologue, never duplicate logic — the Go
/// analogue of C# `using` directives.
fn go_carrier(kind: &str) -> bool {
    kind == GO_PACKAGE_CLAUSE || kind == "import_declaration"
}

/// [LANG-CAND-C-PREPROC] `#include` is an import; `#define` bodies are
/// opaque `preproc_arg` leaves the grammar never parses, so two unrelated
/// macros would fingerprint identically — neither is clone evidence.
fn c_carrier(kind: &str) -> bool {
    matches!(
        kind,
        "preproc_include" | "preproc_def" | "preproc_function_def"
    )
}

/// Returns true when `kind` is the clause `language` requires every file to
/// open with — Go's `package` clause. An import is a line the author chose
/// and may have copied along with the code below it; this clause is dictated
/// by where the file lives and would read the same whether or not anything
/// in the file was copied, so a whole-file view that includes it claims
/// something nobody duplicated ([PIPELINE-FINGERPRINT-MERKLE-ROOT]).
#[must_use]
pub fn is_mandated_prologue(language: &str, kind: &str) -> bool {
    language == "go" && kind == GO_PACKAGE_CLAUSE
}

/// Language-specific wrappers whose children encode prologue-only syntax.
fn is_language_specific_boilerplate_subtree(language: &str, node: &NormalizedNode) -> bool {
    match language {
        "python" => is_python_prologue_subtree(node),
        "javascript" | "typescript" | "tsx" => is_ecmascript_reexport(node),
        "php" => is_php_prologue_statement(node),
        "c" => is_c_prototype(node),
        _ => false,
    }
}

/// [LANG-CAND-C-PROTOTYPE] A function prototype is a `declaration` whose
/// declarator is a `function_declarator` (possibly behind a
/// `pointer_declarator`) and which has no body. After normalisation every
/// one-argument prototype is the same shape, so a header listing `sqrt`,
/// `exp` and `log` would otherwise cluster with any other header listing
/// three unrelated functions. The declaration is a promise about a body
/// that lives elsewhere; the body is the evidence.
fn is_c_prototype(node: &NormalizedNode) -> bool {
    node.kind == "declaration" && node.children.iter().any(is_c_function_declarator)
}

/// The `function_declarator` of a prototype, unwrapping pointer-returning
/// declarators such as `const char *name(int)`.
fn is_c_function_declarator(node: &NormalizedNode) -> bool {
    match node.kind {
        "function_declarator" => true,
        "pointer_declarator" => node.children.iter().any(is_c_function_declarator),
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
