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
    report::{Report, ReportCluster, ReportOccurrence, ReportSignals},
};

use crate::common::{analyse_refactor_fixture as analyse, fixture};

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
        plan.symbol == "normalise_labels",
        "symbol recorded, got {}",
        plan.symbol
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
        occurrences.push(ReportOccurrence {
            path: PathBuf::from(name),
            start_byte: start,
            end_byte: end,
            start_line: 0,
            end_line: 0,
            hidden: false,
        });
        let _inserted = sources.insert(PathBuf::from(name), content.as_bytes().to_vec());
    }
    let cluster = ReportCluster {
        id: "abcdef0123456789".to_owned(),
        weight: 1.0,
        size: occurrences.len(),
        canonical_node_count: 40,
        signals: ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
            fused: 1.0,
        },
        bucket: "identical".to_owned(),
        category: "logic".to_owned(),
        occurrences_total: occurrences.len(),
        occurrences,
        occurrences_truncated: false,
        summary: String::new(),
        interpretation: String::new(),
    };
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
