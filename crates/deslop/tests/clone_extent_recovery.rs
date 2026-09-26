//! [ACCURACY-RECOVERY-TWIN-FN] Three related constructor checks form one
//! authored sync/async copy, even though two later calls add `Async` to
//! their targets. Reporting only the first check loses most of the copy.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::common::*;

#[path = "clone_extent_recovery/adjacent_calls.rs"]
mod adjacent_calls;

const FIXTURE: &str = "csharp-constructor-check-family";
const SIDES: [&str; 2] = ["Sync.cs", "Async.cs"];
const MIN_NODES: u32 = 30;
const FIRST_LINE: u64 = 5;
const LAST_LINE: u64 = 35;
const FILES_ANALYSED: u64 = 2;
const OCCURRENCE_COUNT: u64 = 2;
const FIRST_RANK: u64 = 1;

#[test]
fn related_constructor_checks_keep_the_complete_three_method_copy() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    let cluster = clone_findings(&report)
        .into_iter()
        .find(|cluster| SIDES.iter().all(|side| has_full_side(cluster, side)))
        .context("one visible cluster contains the complete three-method copy in both files")?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    assert_eq!(cluster_size(&cluster), OCCURRENCE_COUNT, "{report:#}");
    assert_eq!(field(&cluster, "rank").as_u64(), Some(FIRST_RANK));
    assert_eq!(cluster_kind(&cluster), NEARLY_IDENTICAL_KIND, "{report:#}");
    Ok(())
}

fn has_full_side(cluster: &Value, side: &str) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        occurrence_path(occurrence).is_ok_and(|path| path.ends_with(side))
            && occurrence_line_span(occurrence) == (FIRST_LINE, LAST_LINE)
    })
}
