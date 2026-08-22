//! E2E: [AUTOFIX-EXTRACT-PRECONDITIONS] rule 7 (issue #280) — a span
//! that writes one of its own *free* variables refuses the verbatim
//! extract. The helper would mutate its own parameter copy and the
//! caller's variable would silently keep its old value — the mutation
//! loss the type-safety backstop cannot catch. Writes to span-*bound*
//! names stay extractable: they vacate with the span.


use anyhow::{anyhow, Result};

use crate::common::clusters::needle_cluster_plan;

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

/// Asserts the span `needle` still extracts because the name it writes
/// is span-*bound*, and that `expected` is the one free variable that
/// flows in.
fn assert_span_bound_extract(
    text: &str,
    needle: &str,
    file_name: &str,
    expected: &str,
) -> Result<()> {
    let plan = needle_cluster_plan(text, needle, file_name)?
        .ok_or_else(|| anyhow!("a span writing only its own binding must extract"))?;
    assert_eq!(
        plan.free_variables,
        vec![expected.to_owned()],
        "only `{expected}` flows in; the written `padded` is span-bound, not free"
    );
    Ok(())
}

/// C#: `total` is free (declared before the span) and written inside it
/// via compound assignment — extraction must refuse.
#[test]
fn csharp_written_free_variable_refused() -> Result<()> {
    let text = "public class InvoiceMath\n\
                {\n\
                \x20   public int TotalWithTax(int[] amounts, int taxRate)\n\
                \x20   {\n\
                \x20       var total = 0;\n\
                \x20       foreach (var amount in amounts)\n\
                \x20       {\n\
                \x20           var taxed = amount * taxRate / 100;\n\
                \x20           total += amount + taxed;\n\
                \x20       }\n\
                \x20       return total;\n\
                \x20   }\n\
                \n\
                \x20   public int TotalWithTaxAgain(int[] amounts, int taxRate)\n\
                \x20   {\n\
                \x20       var total = 0;\n\
                \x20       foreach (var amount in amounts)\n\
                \x20       {\n\
                \x20           var taxed = amount * taxRate / 100;\n\
                \x20           total += amount + taxed;\n\
                \x20       }\n\
                \x20       return total;\n\
                \x20   }\n\
                }\n";
    let needle = "var taxed = amount * taxRate / 100;\n            total += amount + taxed;";
    assert_refused(
        text,
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
    let text = "count = 0\n\
                step = 1\n\
                count += step\n\
                total = count * 2\n\
                count += step\n\
                total = count * 2\n";
    let needle = "count += step\ntotal = count * 2";
    assert_refused(text, needle, "gate.py", "a span writing free `count`")
}

/// Rust: `counter += 1` is a `compound_assignment_expr` writing the
/// enclosing function's local — extraction must refuse.
#[test]
fn rust_compound_assignment_of_free_name_refused() -> Result<()> {
    let text = "fn alpha(seed: i64) -> i64 {\n\
                \x20   let mut counter = seed;\n\
                \x20   counter += 1;\n\
                \x20   counter += 2;\n\
                \x20   counter\n\
                }\n\
                \n\
                fn beta(seed: i64) -> i64 {\n\
                \x20   let mut counter = seed;\n\
                \x20   counter += 1;\n\
                \x20   counter += 2;\n\
                \x20   counter\n\
                }\n";
    let needle = "counter += 1;\n    counter += 2;";
    assert_refused(text, needle, "gate.rs", "a span writing free `counter`")
}

/// C#: writes to a span-*bound* name (`padded` is declared inside the
/// span) do not trip rule 7 — the binding vacates with the span.
#[test]
fn csharp_write_of_span_bound_name_still_extracts() -> Result<()> {
    let text = "public class Padding\n\
                {\n\
                \x20   public int PadA(int size)\n\
                \x20   {\n\
                \x20       var padded = size;\n\
                \x20       padded += 4;\n\
                \x20       return padded * 2;\n\
                \x20   }\n\
                \n\
                \x20   public int PadB(int size)\n\
                \x20   {\n\
                \x20       var padded = size;\n\
                \x20       padded += 4;\n\
                \x20       return padded * 2;\n\
                \x20   }\n\
                }\n";
    let needle = "var padded = size;\n        padded += 4;\n        return padded * 2;";
    assert_span_bound_extract(text, needle, "Padding.cs", "size")
}

/// C#: `total++` mutates the free `total` with no assignment node at
/// all (`postfix_unary_expression`) — extraction must refuse.
#[test]
fn csharp_increment_of_free_name_refused() -> Result<()> {
    let text = "public class Bumper\n\
                {\n\
                \x20   public int BumpA(int seed)\n\
                \x20   {\n\
                \x20       var total = seed;\n\
                \x20       total++;\n\
                \x20       var report = total * 2;\n\
                \x20       return report;\n\
                \x20   }\n\
                \n\
                \x20   public int BumpB(int seed)\n\
                \x20   {\n\
                \x20       var total = seed;\n\
                \x20       total++;\n\
                \x20       var report = total * 2;\n\
                \x20       return report;\n\
                \x20   }\n\
                }\n";
    let needle = "total++;\n        var report = total * 2;\n        return report;";
    assert_refused(text, needle, "Bumper.cs", "an increment of free `total`")
}

/// C#: `out total` mutates the free `total` through the callee — no
/// assignment node, just an argument modifier. Extraction must refuse.
#[test]
fn csharp_out_argument_write_of_free_name_refused() -> Result<()> {
    let text = "public class Parser\n\
                {\n\
                \x20   public int ParseA(string text)\n\
                \x20   {\n\
                \x20       var total = 0;\n\
                \x20       int.TryParse(text, out total);\n\
                \x20       return total * 2;\n\
                \x20   }\n\
                \n\
                \x20   public int ParseB(string text)\n\
                \x20   {\n\
                \x20       var total = 0;\n\
                \x20       int.TryParse(text, out total);\n\
                \x20       return total * 2;\n\
                \x20   }\n\
                }\n";
    let needle = "int.TryParse(text, out total);\n        return total * 2;";
    assert_refused(
        text,
        needle,
        "Parser.cs",
        "an `out` argument writing free `total`",
    )
}

/// C#: tuple deconstruction rebinds both free names even though the
/// assignment target is not a bare identifier — extraction must refuse.
#[test]
fn csharp_tuple_deconstruction_of_free_names_refused() -> Result<()> {
    let text = "public class Swapper\n\
                {\n\
                \x20   public int SwapA(int min, int max)\n\
                \x20   {\n\
                \x20       (min, max) = (max, min);\n\
                \x20       return min - max;\n\
                \x20   }\n\
                \n\
                \x20   public int SwapB(int min, int max)\n\
                \x20   {\n\
                \x20       (min, max) = (max, min);\n\
                \x20       return min - max;\n\
                \x20   }\n\
                }\n";
    let needle = "(min, max) = (max, min);\n        return min - max;";
    assert_refused(
        text,
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
    let text = "public class Resetter\n\
                {\n\
                \x20   public int ResetA(int seed)\n\
                \x20   {\n\
                \x20       var total = seed;\n\
                \x20       total = 7;\n\
                \x20       return total + seed;\n\
                \x20   }\n\
                \n\
                \x20   public int ResetB(int seed)\n\
                \x20   {\n\
                \x20       var total = seed;\n\
                \x20       total = 7;\n\
                \x20       return total + seed;\n\
                \x20   }\n\
                }\n";
    let needle = "total = 7;\n        return total + seed;";
    assert_refused(
        text,
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
    let text = "fn alpha(seed: i64) -> i64 {\n\
                \x20   let mut counter = seed;\n\
                \x20   counter = counter + 1;\n\
                \x20   counter = counter + 2;\n\
                \x20   counter\n\
                }\n\
                \n\
                fn beta(seed: i64) -> i64 {\n\
                \x20   let mut counter = seed;\n\
                \x20   counter = counter + 1;\n\
                \x20   counter = counter + 2;\n\
                \x20   counter\n\
                }\n";
    let needle = "counter = counter + 1;\n    counter = counter + 2;";
    assert_refused(
        text,
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
    let text = "def outer_a():\n\
                \x20   count = 0\n\
                \n\
                \x20   def bump():\n\
                \x20       nonlocal count\n\
                \x20       count += 1\n\
                \n\
                \x20   bump()\n\
                \x20   return count\n\
                \n\
                \n\
                def outer_b():\n\
                \x20   count = 0\n\
                \n\
                \x20   def bump():\n\
                \x20       nonlocal count\n\
                \x20       count += 1\n\
                \n\
                \x20   bump()\n\
                \x20   return count\n";
    let needle = "nonlocal count\n        count += 1";
    assert_refused(text, needle, "gate.py", "a span declaring `nonlocal count`")
}

/// Python: `global` survives relocation — a module-scope helper in the
/// same file resolves the same module globals, so the span extracts
/// with an empty parameter list.
#[test]
fn python_global_write_span_still_extracts() -> Result<()> {
    let text = "count = 0\n\
                \n\
                \n\
                def bump_a():\n\
                \x20   global count\n\
                \x20   count += 1\n\
                \n\
                \n\
                def bump_b():\n\
                \x20   global count\n\
                \x20   count += 1\n";
    let needle = "global count\n    count += 1";
    let plan = needle_cluster_plan(text, needle, "gate.py")?
        .ok_or_else(|| anyhow!("a `global` span must extract — same module, same binding"))?;
    assert!(
        plan.free_variables.is_empty(),
        "`global count` binds `count`, so nothing flows in: {:?}",
        plan.free_variables
    );
    Ok(())
}

/// Python: augmented assignment of a span-bound name does not trip
/// rule 7 either — the plain assignment above it binds `padded` inside
/// the span.
#[test]
fn python_write_of_span_bound_name_still_extracts() -> Result<()> {
    let text = "base = 3\n\
                padded = base\n\
                padded += 4\n\
                total = padded * 2\n\
                padded = base\n\
                padded += 4\n\
                total = padded * 2\n";
    let needle = "padded = base\npadded += 4\ntotal = padded * 2";
    assert_span_bound_extract(text, needle, "gate.py", "base")
}
