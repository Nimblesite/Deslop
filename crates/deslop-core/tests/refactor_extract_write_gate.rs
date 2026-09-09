//! E2E: [AUTOFIX-EXTRACT-PRECONDITIONS] rule 7 (issue #280) — a span
//! that writes one of its own *free* variables refuses the verbatim
//! extract. The helper would mutate its own parameter copy and the
//! caller's variable would silently keep its old value — the mutation
//! loss the type-safety backstop cannot catch. Writes to span-*bound*
//! names stay extractable: they vacate with the span.

use anyhow::{anyhow, Result};

use crate::common::clusters::needle_cluster_plan;

/// Twin members every fixture emits, so the needle span occurs exactly
/// twice — the fewest occurrences a cluster can have.
const TWIN_COUNT: usize = 2;
/// Indent of a member inside a C# class body.
const CSHARP_MEMBER_INDENT: &str = "    ";
/// Indent of a statement inside a C# method body.
const CSHARP_BODY_INDENT: &str = "        ";
/// Indent of a statement inside a Rust function body.
const RUST_BODY_INDENT: &str = "    ";
/// Indent of a statement inside a Python `def` body.
const PYTHON_BODY_INDENT: &str = "    ";
/// Python and C# put a blank line between sibling definitions.
const MEMBER_SEPARATOR: &str = "\n";
/// PEP 8 separates top-level Python definitions by two blank lines.
const PYTHON_TOP_LEVEL_SEPARATOR: &str = "\n\n";
/// The twin Rust function names every twin-function fixture emits.
const RUST_TWIN_NAMES: [&str; TWIN_COUNT] = ["alpha", "beta"];

/// Renders `body` one statement per line at `indent`, leaving blank
/// lines bare so no fixture carries trailing whitespace.
fn indented(indent: &str, body: &[&str]) -> String {
    body.iter()
        .map(|line| {
            if line.is_empty() {
                "\n".to_owned()
            } else {
                format!("{indent}{line}\n")
            }
        })
        .collect()
}

/// One `public int` method of a twin C# class.
fn csharp_method(name: &str, signature: &str, body: &[&str]) -> String {
    format!(
        "{indent}public int {name}({signature})\n{indent}{{\n{statements}{indent}}}\n",
        indent = CSHARP_MEMBER_INDENT,
        statements = indented(CSHARP_BODY_INDENT, body),
    )
}

/// A C# class whose `methods` share a byte-identical `body`, so the
/// needle span occurs exactly `TWIN_COUNT` times.
fn csharp_twin_methods(
    class: &str,
    methods: [&str; TWIN_COUNT],
    signature: &str,
    body: &[&str],
) -> String {
    let members = methods
        .iter()
        .map(|name| csharp_method(name, signature, body))
        .collect::<Vec<_>>()
        .join(MEMBER_SEPARATOR);
    format!("public class {class}\n{{\n{members}}}\n")
}

/// Twin Rust functions sharing a byte-identical `body`.
fn rust_twin_fns(signature: &str, body: &[&str]) -> String {
    RUST_TWIN_NAMES
        .iter()
        .map(|name| {
            format!(
                "fn {name}{signature} {{\n{statements}}}\n",
                statements = indented(RUST_BODY_INDENT, body),
            )
        })
        .collect::<Vec<_>>()
        .join(MEMBER_SEPARATOR)
}

/// Twin top-level Python `def`s sharing a byte-identical `body`.
fn python_twin_defs(names: [&str; TWIN_COUNT], body: &[&str]) -> String {
    names
        .iter()
        .map(|name| {
            format!(
                "def {name}():\n{statements}",
                statements = indented(PYTHON_BODY_INDENT, body),
            )
        })
        .collect::<Vec<_>>()
        .join(PYTHON_TOP_LEVEL_SEPARATOR)
}

/// Module-scope Python: `prelude` once, then `body` `TWIN_COUNT` times.
fn python_twin_statements(prelude: &[&str], body: &[&str]) -> String {
    let repeated: String = (0..TWIN_COUNT).map(|_| indented("", body)).collect();
    format!("{prelude}{repeated}", prelude = indented("", prelude))
}

