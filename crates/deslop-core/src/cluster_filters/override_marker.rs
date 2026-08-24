//! [CLONE-NOISE-POLYMORPHIC-CONTRACT] — the compiler's own proof that a
//! contract declares a member, for the contracts a scan never reaches.
//!
//! `contract_index` can only index what the corpus contains, so a method
//! implementing a framework interface resolves to no declaring base and
//! reads as an ordinary same-named function across files. The languages
//! here spell the override relationship explicitly and reject the marker
//! when nothing is being overridden, so its presence is evidence the
//! index cannot supply and cannot be forged by convention.
//!
//! Only the languages this crate parses carry a row; a marker for a
//! grammar the scan cannot load would be unreachable code.

use tree_sitter::Node;

/// The node kind that marks a member as implementing a contract declared
/// elsewhere, and the exact marker the language spells it with
/// ([CLONE-NOISE-POLYMORPHIC-CONTRACT]).
///
/// The corpus-wide contract index can only see contracts the scan
/// reached, so a method implementing a *framework* interface — Flutter's
/// `State<T>.build`, an ASP.NET base controller, a `@types` interface —
/// resolves to no declaring base and reads as an ordinary same-named
/// function. An explicit override marker is the compiler's own proof
/// that some contract declares it: the languages below reject the marker
/// outright when nothing is being overridden, so it cannot be present by
/// accident or by convention.
///
/// Languages with no such marker return `None` and keep relying on the
/// index alone. Python's inheritance is checked, never declared, which
/// is exactly why `[CLONE-NOISE-POLYMORPHIC-CONTRACT]` was tightened to
/// require a declaring base in the first place (gh #373).
pub(super) const fn override_marker_kind(language: &str) -> Option<(&'static str, &'static [u8])> {
    match language.as_bytes() {
        b"dart" => Some(("annotation", b"override")),
        b"csharp" => Some(("modifier", b"override")),
        b"typescript" | b"tsx" => Some(("override_modifier", b"override")),
        _ => None,
    }
}

/// True when `function` carries its language's override marker as a
/// direct child ([`override_marker_kind`]).
///
/// Only direct children count: an annotation nested inside the body
/// marks something else, and the marker must qualify the member itself.
/// The marker's identity is read off its `name` field where the grammar
/// gives it one — Dart wraps `@override` in an `annotation` around an
/// identifier — and off the node itself where the marker *is* the token,
/// as for a C# or TypeScript modifier.
pub(super) fn carries_override_marker(function: Node<'_>, language: &str, source: &[u8]) -> bool {
    let Some((kind, marker)) = override_marker_kind(language) else {
        return false;
    };
    let mut cursor = function.walk();
    let found = function.named_children(&mut cursor).any(|child| {
        child.kind() == kind
            && source.get(
                child
                    .child_by_field_name("name")
                    .unwrap_or(child)
                    .byte_range(),
            ) == Some(marker)
    });
    found
}

#[cfg(test)]
mod tests;
