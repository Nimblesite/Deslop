//! E2E coverage for cross-file consolidation ([AUTOFIX-CONSOLIDATE],
//! [AUTOFIX-CONSOLIDATE-GATE], [AUTOFIX-CONSOLIDATE-EDIT]).
//!
//! Drives the real pipeline over a two-module Rust crate with an
//! identical `pub fn` in both modules, consolidates it, and proves the
//! rewritten crate **compiles** with `rustc`. Negative shapes (private
//! canonical, would-empty file, non-definition occurrences, non-Rust
//! languages) must refuse with a reason.

mod common;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    lang::{csharp::CSharpParser, rust_lang::RustParser},
    refactor::consolidate::{compute_consolidation_plan, ConsolidationOutcome},
    report::{Report, ReportCluster},
};

use crate::common::{
    analyse_refactor_fixture as analyse, fixture, report_occurrence, synthetic_report_cluster,
};

/// Reads every occurrence path of a cluster into the sources map the
/// consolidation engine consumes.
fn sources_for(root: &Path, cluster: &ReportCluster) -> Result<HashMap<PathBuf, Vec<u8>>> {
    let mut sources = HashMap::new();
    for occurrence in &cluster.occurrences {
        let bytes = fs::read(root.join(&occurrence.path))
            .with_context(|| format!("read {}", occurrence.path.display()))?;
        let _inserted = sources.insert(occurrence.path.clone(), bytes);
    }
    Ok(sources)
}

/// The fixture's cross-file cluster: two occurrences in two files.
fn cross_file_cluster(report: &Report) -> Result<ReportCluster> {
    report
        .clusters
        .iter()
        .find(|cluster| {
            cluster.occurrences.len() == 2
                && cluster
                    .occurrences
                    .first()
                    .map(|occurrence| &occurrence.path)
                    != cluster
                        .occurrences
                        .last()
                        .map(|occurrence| &occurrence.path)
        })
        .cloned()
        .context("a cross-file cluster must surface")
}

/// [AUTOFIX-CONSOLIDATE]: the duplicated `pub fn` consolidates to one
/// canonical copy, the duplicate file imports it, and the rewritten
/// crate compiles (`rustc --emit=metadata`).
#[test]
fn rust_cross_file_definition_consolidates_and_compiles() -> Result<()> {
    let root = fixture("rust-consolidate");
    let report = analyse(&root)?;
    let cluster = cross_file_cluster(&report)?;
    let sources = sources_for(&root, &cluster)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Mechanical(plan) = outcome else {
        return Err(anyhow!(
            "the fixture must consolidate mechanically: {outcome:?}"
        ));
    };

    ensure!(
        plan.symbols == ["normalise_labels"],
        "symbol recorded, got {:?}",
        plan.symbols
    );
    ensure!(
        plan.edits.len() == 2,
        "one deletion plus one import insertion expected, got {}",
        plan.edits.len()
    );
    let import = plan
        .edits
        .iter()
        .find(|edit| !edit.new_text.is_empty())
        .context("import insertion present")?;
    let canonical_module = plan
        .canonical_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("canonical module stem")?;
    ensure!(
        import.new_text == format!("use crate::{canonical_module}::normalise_labels;\n\n"),
        "Schäfer-preserving import, got {:?}",
        import.new_text
    );

    let staging = tempfile::tempdir()?;
    for name in ["lib.rs", "pricing_a.rs", "pricing_b.rs"] {
        let _copied = fs::copy(root.join(name), staging.path().join(name))?;
    }
    let duplicate_path = plan
        .edits
        .first()
        .map(|edit| edit.path.clone())
        .context("edited path present")?;
    let mut buffer = fs::read_to_string(root.join(&duplicate_path))?;
    for edit in &plan.edits {
        ensure!(
            edit.path == duplicate_path,
            "v1 edits target one duplicate file"
        );
        ensure!(edit.end_byte <= buffer.len(), "edit in bounds");
        buffer.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
    }
    fs::write(staging.path().join(&duplicate_path), &buffer)?;

    let output = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit=metadata",
            "lib.rs",
        ])
        .current_dir(staging.path())
        .output()
        .context("rustc available")?;
    ensure!(
        output.status.success(),
        "the consolidated crate must compile ([AUTOFIX-CONSOLIDATE-GATE] backstop):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    ensure!(
        buffer.starts_with("use crate::") && !buffer.contains("pub fn normalise_labels"),
        "duplicate file imports the canonical symbol and no longer defines it:\n{buffer}"
    );
    Ok(())
}