/// Asserts the span `needle` — present twice in `text`, parsed as
/// `file_name`'s language — refuses the verbatim extract. `subject`
/// names the write that trips [AUTOFIX-EXTRACT-PRECONDITIONS] rule 7.
fn assert_refused(text: &str, needle: &str, file_name: &str, subject: &str) -> Result<()> {
    let plan = needle_cluster_plan(text, needle, file_name)?;
    assert!(
        plan.is_none(),
        "{subject} must refuse the extract \
         ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7, #280): {plan:?}"
    );
    Ok(())
}

/// Asserts the span `needle` still extracts, and that its parameter
/// list is exactly `expected` — empty when nothing flows in.
fn assert_extracts_with_free_variables(
    text: &str,
    needle: &str,
    file_name: &str,
    expected: &[&str],
) -> Result<()> {
    let plan = needle_cluster_plan(text, needle, file_name)?
        .ok_or_else(|| anyhow!("a span writing only its own binding must extract"))?;
    let expected: Vec<String> = expected.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        plan.free_variables, expected,
        "only {expected:?} flows in; the written name is span-bound, not free"
    );
    Ok(())
}

/// C#: `total` is free (declared before the span) and written inside it
/// via compound assignment — extraction must refuse.
#[test]
fn csharp_written_free_variable_refused() -> Result<()> {
    let text = csharp_twin_methods(
        "InvoiceMath",
        ["TotalWithTax", "TotalWithTaxAgain"],
        "int[] amounts, int taxRate",
        &[
            "var total = 0;",
            "foreach (var amount in amounts)",
            "{",
            "    var taxed = amount * taxRate / 100;",
            "    total += amount + taxed;",
            "}",
            "return total;",
        ],
    );
    let needle = "var taxed = amount * taxRate / 100;\n            total += amount + taxed;";
    assert_refused(
        &text,
        needle,
        "InvoiceMath.cs",
        "a span writing free `total`",
    )
}

/// Python: `count += step` targets the *module* binding — augmented
/// assignment is deliberately not a binding kind, so `count` is free
/// and written. Extraction must refuse.
#[test]
fn python_augmented_assignment_of_free_name_refused() -> Result<()> {
    let text = python_twin_statements(
        &["count = 0", "step = 1"],
        &["count += step", "total = count * 2"],
    );
    let needle = "count += step\ntotal = count * 2";
    assert_refused(&text, needle, "gate.py", "a span writing free `count`")
}

/// Rust: `counter += 1` is a `compound_assignment_expr` writing the
/// enclosing function's local — extraction must refuse.
#[test]
fn rust_compound_assignment_of_free_name_refused() -> Result<()> {
    let text = rust_twin_fns(
        "(seed: i64) -> i64",
        &[
            "let mut counter = seed;",
            "counter += 1;",
            "counter += 2;",
            "counter",
        ],
    );
    let needle = "counter += 1;\n    counter += 2;";
    assert_refused(&text, needle, "gate.rs", "a span writing free `counter`")
}

/// C#: writes to a span-*bound* name (`padded` is declared inside the
/// span) do not trip rule 7 — the binding vacates with the span.
#[test]
fn csharp_write_of_span_bound_name_still_extracts() -> Result<()> {
    let text = csharp_twin_methods(
        "Padding",
        ["PadA", "PadB"],
        "int size",
        &["var padded = size;", "padded += 4;", "return padded * 2;"],
    );
    let needle = "var padded = size;\n        padded += 4;\n        return padded * 2;";
    assert_extracts_with_free_variables(&text, needle, "Padding.cs", &["size"])
}

/// C#: `total++` mutates the free `total` with no assignment node at
/// all (`postfix_unary_expression`) — extraction must refuse.
#[test]
fn csharp_increment_of_free_name_refused() -> Result<()> {
    let text = csharp_twin_methods(
        "Bumper",
        ["BumpA", "BumpB"],
        "int seed",
        &[
            "var total = seed;",
            "total++;",
            "var report = total * 2;",
            "return report;",
        ],
    );
    let needle = "total++;\n        var report = total * 2;\n        return report;";
    assert_refused(&text, needle, "Bumper.cs", "an increment of free `total`")
}

