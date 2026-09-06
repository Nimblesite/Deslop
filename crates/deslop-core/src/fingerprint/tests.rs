//! [PIPELINE-FINGERPRINT-MERKLE-ROOT] Which files get a whole-file view.
//!
//! The end-to-end suites pin the report each rule produces; these isolate
//! the rule itself, per language, on the smallest tree that exercises it.

use std::path::PathBuf;

use super::{collect_fingerprints, collect_non_boilerplate_fingerprints, Fingerprint};
use crate::{
    ast::NormalizedNode,
    lang::{go::GoParser, python::PythonParser, LanguageParser},
    state::FileRegistry,
};

/// Every subtree is a candidate, so only the root rule decides.
const EVERY_SUBTREE: usize = 1;
/// Language ids the parsers below register under.
const GO: &str = "go";
const PYTHON: &str = "python";
/// A Go file whose two functions are each a view; only the mandated
/// `package` clause could put the whole file in front of them.
const GO_MODULE: &str = "package alpha\n\nfunc A() int { return 1 }\n\nfunc B() int { return 2 }\n";
/// The functions `GO_MODULE` declares.
const GO_FUNCTIONS: usize = 2;
/// A Python module whose import was chosen by its author and copied with
/// the functions below it.
const PYTHON_MODULE: &str = "import os\n\ndef a():\n    return 1\n\ndef b():\n    return 2\n";
/// A Python module whose only declaration already covers the root's extent.
const PYTHON_SINGLE_DEF: &str = "def a():\n    return 1\n";

/// Parses `source` with `parser` into a normalised tree.
fn parse(parser: &dyn LanguageParser, source: &str) -> Result<NormalizedNode, String> {
    let mut registry = FileRegistry::new();
    let file_id = registry.register(PathBuf::from("module"));
    parser
        .parse_and_normalize(source.as_bytes(), file_id)
        .map_err(|error| format!("the fixture must parse: {error}"))
}

/// The fingerprints that span the tree's own extent.
fn views_of_whole_file(tree: &NormalizedNode, fingerprints: &[Fingerprint]) -> usize {
    fingerprints
        .iter()
        .filter(|fingerprint| fingerprint.byte_range == tree.byte_range)
        .count()
}

#[test]
fn a_python_module_copied_whole_is_a_view_at_the_extent_of_the_file() -> Result<(), String> {
    let tree = parse(&PythonParser, PYTHON_MODULE)?;
    let fingerprints = collect_non_boilerplate_fingerprints(&tree, EVERY_SUBTREE, PYTHON);
    assert_eq!(
        views_of_whole_file(&tree, &fingerprints),
        1,
        "the root is the one view spanning the whole module: {fingerprints:?}"
    );
    Ok(())
}

#[test]
fn a_go_file_is_never_a_view_because_its_package_clause_is_mandated() -> Result<(), String> {
    let tree = parse(&GoParser, GO_MODULE)?;
    let fingerprints = collect_non_boilerplate_fingerprints(&tree, EVERY_SUBTREE, GO);
    assert_eq!(
        views_of_whole_file(&tree, &fingerprints),
        0,
        "no view may span the package clause: {fingerprints:?}"
    );
    let top_level_views = tree
        .children
        .iter()
        .filter(|child| {
            fingerprints
                .iter()
                .any(|fingerprint| fingerprint.byte_range == child.byte_range)
        })
        .count();
    assert_eq!(
        top_level_views, GO_FUNCTIONS,
        "both functions stay views in the root's place, and the package clause is not one: {fingerprints:?}"
    );
    Ok(())
}

#[test]
fn a_root_that_re_describes_its_only_child_yields_to_that_child() -> Result<(), String> {
    let tree = parse(&PythonParser, PYTHON_SINGLE_DEF)?;
    let fingerprints = collect_non_boilerplate_fingerprints(&tree, EVERY_SUBTREE, PYTHON);
    assert_eq!(
        views_of_whole_file(&tree, &fingerprints),
        1,
        "the declaration is the one view of its extent, never the root beside it: {fingerprints:?}"
    );
    let without_language = collect_fingerprints(&tree, EVERY_SUBTREE);
    assert_eq!(
        views_of_whole_file(&tree, &without_language),
        1,
        "the only-child rule needs no language to apply: {without_language:?}"
    );
    Ok(())
}
