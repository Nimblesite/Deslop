//! Negative-path E2E coverage for the verbatim extract action
//! ([AUTOFIX-EXTRACT-TESTING] case 3): Type-2, cross-file, cross-class,
//! single-occurrence, truncated, hidden, overlapping, mid-expression,
//! and non-exact-bucket clusters must all be silently refused —
//! `Ok(None)`, never an error, never a partial plan.

mod common;

use std::fs;

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    lang::{csharp::CSharpParser, LanguageParser},
    refactor,
    report::ReportCluster,
};

use crate::common::{
    analyse_refactor_fixture as analyse,
    clusters::{both_spans, needle_cluster_plan, report_occurrence, synthetic_report_cluster},
    fixture,
};

const INVOICE_MATH_FILE: &str = "InvoiceMath.cs";
const IDENTICAL_BUCKET: &str = "identical";
const METRICS_FILE: &str = "metrics.py";

/// Asserts that no cluster in the fixture's ranked report yields an
/// extract plan for `file_name`.
fn assert_no_plan(fixture_name: &str, file_name: &str) -> Result<()> {
    let root = fixture(fixture_name);
    let source = fs::read(root.join(file_name)).context("fixture source")?;
    let report = analyse(&root)?;
    ensure!(
        !report.clusters.is_empty(),
        "{fixture_name}: the fixture must produce clusters for the refusal to be meaningful"
    );
    let parser = refactor::parser_for_path(std::path::Path::new(file_name))
        .ok_or_else(|| anyhow!("no parser for {file_name}"))?;
    for cluster in &report.clusters {
        let plan = refactor::compute_plan(cluster, &source, parser.as_ref())?;
        ensure!(
            plan.is_none(),
            "{fixture_name}: cluster {} (bucket {}) must be refused, got a plan",
            cluster.id,
            cluster.bucket
        );
    }
    Ok(())
}

/// Type-2 clusters (renamed identifiers inside the bodies) are refused:
/// the effective spans are not byte-equivalent
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 1) — they belong to
/// [AUTOFIX-MERGE], not to this action.
#[test]
fn type2_renamed_identifiers_refused() -> Result<()> {
    assert_no_plan("csharp-extract-type2", "RateMath.cs")
}

/// Cross-file identical definitions are refused (rule 3) — they belong
/// to [AUTOFIX-CONSOLIDATE].
#[test]
fn cross_file_occurrences_refused() -> Result<()> {
    assert_no_plan("csharp-extract-crossfile", "InvoiceTotals.cs")?;
    assert_no_plan("csharp-extract-crossfile", "ReceiptTotals.cs")
}

/// Same-file occurrences in two different classes are refused (rule 4:
/// shared enclosing parent).
#[test]
fn cross_class_occurrences_refused() -> Result<()> {
    assert_no_plan("csharp-extract-crossclass", "Totals.cs")
}

/// A byte span inside the fixture source.
type Span = (usize, usize);

/// The positive fixture's source plus its two full statement-run spans
/// (`var total…` through `return total;`).
fn positive_fixture() -> Result<(Vec<u8>, Span, Span)> {
    let source = fs::read_to_string(fixture("csharp-extract-type1").join(INVOICE_MATH_FILE))?;
    let (first_start, _) = both_spans(&source, "var total = 0;")?;
    let (_, second_start) = both_spans(&source, "var total = 0;")?;
    let ((_, first_end), (_, second_end)) = both_spans(&source, "return total;")?;
    Ok((
        source.clone().into_bytes(),
        (first_start.0, first_end),
        (second_start.0, second_end),
    ))
}

/// Runs `compute_plan` on a synthetic cluster and asserts refusal.
fn assert_refused(cluster: &ReportCluster, source: &[u8], label: &str) -> Result<()> {
    let parser = CSharpParser::new();
    let plan = refactor::compute_plan(cluster, source, &parser)
        .map_err(|error| anyhow!("{label}: unexpected error {error}"))?;
    ensure!(plan.is_none(), "{label}: cluster must be refused");
    Ok(())
}

