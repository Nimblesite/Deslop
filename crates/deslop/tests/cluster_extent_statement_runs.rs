//! [PIPELINE-CLUSTER-EXACT-SCOPE] [FUSED-CONTENT-GATE-CALL-TARGET] A run of
//! browser-test statements is not a duplicate of a different run of browser-test
//! statements.
//!
//! Two files of Playwright-style check helpers share a grammar and nothing else.
//! Every statement awaits a call, most select through a `page` receiver, and each
//! ends in an assertion — so normalisation leaves runs that agree on shape while
//! naming entirely different selectors, roles, counts and messages. Nothing here
//! was copied: no two helpers check the same thing.
//!
//! Admitting such runs published a component whose members were not the same
//! region. The occurrences covered different numbers of rows, so a single
//! `canonical_node_count` priced members no member's span could hold, and
//! [RANK-MASS-SUM] multiplied that count across them. Windows opened part-way
//! along a method's signature line, so a published region began inside a
//! declaration it did not contain. Both files were welded together, and the
//! repository read as a third duplicated.
//!
//! The content gate refuses these edges because a member-call selector names
//! behaviour: `toHaveCount` against `toContainText` is a different operation, not
//! a renamed local ([FUSED-CONTENT-GATE-CALL-TARGET]). The fixture therefore
//! publishes nothing at all, at every threshold that admits its statement runs.

use std::collections::BTreeSet;

use anyhow::Result;
use serde_json::Value;

use crate::common::*;

/// Two files of unrelated browser checks, sharing grammar and no authored logic.
const STATEMENT_RUN_FIXTURE: &str = "js-cluster-extent-statement-runs";

/// Navigation helpers: drawer, sidebar groups, statistics, tabs, localised links.
const NAVIGATION: &str = "navigation_checks.js";
/// Filter helpers: graph filters, priority chips, testimonials, cookies, search.
const FILTERS: &str = "filter_checks.js";

/// Node floors spanning the range that admits these runs. The lowest lets a
/// two-statement window clear the floor, the highest only whole helper bodies —
/// so a defect at any depth of the fingerprint tree is caught by one of them.
const NODE_FLOORS: [u32; 3] = [12, 30, 45];

/// Nothing in either file is a copy of anything else, so an honest report
/// publishes no cluster.
const NO_CLUSTERS: usize = 0;

/// An honest report of two files that share no authored logic.
const NO_DUPLICATION_PERCENT: f64 = 0.0;

/// The three site/tests regions gh #520 welded into its rank-1 finding. Their
/// fixture counterparts are the two-statement heading check, the three-statement
/// sidebar check and the five-statement tab check — the 2-, 3- and 5-row members
/// that could not all hold one canonical node count. Each entry names a line
/// only that helper's body covers.
const UNRELATED_REGIONS: [(&str, u64); 3] = [(NAVIGATION, 17), (NAVIGATION, 12), (NAVIGATION, 25)];

#[test]
fn unrelated_statement_runs_are_never_published_as_one_duplication() -> Result<()> {
    let root = fixture(STATEMENT_RUN_FIXTURE);
    for floor in NODE_FLOORS {
        let report = run_report(&root, floor)?;
        assert_no_cluster_mixes_row_counts(&report, floor);
        assert_no_cluster_welds_unrelated_regions(&report, floor);
        assert_no_occurrence_opens_mid_line(&root, &report, floor)?;
        assert_node_count_fits_every_member(&report, floor);
        assert_nothing_is_published(&report, floor);
    }
    Ok(())
}

/// One duplication is one region repeated, so every occurrence covers the same
/// number of rows ([PIPELINE-CLUSTER-EXACT-SCOPE]).
fn assert_no_cluster_mixes_row_counts(report: &Value, floor: u32) {
    for cluster in clusters(report) {
        let rows: BTreeSet<u64> = cluster_line_spans(cluster)
            .into_iter()
            .map(|(start, end)| end.saturating_sub(start).saturating_add(1))
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "--min-nodes {floor}: one duplication is one region repeated, so every \
             occurrence covers the same rows; this cluster mixes {rows:?}: {cluster:#}"
        );
    }
}

