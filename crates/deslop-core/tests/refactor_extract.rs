//! E2E coverage for the verbatim extract-method refactor
//! ([AUTOFIX-EXTRACT], [AUTOFIX-EXTRACT-FREE-VARS],
//! [AUTOFIX-EXTRACT-EMITTER-CSHARP], [AUTOFIX-EXTRACT-EMITTER-RUST],
//! [AUTOFIX-EXTRACT-EMITTER-PYTHON], [AUTOFIX-EXTRACT-TESTING]).
//!
//! Drives the real pipeline over fixture workspaces, feeds the ranked
//! report's clusters to `refactor::compute_plan`, and asserts the
//! free-variable list, the deterministic method name, and the
//! fully-applied buffer against golden snapshots shared with the LSP
//! code-action tests.

mod common;

use std::fs;

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    refactor::{self, ExtractMethodPlan},
    report::{Report, ReportCluster},
};

use crate::common::{
    analyse_refactor_fixture as analyse,
    census::{assert_body_deduplicated, statement_count},
    clusters::{assert_planned_from_enclosing_view, needle_cluster_plan},
    fixture, refactor_golden as golden,
};

/// Returns the best-ranked cluster for which the verbatim extract plan
/// computes, along with its plan.
fn first_extract_plan(
    report: &Report,
    source: &[u8],
    file_name: &str,
) -> Result<(ReportCluster, ExtractMethodPlan)> {
    let parser = refactor::parser_for_path(std::path::Path::new(file_name))
        .ok_or_else(|| anyhow!("no parser registered for {file_name}"))?;
    for cluster in &report.clusters {
        if let Some(plan) = refactor::compute_plan(cluster, source, parser.as_ref())? {
            return Ok((cluster.clone(), plan));
        }
    }
    Err(anyhow!(
        "no cluster in the ranked report produced an extract plan"
    ))
}

/// One language's golden scenario: fixture, expected free variables,
/// expected deterministic name prefix, and the shared golden file.
struct ExtractCase {
    /// Fixture directory name under the shared pool.
    fixture: &'static str,
    /// Source file inside the fixture.
    file: &'static str,
    /// Expected free variables in first-reference order.
    free_variables: &'static [&'static str],
    /// Language-shaped helper-name prefix ([AUTOFIX-EXTRACT-EMITTER]).
    name_prefix: &'static str,
    /// Golden post-apply buffer shared with the LSP E2E suite.
    golden: &'static str,
    /// Every statement of the duplicated body, as whole syntax nodes.
    /// Twice in the source, exactly once after the refactor — the check
    /// that proves the plan covered the whole duplication and not a
    /// nested slice of it, which is what the golden's embedded cluster
    /// id records.
    duplicated_statements: &'static [&'static str],
}

/// Runs one language's end-to-end scenario: pipeline → plan →
/// free-vars → deterministic name → golden apply.
fn assert_extract_case(case: &ExtractCase) -> Result<()> {
    let root = fixture(case.fixture);
    let source = fs::read(root.join(case.file)).context("fixture source")?;
    let report = analyse(&root)?;
    let (cluster, plan) = first_extract_plan(&report, &source, case.file)?;

    assert_planned_from_enclosing_view(&report, &cluster)?;
    ensure!(
        ["identical", "nearly_identical", "structural_only"].contains(&cluster.bucket.as_str()),
        "extract plans must come from exact-structural buckets \
         ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 1), got bucket {}",
        cluster.bucket
    );
    ensure!(
        plan.free_variables == case.free_variables,
        "{}: free variables must be {:?} in first-reference order, got {:?}",
        case.fixture,
        case.free_variables,
        plan.free_variables
    );
    let expected_name = format!(
        "{}{}",
        case.name_prefix,
        cluster.id.get(..6).unwrap_or_default()
    );
    ensure!(
        plan.method_name == expected_name,
        "method name must embed the cluster id prefix: expected {expected_name}, got {}",
        plan.method_name
    );
    ensure!(
        plan.edits.len() == 3,
        "one insertion plus two call-site rewrites expected, got {} edits",
        plan.edits.len()
    );
    ensure!(
        plan.edits
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left.start_byte >= right.start_byte)),
        "edits must be ordered by descending start byte"
    );

    let applied = String::from_utf8(plan.apply_to(&source)).context("applied buffer utf8")?;
    assert_body_deduplicated(
        std::str::from_utf8(&source).context("fixture source utf8")?,
        &applied,
        case.duplicated_statements,
        case.file,
    )?;
    let golden_path = golden(case.golden);
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(&golden_path, &applied).context("blessing golden")?;
    }
    let expected = fs::read_to_string(&golden_path)
        .with_context(|| format!("golden {}", golden_path.display()))?;
    ensure!(
        applied == expected,
        "applied buffer must match golden {}.\n--- applied ---\n{applied}",
        golden_path.display()
    );
    Ok(())
}