/// Rule 2: a single-occurrence cluster is refused.
#[test]
fn single_occurrence_refused() -> Result<()> {
    let (source, first, _) = positive_fixture()?;
    let cluster = synthetic_report_cluster(
        vec![report_occurrence(
            INVOICE_MATH_FILE,
            (first.0, first.1),
            false,
        )],
        IDENTICAL_BUCKET,
    );
    assert_refused(&cluster, &source, "single occurrence")
}

/// A wire-truncated cluster is refused — an unseen occurrence could
/// not be rewritten atomically ([AUTOFIX-EXTRACT-WORKSPACE-EDIT]).
#[test]
fn truncated_cluster_refused() -> Result<()> {
    let (source, first, second) = positive_fixture()?;
    let mut cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (second.0, second.1), false),
        ],
        IDENTICAL_BUCKET,
    );
    cluster.occurrences_truncated = true;
    assert_refused(&cluster, &source, "truncated cluster")
}

/// Hidden occurrences do not count toward rule 2's minimum.
#[test]
fn hidden_occurrence_refused() -> Result<()> {
    let (source, first, second) = positive_fixture()?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (second.0, second.1), true),
        ],
        IDENTICAL_BUCKET,
    );
    assert_refused(&cluster, &source, "hidden second occurrence")
}

/// Overlapping same-file ranges can not be rewritten independently.
#[test]
fn overlapping_ranges_refused() -> Result<()> {
    let (source, first, _) = positive_fixture()?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (first.0 + 10, first.1 + 10), false),
        ],
        IDENTICAL_BUCKET,
    );
    assert_refused(&cluster, &source, "overlapping ranges")
}

/// Rule 5: mid-expression ranges are silently skipped.
#[test]
fn mid_expression_refused() -> Result<()> {
    let (source, ..) = positive_fixture()?;
    let text = String::from_utf8(source.clone())?;
    let (first, second) = both_spans(&text, "amount * taxRate / 100")?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (second.0, second.1), false),
        ],
        IDENTICAL_BUCKET,
    );
    assert_refused(&cluster, &source, "mid-expression range")
}

/// Non-exact buckets (weak LSH / semantic) never reach the slice proof.
#[test]
fn loose_bucket_refused() -> Result<()> {
    let (source, first, second) = positive_fixture()?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (second.0, second.1), false),
        ],
        "loosely_similar",
    );
    assert_refused(&cluster, &source, "loosely_similar bucket")
}

/// Languages without refactor tables (F# today) are refused at the
/// scope-kind gate — the trait default returns `None`.
#[test]
fn language_without_tables_refused() -> Result<()> {
    let (source, first, second) = positive_fixture()?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence(INVOICE_MATH_FILE, (first.0, first.1), false),
            report_occurrence(INVOICE_MATH_FILE, (second.0, second.1), false),
        ],
        IDENTICAL_BUCKET,
    );
    let parser = deslop_core::lang::fsharp::FSharpParser::new();
    ensure!(
        parser.extract_scope_kinds().is_none(),
        "F# has no refactor tables yet"
    );
    let plan = refactor::compute_plan(&cluster, &source, &parser)?;
    ensure!(plan.is_none(), "table-less language must be refused");
    Ok(())
}

/// Rule 4: a statement block with no function-like ancestor (a Rust
/// `const` initializer block) is refused — Rust does not allow module
/// top-level statement runs.
#[test]
fn block_without_enclosing_function_refused() -> Result<()> {
    let source = b"const A: usize = { let seed = 2; seed * 3 };\nconst B: usize = { let seed = 2; seed * 3 };\n";
    let text = std::str::from_utf8(source)?;
    let first = text.find('{').context("first block")?;
    let first_end = text
        .find('}')
        .map(|i| i.saturating_add(1))
        .context("first close")?;
    let resume = first_end;
    let second = text
        .get(resume..)
        .and_then(|rest| rest.find('{'))
        .map(|i| resume.saturating_add(i))
        .context("second block")?;
    let second_end = text
        .get(resume..)
        .and_then(|rest| rest.find('}'))
        .map(|i| resume.saturating_add(i).saturating_add(1))
        .context("second close")?;
    let cluster = synthetic_report_cluster(
        vec![
            report_occurrence("consts.rs", (first, first_end), false),
            report_occurrence("consts.rs", (second, second_end), false),
        ],
        IDENTICAL_BUCKET,
    );
    let parser = deslop_core::lang::rust_lang::RustParser::new();
    let plan = refactor::compute_plan(&cluster, source, &parser)
        .map_err(|error| anyhow!("unexpected error {error}"))?;
    ensure!(
        plan.is_none(),
        "const-initializer blocks have no enclosing function and must be refused"
    );
    Ok(())
}

