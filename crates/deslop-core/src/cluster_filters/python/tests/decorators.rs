//! [CLONE-NOISE-PY-PYTEST-FIXTURE] — decorator identities on class methods.

use super::fixture_decorator_is_recognised;

/// A method's containing class, shared across the decorator cases.
const TEST_CLASS: &str = "class TestOrders:\n";
/// The same method body follows every decorator case.
const TEST_METHOD: &str = "    def orders_session(self):\n        yield sessionmaker()\n";
/// Supported fixture names and calls remain fixtures inside a class.
const FIXTURE_DECORATORS: &[(&str, &str)] = &[
    ("bare name", "    @fixture\n"),
    ("bare call", "    @fixture()\n"),
    ("pytest name", "    @pytest.fixture\n"),
    ("async fixture", "    @pytest_asyncio.fixture\n"),
    ("qualified name", "    @helpers.pytest.fixture\n"),
    (
        "configured fixture",
        "    @pytest.fixture(scope=\"class\")\n",
    ),
    (
        "multiline configuration",
        "    @pytest.fixture(\n        scope=\"class\",\n    )\n",
    ),
    ("stacked above", "    @pytest.fixture\n    @marker\n"),
    ("stacked below", "    @marker\n    @pytest.fixture\n"),
    ("comment", "    @pytest.fixture  # class-scoped setup\n"),
];
/// A fixture token outside the decorator's dotted callee is insufficient.
const ORDINARY_DECORATORS: &[(&str, &str)] = &[
    ("undecorated method", ""),
    ("ordinary decorator", "    @marker\n"),
    ("longer name", "    @pytest.fixture_factory\n"),
    ("different suffix", "    @fixture.marker\n"),
    ("fixture argument", "    @marker(pytest.fixture)\n"),
    ("fixture text argument", "    @marker(\"fixture\")\n"),
    ("fixture keyword", "    @marker(fixture=True)\n"),
    ("computed receiver", "    @factory().fixture\n"),
    ("computed callee", "    @fixture()()\n"),
];
/// A decorator on the class does not decorate its methods.
const DECORATED_CLASS: &str = "@pytest.fixture\nclass TestOrders:\n";

/// Class nesting preserves the fixture decorator's identity and arguments.
#[test]
fn class_methods_recognise_fixture_decorator_forms() {
    for (label, decorator) in FIXTURE_DECORATORS {
        let source = format!("{TEST_CLASS}{decorator}{TEST_METHOD}");
        assert_eq!(
            fixture_decorator_is_recognised(&source),
            Some(true),
            "{label}"
        );
    }
}

/// Ordinary decorators stay distinct even when their arguments mention fixtures.
#[test]
fn class_methods_distinguish_other_decorator_forms() {
    for (label, decorator) in ORDINARY_DECORATORS {
        let source = format!("{TEST_CLASS}{decorator}{TEST_METHOD}");
        assert_eq!(
            fixture_decorator_is_recognised(&source),
            Some(false),
            "{label}"
        );
    }
    let source = format!("{DECORATED_CLASS}{TEST_METHOD}");
    assert_eq!(fixture_decorator_is_recognised(&source), Some(false));
}