/// Builds a synthetic cross-file cluster over two in-memory files.
fn synthetic_cluster(
    files: &[(&str, &str)],
    needle: &str,
) -> Result<(ReportCluster, HashMap<PathBuf, Vec<u8>>)> {
    let mut occurrences = Vec::new();
    let mut sources = HashMap::new();
    for (name, content) in files {
        let start = content.find(needle).context("definition present")?;
        let end = start.saturating_add(needle.len());
        occurrences.push(report_occurrence(name, (start, end), false));
        let _inserted = sources.insert(PathBuf::from(name), content.as_bytes().to_vec());
    }
    let cluster = synthetic_report_cluster(occurrences, "identical");
    Ok((cluster, sources))
}

/// The refusal reason for a synthetic scenario.
fn refusal_reason(files: &[(&str, &str)], needle: &str) -> Result<String> {
    let (cluster, sources) = synthetic_cluster(files, needle)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    match outcome {
        ConsolidationOutcome::Refused(reason) => Ok(reason),
        ConsolidationOutcome::Mechanical(_) => Err(anyhow!("scenario must refuse")),
    }
}

/// A private canonical definition refuses — the duplicates' modules
/// could not see it ([AUTOFIX-CONSOLIDATE-GATE] visibility).
#[test]
fn private_canonical_refuses() -> Result<()> {
    let definition = "fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    ensure!(reason.contains("private"), "visibility gate, got {reason}");
    Ok(())
}

/// A duplicate file that would become empty refuses — file deletion
/// needs the module declaration rewritten first
/// ([AUTOFIX-CONSOLIDATE-EDIT] v1 gate).
#[test]
fn would_empty_duplicate_refuses() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    ensure!(reason.contains("empty"), "empty-file gate, got {reason}");
    Ok(())
}

/// Occurrences that are not whole top-level definitions refuse.
#[test]
fn non_definition_occurrence_refuses() -> Result<()> {
    let needle = "x + 1";
    let file_a = format!("pub fn a(x: usize) -> usize {{ {needle} }}\n");
    let file_b = format!("pub fn b(x: usize) -> usize {{ {needle} }}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], needle)?;
    ensure!(
        reason.contains("whole top-level"),
        "definition-shape gate, got {reason}"
    );
    Ok(())
}

/// Non-Rust languages refuse with the v1 scope reason.
#[test]
fn non_rust_language_refuses() -> Result<()> {
    let (cluster, sources) = synthetic_cluster(
        &[
            ("A.cs", "class A { void M() { } }"),
            ("B.cs", "class B { void M() { } }"),
        ],
        "class",
    )?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &CSharpParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Refused(reason) = outcome else {
        return Err(anyhow!("non-Rust must refuse in v1"));
    };
    ensure!(reason.contains("csharp"), "language gate, got {reason}");
    Ok(())
}

/// A duplicate file with no remaining references gets only the
/// deletion edit — no import is inserted.
#[test]
fn duplicate_without_references_gets_no_import() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n\npub fn b() -> usize {{ 2 }}\n");
    let (cluster, sources) =
        synthetic_cluster(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Mechanical(plan) = outcome else {
        return Err(anyhow!(
            "reference-free duplicate must consolidate: {outcome:?}"
        ));
    };
    ensure!(
        plan.edits.len() == 1 && plan.edits.iter().all(|edit| edit.new_text.is_empty()),
        "only the deletion edit is planned, got {:?}",
        plan.edits
    );
    Ok(())
}

/// Duplicates in a different directory refuse (v1 sibling-module gate).
#[test]
fn different_directory_duplicate_refuses() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let reason = refusal_reason(
        &[("src/a.rs", &file_a), ("other/b.rs", &file_b)],
        definition,
    )?;
    ensure!(reason.contains("directory"), "sibling gate, got {reason}");
    Ok(())
}

