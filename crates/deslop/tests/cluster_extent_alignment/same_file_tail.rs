//! [PIPELINE-CLUSTER-EXACT-SCOPE] Three C# transitions share one executable setup and assertion.

use super::*;

const FIXTURE: &str = "csharp-same-file-copy-tail";
const FILE: &str = "CircuitChecks.cs";
const NODE_FLOOR: u32 = 30;
const IDENTICAL: &str = "identical";
const LOOSELY_SIMILAR: &str = "loosely_similar";
const COPIES: usize = 3;
const BROAD_RANK: u64 = 1;
const COPIED_WINDOWS: [(u64, u64); COPIES] = [(8, 26), (41, 59), (80, 98)];
const METHOD_WINDOWS: [(u64, u64); COPIES] = [(4, 35), (37, 74), (76, 122)];
const SHAPE_ONLY: &str = "structural_only";

fn includes_all_fault_assertions(cluster: &Value) -> bool {
    let copies = occurrences_in(cluster, FILE);
    copies.len() == COPIES
        && COPIED_WINDOWS.iter().all(|(start, end)| {
            copies.iter().any(|copy| {
                let (actual_start, actual_end) = occurrence_line_span(copy);
                actual_start <= *start && actual_end >= *end
            })
        })
}

/// Whether the cluster's copies sit one inside each edited method.
fn one_copy_per_method(cluster: &Value) -> bool {
    let mut spans = occurrence_line_spans(cluster);
    spans.sort_unstable();
    spans.len() == COPIES
        && spans
            .iter()
            .zip(METHOD_WINDOWS)
            .all(|((start, end), (first, last))| first <= *start && *end <= last)
}

/// The three edited methods are one clone, and it holds every copied
/// setup: one per method.
fn broad_method_clone(report: &Value) -> Result<&Value> {
    let broad: Vec<_> = clusters(report)
        .iter()
        .filter(|cluster| {
            let mut spans = occurrence_line_spans(cluster);
            spans.sort_unstable();
            spans == METHOD_WINDOWS
        })
        .collect();
    assert_eq!(broad.len(), 1, "the three edited methods are one clone");
    let cluster = broad
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("three edited methods have one cluster: {report:#}"))?;
    assert_eq!(field(cluster, "kind").as_str(), Some(LOOSELY_SIMILAR));
    assert_eq!(field(cluster, "rank").as_u64(), Some(BROAD_RANK));
    assert!(
        includes_all_fault_assertions(cluster),
        "each method copy holds its copied setup and fault assertions: {cluster:#}"
    );
    Ok(cluster)
}

/// [PIPELINE-CLUSTER-SUBSUME] The exact setup copied once into each
/// method is the method clone read narrower, so it is absorbed.
fn assert_setup_absorbed(report: &Value, broad: &Value) {
    for cluster in clusters(report) {
        if field(cluster, "id") == field(broad, "id") {
            continue;
        }
        assert!(
            !(field(cluster, "kind").as_str() == Some(IDENTICAL) && one_copy_per_method(cluster)),
            "the copied setup re-describes the method clone beside it: {cluster:#}"
        );
    }
}

/// [FUSED-CONTENT-GATE-AUTHORED-RUN] A clone occurrence cannot join two authored tests.
fn assert_clones_stay_within_one_method(report: &Value) {
    for cluster in clusters(report) {
        if field(cluster, "kind").as_str() == Some(SHAPE_ONLY) {
            continue;
        }
        for occurrence in occurrences(cluster) {
            let (start, end) = occurrence_line_span(occurrence);
            assert!(
                METHOD_WINDOWS
                    .iter()
                    .any(|(first, last)| *first <= start && end <= *last),
                "clone occurrence crosses authored C# methods: ({start}, {end}) in {cluster:#}"
            );
        }
    }
}

#[test]
fn same_file_setup_keeps_every_fault_assertion_tail() -> Result<()> {
    let report = run_report(&fixture(FIXTURE), NODE_FLOOR)?;
    let broad = broad_method_clone(&report)?;
    assert_setup_absorbed(&report, broad);
    assert_clones_stay_within_one_method(&report);
    Ok(())
}
