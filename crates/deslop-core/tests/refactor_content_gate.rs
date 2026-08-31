//! [AUTOFIX-EXTRACT-PRECONDITIONS] Refactor safety is proved from the
//! exact source ranges being rewritten. Pair-admission evidence never
//! leaks onto a cluster and therefore cannot authorise an edit.

use std::{fs, path::Path};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    refactor::{self, preconditions},
    report::ReportCluster,
};

use crate::common::{analyse_refactor_fixture as analyse, fixture};

const EXPECTED_REJECTED_CLUSTERS: usize = 0;
const SAME_FILE_FIXTURE_FILES_ANALYSED: usize = 1;
const CROSS_FILE_FIXTURE_FILES_ANALYSED: usize = 2;

/// The single ranked cluster of a two-occurrence fixture.
fn sole_cluster(fixture_name: &str) -> Result<ReportCluster> {
    let report = analyse(&fixture(fixture_name))?;
    ensure!(
        report.clusters.len() == 1,
        "{fixture_name} must report exactly one cluster, got {}",
        report.clusters.len()
    );
    report
        .clusters
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("{fixture_name} reported no cluster"))
}

/// The single-file fixture's source bytes.
fn source_of(fixture_name: &str, file_name: &str) -> Result<Vec<u8>> {
    fs::read(fixture(fixture_name).join(file_name)).context("fixture source")
}

#[test]
fn shape_only_same_file_family_is_rejected_before_closure() -> Result<()> {
    assert_shape_only_family_is_rejected(
        "csharp-shape-only-samefile",
        SAME_FILE_FIXTURE_FILES_ANALYSED,
    )
}

#[test]
fn shape_only_cross_file_family_is_rejected_before_closure() -> Result<()> {
    assert_shape_only_family_is_rejected(
        "csharp-shape-only-crossfile",
        CROSS_FILE_FIXTURE_FILES_ANALYSED,
    )
}

fn assert_shape_only_family_is_rejected(
    fixture_name: &str,
    expected_files_analysed: usize,
) -> Result<()> {
    let report = analyse(&fixture(fixture_name))?;
    assert_eq!(
        report.files_analysed, expected_files_analysed,
        "{fixture_name} must scan every source file before pair admission"
    );
    assert_eq!(
        report.clusters.len(),
        EXPECTED_REJECTED_CLUSTERS,
        "{fixture_name} must be rejected before closure, not sent to refactor"
    );
    assert_eq!(
        report.clusters_hidden, EXPECTED_REJECTED_CLUSTERS,
        "{fixture_name} must not be hidden after cluster formation"
    );
    assert!(
        report.clusters.is_empty(),
        "{fixture_name} must leave no cluster that could reach a refactor action"
    );
    Ok(())
}

#[test]
fn byte_proven_same_file_clone_reaches_verbatim_extract() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-type1")?;
    let source = source_of("csharp-extract-type1", "InvoiceMath.cs")?;
    // The reported windows are whole-method views whose signatures
    // differ in name only; the extract action's equivalence proof runs
    // on the *effective* spans — the body statement runs inside each
    // occurrence ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 5) — and that
    // proof is what must pass, end to end, through compute_plan.
    let parser = refactor::parser_for_path(Path::new("InvoiceMath.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    let plan = refactor::compute_plan(&cluster, &source, parser.as_ref())?
        .ok_or_else(|| anyhow!("the exact clone must still extract"))?;
    assert_eq!(plan.edits.len(), 3);
    assert_eq!(
        plan.method_name,
        format!("ExtractedFromCluster_{}", &cluster.id[..6])
    );
    Ok(())
}

#[test]
fn byte_proven_cross_file_family_reaches_consolidation_resolution() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-crossfile")?;
    assert_eq!(
        deslop_core::report::distinct_visible_path_count(&cluster),
        2
    );
    assert!(preconditions::consolidation_candidate(&cluster));
    Ok(())
}
