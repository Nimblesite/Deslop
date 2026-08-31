//! [AUTOFIX-EXTRACT-PRECONDITIONS] Refactor safety is proved from the
//! exact source ranges being rewritten. Pair-admission evidence never
//! leaks onto a cluster and therefore cannot authorise an edit.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    refactor::{
        self,
        consolidate::{compute_consolidation_plan, ConsolidationOutcome},
        preconditions,
    },
    report::ReportCluster,
    wire_generated::MergeVerdict,
};

use crate::common::{analyse_refactor_fixture as analyse, fixture, merge::merge_plans};

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
fn shape_only_same_file_family_is_refused_by_exact_source_proof() -> Result<()> {
    let cluster = sole_cluster("csharp-shape-only-samefile")?;
    let source = source_of("csharp-shape-only-samefile", "Scaffold.cs")?;
    let ranges = preconditions::eligible_ranges(&cluster)
        .ok_or_else(|| anyhow!("geometry must reach the exact-source proof"))?;
    assert_eq!(ranges.len(), 2);
    assert!(
        !preconditions::slices_equivalent(&source, &ranges),
        "unrelated shape-alike ranges must fail byte-equivalence"
    );
    let parser = refactor::parser_for_path(Path::new("Scaffold.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    assert!(
        refactor::compute_plan(&cluster, &source, parser.as_ref())?.is_none(),
        "no verbatim extract may rewrite unequal source ranges"
    );
    let plans = merge_plans("csharp-shape-only-samefile", "Scaffold.cs")?;
    assert_eq!(plans.len(), 1);
    let plan = plans
        .first()
        .ok_or_else(|| anyhow!("the fixture must produce one merge plan"))?;
    let MergeVerdict::AiOrHuman { reason } = &plan.verdict else {
        return Err(anyhow!("unequal source must never merge mechanically"));
    };
    assert!(plan.workspace_edit.is_none());
    assert!(!reason.is_empty());
    Ok(())
}

#[test]
fn shape_only_cross_file_family_is_refused_after_exact_source_resolution() -> Result<()> {
    let cluster = sole_cluster("csharp-shape-only-crossfile")?;
    assert_eq!(
        deslop_core::report::distinct_visible_path_count(&cluster),
        2
    );
    assert!(
        preconditions::consolidation_candidate(&cluster),
        "the cheap surface screen uses membership only"
    );
    let mut sources: HashMap<PathBuf, Vec<u8>> = HashMap::new();
    for occurrence in &cluster.occurrences {
        let bytes = fs::read(fixture("csharp-shape-only-crossfile").join(&occurrence.path))
            .context("fixture source")?;
        let _previous = sources.insert(occurrence.path.clone(), bytes);
    }
    let parser = refactor::parser_for_path(Path::new("LedgerPosting.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    let outcome = compute_consolidation_plan(&cluster, &sources, parser.as_ref())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Refused(reason) = outcome else {
        return Err(anyhow!(
            "consolidating unequal source would delete a live method"
        ));
    };
    assert!(!reason.is_empty());
    Ok(())
}

#[test]
fn byte_proven_same_file_clone_reaches_verbatim_extract() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-type1")?;
    let source = source_of("csharp-extract-type1", "InvoiceMath.cs")?;
    let ranges = preconditions::eligible_ranges(&cluster)
        .ok_or_else(|| anyhow!("same-file duplicate must yield ranges"))?;
    assert_eq!(ranges.len(), 2);
    assert!(
        preconditions::slices_equivalent(&source, &ranges),
        "Type-1 fixture must be byte-equivalent after whitespace canonicalisation"
    );
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
