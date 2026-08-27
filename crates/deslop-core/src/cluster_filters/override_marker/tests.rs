//! Unit tests for [`super::carries_override_marker`].
//!
//! One language per row of [`super::override_marker_kind`], because each
//! row names a different grammar's node kind and a wrong kind is a silent
//! `false` — the filter would simply stop suppressing, and the framework
//! false positive would come back with no test to say so. The E2E
//! (`issue_331_336_shape_only_saturation.rs`) reaches Dart only.

use super::*;
use crate::ast::named_children;

/// The Dart member the false positive was reported on: a Flutter `build`
/// override, whose declaring contract (`State<T>`) is never in the scan.
const DART_OVERRIDE: &str = "class _S extends State<W> {\n  @override\n  Widget build(BuildContext c) {\n    return Row();\n  }\n}\n";

/// The same shape with the marker removed — an ordinary method.
const DART_PLAIN: &str =
    "class _S extends State<W> {\n  Widget build(BuildContext c) {\n    return Row();\n  }\n}\n";

/// A C# member overriding a base-class method, marker as a modifier.
const CSHARP_OVERRIDE: &str = "class S : B {\n  public override int Run() { return 1; }\n}\n";

/// The same C# member declared fresh rather than overriding.
const CSHARP_PLAIN: &str = "class S : B {\n  public int Run() { return 1; }\n}\n";

/// A TypeScript member carrying the `override` modifier.
const TYPESCRIPT_OVERRIDE: &str =
    "class S extends B {\n  override run(): number {\n    return 1;\n  }\n}\n";

/// Python spells no override relationship, so the row is absent and the
/// contract index stays the only proof (gh #373).
const PYTHON_METHOD: &str = "class S(B):\n    def run(self):\n        return 1\n";

/// A Dart annotation that is *not* `override`. The marker is matched by
/// identity, so an unrelated annotation must not stand in for it.
const DART_OTHER_ANNOTATION: &str = "class _S extends State<W> {\n  @deprecated\n  Widget build(BuildContext c) {\n    return Row();\n  }\n}\n";

/// The grammar for `language`, for the languages these tests name.
fn grammar(language: &str) -> tree_sitter::Language {
    match language {
        "dart" => tree_sitter_dart::LANGUAGE.into(),
        "csharp" => tree_sitter_c_sharp::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        _ => tree_sitter_python::LANGUAGE.into(),
    }
}

/// Whether the first node of one of `kinds` in `source` carries the
/// marker, or `None` when the grammar, the parse or the fixture itself
/// did not produce such a node.
///
/// `None` rather than a panic so a fixture that stops parsing fails the
/// assertion it belongs to — an `expect` here would report "fixture
/// parses" and say nothing about which contract went unproven.
fn marks_the_member(language: &str, source: &str, kinds: &[&str]) -> Option<bool> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar(language)).ok()?;
    let tree = parser.parse(source, None)?;
    let function = find_kind(tree.root_node(), kinds)?;
    Some(carries_override_marker(
        function,
        language,
        source.as_bytes(),
    ))
}

/// The first node of any kind in `kinds`, depth-first.
fn find_kind<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    if kinds.contains(&node.kind()) {
        return Some(node);
    }
    named_children(node)
        .into_iter()
        .find_map(|child| find_kind(child, kinds))
}

#[test]
fn every_language_row_recognises_its_own_override_marker() {
    assert_eq!(
        marks_the_member("dart", DART_OVERRIDE, &["method_declaration"]),
        Some(true),
        "Dart wraps `@override` in an `annotation` whose name field is the \
         identifier; missing it is the Flutter false positive this row exists \
         to close"
    );
    assert_eq!(
        marks_the_member("csharp", CSHARP_OVERRIDE, &["method_declaration"]),
        Some(true),
        "C# spells the marker as a bare `modifier` token with no name field, \
         so the node itself carries the identity"
    );
    assert_eq!(
        marks_the_member("typescript", TYPESCRIPT_OVERRIDE, &["method_definition"]),
        Some(true),
        "TypeScript spells the marker as an `override_modifier` token"
    );
}

#[test]
fn a_member_that_overrides_nothing_is_not_marked() {
    assert_eq!(
        marks_the_member("dart", DART_PLAIN, &["method_declaration"]),
        Some(false),
        "an unannotated Dart method implements no contract the index cannot \
         see, so it must keep relying on the contract index"
    );
    assert_eq!(
        marks_the_member("csharp", CSHARP_PLAIN, &["method_declaration"]),
        Some(false),
        "a C# method declared without `override` is a fresh declaration"
    );
    assert_eq!(
        marks_the_member("dart", DART_OTHER_ANNOTATION, &["method_declaration"]),
        Some(false),
        "the marker is matched by identity: `@deprecated` says nothing about \
         a contract, and accepting any annotation would hide copy-paste \
         behind a docs tag"
    );
}

#[test]
fn a_language_with_no_override_marker_never_matches() {
    assert_eq!(
        override_marker_kind("python"),
        None,
        "Python's inheritance is checked, never declared — which is why \
         [CLONE-NOISE-POLYMORPHIC-CONTRACT] requires a declaring base there \
         (gh #373)"
    );
    assert_eq!(
        marks_the_member("python", PYTHON_METHOD, &["function_definition"]),
        Some(false),
        "a language with no row must return false rather than guess, so the \
         contract index stays the only proof"
    );
    for language in ["rust", "go", "fsharp", "javascript"] {
        assert_eq!(
            override_marker_kind(language),
            None,
            "{language} has no compiler-enforced override marker on this \
             surface; adding a row without one would suppress real clones"
        );
    }
}
