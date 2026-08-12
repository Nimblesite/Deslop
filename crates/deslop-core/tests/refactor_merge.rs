//! E2E coverage for the mechanical call-site merge ([AUTOFIX-MERGE],
//! [AUTOFIX-MERGE-GATE], [AUTOFIX-MERGE-ANTIUNIFY],
//! [AUTOFIX-MERGE-SAFETY], [AUTOFIX-MERGE-NAMES]).
//!
//! Drives the real pipeline over merge fixtures, computes the
//! `MergePlan` through the public API, applies the wire
//! `WorkspaceEdit`, and asserts the result against a golden. Negative
//! fixtures (control-flow drift, type conflicts) must route to
//! `ai_or_human` with a reason — never a partial plan.

mod common;

use std::{fs, path::Path};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    ast::ByteRange,
    refactor::{self, merge},
    wire_generated::{MergePlan, MergeVerdict},
};
use serde_json::Value;

use crate::common::{
    clusters::{report_occurrence, synthetic_report_cluster},
    fixture, refactor_golden as golden, refactor_pipeline_session as session,
};

/// Computes merge plans for every cluster of a single-file fixture and
/// returns them in rank order.
fn merge_plans(fixture_name: &str, file_name: &str) -> Result<Vec<MergePlan>> {
    merge_plans_under(&fixture(fixture_name), file_name)
}

/// Computes merge plans for a fixture rooted at an already-resolved
/// path, so a caller can choose the shape of the absolute path the
/// plans are addressed with (plain, or canonicalised as the servers do).
fn merge_plans_under(root: &Path, file_name: &str) -> Result<Vec<MergePlan>> {
    let (session, report) = session(root)?;
    let absolute = root.join(file_name);
    let file_id = session
        .file_id_for(&absolute)
        .context("fixture file registered")?;
    let source = session
        .source_bytes_for(file_id)
        .context("source retrievable")?
        .to_vec();
    let file_root = session
        .subtree_at_range(
            file_id,
            ByteRange {
                start: 0,
                end: source.len(),
            },
        )
        .context("file root retrievable")?;
    let parser = refactor::parser_for_path(&absolute).context("parser registered for fixture")?;
    ensure!(
        !report.clusters.is_empty(),
        "{} must produce clusters",
        root.display()
    );
    report
        .clusters
        .iter()
        .map(|cluster| {
            merge::compute_merge_plan(cluster, &source, file_root, &absolute, parser.as_ref())
                .map_err(|error| anyhow!("merge plan failed: {error}"))
        })
        .collect()
}

/// The first mechanical plan in rank order; errors list every
/// refusal reason so a failing fixture explains itself.
fn first_mechanical(plans: Vec<MergePlan>) -> Result<MergePlan> {
    let reasons: Vec<String> = plans
        .iter()
        .map(|plan| match &plan.verdict {
            MergeVerdict::Mechanical => format!("{}: mechanical", plan.cluster_id),
            MergeVerdict::AiOrHuman { reason } => format!("{}: {reason}", plan.cluster_id),
        })
        .collect();
    plans
        .into_iter()
        .find(|plan| matches!(plan.verdict, MergeVerdict::Mechanical))
        .ok_or_else(|| anyhow!("no mechanical plan; verdicts:\n{}", reasons.join("\n")))
}

/// Applies a wire `WorkspaceEdit` (documentChanges form) to an ASCII
/// buffer, mirroring an LSP host.
fn apply_workspace_edit(source: &str, edit: &Value) -> Result<String> {
    ensure!(source.is_ascii(), "fixture must stay ASCII for offset math");
    let edits = edit
        .pointer("/documentChanges/0/edits")
        .and_then(Value::as_array)
        .context("documentChanges edits present")?;
    let mut buffer = source.to_owned();
    for entry in edits {
        let start = byte_offset(source, entry.pointer("/range/start").context("start")?)?;
        let end = byte_offset(source, entry.pointer("/range/end").context("end")?)?;
        let new_text = entry
            .pointer("/newText")
            .and_then(Value::as_str)
            .context("newText")?;
        ensure!(start <= end && end <= buffer.len(), "edit range in bounds");
        buffer.replace_range(start..end, new_text);
    }
    Ok(buffer)
}

/// ASCII line/character → byte offset against the original buffer.
fn byte_offset(source: &str, position: &Value) -> Result<usize> {
    let line = position
        .pointer("/line")
        .and_then(Value::as_u64)
        .context("line")?;
    let character = position
        .pointer("/character")
        .and_then(Value::as_u64)
        .context("character")?;
    let line_start = source
        .split_inclusive('\n')
        .scan(0_usize, |offset, text| {
            let start = *offset;
            *offset = offset.saturating_add(text.len());
            Some(start)
        })
        .nth(usize::try_from(line).context("line fits")?)
        .context("line exists")?;
    Ok(line_start.saturating_add(usize::try_from(character).context("char fits")?))
}

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

    let root = fixture("csharp-merge-leafgap");
    let source = fs::read_to_string(root.join("RateLimits.cs"))?;
    let edit = plan.workspace_edit.as_ref().context("wire edit present")?;
    let uri = edit
        .pointer("/documentChanges/0/textDocument/uri")
        .and_then(Value::as_str)
        .context("edit uri present")?;
    ensure!(
        uri.starts_with("file://") && uri.ends_with("/RateLimits.cs"),
        "edit targets the fixture file, got {uri}"
    );
    let applied = apply_workspace_edit(&source, edit)?;
    let golden_path = golden("RateLimits.merged.cs");
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(&golden_path, &applied).context("blessing golden")?;
    }
    let expected = fs::read_to_string(&golden_path).context("golden")?;
    ensure!(
        applied == expected,
        "applied merge must match golden.\n--- applied ---\n{applied}"
    );
    Ok(())
}

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

