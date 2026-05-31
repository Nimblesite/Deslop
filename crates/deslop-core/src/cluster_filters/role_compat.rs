//! Role-compatibility gate for embedding-dominant `same_behavior`
//! clusters.
//!
//! Issue **#119** [CLONE-NOISE-EMBEDDING-ROLE-MISMATCH]: the embedding
//! pass can pair two snippets that share a topic vocabulary but live in
//! structurally incompatible constructs — e.g. a reusable transport
//! helper *class* and a constructor-storage *test method*. Such a pair
//! reaches `structural=0.00`, `embedding_cos>=0.80`, and surfaces as
//! "Same behavior, different code". There is no safe extraction across
//! a class definition and a function/method: their roles differ.
//!
//! This filter engages **only** for embedding-dominant matches (the
//! caller restricts it to the `same_behavior` bucket). It suppresses a
//! cluster when its members do not all share one top-level construct
//! role — all classes, or all functions/methods. Two genuinely
//! behaviour-equivalent functions (same role) that the embedding
//! correctly pairs keep clustering.

use tree_sitter::Node;

use super::{enclosing_kind, function_kinds, parse_for, trimmed_snippet_range, Snippet};

/// Top-level construct role a cluster member resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberRole {
    /// The matched range's innermost enclosing construct is a class or
    /// other type definition (`class_definition`, `class_declaration`,
    /// `struct_item`, …).
    TypeDef,
    /// The matched range's innermost enclosing construct is a
    /// function or method definition.
    Function,
}

/// Returns true when this embedding-dominant cluster pairs members of
/// incompatible top-level roles (a class/type definition with a
/// function/method). The caller restricts this to `same_behavior`
/// clusters so deterministic Type-1/2/3 buckets are untouched.
///
/// Returns false (does not suppress) when any member's role cannot be
/// resolved — an unknown role is never grounds to hide duplication.
pub(super) fn is_role_incompatible_embedding_match(snippets: &[Snippet<'_>]) -> bool {
    if snippets.len() < 2 {
        return false;
    }
    let roles: Option<Vec<MemberRole>> = snippets.iter().map(member_role).collect();
    let Some(roles) = roles else { return false };
    let Some(first) = roles.first() else {
        return false;
    };
    roles.iter().any(|role| role != first)
}

/// Resolves the innermost enclosing top-level construct role for one
/// member by re-parsing its source and picking the tightest enclosing
/// node that is either a type definition or a function. Returns `None`
/// when the language has no registered grammar or the range is not
/// inside any such construct.
fn member_role(snippet: &Snippet<'_>) -> Option<MemberRole> {
    let tree = parse_for(snippet)?;
    let range = trimmed_snippet_range(snippet).unwrap_or(snippet.range);
    let node = enclosing_kind(tree.root_node(), range, &role_kinds(snippet.language))?;
    role_of(node, snippet.language)
}

/// Classifies an enclosing construct node into its [`MemberRole`].
fn role_of(node: Node<'_>, language: &str) -> Option<MemberRole> {
    if function_kinds(language).contains(&node.kind()) {
        Some(MemberRole::Function)
    } else if type_kinds(language).contains(&node.kind()) {
        Some(MemberRole::TypeDef)
    } else if node.kind() == "decorated_definition" {
        decorated_definition_role(node, language)
    } else {
        None
    }
}

/// Resolves the role inside a Python decorated definition wrapper.
fn decorated_definition_role(node: Node<'_>, language: &str) -> Option<MemberRole> {
    let mut cursor = node.walk();
    let role = node
        .named_children(&mut cursor)
        .find_map(|child| role_of(child, language));
    role
}

/// All node kinds that count as a top-level construct for role
/// resolution: every function kind plus every type-definition kind for
/// the language.
fn role_kinds(language: &str) -> Vec<&'static str> {
    let mut kinds = function_kinds(language).to_vec();
    kinds.extend_from_slice(type_kinds(language));
    if language == "python" {
        kinds.push("decorated_definition");
    }
    kinds
}

/// Type-definition node kinds per language. A member enclosed by one of
/// these (and by no tighter function) resolves to [`MemberRole::TypeDef`].
const fn type_kinds(language: &str) -> &'static [&'static str] {
    match language.as_bytes() {
        b"python" => &["class_definition"],
        b"csharp" => &[
            "class_declaration",
            "struct_declaration",
            "record_declaration",
        ],
        b"rust" => &[
            "struct_item",
            "enum_item",
            "trait_item",
            "impl_item",
            "union_item",
        ],
        b"dart" => &[
            "class_declaration",
            "mixin_declaration",
            "enum_declaration",
            "extension_declaration",
        ],
        _ => &[],
    }
}
