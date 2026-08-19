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
    lang::{csharp::CSharpParser, rust_lang::RustParser, LanguageParser},
    refactor::consolidate::{
        compute_consolidation_plan, ConsolidatePlan, ConsolidationOutcome, PlannedFileEdit,
    },
    report::{Report, ReportCluster},
};

use crate::common::{
    analyse_refactor_fixture as analyse,
    clusters::{report_occurrence, synthetic_report_cluster},
    fixture,
};

/// File bytes keyed by the occurrence path, as the engine consumes them.
type Sources = HashMap<PathBuf, Vec<u8>>;

/// The duplicated definition every `helper`-shaped scenario shares.
const HELPER: &str = "pub fn helper(x: usize) -> usize {\n    x + 1\n}";

/// The free-reference definition the binding-drift gates share.
const RUN_SCALE: &str = "pub fn run(value: usize) -> usize {\n    scale(value)\n}";

/// The sibling file names a synthetic scenario defaults to.
const SIBLINGS: (&str, &str) = ("a.rs", "b.rs");

/// Reads every occurrence path of a cluster into the sources map the
/// consolidation engine consumes.
fn sources_for(root: &Path, cluster: &ReportCluster) -> Result<Sources> {
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

/// Builds a synthetic cluster over in-memory files sharing `needle`.
fn synthetic_cluster(files: &[(&str, &str)], needle: &str) -> Result<(ReportCluster, Sources)> {
    let mut occurrences = Vec::new();
    let mut sources = HashMap::new();
    for (name, content) in files {
        let start = content.find(needle).context("definition present")?;
        let end = start.saturating_add(needle.len());
        occurrences.push(report_occurrence(name, (start, end), false));
        let _inserted = sources.insert(PathBuf::from(name), content.as_bytes().to_vec());
    }
    Ok((synthetic_report_cluster(occurrences, "identical"), sources))
}

/// Cluster and sources for two sibling files sharing `needle`.
fn pair_cluster(
    files: (&str, &str),
    names: (&str, &str),
    needle: &str,
) -> Result<(ReportCluster, Sources)> {
    synthetic_cluster(&[(names.0, files.0), (names.1, files.1)], needle)
}

/// The engine's verdict for an already-built cluster.
fn outcome_for(
    cluster: &ReportCluster,
    sources: &Sources,
    parser: &dyn LanguageParser,
) -> Result<ConsolidationOutcome> {
    compute_consolidation_plan(cluster, sources, parser)
        .map_err(|error| anyhow!("consolidation failed: {error}"))
}

/// The plan for a cluster the scenario expects to consolidate.
fn plan_for(
    scenario: &str,
    cluster: &ReportCluster,
    sources: &Sources,
    parser: &dyn LanguageParser,
) -> Result<ConsolidatePlan> {
    match outcome_for(cluster, sources, parser)? {
        ConsolidationOutcome::Mechanical(plan) => Ok(plan),
        refused @ ConsolidationOutcome::Refused(_) => {
            Err(anyhow!("{scenario} must consolidate: {refused:?}"))
        }
    }
}

/// The reason for a cluster the scenario expects to refuse.
fn reason_for(
    scenario: &str,
    cluster: &ReportCluster,
    sources: &Sources,
    parser: &dyn LanguageParser,
) -> Result<String> {
    match outcome_for(cluster, sources, parser)? {
        ConsolidationOutcome::Refused(reason) => Ok(reason),
        planned @ ConsolidationOutcome::Mechanical(_) => {
            Err(anyhow!("{scenario} must refuse: {planned:?}"))
        }
    }
}

/// The refusal reason for two sibling Rust files sharing `needle`.
fn pair_reason(file_a: &str, file_b: &str, names: (&str, &str), needle: &str) -> Result<String> {
    let (cluster, sources) = pair_cluster((file_a, file_b), names, needle)?;
    reason_for("the scenario", &cluster, &sources, &RustParser::new())
}

/// The plan for two sibling Rust files sharing `needle`.
fn pair_plan(
    scenario: &str,
    files: (&str, &str),
    names: (&str, &str),
    needle: &str,
) -> Result<ConsolidatePlan> {
    let (cluster, sources) = pair_cluster(files, names, needle)?;
    plan_for(scenario, &cluster, &sources, &RustParser::new())
}

/// `prefix` (possibly empty) followed by the shared definition.
fn prefixed(prefix: &str, definition: &str) -> String {
    format!("{prefix}{definition}\n")
}

/// [`prefixed`] plus a trailing item, so the duplicate keeps content
/// of its own once the definition moves out.
fn with_trailer(prefix: &str, definition: &str, trailer: &str) -> String {
    format!("{}\n{trailer}", prefixed(prefix, definition))
}

/// A `keep_*` fn that stops a duplicate file from becoming empty.
fn keeper(name: &str, value: usize) -> String {
    format!("pub fn keep_{name}() -> usize {{\n    {value}\n}}\n")
}

/// A unit struct with one inherent method returning `body`.
fn inherent_impl(type_name: &str, method: &str, receiver: &str, body: usize) -> String {
    format!(
        "pub struct {type_name};\n\nimpl {type_name} {{\n    pub fn {method}({receiver}) -> u32 {{\n        {body}\n    }}\n}}\n\n"
    )
}

/// A file defining `definition`, then a caller that invokes `helper`.
fn helper_caller(definition: &str, caller: &str, argument: usize) -> String {
    format!("{definition}\n\npub fn {caller}() -> usize {{ helper({argument}) }}\n")
}

/// The stock sibling pair: both files define `definition` and call it.
fn helper_pair(definition: &str) -> (String, String) {
    (
        helper_caller(definition, "a", 1),
        helper_caller(definition, "b", 2),
    )
}

/// `source` with `edits` applied in the order given.
fn apply_edits(source: &str, edits: &[PlannedFileEdit]) -> String {
    edits.iter().fold(source.to_owned(), |mut buffer, edit| {
        buffer.replace_range(edit.start_byte..edit.end_byte, &edit.new_text);
        buffer
    })
}

/// [`apply_edits`], proving each edit targets `expected` and is in bounds.
fn apply_checked_edits(source: &str, edits: &[PlannedFileEdit], expected: &Path) -> Result<String> {
    for edit in edits {
        ensure!(
            edit.path == expected,
            "edits target the one duplicate file {}, got {}",
            expected.display(),
            edit.path.display()
        );
        ensure!(edit.end_byte <= source.len(), "edit in bounds");
    }
    Ok(apply_edits(source, edits))
}

/// Proves the staged crate compiles ([AUTOFIX-CONSOLIDATE-GATE] backstop).
fn ensure_compiles(staging: &Path) -> Result<()> {
    let output = std::process::Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit=metadata",
            "lib.rs",
        ])
        .current_dir(staging)
        .output()
        .context("rustc available")?;
    ensure!(
        output.status.success(),
        "the consolidated crate must compile ([AUTOFIX-CONSOLIDATE-GATE] backstop):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

/// The fixture plan's import edit names the canonical module.
fn ensure_canonical_import(plan: &ConsolidatePlan) -> Result<()> {
    let inserted = plan.edits.iter().find(|edit| !edit.new_text.is_empty());
    let import = inserted.context("import insertion present")?;
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
    Ok(())
}

/// Applies the plan to a staged copy of the fixture crate, proves it
/// compiles, and proves the duplicate re-points at the canonical copy.
fn rewrite_and_compile_fixture(root: &Path, plan: &ConsolidatePlan) -> Result<()> {
    let staging = tempfile::tempdir()?;
    for name in ["lib.rs", "pricing_a.rs", "pricing_b.rs"] {
        let _copied = fs::copy(root.join(name), staging.path().join(name))?;
    }
    let duplicate = &plan.edits.first().context("edited path present")?.path;
    let source = fs::read_to_string(root.join(duplicate))?;
    let buffer = apply_checked_edits(&source, &plan.edits, duplicate)?;
    fs::write(staging.path().join(duplicate), &buffer)?;
    ensure_compiles(staging.path())?;
    ensure!(
        buffer.starts_with("use crate::"),
        "duplicate file imports the canonical symbol:\n{buffer}"
    );
    ensure!(
        !buffer.contains("pub fn normalise_labels"),
        "duplicate file no longer defines it:\n{buffer}"
    );
    Ok(())
}

/// Stages `mod_a.rs`/`mod_b.rs` under a `lib.rs` re-exporting both totals,
/// and proves the crate compiles.
fn compile_two_module_crate(file_a: &str, file_b: &str) -> Result<()> {
    let staging = tempfile::tempdir()?;
    fs::write(
        staging.path().join("lib.rs"),
        "mod mod_a;\nmod mod_b;\npub use mod_a::total_a;\npub use mod_b::total_b;\n",
    )?;
    fs::write(staging.path().join("mod_a.rs"), file_a)?;
    fs::write(staging.path().join("mod_b.rs"), file_b)?;
    ensure_compiles(staging.path())
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
    let plan = plan_for("the fixture", &cluster, &sources, &RustParser::new())?;
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
    ensure_canonical_import(&plan)?;
    rewrite_and_compile_fixture(&root, &plan)
}

/// A private canonical definition refuses — the duplicates' modules
/// could not see it ([AUTOFIX-CONSOLIDATE-GATE] visibility).
#[test]
fn private_canonical_refuses() -> Result<()> {
    let definition = "fn helper(x: usize) -> usize {\n    x + 1\n}";
    let (file_a, file_b) = helper_pair(definition);
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, definition)?;
    ensure!(reason.contains("private"), "visibility gate, got {reason}");
    Ok(())
}

