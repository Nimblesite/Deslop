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
    for cluster in &clone_findings(report) {
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
    for cluster in &clone_findings(report) {
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
    for cluster in &clone_findings(report) {
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
        clone_findings(report).len(),
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

/// A scenario copied whole with one URL changed, beside a scenario that
/// shares only the `locator().boundingBox()` idiom with it.
const SCENARIO_TAIL_FIXTURE: &str = "js-cluster-extent-scenario-tail";

/// One scenario copied once is one duplication.
const ONE_CLUSTER: usize = 1;

/// Only the file holding the copy carries duplication.
const ONE_DUPLICATED_FILE: u64 = 1;

/// One file's copy, expected to publish whole and alone while the file
/// beside it shares an idiom with the copy and no authored logic.
struct CopiedRun {
    /// The file holding both occurrences of the copy.
    copied: &'static str,
    /// The file that must never join the copy's cluster.
    stranger: &'static str,
    /// The copy as authored, both occurrences, and nothing narrower
    /// ([PIPELINE-CLUSTER-EXACT-SCOPE]).
    spans: [(u64, u64); 2],
    /// What welding the stranger on would be.
    weld: &'static str,
}

/// Two test bodies that are one copy of each other, differing in the page
/// they open, beside one test body sharing an idiom with them.
const COPIED_SCENARIO: CopiedRun = CopiedRun {
    copied: "layout.spec.js",
    stranger: "publication.spec.js",
    spans: [(3, 9), (11, 17)],
    weld: "welding its tail on as a third occurrence is gh #520",
};

/// gh #520, the shape that survived its headline fix: a proven copy plus a
/// shape-compatible stranger ([CLONE-NOISE-VERBATIM-SUBGROUP]). Two
/// `locator().boundingBox()` lines on other selectors are literal-variation
/// scaffolding ([CLONE-NOISE-LITERAL-VARIATION-CALLS]), not a third occurrence
/// of the copy — and the stranger arrived one statement wider than the members
/// it was welded to, priced at a node count its own span was never measured
/// for. It rode the shared-subtree rescue on a whole-endpoint content floor
/// while the two lines it actually shares had been refused at the gate; a
/// rescued pair is judged on that shared core ([FUSED-SHARED-SUBTREE-CORE]).
#[test]
fn a_scenario_tail_is_never_welded_onto_a_copied_scenario() -> Result<()> {
    let root = fixture(SCENARIO_TAIL_FIXTURE);
    for floor in NODE_FLOORS {
        let report = run_report(&root, floor)?;
        assert_no_cluster_mixes_row_counts(&report, floor);
        assert_no_occurrence_opens_mid_line(&root, &report, floor)?;
        assert_node_count_fits_every_member(&report, floor);
        assert_only_the_copy_is_published(&report, floor, &COPIED_SCENARIO);
    }
    Ok(())
}

/// Two tests opening with one copied three-statement preamble, beside a
/// test that shares the two Playwright lines every test starts with — a
/// viewport and a `goto` — and nothing else.
const SHARED_PREAMBLE_FIXTURE: &str = "js-cluster-extent-shared-preamble";

/// The copied preamble is three statements of 31 nodes and the stranger's
/// own run is 33, so this floor admits both as windows; the floor above
/// admits neither.
const PREAMBLE_FLOOR: u32 = 30;
/// A floor no window of the fixture reaches.
const ABOVE_PREAMBLE_FLOOR: u32 = 45;

/// The copied preamble, both occurrences, beside the docs test that shares
/// its first two lines.
const COPIED_PREAMBLE: CopiedRun = CopiedRun {
    copied: "publication.spec.js",
    stranger: "docs.spec.js",
    spans: [(10, 12), (19, 21)],
    weld: "rescuing it onto the copy over the two lines every test starts with is gh #520",
};

// gh #520's last survivor on `site/tests`: three tests open with the same
// two Playwright lines, and two of them go on to copy a third statement.
// The third test shares that preamble and nothing else, yet it rode the
// shared-subtree rescue onto the copy: its aligned core was the two
// preamble lines and one identifier — twenty nodes, measuring exactly the
// content floor. What two endpoints share must itself be a clone the scan
// would report, so a core below the node floor every rescued endpoint
// clears admits nothing ([FUSED-SHARED-SUBTREE-CORE]).
#[test]
fn a_shared_preamble_never_rescues_a_stranger_onto_a_copied_run() -> Result<()> {
    let root = fixture(SHARED_PREAMBLE_FIXTURE);
    let report = run_report(&root, PREAMBLE_FLOOR)?;
    assert_no_cluster_mixes_row_counts(&report, PREAMBLE_FLOOR);
    assert_no_occurrence_opens_mid_line(&root, &report, PREAMBLE_FLOOR)?;
    assert_node_count_fits_every_member(&report, PREAMBLE_FLOOR);
    assert_only_the_copy_is_published(&report, PREAMBLE_FLOOR, &COPIED_PREAMBLE);
    let above = run_report(&root, ABOVE_PREAMBLE_FLOOR)?;
    assert_nothing_is_published(&above, ABOVE_PREAMBLE_FLOOR);
    Ok(())
}

/// The copy is the whole finding, and the only one: one cluster, in the one
/// file that holds it.
fn assert_only_the_copy_is_published(report: &Value, floor: u32, run: &CopiedRun) {
    assert_eq!(
        clone_findings(report).len(),
        ONE_CLUSTER,
        "--min-nodes {floor}: one run copied once is one duplication: {report:#}"
    );
    for cluster in &clone_findings(report) {
        assert_copy_is_the_whole_finding(cluster, floor, run);
    }
    assert_eq!(
        metric_field(report, "duplicated_files").as_u64(),
        Some(ONE_DUPLICATED_FILE),
        "--min-nodes {floor}: only {} carries duplication: {report:#}",
        run.copied
    );
}

/// Both occurrences of the copied file at their authored extent, and no
/// occurrence from the file that merely shares their idiom.
fn assert_copy_is_the_whole_finding(cluster: &Value, floor: u32, run: &CopiedRun) {
    let files = cluster_file_set(cluster);
    assert!(
        !files.iter().any(|path| path.ends_with(run.stranger)),
        "--min-nodes {floor}: {} shares an idiom, not authored logic, with {}; {}: {cluster:#}",
        run.stranger,
        run.copied,
        run.weld
    );
    let mut spans = cluster_line_spans(cluster);
    spans.sort_unstable();
    assert_eq!(
        spans, run.spans,
        "--min-nodes {floor}: the copy is both occurrences in {}, whole: {cluster:#}",
        run.copied
    );
}