/// C#: `out total` mutates the free `total` through the callee — no
/// assignment node, just an argument modifier. Extraction must refuse.
#[test]
fn csharp_out_argument_write_of_free_name_refused() -> Result<()> {
    let text = csharp_twin_methods(
        "Parser",
        ["ParseA", "ParseB"],
        "string text",
        &[
            "var total = 0;",
            "int.TryParse(text, out total);",
            "return total * 2;",
        ],
    );
    let needle = "int.TryParse(text, out total);\n        return total * 2;";
    assert_refused(
        &text,
        needle,
        "Parser.cs",
        "an `out` argument writing free `total`",
    )
}

/// C#: tuple deconstruction rebinds both free names even though the
/// assignment target is not a bare identifier — extraction must refuse.
#[test]
fn csharp_tuple_deconstruction_of_free_names_refused() -> Result<()> {
    let text = csharp_twin_methods(
        "Swapper",
        ["SwapA", "SwapB"],
        "int min, int max",
        &["(min, max) = (max, min);", "return min - max;"],
    );
    let needle = "(min, max) = (max, min);\n        return min - max;";
    assert_refused(
        &text,
        needle,
        "Swapper.cs",
        "tuple deconstruction writing free `min`/`max`",
    )
}

/// C#: a write-only plain assignment (`total = 7;` — `total` never read
/// in the span) still counts: the free-variable walk records the target
/// as a reference, so rule 7 sees it. Pins the property the gate's
/// contract depends on.
#[test]
fn csharp_plain_write_only_target_refused() -> Result<()> {
    let text = csharp_twin_methods(
        "Resetter",
        ["ResetA", "ResetB"],
        "int seed",
        &["var total = seed;", "total = 7;", "return total + seed;"],
    );
    let needle = "total = 7;\n        return total + seed;";
    assert_refused(
        &text,
        needle,
        "Resetter.cs",
        "a write-only plain assignment to free `total`",
    )
}

/// Rust: plain assignment (`assignment_expression`, distinct from
/// `compound_assignment_expr`) of a free name must refuse — pins the
/// table entry the compound-only test cannot.
#[test]
fn rust_plain_assignment_of_free_name_refused() -> Result<()> {
    let text = rust_twin_fns(
        "(seed: i64) -> i64",
        &[
            "let mut counter = seed;",
            "counter = counter + 1;",
            "counter = counter + 2;",
            "counter",
        ],
    );
    let needle = "counter = counter + 1;\n    counter = counter + 2;";
    assert_refused(
        &text,
        needle,
        "gate.rs",
        "a plain assignment to free `counter`",
    )
}

/// Python: a span declaring `nonlocal` cannot relocate — the emitted
/// module-scope helper has no enclosing function binding, so the file
/// dies with `SyntaxError` and the outer mutation is lost. Refuse.
#[test]
fn python_nonlocal_write_span_refused() -> Result<()> {
    let text = python_twin_defs(
        ["outer_a", "outer_b"],
        &[
            "count = 0",
            "",
            "def bump():",
            "    nonlocal count",
            "    count += 1",
            "",
            "bump()",
            "return count",
        ],
    );
    let needle = "nonlocal count\n        count += 1";
    assert_refused(&text, needle, "gate.py", "a span declaring `nonlocal count`")
}

/// Python: `global` survives relocation — a module-scope helper in the
/// same file resolves the same module globals, so the span extracts
/// with an empty parameter list.
#[test]
fn python_global_write_span_still_extracts() -> Result<()> {
    let defs = python_twin_defs(["bump_a", "bump_b"], &["global count", "count += 1"]);
    let text = format!("count = 0\n{PYTHON_TOP_LEVEL_SEPARATOR}{defs}");
    let needle = "global count\n    count += 1";
    assert_extracts_with_free_variables(&text, needle, "gate.py", &[])
}

/// Python: augmented assignment of a span-bound name does not trip
/// rule 7 either — the plain assignment above it binds `padded` inside
/// the span.
#[test]
fn python_write_of_span_bound_name_still_extracts() -> Result<()> {
    let text = python_twin_statements(
        &["base = 3"],
        &["padded = base", "padded += 4", "total = padded * 2"],
    );
    let needle = "padded = base\npadded += 4\ntotal = padded * 2";
    assert_extracts_with_free_variables(&text, needle, "gate.py", &["base"])
}