/// Every statement of the body duplicated across `TotalWithTax` and
/// `SubtotalWithTax` — shared with the nested-view control below.
const INVOICE_MATH_BODY: &[&str] = &[
    "var total = 0;",
    "foreach (var amount in amounts) { var taxed = amount * taxRate / 100; \
     total += amount + taxed; }",
    "var taxed = amount * taxRate / 100;",
    "total += amount + taxed;",
    "if (total < 0) { total = 0; }",
    "return total;",
];

/// [AUTOFIX-EXTRACT-EMITTER-CSHARP]: `private static` helper at the top
/// of the enclosing class, `object` placeholders, both bodies rewritten.
#[test]
fn csharp_type1_cluster_extracts_to_golden() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "csharp-extract-type1",
        file: "InvoiceMath.cs",
        free_variables: &["amounts", "taxRate"],
        name_prefix: "ExtractedFromCluster_",
        golden: "InvoiceMath.applied.cs",
        duplicated_statements: INVOICE_MATH_BODY,
    })
}

/// Every statement of the loop duplicated across `TAX_TABLE`'s two
/// methods — the census for the enclosure control below.
const TAX_TABLE_BODY: &[&str] = &[
    "foreach (var amount in amounts) { var taxed = amount * taxRate / 100; \
     lines.Add(amount + taxed); }",
    "var taxed = amount * taxRate / 100;",
    "lines.Add(amount + taxed);",
];

/// Applies the plan computed over `needle` in `TAX_TABLE` and returns
/// the resulting buffer.
fn tax_table_applied(needle: &str) -> Result<String> {
    let plan = needle_cluster_plan(TAX_TABLE, needle, "TaxTable.cs")?
        .context("the needle must produce an applicable plan")?;
    ensure!(
        plan.edits.len() == 3,
        "one insertion plus two call-site rewrites expected, got {}",
        plan.edits.len()
    );
    String::from_utf8(plan.apply_to(TAX_TABLE.as_bytes())).context("applied buffer utf8")
}

/// Control for [PIPELINE-CLUSTER-EXACT]: the whole-body census has to
/// discriminate, or every golden above is self-certifying.
///
/// `TAX_TABLE` carries both views of one duplication — the whole
/// duplicated `foreach` loop, and the two-statement window nested
/// inside it. Both compute a plan, both apply cleanly, both name their
/// helper after their own cluster id, and both produce a buffer that is
/// internally consistent in every way the other assertions can see.
/// Only the statement census separates them, so this test watches it
/// accept the enclosing view and reject the nested one.
#[test]
fn the_whole_body_census_separates_the_enclosing_view_from_a_nested_one() -> Result<()> {
    let nested = tax_table_applied(
        "var taxed = amount * taxRate / 100;\n            lines.Add(amount + taxed);",
    )?;
    ensure!(
        statement_count(
            &nested,
            "var taxed = amount * taxRate / 100;",
            "TaxTable.cs"
        )? == 1,
        "the nested window itself is deduplicated — that is what makes this \
         plan look correct to every other assertion"
    );
    ensure!(
        statement_count(
            &nested,
            "foreach (var amount in amounts) { \
             ExtractedFromCluster_abcdef(amount, taxRate, lines); }",
            "TaxTable.cs",
        )? == 2,
        "the loop enclosing the window survives duplicated, now calling the \
         helper the nested plan extracted"
    );
    ensure!(
        assert_body_deduplicated(TAX_TABLE, &nested, TAX_TABLE_BODY, "TaxTable.cs").is_err(),
        "the census must reject a plan computed from a nested view"
    );

    let enclosing = tax_table_applied(
        "foreach (var amount in amounts)\n        {\n            \
         var taxed = amount * taxRate / 100;\n            \
         lines.Add(amount + taxed);\n        }",
    )?;
    assert_body_deduplicated(TAX_TABLE, &enclosing, TAX_TABLE_BODY, "TaxTable.cs")?;
    Ok(())
}