/// Control-flow drift routes to `ai_or_human` with a structural reason
/// ([AUTOFIX-MERGE-GATE] step 2) — never a partial plan.
#[test]
fn structural_drift_routes_to_ai_or_human() -> Result<()> {
    let plans = merge_plans("csharp-merge-drift", "DriftLimits.cs")?;
    ensure!(!plans.is_empty(), "drift fixture produces clusters");
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!(
                "drifted cluster {} must not merge",
                plan.cluster_id
            ));
        };
        ensure!(!reason.is_empty(), "refusal carries a reason");
        ensure!(
            plan.workspace_edit.is_none(),
            "refusals carry no workspace edit"
        );
    }
    Ok(())
}

/// A slot whose literals disagree on type routes to `ai_or_human`
/// ([AUTOFIX-MERGE-SAFETY] D) — no `object` guessing.
#[test]
fn literal_type_conflict_routes_to_ai_or_human() -> Result<()> {
    let plans = merge_plans("csharp-merge-typeconflict", "MixedDefaults.cs")?;
    ensure!(!plans.is_empty(), "type-conflict fixture produces clusters");
    for plan in plans {
        ensure!(
            matches!(plan.verdict, MergeVerdict::AiOrHuman { .. }),
            "type-conflicting cluster {} must not merge",
            plan.cluster_id
        );
    }
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

    let root = fixture("rust-merge-leafgap");
    let source = fs::read_to_string(root.join("pricing.rs"))?;
    let edit = plan.workspace_edit.as_ref().context("wire edit present")?;
    let applied = apply_workspace_edit(&source, edit)?;

    let golden_path = golden("pricing.merged.rs");
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(&golden_path, &applied).context("blessing golden")?;
    }
    let expected = fs::read_to_string(&golden_path).context("golden")?;
    ensure!(
        applied == expected,
        "applied merge must match golden.\n--- applied ---\n{applied}"
    );

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

    let root = fixture("dart-merge-leafgap");
    let source = fs::read_to_string(root.join("pricing.dart"))?;
    let edit = plan.workspace_edit.as_ref().context("wire edit present")?;
    let applied = apply_workspace_edit(&source, edit)?;
    let golden_path = golden("pricing.merged.dart");
    if std::env::var_os("DESLOP_BLESS").is_some() {
        fs::write(&golden_path, &applied).context("blessing golden")?;
    }
    let expected = fs::read_to_string(&golden_path).context("golden")?;
    ensure!(
        applied == expected,
        "applied merge must match golden.\n--- applied ---\n{applied}"
    );
    Ok(())
}

/// [AUTOFIX-ZERO-RISK]: Python merges always refuse in v1 — strict
/// type checking cannot be detected yet, and without it there is no
/// compiler backstop.
#[test]
fn python_merge_always_refuses() -> Result<()> {
    let plans = merge_plans("python-extract-type1", "metrics.py")?;
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!("python cluster {} must refuse", plan.cluster_id));
        };
        ensure!(
            reason.contains("python"),
            "the reason names the language gate, got {reason}"
        );
    }
    Ok(())
}

/// Asserts that at least one plan of a fixture refuses with a reason
/// containing `needle`, and that no plan is mechanical.
fn assert_all_refused_with(fixture_name: &str, file_name: &str, needle: &str) -> Result<()> {
    let plans = merge_plans(fixture_name, file_name)?;
    ensure!(!plans.is_empty(), "{fixture_name} produces clusters");
    let mut matched = false;
    for plan in plans {
        let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
            return Err(anyhow!(
                "{fixture_name}: cluster {} must refuse",
                plan.cluster_id
            ));
        };
        matched = matched || reason.contains(needle);
    }
    ensure!(matched, "{fixture_name}: some refusal names `{needle}`");
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

/// [AUTOFIX-MERGE-SAFETY] B: a `return` crossing the boundary refuses.
#[test]
fn boundary_crossing_return_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-return", "EarlyExit.cs", "transfers control")
}

/// [AUTOFIX-MERGE-SAFETY] B: a local declared inside the span and read
/// after it refuses.
#[test]
fn declared_inside_read_after_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-readafter", "Prefix.cs", "read after")
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

/// [AUTOFIX-MERGE-SAFETY] D: a hole identifier written inside the span
/// refuses — call-time evaluation would change behaviour.
#[test]
fn written_hole_identifier_refuses() -> Result<()> {
    assert_all_refused_with("csharp-merge-writtenhole", "Mutator.cs", "written inside")
}

