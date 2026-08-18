//! E2E coverage for the mechanical call-site merge ([AUTOFIX-MERGE],
//! [AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-ANTIUNIFY],
//! [AUTOFIX-MERGE-NAMES], [AUTOFIX-MERGE-MCP], [AUTOFIX-MERGE-DEFAULTS]).
//!
//! Drives the real pipeline over merge fixtures, computes the
//! `MergePlan` through the public API, applies the wire `WorkspaceEdit`
//! as an LSP host would, and asserts the result against a golden shared
//! with the LSP code-action suite. The fixtures a merge must *decline*
//! live in `refactor_merge_refusals.rs`.

mod common;

use std::fs;

use anyhow::{ensure, Context, Result};
use deslop_core::wire_generated::{MergePlan, MergeVerdict};
use serde_json::Value;

use crate::common::{
    fixture,
    merge::{
        assert_merge_golden, first_mechanical, merge_plans, merge_plans_under, synthetic_merge_plan,
    },
};

/// [AUTOFIX-MERGE]: the leaf-gap fixture merges mechanically — typed
/// context parameter, two positional literal parameters, lifted local
/// renames, and a golden applied buffer.
#[test]
fn csharp_leafgap_cluster_merges_to_golden() -> Result<()> {
    let plans = merge_plans("csharp-merge-leafgap", "RateLimits.cs")?;
    let plan = first_mechanical(plans)?;

    ensure!(plan.language == "csharp", "language recorded");
    let expected_name = format!(
        "MergedFromCluster_{}",
        plan.cluster_id.get(..6).unwrap_or_default()
    );
    ensure!(
        plan.helper_name == expected_name,
        "deterministic helper name, got {}",
        plan.helper_name
    );
    let signature: Vec<(String, String)> = plan
        .parameters
        .iter()
        .map(|parameter| (parameter.type_name.clone(), parameter.name.clone()))
        .collect();
    ensure!(
        signature
            == vec![
                ("RatePolicy".to_owned(), "policy".to_owned()),
                ("string".to_owned(), "arg0".to_owned()),
                ("int".to_owned(), "arg1".to_owned()),
            ],
        "typed parameter list (context + holes), got {signature:?}"
    );
    let site_args: Vec<Vec<String>> = plan
        .parameters
        .iter()
        .map(|parameter| parameter.per_site_arguments.clone())
        .collect();
    ensure!(
        site_args
            == vec![
                vec!["policy".to_owned(), "policy".to_owned()],
                vec!["\"standard\"".to_owned(), "\"premium\"".to_owned()],
                vec!["100".to_owned(), "250".to_owned()],
            ],
        "per-site argument lists, got {site_args:?}"
    );
    ensure!(
        plan.helper_body.contains("var label = arg0;")
            && plan.helper_body.contains("var ceiling = arg1;"),
        "holes spliced with parameter names:\n{}",
        plan.helper_body
    );

    let uri = plan
        .workspace_edit
        .as_ref()
        .and_then(|edit| edit.pointer("/documentChanges/0/textDocument/uri"))
        .and_then(Value::as_str)
        .context("edit uri present")?;
    ensure!(
        uri.starts_with("file://") && uri.ends_with("/RateLimits.cs"),
        "edit targets the fixture file, got {uri}"
    );
    let _applied = assert_merge_golden(
        &plan,
        "csharp-merge-leafgap",
        "RateLimits.cs",
        "RateLimits.merged.cs",
        RATE_LIMITS_BODY,
    )?;
    Ok(())
}

/// The statements duplicated across `ApplyStandard` and `ApplyPremium`
/// — every call the two bodies share verbatim. The two leaf gaps
/// (`"standard"`/`"premium"`, `100`/`250`) differ by construction and
/// become parameters, so they are not part of the census.
const RATE_LIMITS_BODY: &[&str] = &[
    "policy.SetCeiling(label, ceiling);",
    "policy.EnableAlerts(label);",
    "policy.Audit(label, ceiling);",
    "policy.Flush(label);",
    "policy.Seal(ceiling);",
    "policy.Commit();",
];

/// Asserts the RFC 8089 shape every wire `WorkspaceEdit` URI must have
/// (issue #290): an empty authority and exactly three slashes, a
/// forward-slash separated path ending at `tail`, and — because fixture
/// paths are entirely unreserved — no percent-encoding whatsoever.
fn ensure_rfc8089_uri(plan: &MergePlan, tail: &str) -> Result<()> {
    let uri = plan
        .workspace_edit
        .as_ref()
        .context("wire edit present")?
        .pointer("/documentChanges/0/textDocument/uri")
        .and_then(Value::as_str)
        .context("edit uri present")?;
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
    let plan = first_mechanical(merge_plans("csharp-merge-leafgap", "RateLimits.cs")?)?;
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
    let root = fs::canonicalize(fixture("csharp-merge-leafgap")).context("canonical fixture")?;
    let plan = first_mechanical(merge_plans_under(&root, "RateLimits.cs")?)?;
    ensure_rfc8089_uri(&plan, "/csharp-merge-leafgap/RateLimits.cs")
}

