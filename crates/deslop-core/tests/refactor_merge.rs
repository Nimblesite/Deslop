//! E2E coverage for the mechanical call-site merge ([AUTOFIX-MERGE],
//! [AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-ANTIUNIFY],
//! [AUTOFIX-MERGE-NAMES], [AUTOFIX-MERGE-MCP], [AUTOFIX-MERGE-DEFAULTS]).
//!
//! Drives the real pipeline over merge fixtures, computes the
//! `MergePlan` through the public API, applies the wire `WorkspaceEdit`
//! as an LSP host would, and asserts the result against a golden shared
//! with the LSP code-action suite. The fixtures a merge must *decline*
//! live in `refactor_merge_refusals.rs`.

use std::fs;

use anyhow::{ensure, Context, Result};
use deslop_core::wire_generated::{MergeParameter, MergePlan, MergeVerdict};
use serde_json::Value;

use crate::common::{
    fixture,
    merge::{
        assert_merge_golden, first_mechanical, merge_plans, merge_plans_under, synthetic_merge_plan,
    },
};

/// One leaf-gap fixture whose top cluster must merge mechanically.
/// Every language asserts the same shape — only these values differ, so
/// a language that regressed would otherwise be a copy of its siblings.
struct LeafgapCase {
    /// Fixture directory under `tests/fixtures`.
    fixture: &'static str,
    /// The single source file inside that fixture.
    file: &'static str,
    /// Language the plan must record.
    language: &'static str,
    /// Helper-name prefix, in the language's casing convention.
    helper_prefix: &'static str,
    /// Declared parameter types, in declaration order.
    parameter_types: &'static [&'static str],
    /// Golden holding the applied buffer.
    golden: &'static str,
    /// Statements the duplicated bodies share verbatim; the merge must
    /// leave each behind exactly once.
    duplicated_statements: &'static [&'static str],
}

/// The C# leaf gap: a typed context parameter plus two literal holes,
/// behind a `PascalCase` helper. The census omits the two leaf gaps
/// (`"standard"`/`"premium"`, `100`/`250`) because they differ by
/// construction and become parameters.
const CSHARP_LEAFGAP: LeafgapCase = LeafgapCase {
    fixture: "csharp-merge-leafgap",
    file: "RateLimits.cs",
    language: "csharp",
    helper_prefix: "MergedFromCluster_",
    parameter_types: &["RatePolicy", "string", "int"],
    golden: "RateLimits.merged.cs",
    duplicated_statements: &[
        "policy.SetCeiling(label, ceiling);",
        "policy.EnableAlerts(label);",
        "policy.Audit(label, ceiling);",
        "policy.Flush(label);",
        "policy.Seal(ceiling);",
        "policy.Commit();",
    ],
};

/// The Rust leaf gap: a `snake_case` helper whose declared types must be
/// real enough for `rustc` to accept the merged file.
const RUST_LEAFGAP: LeafgapCase = LeafgapCase {
    fixture: "rust-merge-leafgap",
    file: "pricing.rs",
    language: "rust",
    helper_prefix: "merged_from_cluster_",
    parameter_types: &["&mut Vec<(String, i64)>", "&'static str", "i64"],
    golden: "pricing.merged.rs",
    duplicated_statements: &[
        "book.push((label.to_owned(), ceiling));",
        "book.push((label.to_uppercase(), ceiling * 2));",
        "book.push((label.to_lowercase(), ceiling + 7));",
        "book.push((label.trim().to_owned(), ceiling - 1));",
        "book.push((label.repeat(2), ceiling / 2));",
        "book.sort();",
    ],
};

/// The Dart leaf gap: a `lowerCamel` helper pinned by its golden, since
/// CI has no Dart compiler.
const DART_LEAFGAP: LeafgapCase = LeafgapCase {
    fixture: "dart-merge-leafgap",
    file: "pricing.dart",
    language: "dart",
    helper_prefix: "mergedFromCluster_",
    parameter_types: &["List<String>", "String", "int"],
    golden: "pricing.merged.dart",
    duplicated_statements: &[
        "book.add(label);",
        "book.add(label.toUpperCase());",
        "book.add(label.toLowerCase());",
        "book.add(label.trim());",
        "book.add(ceiling.toString());",
        "book.sort();",
    ],
};