/// [AUTOFIX-EXTRACT-EMITTER-RUST]: module-scope free function above the
/// first occurrence, `DeslopTodo` alias, `snake_case` deterministic name.
#[test]
fn rust_type1_cluster_extracts_to_golden() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "rust-extract-type1",
        file: "metrics.rs",
        free_variables: &["amounts", "tax_rate"],
        name_prefix: "extracted_from_cluster_",
        golden: "metrics.applied.rs",
        duplicated_statements: &[
            "let mut total = 0;",
            "for amount in amounts { let taxed = amount * tax_rate / 100; \
             total += amount + taxed; }",
            "let taxed = amount * tax_rate / 100;",
            "total += amount + taxed;",
            "if total > 10_000 { total = 10_000; }",
        ],
    })
}

/// [AUTOFIX-EXTRACT-EMITTER-PYTHON]: module-scope `def` above the first
/// occurrence with PEP 8 two-blank-line spacing, bare parameter names.
#[test]
fn python_type1_cluster_extracts_to_golden() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "python-extract-type1",
        file: "metrics.py",
        free_variables: &["amounts", "tax_rate"],
        name_prefix: "extracted_from_cluster_",
        golden: "metrics.applied.py",
        duplicated_statements: &[
            "total = 0",
            "for amount in amounts: taxed = amount * tax_rate // 100 \
             total = total + amount + taxed",
            "taxed = amount * tax_rate // 100",
            "total = total + amount + taxed",
            "if total > 10000: total = 10000",
            "return total",
        ],
    })
}

/// [AUTOFIX-EXTRACT-FREE-VARS] on richer C# shapes: lambda frames bind
/// their parameters, member / call-target / type positions are not
/// references, `out var` binds, and a return inside a lambda does not
/// force a value-returning helper (`void` path).
#[test]
fn csharp_lambda_and_member_access_free_vars() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "csharp-extract-freevars",
        file: "OrderFormatter.cs",
        free_variables: &["orders", "prefix", "Console"],
        name_prefix: "ExtractedFromCluster_",
        golden: "OrderFormatter.applied.cs",
        duplicated_statements: &[
            "var lines = orders.Select(order => { return prefix + order.Name; }).ToList();",
            "foreach (var line in lines) { Console.WriteLine(line); }",
            "Console.WriteLine(line);",
            "if (int.TryParse(prefix, out var code)) { Console.WriteLine(code); }",
            "Console.WriteLine(code);",
        ],
    })
}

/// [AUTOFIX-EXTRACT-FREE-VARS] on richer Rust shapes: closures bind
/// their parameters, match arms bind patterns, macro names and paths
/// are not references, and the helper inserts above the `#[...]`
/// attribute chain with a semicolon-preserving call.
#[test]
fn rust_closure_match_and_attribute_free_vars() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "rust-extract-attrs",
        file: "recorder.rs",
        free_variables: &["items", "label"],
        name_prefix: "extracted_from_cluster_",
        golden: "recorder.applied.rs",
        duplicated_statements: &[
            "let mapped: Vec<String> = items.iter().map(|item| format!(\"{label}: {item}\")).collect();",
            "match mapped.first() { Some(first) => log_line(first), None => log_line(label), }",
            "Some(first) => log_line(first),",
            "None => log_line(label),",
            "log_line(label);",
        ],
    })
}

/// [AUTOFIX-EXTRACT-PRECONDITIONS] rule 4 module-top-level (Python) +
/// [AUTOFIX-EXTRACT-FREE-VARS] comprehension scoping: the comprehension
/// variable binds before its textually-earlier body reads it, and
/// module-level bodies re-indent one step in the helper.
#[test]
fn python_module_level_run_extracts_to_golden() -> Result<()> {
    assert_extract_case(&ExtractCase {
        fixture: "python-extract-module",
        file: "pipeline.py",
        free_variables: &["values", "BASELINE", "sum", "math", "len", "print"],
        name_prefix: "extracted_from_cluster_",
        golden: "pipeline.applied.py",
        duplicated_statements: &[
            "scaled = [value * BASELINE for value in values]",
            "report = {\"total\": sum(scaled), \"sqrt\": math.sqrt(len(scaled)), \"count\": len(values)}",
            "print(report)",
        ],
    })
}

