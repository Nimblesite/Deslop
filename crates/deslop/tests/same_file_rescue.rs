//! [FUSED-SHARED-SUBTREE-SAME-FILE] A near-miss rescued inside one file
//! is held to the same echo rule as one rescued across files.
//!
//! Two sibling classes in one file each hold a byte-identical method.
//! The classes measure high overlap *because of that method*; a rescue
//! that admitted them would hand [PIPELINE-CLUSTER-SUBSUME] a wider,
//! byte-divergent view that encloses the exact one and replaces it. The
//! finding is the method, at its own lines, in both classes.

use std::{collections::BTreeSet, ops::RangeInclusive};

use anyhow::Result;

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, distinct_texts,
    has_verbatim_pair,
};
use crate::common::*;

/// One file, two classes, one method copied byte for byte between them.
const SIBLING_CLASS_FIXTURE: &str = "csharp-same-file-class-echo";
/// The only file the fixture holds.
const SIBLING_CLASS_FILE: &str = "Ledgers.cs";
/// The 1-based lines of `AlphaLedger.Reconcile`.
const ALPHA_RECONCILE_LINES: RangeInclusive<u64> = 7..=22;
/// The 1-based lines of `BetaLedger.Reconcile`, the byte-identical copy.
const BETA_RECONCILE_LINES: RangeInclusive<u64> = 31..=46;
/// A floor above the one-line `WithinCeiling` accessors and the field
/// declarations, so the method pair is the only duplication in reach.
const SIBLING_CLASS_MIN_NODES: u32 = 20;
/// Files the fixture holds; a one-file scan must still analyse it.
const SIBLING_CLASS_FILE_COUNT: u64 = 1;

#[test]
fn sibling_classes_wrapping_one_exact_method_publish_the_method() -> Result<()> {
    let scan_root = fixture(SIBLING_CLASS_FIXTURE);
    let report = run_report(&scan_root, SIBLING_CLASS_MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(SIBLING_CLASS_FILE_COUNT),
        "the one file must be analysed: {report:#}"
    );
    // [FUSED-SHARED-SUBTREE-ECHO] The class pair shares nothing beyond
    // the method it wraps, so it is refused and cannot widen the finding.
    let published = clusters(&report);
    assert_eq!(
        published.len(),
        1,
        "the copied method is the only duplication at this floor: {report:#}"
    );
    let cluster = published
        .first()
        .ok_or_else(|| anyhow::anyhow!("one cluster asserted above"))?;
    assert_method_pair_extents(cluster)?;
    // [PIPELINE-CLUSTER-EXACT-SCOPE] Both occurrences are the authored
    // method: byte-identical, verbatim, mass-honest, clean-surfaced.
    assert_eq!(
        distinct_texts(&scan_root, cluster)?.len(),
        1,
        "the two methods slice to one text: {cluster:#}"
    );
    assert!(
        has_verbatim_pair(&scan_root, cluster)?,
        "a byte-for-byte copy is a verbatim pair: {cluster:#}"
    );
    assert_structural_only_contract(cluster, SIBLING_CLASS_FIXTURE);
    assert_no_pair_surface_on_cluster(cluster, SIBLING_CLASS_FIXTURE);
    assert_published_lines_are_the_two_methods(&report);
    Ok(())
}

/// Each occurrence sits in the one file at its method's own lines; the
/// class shells, fields and accessors around them are not duplicated.
fn assert_method_pair_extents(cluster: &serde_json::Value) -> Result<()> {
    let mut extents: Vec<(String, u64, u64)> = occurrences(cluster)
        .iter()
        .map(|occurrence| {
            Ok((
                occurrence_path(occurrence)?.to_owned(),
                field(occurrence, "start_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("start_line missing: {occurrence:#}"))?,
                field(occurrence, "end_line")
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("end_line missing: {occurrence:#}"))?,
            ))
        })
        .collect::<Result<_>>()?;
    extents.sort();
    let expected: Vec<(String, u64, u64)> = [ALPHA_RECONCILE_LINES, BETA_RECONCILE_LINES]
        .iter()
        .map(|lines| (SIBLING_CLASS_FILE.to_owned(), *lines.start(), *lines.end()))
        .collect();
    assert_eq!(
        extents, expected,
        "both occurrences are the `Reconcile` method, never its class: {cluster:#}"
    );
    Ok(())
}

/// [METRICS-REPO] The duplicated lines are exactly the two method
/// bodies, so the headline count is honest about what was found.
fn assert_published_lines_are_the_two_methods(report: &serde_json::Value) {
    let expected: BTreeSet<u64> = ALPHA_RECONCILE_LINES.chain(BETA_RECONCILE_LINES).collect();
    let published = visible_duplicated_lines(report);
    assert_eq!(
        published.get(SIBLING_CLASS_FILE),
        Some(&expected),
        "only the two methods are duplicated lines: {report:#}"
    );
    assert_eq!(
        visible_duplicated_loc(report),
        line_count(&expected),
        "the duplicated line count is the two methods: {report:#}"
    );
}