/// Single-file clusters refuse the consolidation shape gate.
#[test]
fn single_file_cluster_refuses_consolidation() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let (mut cluster, sources) = synthetic_cluster(&[("a.rs", &file_a)], definition)?;
    cluster.occurrences = vec![
        cluster.occurrences.first().cloned().context("occurrence")?,
        cluster.occurrences.first().cloned().context("occurrence")?,
    ];
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Refused(reason) = outcome else {
        return Err(anyhow!("single-file cluster must refuse"));
    };
    ensure!(reason.contains("two files"), "shape gate, got {reason}");
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] binding-drift (issue #277): byte-identical
/// definitions that call a module-local `next` which differs per file
/// must refuse — after consolidation the moved reference would bind to
/// the canonical module's `next`, changing behaviour (the traffic-light
/// examples shape).
#[test]
fn module_local_reference_drift_refuses() -> Result<()> {
    let run = "pub fn run(initial: usize) -> usize {\n    next(initial)\n}";
    let file_a = format!("pub fn next(state: usize) -> usize {{\n    state + 1\n}}\n\n{run}\n");
    let file_b = format!("pub fn next(state: usize) -> usize {{\n    state + 2\n}}\n\n{run}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("next"),
        "binding-drift gate must name the drifting symbol, got {reason}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] definition runs (issue #277): an
/// occurrence covering two adjacent whole definitions splits per
/// definition, consolidates both, and the rewritten crate compiles.
#[test]
fn definition_run_spanning_two_functions_consolidates() -> Result<()> {
    let shared = "pub fn scale(value: usize) -> usize {\n    value * 2\n}\n\npub fn offset(value: usize) -> usize {\n    value + 7\n}";
    let file_a = format!("{shared}\n\npub fn total_a() -> usize {{\n    scale(offset(1))\n}}\n");
    let file_b = format!("{shared}\n\npub fn total_b() -> usize {{\n    scale(offset(2))\n}}\n");
    let (cluster, sources) =
        synthetic_cluster(&[("mod_a.rs", &file_a), ("mod_b.rs", &file_b)], shared)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Mechanical(plan) = outcome else {
        return Err(anyhow!(
            "a run of whole definitions must consolidate: {outcome:?}"
        ));
    };
    let staging = tempfile::tempdir()?;
    fs::write(
        staging.path().join("lib.rs"),
        "mod mod_a;\nmod mod_b;\npub use mod_a::total_a;\npub use mod_b::total_b;\n",
    )?;
    fs::write(staging.path().join("mod_a.rs"), &file_a)?;
    let mut buffer = file_b.clone();
    let mut edits = plan.edits.clone();
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    for edit in &edits {
        ensure!(
            edit.path == Path::new("mod_b.rs"),
            "edits target the duplicate file, got {}",
            edit.path.display()
        );
        ensure!(edit.end_byte <= buffer.len(), "edit in bounds");
        buffer.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
    }
    fs::write(staging.path().join("mod_b.rs"), &buffer)?;
    let output = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit=metadata",
            "lib.rs",
        ])
        .current_dir(staging.path())
        .output()
        .context("rustc available")?;
    ensure!(
        output.status.success(),
        "the consolidated crate must compile ([AUTOFIX-CONSOLIDATE-GATE] backstop):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    ensure!(
        !buffer.contains("pub fn scale") && !buffer.contains("pub fn offset"),
        "duplicate no longer defines the consolidated run:\n{buffer}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] v1.1 (issue #279): `use` declarations
/// binding a free reference must be textually identical across the
/// duplicate files — otherwise the moved reference re-binds.
#[test]
fn use_declaration_drift_refuses() -> Result<()> {
    let run = "pub fn run(value: usize) -> usize {\n    scale(value)\n}";
    let file_a =
        format!("use crate::mathx::scale;\n\n{run}\n\npub fn keep_a() -> usize {{\n    7\n}}\n");
    let file_b =
        format!("use crate::mathy::scale;\n\n{run}\n\npub fn keep_b() -> usize {{\n    9\n}}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("use") && reason.contains("scale"),
        "use-binding drift names the symbol, got {reason}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] v1.1 (issue #279): a free reference
/// defined in only some duplicate files refuses — resolution would
/// differ per module.
#[test]
fn partially_defined_reference_refuses() -> Result<()> {
    let run = "pub fn run(value: usize) -> usize {\n    scale(value)\n}";
    let file_a = format!("fn scale(value: usize) -> usize {{\n    value * 2\n}}\n\n{run}\n");
    let file_b =
        format!("use crate::mathz::scale;\n\n{run}\n\npub fn keep_b() -> usize {{\n    3\n}}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("scale") && reason.contains("exactly once"),
        "partially defined reference names the symbol, got {reason}"
    );
    Ok(())
}

/// A canonical file whose stem is not a valid Rust module name refuses
/// — the import rewrite could not name it.
#[test]
fn invalid_canonical_module_name_refuses() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let reason = refusal_reason(&[("9a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    ensure!(
        reason.contains("module name"),
        "module-stem gate, got {reason}"
    );
    Ok(())
}

/// v1.1.1 (#279 review): a name possibly bound by a glob `use` refuses
/// — the gate cannot see through wildcards.
#[test]
fn glob_import_reference_refuses() -> Result<()> {
    let run = "pub fn run(value: usize) -> usize {\n    scale(value)\n}";
    let file_a =
        format!("use crate::mathx::*;\n\n{run}\n\npub fn keep_a() -> usize {{\n    7\n}}\n");
    let file_b =
        format!("use crate::mathy::*;\n\n{run}\n\npub fn keep_b() -> usize {{\n    9\n}}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("scale"),
        "glob-bound reference refuses naming the symbol, got {reason}"
    );
    Ok(())
}

/// v1.1.1 (#279 review): an associated fn lives inside an `impl` block
/// — not a top-level item — so its cross-file resolution refuses.
#[test]
fn impl_associated_fn_drift_refuses() -> Result<()> {
    let run = "pub fn run() -> u32 {\n    Light::next()\n}";
    let file_a = format!(
        "pub struct Light;\n\nimpl Light {{\n    pub fn next() -> u32 {{\n        1\n    }}\n}}\n\n{run}\n"
    );
    let file_b = format!(
        "pub struct Light;\n\nimpl Light {{\n    pub fn next() -> u32 {{\n        2\n    }}\n}}\n\n{run}\n\npub fn drive() -> u32 {{\n    run()\n}}\n"
    );
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("next"),
        "impl-defined reference refuses, got {reason}"
    );
    Ok(())
}

/// v1.1.1 (#279 review): method calls may resolve to impl-defined
/// items whose bodies drift — refused when any occurrence file's impls
/// define the method name.
#[test]
fn method_call_on_local_impl_refuses() -> Result<()> {
    let run = "pub fn run(gauge: Gauge) -> u32 {\n    gauge.scale()\n}";
    let file_a = format!(
        "pub struct Gauge;\n\nimpl Gauge {{\n    pub fn scale(self) -> u32 {{\n        1\n    }}\n}}\n\n{run}\n"
    );
    let file_b = format!(
        "pub struct Gauge;\n\nimpl Gauge {{\n    pub fn scale(self) -> u32 {{\n        2\n    }}\n}}\n\n{run}\n\npub fn drive() -> u32 {{\n    run(Gauge)\n}}\n"
    );
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("scale"),
        "method resolution drift refuses, got {reason}"
    );
    Ok(())
}

/// v1.1.1 (#279 review): stability is transitive — a byte-equivalent
/// helper whose own references drift refuses, naming the leaf.
#[test]
fn transitive_helper_drift_refuses() -> Result<()> {
    let run = "pub fn run(value: usize) -> usize {\n    scale(value)\n}";
    let scale = "pub fn scale(value: usize) -> usize {\n    base() + value\n}";
    let file_a = format!("{scale}\n\nfn base() -> usize {{\n    1\n}}\n\n{run}\n");
    let file_b = format!(
        "{scale}\n\nfn base() -> usize {{\n    2\n}}\n\n{run}\n\npub fn drive() -> usize {{\n    run(4)\n}}\n"
    );
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("base"),
        "transitive drift refuses naming the leaf, got {reason}"
    );
    Ok(())
}

/// v1.1.1 (#279 review): names provable neither locally, via `use`,
/// nor in the std prelude refuse — the gate proves stability, never
/// assumes it.
#[test]
fn unprovable_reference_refuses() -> Result<()> {
    let run = "pub fn run(value: usize) -> usize {\n    helpers::scale(value)\n}";
    let file_a = format!("{run}\n\npub fn keep_a() -> usize {{\n    7\n}}\n");
    let file_b = format!("{run}\n\npub fn keep_b() -> usize {{\n    9\n}}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], run)?;
    ensure!(
        reason.contains("helpers"),
        "unprovable reference refuses, got {reason}"
    );
    Ok(())
}

/// Hidden occurrences do not count toward the cross-file screen — the
/// LSP offer and the engine must agree ([AUTOFIX-CONSOLIDATE-SURFACE]
/// parity, #279 review).
#[test]
fn hidden_occurrence_does_not_count_toward_cross_file() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let (mut cluster, sources) =
        synthetic_cluster(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    if let Some(second) = cluster.occurrences.get_mut(1) {
        second.hidden = true;
    }
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Refused(reason) = outcome else {
        return Err(anyhow!("hidden second file must not consolidate"));
    };
    ensure!(reason.contains("two files"), "shape gate, got {reason}");
    Ok(())
}

/// The import lands after inner doc comments (`//!`) — inserting at
/// byte 0 would make them invalid (#279 review).
#[test]
fn import_lands_after_inner_doc_comments() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b =
        format!("//! Ledger sibling.\n\n{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let (cluster, sources) = synthetic_cluster(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Mechanical(plan) = outcome else {
        return Err(anyhow!("doc-headed duplicate must consolidate: {outcome:?}"));
    };
    let mut buffer = file_b.clone();
    for edit in &plan.edits {
        buffer.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
    }
    ensure!(
        buffer.starts_with("//! Ledger sibling.\n\nuse crate::a::helper;"),
        "import lands after the inner doc comment:\n{buffer}"
    );
    Ok(())
}

/// An outer attribute present only on the duplicate's definition makes
/// the moved spans differ — refused, never an orphaned attribute
/// (#279 review).
#[test]
fn attribute_only_on_duplicate_refuses() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("#[inline]\n{definition}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    ensure!(
        reason.contains("byte-equivalent"),
        "attribute asymmetry refuses, got {reason}"
    );
    Ok(())
}

/// Outer attributes and doc comments shared by every copy move with
/// the definition — the duplicate keeps neither (#279 review).
#[test]
fn shared_attribute_and_docs_move_with_definition() -> Result<()> {
    let decorated =
        "/// Doubles and offsets.\n#[inline]\npub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{decorated}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!("{decorated}\n\npub fn b() -> usize {{ helper(2) }}\n");
    let needle = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let (cluster, sources) = synthetic_cluster(&[("a.rs", &file_a), ("b.rs", &file_b)], needle)?;
    let outcome = compute_consolidation_plan(&cluster, &sources, &RustParser::new())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Mechanical(plan) = outcome else {
        return Err(anyhow!("decorated duplicate must consolidate: {outcome:?}"));
    };
    let mut buffer = file_b.clone();
    for edit in &plan.edits {
        buffer.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
    }
    ensure!(
        !buffer.contains("#[inline]") && !buffer.contains("/// Doubles"),
        "attribute and doc comment are deleted with the definition:\n{buffer}"
    );
    ensure!(
        buffer.starts_with("use crate::a::helper;"),
        "import re-binds the caller:\n{buffer}"
    );
    Ok(())
}

/// A duplicate file defining the symbol twice refuses — resolution
/// would be ambiguous.
#[test]
fn double_definition_duplicate_refuses() -> Result<()> {
    let definition = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";
    let file_a = format!("{definition}\n\npub fn a() -> usize {{ helper(1) }}\n");
    let file_b = format!(
        "{definition}\n\npub fn helper(x: usize) -> usize {{ x }}\n\npub fn b() -> usize {{ helper(2) }}\n"
    );
    let reason = refusal_reason(&[("a.rs", &file_a), ("b.rs", &file_b)], definition)?;
    ensure!(
        reason.contains("more than once"),
        "ambiguity gate, got {reason}"
    );
    Ok(())
}