/// A duplicate file that would become empty refuses — file deletion
/// needs the module declaration rewritten first
/// ([AUTOFIX-CONSOLIDATE-EDIT] v1 gate).
#[test]
fn would_empty_duplicate_refuses() -> Result<()> {
    let file_a = helper_caller(HELPER, "a", 1);
    let file_b = format!("{HELPER}\n");
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, HELPER)?;
    ensure!(reason.contains("empty"), "empty-file gate, got {reason}");
    Ok(())
}

/// Occurrences that are not whole top-level definitions refuse.
#[test]
fn non_definition_occurrence_refuses() -> Result<()> {
    let needle = "x + 1";
    let file_a = format!("pub fn a(x: usize) -> usize {{ {needle} }}\n");
    let file_b = format!("pub fn b(x: usize) -> usize {{ {needle} }}\n");
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, needle)?;
    ensure!(
        reason.contains("whole top-level"),
        "definition-shape gate, got {reason}"
    );
    Ok(())
}

/// Non-Rust languages refuse with the v1 scope reason.
#[test]
fn non_rust_language_refuses() -> Result<()> {
    let files = ("class A { void M() { } }", "class B { void M() { } }");
    let (cluster, sources) = pair_cluster(files, ("A.cs", "B.cs"), "class")?;
    let reason = reason_for("non-Rust in v1", &cluster, &sources, &CSharpParser::new())?;
    ensure!(reason.contains("csharp"), "language gate, got {reason}");
    Ok(())
}

