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
const CIRCUIT_FIXTURE: &str = "csharp-circuit-breaker-runs";
/// [PIPELINE-CLUSTER-SUBSUME-STRADDLE] The eleven renamed configuration
/// checks and the region directives that follow them, one file at a time:
/// every sibling from `Should_throw_if_failure_threshold_is_zero` to the
/// `#region` that opens the next section is copied with `Async` appended
/// to the breaker calls. Longer than one sibling window, so the run only
/// reaches the report whole when its windows are joined.
const CIRCUIT_SIDES: [(&str, u64, u64); 2] = [
    ("AdvancedCircuitBreakerSpecs.cs", 32, 165),
    ("AdvancedCircuitBreakerAsyncSpecs.cs", 31, 164),
];

#[test]
fn related_constructor_checks_keep_the_complete_three_method_copy() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), MIN_NODES)?;
    let cluster = clone_findings(&report)
        .into_iter()
        .find(|cluster| {
            SIDES
                .iter()
                .all(|side| has_side_span(cluster, side, FIRST_LINE, LAST_LINE))
        })
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

/// A renamed copy longer than one sibling window is one finding at its
/// full extent — not three overlapping eight-method fragments of it, and
/// not the fragments' shared core. The run outranks everything else, and
/// no other clone finding reports any part of it again.
#[test]
fn circuit_breaker_sync_async_copy_is_one_complete_run() -> Result<()> {
    let report = run_report(&fixture(CIRCUIT_FIXTURE), MIN_NODES)?;
    let findings = clone_findings(&report);
    let run = findings
        .iter()
        .find(|cluster| {
            CIRCUIT_SIDES
                .iter()
                .all(|(side, start, end)| has_side_span(cluster, side, *start, *end))
        })
        .context("one nearly identical cluster is the whole renamed sync/async run")?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FILES_ANALYSED)
    );
    assert_eq!(cluster_size(run), OCCURRENCE_COUNT, "{report:#}");
    assert_eq!(cluster_kind(run), NEARLY_IDENTICAL_KIND, "{report:#}");
    assert_eq!(field(run, "rank").as_u64(), Some(FIRST_RANK));
    let fragments: Vec<(String, (u64, u64))> = findings
        .iter()
        .filter(|cluster| field(cluster, "id") != field(run, "id"))
        .flat_map(|cluster| occurrences(cluster).iter())
        .filter_map(|occurrence| {
            let path = occurrence_path(occurrence).ok()?;
            let span = occurrence_line_span(occurrence);
            CIRCUIT_SIDES
                .iter()
                .any(|(side, start, end)| {
                    path.ends_with(side) && span.0 <= *end && *start <= span.1
                })
                .then_some((path.to_owned(), span))
        })
        .collect();
    assert_eq!(
        fragments,
        Vec::new(),
        "no other clone finding reports any part of the run: {report:#}"
    );
    Ok(())
}

fn has_side_span(cluster: &Value, side: &str, first: u64, last: u64) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        occurrence_path(occurrence).is_ok_and(|path| path.ends_with(side))
            && occurrence_line_span(occurrence) == (first, last)
    })
}