/// The first mechanical plan of a single-file fixture.
fn mechanical_plan(fixture_name: &str, file_name: &str) -> Result<MergePlan> {
    first_mechanical(merge_plans(fixture_name, file_name)?)
}

/// Projects one field out of every helper parameter, in declaration
/// order.
fn map_parameters<'plan, Field>(
    plan: &'plan MergePlan,
    project: impl FnMut(&'plan MergeParameter) -> Field,
) -> Vec<Field> {
    plan.parameters.iter().map(project).collect()
}

/// The `(declared type, name)` pair of every helper parameter.
fn parameter_signature(plan: &MergePlan) -> Vec<(&str, &str)> {
    map_parameters(plan, |parameter| {
        (parameter.type_name.as_str(), parameter.name.as_str())
    })
}

/// The document URI the plan's wire `WorkspaceEdit` edits.
fn edit_uri(plan: &MergePlan) -> Result<&str> {
    plan.workspace_edit
        .as_ref()
        .context("wire edit present")?
        .pointer("/documentChanges/0/textDocument/uri")
        .and_then(Value::as_str)
        .context("edit uri present")
}

/// Computes the fixture's first mechanical plan, asserts the shape every
/// leaf-gap plan must record, then applies the wire edit and asserts the
/// statement census plus the golden. Returns both so a language can
/// assert its own extras.
fn assert_leafgap_merge(case: &LeafgapCase) -> Result<(MergePlan, String)> {
    let plan = mechanical_plan(case.fixture, case.file)?;
    assert_plan_shape(&plan, case)?;
    let applied = assert_merge_golden(
        &plan,
        case.fixture,
        case.file,
        case.golden,
        case.duplicated_statements,
    )?;
    Ok((plan, applied))
}

/// The language, the deterministic helper name (the casing convention
/// applied to the first six cluster-id characters), and the declared
/// parameter types.
fn assert_plan_shape(plan: &MergePlan, case: &LeafgapCase) -> Result<()> {
    ensure!(
        plan.language == case.language,
        "language recorded, got {}",
        plan.language
    );
    let cluster_prefix = plan.cluster_id.get(..6).unwrap_or_default();
    let expected_name = format!("{}{cluster_prefix}", case.helper_prefix);
    ensure!(
        plan.helper_name == expected_name,
        "deterministic helper name {expected_name} expected, got {}",
        plan.helper_name
    );
    let types = map_parameters(plan, |parameter| parameter.type_name.as_str());
    ensure!(
        types.as_slice() == case.parameter_types,
        "declared parameter types, got {types:?}"
    );
    Ok(())
}

/// [AUTOFIX-MERGE]: the leaf-gap fixture merges mechanically — typed
/// context parameter, two positional literal parameters, lifted local
/// renames, and a golden applied buffer.
#[test]
fn csharp_leafgap_cluster_merges_to_golden() -> Result<()> {
    let (plan, _applied) = assert_leafgap_merge(&CSHARP_LEAFGAP)?;
    assert_rate_limits_signature(&plan)?;
    ensure!(
        plan.helper_body.contains("var label = arg0;")
            && plan.helper_body.contains("var ceiling = arg1;"),
        "holes spliced with parameter names:\n{}",
        plan.helper_body
    );
    let uri = edit_uri(&plan)?;
    ensure!(
        uri.starts_with("file://") && uri.ends_with("/RateLimits.cs"),
        "edit targets the fixture file, got {uri}"
    );
    Ok(())
}

/// The C# leaf-gap signature: the typed context parameter followed by
/// the two literal holes, and the concrete argument every call site
/// passes for each.
fn assert_rate_limits_signature(plan: &MergePlan) -> Result<()> {
    let signature = parameter_signature(plan);
    ensure!(
        signature
            == [
                ("RatePolicy", "policy"),
                ("string", "arg0"),
                ("int", "arg1")
            ],
        "typed parameter list (context + holes), got {signature:?}"
    );
    let site_arguments = map_parameters(plan, |parameter| parameter.per_site_arguments.clone());
    ensure!(
        site_arguments
            == [
                ["policy", "policy"],
                ["\"standard\"", "\"premium\""],
                ["100", "250"],
            ],
        "per-site argument lists, got {site_arguments:?}"
    );
    Ok(())
}