/// A duplicate file with no remaining references gets only the
/// deletion edit — no import is inserted.
#[test]
fn duplicate_without_references_gets_no_import() -> Result<()> {
    let file_a = helper_caller(HELPER, "a", 1);
    let file_b = format!("{HELPER}\n\npub fn b() -> usize {{ 2 }}\n");
    let files = (file_a.as_str(), file_b.as_str());
    let plan = pair_plan("reference-free duplicate", files, SIBLINGS, HELPER)?;
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
    let (file_a, file_b) = helper_pair(HELPER);
    let names = ("src/a.rs", "other/b.rs");
    let reason = pair_reason(&file_a, &file_b, names, HELPER)?;
    ensure!(reason.contains("directory"), "sibling gate, got {reason}");
    Ok(())
}

/// Single-file clusters refuse the consolidation shape gate.
#[test]
fn single_file_cluster_refuses_consolidation() -> Result<()> {
    let file_a = helper_caller(HELPER, "a", 1);
    let (mut cluster, sources) = synthetic_cluster(&[("a.rs", &file_a)], HELPER)?;
    cluster.occurrences = vec![
        cluster.occurrences.first().cloned().context("occurrence")?,
        cluster.occurrences.first().cloned().context("occurrence")?,
    ];
    let parser = RustParser::new();
    let reason = reason_for("a single-file cluster", &cluster, &sources, &parser)?;
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
    let next =
        |step: usize| format!("pub fn next(state: usize) -> usize {{\n    state + {step}\n}}\n\n");
    let (file_a, file_b) = (prefixed(&next(1), run), prefixed(&next(2), run));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, run)?;
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
    let names = ("mod_a.rs", "mod_b.rs");
    let files = (file_a.as_str(), file_b.as_str());
    let plan = pair_plan("a run of whole definitions", files, names, shared)?;
    let mut edits = plan.edits.clone();
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    let buffer = apply_checked_edits(&file_b, &edits, Path::new(names.1))?;
    compile_two_module_crate(&file_a, &buffer)?;
    ensure!(
        !buffer.contains("pub fn scale"),
        "duplicate no longer defines the consolidated `scale`:\n{buffer}"
    );
    ensure!(
        !buffer.contains("pub fn offset"),
        "duplicate no longer defines the consolidated `offset`:\n{buffer}"
    );
    Ok(())
}