/// Determinism ([AUTOFIX-EXTRACT-EMITTER]): recomputing the plan for
/// the same cluster yields byte-identical edits.
#[test]
fn csharp_type1_plan_is_deterministic() -> Result<()> {
    let root = fixture("csharp-extract-type1");
    let source = fs::read(root.join("InvoiceMath.cs")).context("fixture source")?;
    let report = analyse(&root)?;
    let (_, first) = first_extract_plan(&report, &source, "InvoiceMath.cs")?;
    let (_, second) = first_extract_plan(&report, &source, "InvoiceMath.cs")?;
    ensure!(
        first == second,
        "same cluster and source must produce identical plans"
    );
    Ok(())
}

/// Two C# methods sharing one duplicated loop — the anchor for the
/// rule 5 statement-run alignment tests. The loop writes only its own
/// binding and a member of `lines`, so no free name is written
/// (rule 7, issue #280).
const TAX_TABLE: &str = "public class TaxTable\n\
                         {\n\
                         \x20   public void FillA(int[] amounts, int taxRate, List<int> lines)\n\
                         \x20   {\n\
                         \x20       foreach (var amount in amounts)\n\
                         \x20       {\n\
                         \x20           var taxed = amount * taxRate / 100;\n\
                         \x20           lines.Add(amount + taxed);\n\
                         \x20       }\n\
                         \x20   }\n\
                         \n\
                         \x20   public void FillB(int[] amounts, int taxRate, List<int> lines)\n\
                         \x20   {\n\
                         \x20       foreach (var amount in amounts)\n\
                         \x20       {\n\
                         \x20           var taxed = amount * taxRate / 100;\n\
                         \x20           lines.Add(amount + taxed);\n\
                         \x20       }\n\
                         \x20   }\n\
                         }\n";

/// [AUTOFIX-EXTRACT-PRECONDITIONS] rule 5, single-statement alignment:
/// an occurrence that is exactly one statement node extracts as a
/// one-statement run.
#[test]
fn exact_single_statement_occurrence_extracts() -> Result<()> {
    let needle = "foreach (var amount in amounts)\n        {\n            \
                  var taxed = amount * taxRate / 100;\n            \
                  lines.Add(amount + taxed);\n        }";
    let plan = needle_cluster_plan(TAX_TABLE, needle, "TaxTable.cs")?
        .context("single-statement occurrence must extract")?;
    ensure!(
        plan.free_variables == ["amounts", "taxRate", "lines"],
        "the loop reads its collection, the rate, and the output list, got {:?}",
        plan.free_variables
    );
    ensure!(plan.edits.len() == 3, "insertion + two rewrites");
    Ok(())
}

/// [AUTOFIX-EXTRACT-PRECONDITIONS] rule 5, sibling-run alignment: an
/// occurrence spanning a contiguous statement window inside a block
/// extracts that window. The window's only binding (`taxed`) dies
/// inside it (rule 6, issue #278) and no free name is written
/// (rule 7, issue #280).
#[test]
fn sibling_window_occurrence_extracts() -> Result<()> {
    let needle = "var taxed = amount * taxRate / 100;\n            lines.Add(amount + taxed);";
    let plan = needle_cluster_plan(TAX_TABLE, needle, "TaxTable.cs")?
        .context("two-statement window must extract")?;
    ensure!(
        plan.method_name == "ExtractedFromCluster_abcdef",
        "deterministic name from the synthetic id, got {}",
        plan.method_name
    );
    ensure!(
        plan.free_variables == ["amount", "taxRate", "lines"],
        "loop-body window frees its loop variable, the rate, and the list, got {:?}",
        plan.free_variables
    );
    Ok(())
}

/// `apply_to` skips (rather than panics on) an edit whose range lies
/// outside the buffer — unreachable from `compute_plan`, pinned here
/// so the guard stays honest.
#[test]
fn apply_to_ignores_out_of_range_edits() {
    let plan = ExtractMethodPlan {
        method_name: "x".to_owned(),
        free_variables: Vec::new(),
        edits: vec![deslop_core::refactor::PlannedEdit {
            start_byte: 3,
            end_byte: usize::MAX,
            new_text: "gone".to_owned(),
        }],
    };
    assert_eq!(
        plan.apply_to(b"abc"),
        b"abc",
        "out-of-range edit is a no-op"
    );
}