/// Rule 6 ([AUTOFIX-EXTRACT-PRECONDITIONS], issue #278): a span whose
/// own bindings are read after it is refused — rewriting the run as a
/// call would delete `seed` and `scaled` while the enclosing functions
/// still read them after the span, corrupting code outside the
/// rewritten region.
#[test]
fn bindings_read_after_span_refused() -> Result<()> {
    let text = "fn alpha(input: usize) -> usize {\n    let seed = input + 1;\n    let scaled = seed * 3;\n    scaled + seed\n}\n\nfn beta(input: usize) -> usize {\n    let seed = input + 1;\n    let scaled = seed * 3;\n    scaled - seed\n}\n";
    let needle = "let seed = input + 1;\n    let scaled = seed * 3;";
    let plan = needle_cluster_plan(text, needle, "wallets.rs")?;
    ensure!(
        plan.is_none(),
        "a span whose bindings are read after it must be refused (issue #278)"
    );
    Ok(())
}

/// Rule 6 over the C# fixture's pre-#278 sibling window — `var total…`
/// through the loop end. The window binds `total`, which the `if`
/// guard and `return` read after the span; this is exactly the
/// corrupting shape the old positive test asserted before issue #278
/// retargeted it, pinned here as a refusal.
#[test]
fn csharp_binding_escaping_sibling_window_refused() -> Result<()> {
    let source = fs::read_to_string(fixture("csharp-extract-type1").join(INVOICE_MATH_FILE))
        .context("fixture source")?;
    let needle = "var total = 0;\n        foreach (var amount in amounts)\n        {\n            var taxed = amount * taxRate / 100;\n            total += amount + taxed;\n        }";
    let plan = needle_cluster_plan(&source, needle, INVOICE_MATH_FILE)?;
    ensure!(
        plan.is_none(),
        "the window binds `total`, which is read after the span — must refuse (issue #278)"
    );
    Ok(())
}

/// Rule 6's late-binding half ([AUTOFIX-EXTRACT-PRECONDITIONS], issue
/// #278 hardening): a Python function defined lexically *before* the
/// span reads a span-bound module name when called after it — the body
/// executes at call time, so lexical position does not bound the read.
/// Extracting would leave `total`/`offset` local to the helper and
/// `show()` raising `NameError`.
#[test]
fn python_late_binding_function_read_refused() -> Result<()> {
    let text = "def show():\n    print(total)\n\n\ntotal = base + 1\noffset = total * 2\nmark = 0\ntotal = base + 1\noffset = total * 2\n";
    let needle = "total = base + 1\noffset = total * 2";
    let plan = needle_cluster_plan(text, needle, METRICS_FILE)?;
    ensure!(
        plan.is_none(),
        "a function defined before the span reads its bindings at call time — must refuse"
    );
    Ok(())
}

/// Rule 6's escape-declaration half: a function before the span
/// declares `global total`, so its reads resolve at module scope no
/// matter what its own frame binds — the extract must still refuse.
#[test]
fn python_global_declaration_read_refused() -> Result<()> {
    let text = "def bump():\n    global total\n    return total + 1\n\n\ntotal = base + 1\noffset = total * 2\nmark = 0\ntotal = base + 1\noffset = total * 2\n";
    let needle = "total = base + 1\noffset = total * 2";
    let plan = needle_cluster_plan(text, needle, METRICS_FILE)?;
    ensure!(
        plan.is_none(),
        "a `global` declaration re-binds reads to module scope — must refuse"
    );
    Ok(())
}

