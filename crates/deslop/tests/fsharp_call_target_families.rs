//! [PIPELINE-CLUSTER-SUBSUME-STRADDLE] A copied extension-method family keeps
//! its full extent when the second family systematically changes call targets.

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::*;

const FIXTURE: &str = "fsharp-call-target-family";
const SOURCE: &str = "HtmlOperations.fs";
const MIN_NODES: u32 = 30;
const FILES_ANALYSED: u64 = 1;
const FIRST_FAMILY: (u64, u64) = (8, 45);
const SECOND_FAMILY: (u64, u64) = (56, 93);
const PAIR_OCCURRENCES: u64 = 2;
const FIRST_RANK: u64 = 1;
const FULL_MASS: u64 = 368;
const FULL_NODES: u64 = 368;

#[test]
fn systematic_extension_family_keeps_its_full_copied_extent() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    let whole = clusters(&report)
        .iter()
        .find(|cluster| family_pair(cluster))
        .ok_or_else(|| anyhow!("both complete extension families belong to one cluster"))?;
    assert_eq!(cluster_size(whole), PAIR_OCCURRENCES, "{report:#}");
    assert_eq!(field(whole, "rank").as_u64(), Some(FIRST_RANK));
    assert_eq!(field(whole, "mass").as_u64(), Some(FULL_MASS));
    assert_eq!(
        field(whole, "canonical_node_count").as_u64(),
        Some(FULL_NODES)
    );
    assert_eq!(cluster_kind(whole), NEARLY_IDENTICAL_KIND, "{report:#}");
    Ok(())
}

/// Both complete copy windows must occur in one pair, not scattered fragments.
fn family_pair(cluster: &Value) -> bool {
    let members = occurrences(cluster);
    members
        .iter()
        .any(|member| matches_window(member, FIRST_FAMILY))
        && members
            .iter()
            .any(|member| matches_window(member, SECOND_FAMILY))
}

/// Whether a reported occurrence has the complete measured source window.
fn matches_window(member: &Value, family: (u64, u64)) -> bool {
    let (start, end) = occurrence_line_span(member);
    occurrence_path(member).is_ok_and(|path| path.ends_with(SOURCE)) && (start, end) == family
}
