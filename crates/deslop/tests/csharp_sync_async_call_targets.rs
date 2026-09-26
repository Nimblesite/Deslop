//! [ACCURACY-RECOVERY-TWIN-FN] A changed external selector is an edit to
//! a large copied body, even when the static receiver keeps its name.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::common::*;

const FIXTURE: &str = "csharp-sync-async-call-targets";
const SIDES: [&str; 2] = ["Sync.cs", "Async.cs"];
const MIN_NODES: u32 = 30;
const FILES_ANALYSED: u64 = 2;
const OCCURRENCES: u64 = 2;
const FIRST_LINE: u64 = 1;
const LAST_LINE: u64 = 32;
const FIRST_RANK: u64 = 1;
const CANONICAL_NODES: u64 = 197;
const DUPLICATED_LOC_PER_FILE: u64 = 32;
const REPO_DUPLICATION_PERCENT: f64 = 100.0;
const FAMILY_FIXTURE: &str = "csharp-call-target-families";
const FAMILY_SIDES: [&str; 2] = ["Sync.cs", "Async.cs"];
const FAMILY_START: u64 = 1;
const FAMILY_END: u64 = 79;
const FAMILY_MASS: u64 = 610;

// [ACCURACY-RECOVERY-TWIN-FN] Systematic selector edits within a copied
// family preserve its full reported class extent.
#[test]
fn copied_method_family_keeps_both_complete_classes() -> Result<()> {
    let report = run_report(&fixture(FAMILY_FIXTURE), MIN_NODES)?;
    let cluster = clusters(&report)
        .iter()
        .find(|cluster| {
            FAMILY_SIDES
                .iter()
                .all(|side| has_complete_family(cluster, side))
        })
        .context("one clone contains both complete eight-method classes")?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    assert_eq!(cluster_size(cluster), OCCURRENCES, "{report:#}");
    assert_eq!(field(cluster, "rank").as_u64(), Some(FIRST_RANK));
    assert_eq!(cluster_kind(cluster), NEARLY_IDENTICAL_KIND, "{report:#}");
    assert_eq!(field(cluster, "mass").as_u64(), Some(FAMILY_MASS));
    Ok(())
}

fn has_complete_family(cluster: &Value, side: &str) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        occurrence_path(occurrence).is_ok_and(|path| path.ends_with(side))
            && occurrence_line_span(occurrence) == (FAMILY_START, FAMILY_END)
    })
}

#[test]
fn large_sync_async_copy_survives_changed_static_call_targets() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    let cluster = expect_cluster_spanning(&report, &SIDES)?;
    assert_eq!(cluster_size(cluster), OCCURRENCES, "{report:#}");
    assert_eq!(field(cluster, "rank").as_u64(), Some(FIRST_RANK));
    assert_eq!(cluster_kind(cluster), LOOSELY_SIMILAR_KIND, "{report:#}");
    assert_eq!(
        field(cluster, "canonical_node_count").as_u64(),
        Some(CANONICAL_NODES)
    );
    assert_eq!(field(cluster, "mass").as_u64(), Some(CANONICAL_NODES));
    assert_eq!(
        metric_field(&report, "duplication_percent").as_f64(),
        Some(REPO_DUPLICATION_PERCENT)
    );
    for side in SIDES {
        assert_side(&report, cluster, side)?;
    }
    Ok(())
}

/// Both complete authored files, including every changed selector, are the copy.
fn assert_side(report: &Value, cluster: &Value, side: &str) -> Result<()> {
    let occurrence = occurrences(cluster)
        .iter()
        .find(|occurrence| occurrence_path(occurrence).is_ok_and(|path| path.ends_with(side)))
        .context("cluster has an occurrence on each side")?;
    assert_eq!(
        occurrence_line_span(occurrence),
        (FIRST_LINE, LAST_LINE),
        "{side}: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for(report, side),
        DUPLICATED_LOC_PER_FILE,
        "{side}: {report:#}"
    );
    Ok(())
}