/// Rule 6 over a PEP 572 walrus: `last := item` inside a comprehension
/// binds in the *enclosing* scope, not the comprehension frame — so
/// `print(last)` after the span reads a span-created binding and the
/// extract must refuse ([AUTOFIX-EXTRACT-FREE-VARS] hoisting).
#[test]
fn python_walrus_binding_read_after_span_refused() -> Result<()> {
    let text = "values = [1, 2]\npeak = max((last := item) for item in values)\nflag = peak > 0\nprint(last)\nvalues = [3, 4]\npeak = max((last := item) for item in values)\nflag = peak > 0\nprint(last)\n";
    let needle = "peak = max((last := item) for item in values)\nflag = peak > 0";
    let plan = needle_cluster_plan(text, needle, METRICS_FILE)?;
    ensure!(
        plan.is_none(),
        "a walrus binding hoists past the comprehension frame and is read after the span — must refuse"
    );
    Ok(())
}

/// Rule 5 alignment for Python single-statement occurrences: the
/// `expression_statement` wrapper is byte-identical to its expression
/// child, so the deepest-node lookup must hop the extent-equal wrapper
/// to find the statement container above it — a one-statement span is
/// a legitimate extract.
#[test]
fn python_single_statement_occurrence_extracts() -> Result<()> {
    let text = "total = base + 1\nmark = 0\ntotal = base + 1\nprint(mark)\n";
    let needle = "total = base + 1";
    let plan = needle_cluster_plan(text, needle, METRICS_FILE)?
        .ok_or_else(|| anyhow!("a single-statement module-level span must extract"))?;
    ensure!(
        plan.free_variables == ["base"],
        "the lone statement reads only `base`, got {:?}",
        plan.free_variables
    );
    Ok(())
}

/// Rule 6 must not over-refuse on non-reference identifier positions:
/// after the span, `config` appears only as an attribute name and a
/// keyword-argument name — neither reads the span-bound local, so the
/// extract stays offered (per-language skip rules apply to the
/// read-after scan exactly as they do to the free-variable walk).
#[test]
fn python_attribute_and_kwarg_names_after_span_extract() -> Result<()> {
    let text = "config = build(size)\ntag = str(config)\nmark = 0\nconfig = build(size)\ntag = str(config)\nrender(config=1)\nitem.config = 2\n";
    let needle = "config = build(size)\ntag = str(config)";
    let plan = needle_cluster_plan(text, needle, METRICS_FILE)?
        .ok_or_else(|| anyhow!("attribute/kwarg positions are not reads — must extract"))?;
    ensure!(
        plan.free_variables == ["build", "size", "str"],
        "span reads only its callees and `size`, got {:?}",
        plan.free_variables
    );
    Ok(())
}

/// The `LanguageParser` refactor defaults ([AUTOFIX-EXTRACT-DEPENDENCIES]):
/// a language without overrides recognises nothing, emits nothing, and
/// merges nothing — so no action is ever offered for it.
#[test]
fn trait_defaults_refuse_everything() -> Result<()> {
    let parser = deslop_core::lang::fsharp::FSharpParser::new();
    ensure!(
        parser.binding_node_kinds().is_empty(),
        "default binding table is empty"
    );
    ensure!(
        parser
            .identifier_reference_kinds()
            .reference_kinds
            .is_empty(),
        "default reference table recognises nothing"
    );
    ensure!(parser.extract_scope_kinds().is_none(), "no scope kinds");
    ensure!(parser.merge_tables().is_none(), "no merge tables");

    let source = b"let add x = x + 1\n";
    let tree = deslop_core::lang::shared::parse_source("fsharp", &parser.grammar(), source)
        .map_err(|error| anyhow!("parse: {error}"))?;
    ensure!(
        parser
            .declared_type_of(tree.root_node(), "x", source)
            .is_none(),
        "default type lookup finds nothing"
    );
    let scope = deslop_core::refactor::preconditions::OccurrenceScope {
        run: vec![tree.root_node()],
        function: None,
        shared_parent: tree.root_node(),
    };
    let scopes = vec![scope];
    let request = deslop_core::refactor::emit::EmitRequest {
        source,
        cluster_id: "abcdef0123456789",
        free_variables: &[],
        scopes: &scopes,
    };
    ensure!(
        parser.emit_extract_method(&request).is_none(),
        "default extract emitter refuses"
    );
    let merge_request = deslop_core::refactor::merge::MergeEmitRequest {
        source,
        cluster_id: "abcdef0123456789",
        helper_body: "",
        parameters: &[],
        scopes: &scopes,
    };
    ensure!(
        parser.emit_merge_method(&merge_request).is_none(),
        "default merge emitter refuses"
    );
    Ok(())
}