/// [AUTOFIX-MERGE-SAFETY] / extract rule 7 (#280): a *context* free
/// variable (identical at every site, so never a hole) written inside
/// the span refuses — the helper's by-value parameter copy would
/// absorb the mutation and every caller's variable would keep its old
/// value.
#[test]
fn written_context_variable_refuses() -> Result<()> {
    assert_all_refused_with(
        "csharp-merge-writtencontext",
        "Accumulator.cs",
        "written inside",
    )
}

/// Same context-write refusal through the Dart tables — the only
/// coverage of Dart's `write_kinds` (Dart has no Tier-1 emitter, so an
/// extract-path Dart test would be vacuous).
#[test]
fn dart_written_context_variable_refuses() -> Result<()> {
    assert_all_refused_with(
        "dart-merge-writtencontext",
        "accumulator.dart",
        "written inside",
    )
}

/// [AUTOFIX-MERGE-DEFAULTS]: a trailing slot shared by all-but-one of
/// three sites gains a default and the matching calls elide it. The
/// three-sibling family is hidden by the ranked report (#197), so the
/// cluster is built synthetically over the fixture's method bodies.
#[test]
fn three_site_merge_defaults_trailing_parameter() -> Result<()> {
    let root = fixture("csharp-merge-defaults");
    let (session, _report) = session(&root)?;
    let absolute = root.join("Tiers.cs");
    let file_id = session.file_id_for(&absolute).context("file id")?;
    let source = session
        .source_bytes_for(file_id)
        .context("source")?
        .to_vec();
    let text = String::from_utf8(source.clone())?;
    let spans: Vec<(usize, usize)> = ["\"bronze\"", "\"silver\"", "\"gold\""]
        .iter()
        .map(|label| span_for_body(&text, label))
        .collect::<Result<_>>()?;
    let occurrences: Vec<deslop_core::report::ReportOccurrence> = spans
        .iter()
        .map(|span| report_occurrence("Tiers.cs", *span, false))
        .collect();
    let cluster = synthetic_report_cluster(occurrences, "nearly_identical");
    let file_root = session
        .subtree_at_range(
            file_id,
            ByteRange {
                start: 0,
                end: source.len(),
            },
        )
        .context("file root")?;
    let parser = refactor::parser_for_path(&absolute).context("parser")?;
    let plan = merge::compute_merge_plan(&cluster, &source, file_root, &absolute, parser.as_ref())
        .map_err(|error| anyhow!("merge failed: {error}"))?;
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

/// The residual byte proof: operator drift outside the holes refuses
/// even though the normalised skeletons match.
#[test]
fn operator_drift_refuses_via_residual_proof() -> Result<()> {
    assert_all_refused_with(
        "csharp-merge-operatordrift",
        "Drift.cs",
        "not byte-equivalent",
    )
}

/// [AUTOFIX-MERGE-GATE] 4b/4c: too many differing leaves refuse.
#[test]
fn too_many_holes_refuse() -> Result<()> {
    let root = fixture("csharp-merge-manyholes");
    let (session, _report) = session(&root)?;
    let absolute = root.join("Sprawl.cs");
    let file_id = session.file_id_for(&absolute).context("file id")?;
    let source = session
        .source_bytes_for(file_id)
        .context("source")?
        .to_vec();
    let text = String::from_utf8(source.clone())?;
    let spans: Vec<(usize, usize)> = ["\"a1\"", "\"a2\""]
        .iter()
        .map(|anchor| sprawl_body_span(&text, anchor))
        .collect::<Result<_>>()?;
    let occurrences: Vec<deslop_core::report::ReportOccurrence> = spans
        .iter()
        .map(|span| report_occurrence("Sprawl.cs", *span, false))
        .collect();
    let cluster = synthetic_report_cluster(occurrences, "nearly_identical");
    let file_root = session
        .subtree_at_range(
            file_id,
            ByteRange {
                start: 0,
                end: source.len(),
            },
        )
        .context("file root")?;
    let parser = refactor::parser_for_path(&absolute).context("parser")?;
    let plan = merge::compute_merge_plan(&cluster, &source, file_root, &absolute, parser.as_ref())
        .map_err(|error| anyhow!("merge failed: {error}"))?;
    let MergeVerdict::AiOrHuman { reason } = plan.verdict else {
        return Err(anyhow!("twelve distinct substitutions must refuse"));
    };
    ensure!(
        reason.contains("exceed the budget"),
        "the substitution budget names itself, got {reason}"
    );
    Ok(())
}

/// The statement span of one `Sprawl.cs` method body.
fn sprawl_body_span(text: &str, anchor: &str) -> Result<(usize, usize)> {
    let position = text.find(anchor).context("anchor present")?;
    let start = text
        .get(..position)
        .and_then(|head| head.rfind("policy.Set("))
        .context("body start")?;
    let end = text
        .get(position..)
        .and_then(|tail| tail.find("policy.Commit();"))
        .map(|offset| {
            position
                .saturating_add(offset)
                .saturating_add("policy.Commit();".len())
        })
        .context("body end")?;
    Ok((start, end))
}