/// Asserts the RFC 8089 shape every wire `WorkspaceEdit` URI must have
/// (issue #290): an empty authority and exactly three slashes, a
/// forward-slash separated path ending at `tail`, and — because fixture
/// paths are entirely unreserved — no percent-encoding whatsoever.
fn ensure_rfc8089_uri(plan: &MergePlan, tail: &str) -> Result<()> {
    let uri = edit_uri(plan)?;
    let path = uri
        .strip_prefix("file:///")
        .with_context(|| format!("empty authority plus a rooted path expected, got {uri}"))?;
    ensure!(
        !path.starts_with('/'),
        "exactly three slashes — a fourth leaves an empty first path segment, got {uri}"
    );
    ensure!(
        path.ends_with(tail),
        "forward-slash separated path ending at {tail} expected, got {uri}"
    );
    ensure!(
        !uri.contains('%'),
        "an unreserved path needs no percent-encoding — `%3A` is an \
         encoded drive colon, `%5C` a backslash separator, `%3F` a \
         leaked verbatim marker, got {uri}"
    );
    Ok(())
}

/// [AUTOFIX-MERGE-MCP] (issue #290): the wire `WorkspaceEdit` names the
/// edited file with an RFC 8089 `file:///` URI on every platform.
/// Windows absolute paths must render as `file:///C:/…`, not with a
/// percent-encoded drive colon or backslash separators, or LSP and MCP
/// clients cannot resolve the document.
#[test]
fn wire_edit_uri_is_rfc8089_for_the_platform_absolute_path() -> Result<()> {
    let plan = mechanical_plan(CSHARP_LEAFGAP.fixture, CSHARP_LEAFGAP.file)?;
    ensure_rfc8089_uri(
        &plan,
        "/deslop/tests/fixtures/csharp-merge-leafgap/RateLimits.cs",
    )
}

/// [AUTOFIX-MERGE-MCP] (issue #290): the same URI contract holds for a
/// canonicalised root, which is the shape the shipping servers address
/// plans with — `deslop-mcp` canonicalises `--root` at startup, and on
/// Windows `fs::canonicalize` returns the verbatim `\\?\C:\…` form. The
/// verbatim marker must not survive into the URI.
#[test]
fn wire_edit_uri_is_rfc8089_for_a_canonicalised_absolute_path() -> Result<()> {
    let root = fs::canonicalize(fixture(CSHARP_LEAFGAP.fixture)).context("canonical fixture")?;
    let plan = first_mechanical(merge_plans_under(&root, CSHARP_LEAFGAP.file)?)?;
    ensure_rfc8089_uri(&plan, "/csharp-merge-leafgap/RateLimits.cs")
}

/// Determinism: recomputing the plan yields identical JSON.
#[test]
fn csharp_leafgap_plan_is_deterministic() -> Result<()> {
    let first = mechanical_plan(CSHARP_LEAFGAP.fixture, CSHARP_LEAFGAP.file)?;
    let second = mechanical_plan(CSHARP_LEAFGAP.fixture, CSHARP_LEAFGAP.file)?;
    ensure!(
        serde_json::to_string(&first)? == serde_json::to_string(&second)?,
        "same cluster and source must produce identical plans"
    );
    Ok(())
}

/// [AUTOFIX-MERGE] on Rust plus the type-safety backstop: the merged
/// workspace **compiles** (`rustc --emit=metadata`), proving the typed
/// parameters are real types, not placeholders.
#[test]
fn rust_leafgap_merges_and_compiles() -> Result<()> {
    let (_plan, applied) = assert_leafgap_merge(&RUST_LEAFGAP)?;
    ensure_merged_rust_compiles(&applied)
}