/// No cluster may hold two of the unrelated helper regions, and none may reach
/// across both files, which share no authored logic.
fn assert_no_cluster_welds_unrelated_regions(report: &Value, floor: u32) {
    for cluster in clusters(report) {
        let held = UNRELATED_REGIONS
            .iter()
            .filter(|(path, line)| cluster_covers(cluster, path, *line))
            .count();
        assert!(
            held < 2,
            "--min-nodes {floor}: {held} unrelated helper regions welded into one \
             cluster: {cluster:#}"
        );
        assert_eq!(
            cluster_file_set(cluster).len(),
            1,
            "--min-nodes {floor}: {NAVIGATION} and {FILTERS} share no authored logic, \
             so no cluster may span both: {cluster:#}"
        );
    }
}

/// Whether any occurrence of `cluster` covers `line` of `path`.
fn cluster_covers(cluster: &Value, path: &str, line: u64) -> bool {
    occurrences(cluster).iter().any(|occurrence| {
        let (start, end) = occurrence_line_span(occurrence);
        occurrence_path(occurrence).is_ok_and(|found| found.ends_with(path))
            && (start..=end).contains(&line)
    })
}

/// A multi-line region that opens part-way along its first line begins inside a
/// construct it does not contain — a method signature, a parameter list, a brace.
fn assert_no_occurrence_opens_mid_line(
    root: &std::path::Path,
    report: &Value,
    floor: u32,
) -> Result<()> {
    for cluster in clusters(report) {
        for occurrence in occurrences(cluster) {
            let (start, end) = occurrence_line_span(occurrence);
            if start == end {
                continue;
            }
            let source = std::fs::read(root.join(occurrence_path(occurrence)?))?;
            let opening = occurrence_byte(occurrence, "start_byte")?;
            let prefix = opening_line_prefix(&source, opening);
            assert!(
                prefix.trim().is_empty(),
                "--min-nodes {floor}: a reported region opens inside the construct \
                 {prefix:?} that it does not contain: {occurrence:#}"
            );
        }
    }
    Ok(())
}

/// The bytes between the start of an occurrence's opening line and the
/// occurrence itself. Anything but whitespace there means the region begins
/// inside a construct it does not contain.
fn opening_line_prefix(source: &[u8], opening: usize) -> String {
    let before = source.get(..opening).unwrap_or_default();
    let line_start = before
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index.saturating_add(1));
    String::from_utf8_lossy(before.get(line_start..).unwrap_or_default()).into_owned()
}

/// `canonical_node_count` prices every member through [RANK-MASS-SUM], so no
/// cluster may claim an element count no member's span can hold.
fn assert_node_count_fits_every_member(report: &Value, floor: u32) {
    for cluster in clusters(report) {
        let nodes = field(cluster, "canonical_node_count")
            .as_u64()
            .unwrap_or_default();
        for (start, end) in cluster_line_spans(cluster) {
            let rows = end.saturating_sub(start).saturating_add(1);
            assert!(
                nodes <= rows.saturating_mul(MAX_NODES_PER_ROW),
                "--min-nodes {floor}: cluster claims {nodes} elements, which the \
                 {rows}-row member at {start}:{end} cannot hold: {cluster:#}"
            );
        }
    }
}

/// The widest element count one row of this grammar can carry. A row holding a
/// chained await, its selector string and its assertion argument stays far below
/// it; a member priced by another member's span does not.
const MAX_NODES_PER_ROW: u64 = 24;

/// Two files that share no authored logic duplicate nothing, so the repository
/// figure is exactly zero and no file is named as duplicated.
fn assert_nothing_is_published(report: &Value, floor: u32) {
    assert_eq!(
        cluster_count(report),
        NO_CLUSTERS,
        "--min-nodes {floor}: unrelated browser checks publish nothing: {report:#}"
    );
    let percent = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(f64::NAN);
    assert!(
        approx(percent, NO_DUPLICATION_PERCENT),
        "--min-nodes {floor}: repository duplication is {percent}, not \
         {NO_DUPLICATION_PERCENT}: {report:#}"
    );
    assert_eq!(
        metric_field(report, "duplicated_files").as_u64(),
        Some(0),
        "--min-nodes {floor}: no file carries duplication: {report:#}"
    );
}