/// Determinism: recomputing the plan yields identical JSON.
#[test]
fn csharp_leafgap_plan_is_deterministic() -> Result<()> {
    let first = first_mechanical(merge_plans("csharp-merge-leafgap", "RateLimits.cs")?)?;
    let second = first_mechanical(merge_plans("csharp-merge-leafgap", "RateLimits.cs")?)?;
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
    let plans = merge_plans("rust-merge-leafgap", "pricing.rs")?;
    let plan = first_mechanical(plans)?;
    ensure!(plan.language == "rust", "language recorded");
    ensure!(
        plan.helper_name
            == format!(
                "merged_from_cluster_{}",
                plan.cluster_id.get(..6).unwrap_or_default()
            ),
        "snake_case deterministic helper name, got {}",
        plan.helper_name
    );
    let types: Vec<&str> = plan
        .parameters
        .iter()
        .map(|parameter| parameter.type_name.as_str())
        .collect();
    ensure!(
        types == ["&mut Vec<(String, i64)>", "&'static str", "i64"],
        "declared parameter types, got {types:?}"
    );

    let applied = assert_merge_golden(
        &plan,
        "rust-merge-leafgap",
        "pricing.rs",
        "pricing.merged.rs",
        &[
            "book.push((label.to_owned(), ceiling));",
            "book.push((label.to_uppercase(), ceiling * 2));",
            "book.push((label.to_lowercase(), ceiling + 7));",
            "book.push((label.trim().to_owned(), ceiling - 1));",
            "book.push((label.repeat(2), ceiling / 2));",
            "book.sort();",
        ],
    )?;

    let staging = tempfile::tempdir()?;
    let merged_file = staging.path().join("pricing.rs");
    fs::write(&merged_file, &applied)?;
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
    let plans = merge_plans("dart-merge-leafgap", "pricing.dart")?;
    let plan = first_mechanical(plans)?;
    ensure!(plan.language == "dart", "language recorded");
    ensure!(
        plan.helper_name
            == format!(
                "mergedFromCluster_{}",
                plan.cluster_id.get(..6).unwrap_or_default()
            ),
        "lowerCamel deterministic helper name, got {}",
        plan.helper_name
    );
    let types: Vec<&str> = plan
        .parameters
        .iter()
        .map(|parameter| parameter.type_name.as_str())
        .collect();
    ensure!(
        types == ["List<String>", "String", "int"],
        "declared parameter types, got {types:?}"
    );

    let _applied = assert_merge_golden(
        &plan,
        "dart-merge-leafgap",
        "pricing.dart",
        "pricing.merged.dart",
        &[
            "book.add(label);",
            "book.add(label.toUpperCase());",
            "book.add(label.toLowerCase());",
            "book.add(label.trim());",
            "book.add(ceiling.toString());",
            "book.sort();",
        ],
    )?;
    Ok(())
}

/// Baker rename lifting ([AUTOFIX-MERGE-GATE] step 3): consistently
/// renamed locals lift — the helper keeps the canonical names and the
/// only parameters are the typed context ones.
#[test]
fn consistent_renames_lift_without_parameters() -> Result<()> {
    let plans = merge_plans("csharp-merge-rename", "RateMath.cs")?;
    let plan = first_mechanical(plans)?;
    let names: Vec<&str> = plan
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect();
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
    let plans = merge_plans("csharp-merge-identhole", "Router.cs")?;
    let plan = first_mechanical(plans)?;
    let signature: Vec<(&str, &str)> = plan
        .parameters
        .iter()
        .map(|parameter| (parameter.type_name.as_str(), parameter.name.as_str()))
        .collect();
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
    let plan = synthetic_merge_plan(
        &root,
        "Tiers.cs",
        &["\"bronze\"", "\"silver\"", "\"gold\""],
        span_for_body,
    )?;
    ensure!(
        matches!(plan.verdict, MergeVerdict::Mechanical),
        "the three-site fixture merges mechanically, got {:?}",
        plan.verdict
    );
    let ceiling = plan
        .parameters
        .last()
        .context("trailing parameter present")?;
    ensure!(
        ceiling.default_value.as_deref() == Some("100") && !ceiling.is_required,
        "trailing slot defaults to the modal value, got {ceiling:?}"
    );
    let edit = plan.workspace_edit.as_ref().context("edit present")?;
    let rendered = edit.to_string();
    ensure!(
        rendered.contains("MergedFromCluster_") && rendered.contains("= 100"),
        "the helper renders the default"
    );
    let elided = plan
        .parameters
        .last()
        .map(|parameter| parameter.per_site_arguments.clone())
        .unwrap_or_default();
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
