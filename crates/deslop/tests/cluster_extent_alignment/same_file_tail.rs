//! [PIPELINE-CLUSTER-EXACT-SCOPE] Three C# transitions share one executable setup and assertion.

use super::*;

const FIXTURE: &str = "csharp-same-file-copy-tail";
const FILE: &str = "CircuitChecks.cs";
const NODE_FLOOR: u32 = 30;
const IDENTICAL: &str = "identical";
const LOOSELY_SIMILAR: &str = "loosely_similar";
const COPIES: usize = 3;
const BROAD_RANK: u64 = 1;
const EXPECTED_RANK: u64 = 5;
const COPIED_WINDOWS: [(u64, u64); COPIES] = [(8, 26), (41, 59), (80, 98)];
const DISTINCT_SUFFIX_LINES: [u64; 2] = [68, 106];
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

fn assert_exact_copied_extent(cluster: &Value) {
    let mut spans = occurrence_line_spans(cluster);
    spans.sort_unstable();
    assert_eq!(spans, COPIED_WINDOWS);
    for suffix in DISTINCT_SUFFIX_LINES {
        assert!(spans
            .iter()
            .all(|(start, end)| !(*start..=*end).contains(&suffix)));
    }
}

fn copied_setup_cluster(report: &Value) -> Result<&Value> {
    let full = clusters(report)
        .iter()
        .filter(|cluster| {
            field(cluster, "kind").as_str() == Some(IDENTICAL)
                && includes_all_fault_assertions(cluster)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        full.len(),
        1,
        "three copied setups and fault assertions stay one clone: {report:#}"
    );
    full.first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("missing copied C# cluster"))
}

fn assert_complete_setup(scan_root: &std::path::Path, cluster: &Value) -> Result<()> {
    assert_eq!(occurrences(cluster).len(), COPIES);
    assert_eq!(field(cluster, "kind").as_str(), Some(IDENTICAL));
    assert_eq!(field(cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    assert_exact_copied_extent(cluster);
    assert!(crate::common::signals::has_verbatim_pair(
        scan_root, cluster
    )?);
    Ok(())
}

/// [PIPELINE-CLUSTER-SUBSUME-KIND] The edited methods and their exact
/// copied setup are both real, distinct findings.
fn assert_broad_method_clone(report: &Value) -> Result<()> {
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
        .ok_or_else(|| anyhow::anyhow!("three edited methods have one cluster: {report:#}"))?;
    assert_eq!(field(cluster, "kind").as_str(), Some(LOOSELY_SIMILAR));
    assert_eq!(field(cluster, "rank").as_u64(), Some(BROAD_RANK));
    Ok(())
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
    let scan_root = fixture(FIXTURE);
    let report = run_report(&scan_root, NODE_FLOOR)?;
    assert_complete_setup(&scan_root, copied_setup_cluster(&report)?)?;
    assert_broad_method_clone(&report)?;
    assert_clones_stay_within_one_method(&report);
    Ok(())
}