/// [AUTOFIX-CONSOLIDATE-GATE] v1.1 (issue #279): `use` declarations
/// binding a free reference must be textually identical across the
/// duplicate files — otherwise the moved reference re-binds.
#[test]
fn use_declaration_drift_refuses() -> Result<()> {
    let file_a = with_trailer("use crate::mathx::scale;\n\n", RUN_SCALE, &keeper("a", 7));
    let file_b = with_trailer("use crate::mathy::scale;\n\n", RUN_SCALE, &keeper("b", 9));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, RUN_SCALE)?;
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
    let local = "fn scale(value: usize) -> usize {\n    value * 2\n}\n\n";
    let file_a = prefixed(local, RUN_SCALE);
    let file_b = with_trailer("use crate::mathz::scale;\n\n", RUN_SCALE, &keeper("b", 3));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, RUN_SCALE)?;
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
    let (file_a, file_b) = helper_pair(HELPER);
    let reason = pair_reason(&file_a, &file_b, ("9a.rs", "b.rs"), HELPER)?;
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
    let file_a = with_trailer("use crate::mathx::*;\n\n", RUN_SCALE, &keeper("a", 7));
    let file_b = with_trailer("use crate::mathy::*;\n\n", RUN_SCALE, &keeper("b", 9));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, RUN_SCALE)?;
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
    let drive = "pub fn drive() -> u32 {\n    run()\n}\n";
    let file_a = prefixed(&inherent_impl("Light", "next", "", 1), run);
    let file_b = with_trailer(&inherent_impl("Light", "next", "", 2), run, drive);
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, run)?;
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
    let drive = "pub fn drive() -> u32 {\n    run(Gauge)\n}\n";
    let file_a = prefixed(&inherent_impl("Gauge", "scale", "self", 1), run);
    let file_b = with_trailer(&inherent_impl("Gauge", "scale", "self", 2), run, drive);
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, run)?;
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
    let scale = "pub fn scale(value: usize) -> usize {\n    base() + value\n}";
    let with_base = |base: usize| format!("{scale}\n\nfn base() -> usize {{\n    {base}\n}}\n\n");
    let file_a = prefixed(&with_base(1), RUN_SCALE);
    let drive = "pub fn drive() -> usize {\n    run(4)\n}\n";
    let file_b = with_trailer(&with_base(2), RUN_SCALE, drive);
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, RUN_SCALE)?;
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
    let file_a = with_trailer("", run, &keeper("a", 7));
    let file_b = with_trailer("", run, &keeper("b", 9));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, run)?;
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
    let (file_a, file_b) = helper_pair(HELPER);
    let files = (file_a.as_str(), file_b.as_str());
    let (mut cluster, sources) = pair_cluster(files, SIBLINGS, HELPER)?;
    if let Some(second) = cluster.occurrences.get_mut(1) {
        second.hidden = true;
    }
    let parser = RustParser::new();
    let reason = reason_for("a hidden second file", &cluster, &sources, &parser)?;
    ensure!(reason.contains("two files"), "shape gate, got {reason}");
    Ok(())
}

/// The import lands after inner doc comments (`//!`) — inserting at
/// byte 0 would make them invalid (#279 review).
#[test]
fn import_lands_after_inner_doc_comments() -> Result<()> {
    let file_a = helper_caller(HELPER, "a", 1);
    let file_b = format!("//! Ledger sibling.\n\n{}", helper_caller(HELPER, "b", 2));
    let files = (file_a.as_str(), file_b.as_str());
    let plan = pair_plan("a doc-headed duplicate", files, SIBLINGS, HELPER)?;
    let buffer = apply_edits(&file_b, &plan.edits);
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
    let file_a = helper_caller(HELPER, "a", 1);
    let file_b = format!("#[inline]\n{}", helper_caller(HELPER, "b", 2));
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, HELPER)?;
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
    let decorated = format!("/// Doubles and offsets.\n#[inline]\n{HELPER}");
    let (file_a, file_b) = helper_pair(&decorated);
    let files = (file_a.as_str(), file_b.as_str());
    let plan = pair_plan("a decorated duplicate", files, SIBLINGS, HELPER)?;
    let buffer = apply_edits(&file_b, &plan.edits);
    ensure!(
        !buffer.contains("#[inline]"),
        "the attribute is deleted with the definition:\n{buffer}"
    );
    ensure!(
        !buffer.contains("/// Doubles"),
        "the doc comment is deleted with the definition:\n{buffer}"
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
    let file_a = helper_caller(HELPER, "a", 1);
    let second = "pub fn helper(x: usize) -> usize { x }\n\n";
    let file_b = format!("{HELPER}\n\n{second}pub fn b() -> usize {{ helper(2) }}\n");
    let reason = pair_reason(&file_a, &file_b, SIBLINGS, HELPER)?;
    ensure!(
        reason.contains("more than once"),
        "ambiguity gate, got {reason}"
    );
    Ok(())
}
