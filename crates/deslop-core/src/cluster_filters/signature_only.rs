//! [CLONE-NOISE-SIGNATURE-ONLY] and the per-grammar function tables the
//! signature filters read.
//!
//! Identifier and literal normalisation makes every `fn check(ctx: &mut
//! Ctx)` the same shape, so a match confined to a signature says nothing
//! about the bodies. These filters re-parse the members and compare the
//! enclosing bodies instead.

use std::sync::Arc;

use super::{
    body_shape,
    helpers::enclosing_kind,
    snippets::{parse_for, ParseCache, Snippet},
};

/// Detects **** [CLONE-NOISE-SIGNATURE-ONLY]: every cluster
/// member's matched subtree sits entirely inside a function/method
/// signature — it does not overlap the body. After identifier and
/// literal normalisation, `fn check_foo(ctx: &mut Ctx)` collapses to
/// the same shape as `fn check_bar(ctx: &mut Ctx)`, so the structural
/// fingerprint is 1.0 but the function bodies are unrelated. Token
/// Jaccard cannot refute the match because identifier normalisation
/// erases the distinguishing tokens too.
///
/// We suppress these clusters only when at least two of the enclosing
/// function bodies differ as normalised trees
/// ([`body_shape::body_kind_stream`]). A real Type-1/Type-2 clone
/// where two functions share the same signature and body — byte for
/// byte or under a consistent rename — has equal streams, so genuine
/// duplication keeps clustering.
pub(super) fn is_signature_only_cluster(snippets: &[Snippet<'_>], cache: &ParseCache) -> bool {
    if snippets.len() < 2 {
        return false;
    }
    let shapes: Option<Vec<Arc<Vec<body_shape::OwnedShapeToken>>>> = snippets
        .iter()
        .map(|snippet| {
            cache.signature_shape(snippet, || {
                snippet_body_shape_when_signature_only(snippet).map(|stream| {
                    stream
                        .iter()
                        .map(body_shape::OwnedShapeToken::from)
                        .collect()
                })
            })
        })
        .collect();
    let Some(shapes) = shapes else { return false };
    let Some(first) = shapes.first() else {
        return false;
    };
    shapes.iter().any(|shape| shape != first)
}

/// Returns the enclosing function body's normalised kind stream
/// ([`body_shape::body_kind_stream`]) when `snippet.range` lies
/// entirely inside that function's signature (before the body) — the
/// signature-only match condition for [CLONE-NOISE-SIGNATURE-ONLY].
/// Two bodies that share AST shape (and differ only by literals,
/// identifiers, or comments) compare equal — i.e. a legitimate
/// near-miss cluster keeps clustering. Returns `None` when the snippet
/// is not contained in a function, when the function has no `body`
/// field, or when the range intersects the body in any way.
fn snippet_body_shape_when_signature_only<'src>(
    snippet: &Snippet<'src>,
) -> Option<Vec<body_shape::ShapeToken<'src>>> {
    let tree = parse_for(snippet)?;
    let function = enclosing_kind(
        tree.root_node(),
        snippet.range,
        function_kinds(snippet.language),
    )?;
    let body = function.child_by_field_name("body")?;
    if snippet.range.end > body.start_byte() {
        return None;
    }
    Some(body_shape::body_kind_stream(body, snippet.source))
}

/// Returns the set of tree-sitter node kinds that count as function
/// declarations for the purpose of polymorphism detection.
pub(crate) const fn function_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        // Python and C each have one function form exposing a `body` field.
        b"python" | b"c" => &["function_definition"],
        b"csharp" => &["method_declaration", "local_function_statement"],
        b"rust" => &["function_item"],
        // Dart `function_declaration`/`method_declaration` both expose a
        // `body` field, so the signature-only filter can compare
        // bodies after a signature-only structural match.
        b"dart" => &["function_declaration", "method_declaration"],
        // JS/TS/TSX function forms that expose a `body` field, so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match — parity
        // with the other languages.
        b"javascript" | b"typescript" | b"tsx" => &[
            "function_declaration",
            "generator_function_declaration",
            "method_definition",
            "function_expression",
            "arrow_function",
        ],
        // PHP functions and class methods that expose a `body` field so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match.
        b"php" => &["function_definition", "method_declaration"],
        // F# `let`/`member` bindings both expose a `body` field so the
        // signature-only and polymorphic-signature filters can
        // compare bodies after a signature-only structural match. Only
        // `method_or_prop_defn` carries a `name` field (members), so #69 is
        // limited to members; top-level `let` bindings still get #154.
        b"fsharp" => &["function_or_value_defn", "method_or_prop_defn"],
        // Go functions, methods, and closures all expose a `body` field so
        // the signature-only and polymorphic-signature filters
        // can compare bodies after a signature-only structural match.
        b"go" => &["function_declaration", "method_declaration", "func_literal"],
        _ => &[],
    }
}

/// The declaration shell the widened polymorphic-subject resolution may
/// walk through ([CLONE-NOISE-POLYMORPHIC-SIGNATURE]): when a member
/// view is wider than any single function, the subject is the sole
/// function the range contains with nothing but declaration scaffolding
/// around it. A row lists the grammar's inert shell kinds — the file
/// root, the class/mixin/enum/extension containers, and constructor
/// signatures, none of which execute; kinds that can carry behaviour
/// (function bodies, field initialisers) are deliberately absent, so a
/// member containing them vetoes the widened resolution and keeps the
/// component surfaced. A language with no row keeps the
/// containing-function behaviour.
pub(super) const fn declaration_shell_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &[
            "module",
            "class_definition",
            "block",
            "decorated_definition",
            "decorator",
        ],
        b"dart" => &[
            "source_file",
            "class_declaration",
            "class_body",
            "class_member",
            "declaration",
            "constant_constructor_signature",
            "constructor_signature",
            "constructor_param",
            "formal_parameter_list",
            "optional_formal_parameters",
            "formal_parameter",
            "super_formal_parameter",
            "super",
            "superclass",
            "type",
            "type_identifier",
            "identifier",
            "annotation",
            "enum_declaration",
            "enum_body",
            "mixin_declaration",
            "extension_declaration",
            "extension_body",
            "function_signature",
        ],
        _ => &[],
    }
}