/// Type-checks a merged Rust buffer in a scratch directory.
fn ensure_merged_rust_compiles(applied: &str) -> Result<()> {
    let staging = tempfile::tempdir()?;
    let merged_file = staging.path().join(RUST_LEAFGAP.file);
    fs::write(&merged_file, applied)?;
    let output = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit=metadata",
        ])
        .arg(&merged_file)
        .current_dir(staging.path())
        .output()
        .context("rustc available")?;
    ensure!(
        output.status.success(),
        "the merged Rust workspace must compile ([AUTOFIX-MERGE-NAMES] backstop):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// [AUTOFIX-MERGE] on Dart: typed context + hole parameters and the
/// golden applied buffer (Dart has no compiler in CI; the golden pins
/// the emitted shape).
#[test]
fn dart_leafgap_merges_to_golden() -> Result<()> {
    let _merged = assert_leafgap_merge(&DART_LEAFGAP)?;
    Ok(())
}

/// Baker rename lifting ([AUTOFIX-MERGE-GATE] step 3): consistently
/// renamed locals lift — the helper keeps the canonical names and the
/// only parameters are the typed context ones.
#[test]
fn consistent_renames_lift_without_parameters() -> Result<()> {
    let plan = mechanical_plan("csharp-merge-rename", "RateMath.cs")?;
    let names = map_parameters(&plan, |parameter| parameter.name.as_str());
    ensure!(
        names == ["amounts", "taxRate"],
        "only context parameters survive a pure rename, got {names:?}"
    );
    ensure!(
        plan.helper_body.contains("= 0;") && plan.helper_body.contains("* taxRate / 100;"),
        "the helper keeps one canonical set of local names:\n{}",
        plan.helper_body
    );
    Ok(())
}

/// [AUTOFIX-MERGE-SAFETY] D: a free identifier hole with a unified
/// declared type becomes a typed positional parameter.
#[test]
fn free_identifier_hole_becomes_typed_parameter() -> Result<()> {
    let plan = mechanical_plan("csharp-merge-identhole", "Router.cs")?;
    let signature = parameter_signature(&plan);
    ensure!(
        signature == [("RatePolicy", "policy"), ("int", "arg0")],
        "typed positional identifier parameter, got {signature:?}"
    );
    let arguments: Vec<&str> = plan
        .parameters
        .iter()
        .filter_map(|parameter| parameter.per_site_arguments.last())
        .map(String::as_str)
        .collect();
    ensure!(
        arguments == ["policy", "premiumLimit"],
        "per-site identifier arguments, got {arguments:?}"
    );
    Ok(())
}

/// [AUTOFIX-MERGE-DEFAULTS]: a trailing slot shared by all-but-one of
/// three sites gains a default and the matching calls elide it. The
/// three-sibling family is hidden by the ranked report (#197), so the
/// cluster is built synthetically over the fixture's method bodies.
#[test]
fn three_site_merge_defaults_trailing_parameter() -> Result<()> {
    let root = fixture("csharp-merge-defaults");
    let tiers = ["\"bronze\"", "\"silver\"", "\"gold\""];
    let plan = synthetic_merge_plan(&root, "Tiers.cs", &tiers, span_for_body)?;
    ensure!(
        matches!(plan.verdict, MergeVerdict::Mechanical),
        "the three-site fixture merges mechanically, got {:?}",
        plan.verdict
    );
    assert_trailing_default(&plan)?;
    let edit = plan.workspace_edit.as_ref().context("edit present")?;
    let rendered = edit.to_string();
    ensure!(
        rendered.contains("MergedFromCluster_") && rendered.contains("= 100"),
        "the helper renders the default"
    );
    Ok(())
}

/// The trailing parameter of the three-site plan: it defaults to the
/// modal value, is optional, and the two sites that pass that value
/// elide it.
fn assert_trailing_default(plan: &MergePlan) -> Result<()> {
    let ceiling = plan
        .parameters
        .last()
        .context("trailing parameter present")?;
    ensure!(
        ceiling.default_value.as_deref() == Some("100") && !ceiling.is_required,
        "trailing slot defaults to the modal value, got {ceiling:?}"
    );
    let elided = &ceiling.per_site_arguments;
    ensure!(
        elided
            .iter()
            .filter(|value| value.as_str() == "100")
            .count()
            == 2,
        "two sites share the default, got {elided:?}"
    );
    Ok(())
}

/// The statement span of one `Tiers.cs` method body, from its label
/// declaration through `policy.Commit();`.
fn span_for_body(text: &str, label: &str) -> Result<(usize, usize)> {
    let anchor = text.find(label).context("label present")?;
    let start = text
        .get(..anchor)
        .and_then(|head| head.rfind("var label"))
        .context("body start")?;
    let end = text
        .get(anchor..)
        .and_then(|tail| tail.find("policy.Commit();"))
        .map(|offset| {
            anchor
                .saturating_add(offset)
                .saturating_add("policy.Commit();".len())
        })
        .context("body end")?;
    Ok((start, end))
}
