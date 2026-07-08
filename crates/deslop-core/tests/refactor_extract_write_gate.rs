//! E2E: [AUTOFIX-EXTRACT-PRECONDITIONS] rule 7 (issue #280) — a span
//! that writes one of its own *free* variables refuses the verbatim
//! extract. The helper would mutate its own parameter copy and the
//! caller's variable would silently keep its old value — the mutation
//! loss the type-safety backstop cannot catch. Writes to span-*bound*
//! names stay extractable: they vacate with the span.

mod common;

use anyhow::Result;

use crate::common::clusters::needle_cluster_plan;

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
    let plan = needle_cluster_plan(text, needle, "InvoiceMath.cs")?;
    assert!(
        plan.is_none(),
        "a span writing free `total` must refuse the extract \
         ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7, #280): {plan:?}"
    );
    Ok(())
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
    let plan = needle_cluster_plan(text, needle, "gate.py")?;
    assert!(
        plan.is_none(),
        "a span writing free `count` must refuse the extract \
         ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7, #280): {plan:?}"
    );
    Ok(())
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
    let plan = needle_cluster_plan(text, needle, "gate.rs")?;
    assert!(
        plan.is_none(),
        "a span writing free `counter` must refuse the extract \
         ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 7, #280): {plan:?}"
    );
    Ok(())
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
    let plan = needle_cluster_plan(text, needle, "Padding.cs")?
        .ok_or_else(|| anyhow::anyhow!("a span writing only its own binding must extract"))?;
    assert_eq!(
        plan.free_variables,
        vec!["size".to_owned()],
        "only `size` flows in; the written `padded` is span-bound, not free"
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
    let plan = needle_cluster_plan(text, needle, "gate.py")?
        .ok_or_else(|| anyhow::anyhow!("a span writing only its own binding must extract"))?;
    assert_eq!(
        plan.free_variables,
        vec!["base".to_owned()],
        "only `base` flows in; the written `padded` is span-bound, not free"
    );
    Ok(())
}
