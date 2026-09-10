//! Unit tests for [CLONE-NOISE-PY-PYTEST-FIXTURE]'s decorator scan.
//!
//! The filter recognises a pytest fixture by the decorator block
//! immediately above its `def`. Whether that `def` sits at module level
//! or inside a `class Test...:` changes nothing about what it is — and
//! class-nested is the more common shape in any suite that groups its
//! cases. If only the module-level form is recognised, every
//! class-nested fixture family surfaces as duplication the user cannot
//! act on: a false positive.
//!
//! These drive [`super::python_function_has_fixture_decorator`] directly
//! rather than the whole filter bank, so a failure names the decorator
//! scan itself instead of whichever earlier stage the bank happens to
//! reject a case on.

use tree_sitter::{Node, Parser};

mod decorators;

/// The decorator line every fixture in these cases carries.
const FIXTURE_DECORATOR: &str = "@pytest.fixture";

/// A pytest ORM-session fixture at module level.
fn module_level_fixture() -> String {
    format!(
        "import pytest\n\
         \n\
         {FIXTURE_DECORATOR}\n\
         def orders_session():\n\
         \x20   engine = create_engine(DATABASE_URL)\n\
         \x20   yield sessionmaker(bind=engine)()\n"
    )
}

/// The same fixture, grouped inside a test class — the shape a suite
/// reaches for as soon as it has more than a handful of cases.
fn class_nested_fixture() -> String {
    format!(
        "import pytest\n\
         \n\
         class TestOrders:\n\
         \x20   {FIXTURE_DECORATOR}\n\
         \x20   def orders_session(self):\n\
         \x20       engine = create_engine(DATABASE_URL)\n\
         \x20       yield sessionmaker(bind=engine)()\n"
    )
}

/// The first `function_definition` in `node`'s subtree, depth first.
fn first_function_definition(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "function_definition" {
        return Some(node);
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
    children.into_iter().find_map(first_function_definition)
}

/// Whether the filter recognises the sole `def` in `source` as carrying
/// a fixture decorator, or `None` when `source` does not parse into one
/// — which would make the case itself invalid rather than the filter
/// wrong, so the tests assert on `Some(true)` and never on a bare bool.
fn fixture_decorator_is_recognised(source: &str) -> Option<bool> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(source, None)?;
    let function = first_function_definition(tree.root_node())?;
    Some(super::python_function_has_fixture_decorator(
        function,
        source.as_bytes(),
    ))
}

/// The baseline the filter already honours.
#[test]
fn module_level_pytest_fixture_is_recognised() {
    assert_eq!(
        fixture_decorator_is_recognised(&module_level_fixture()),
        Some(true),
        "a module-level @pytest.fixture must be recognised as fixture \
         boilerplate"
    );
}

/// The same fixture, grouped into a test class. Indentation is not a
/// semantic difference: this is the same pytest fixture and must be
/// recognised from its own decorator nodes. The former source-line scan
/// mistook the indentation before a nested `def` for the end of its
/// decorator block.
#[test]
fn class_nested_pytest_fixture_is_recognised() {
    assert_eq!(
        fixture_decorator_is_recognised(&class_nested_fixture()),
        Some(true),
        "a class-nested @pytest.fixture is the same fixture boilerplate \
         as a module-level one; failing this means every suite that \
         groups fixtures into classes has them reported as duplicates"
    );
}
