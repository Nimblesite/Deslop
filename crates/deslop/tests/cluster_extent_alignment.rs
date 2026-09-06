//! [PIPELINE-CLUSTER-EXACT-SCOPE] One cluster is one duplication, so
//! every occurrence it publishes must describe the same authored view.
//!
//! `collapse_overlapping_per_file` reduces each file's members to one
//! representative *independently of every other file*, so each file
//! elects whichever of its own overlapping windows is locally widest.
//! Nothing then requires the winners to be the same view. The renamed
//! pair in `python-cluster-extent-alignment` publishes the function
//! **body** in one file against the **whole function** in the other —
//! two different subtrees, one `canonical_node_count`, one mass.
//!
//! That is not a cosmetic range wobble. `mass = canonical nodes x
//! additional visible occurrences` ([RANK-MASS-SUM]) prices the cluster
//! with a node count that describes only one of the two occurrences,
//! and `duplicated_loc` / `duplication_percent` project the mismatched
//! line sets ([METRICS-REPO]). A reader is told two regions are copies
//! of one another when one carries a signature line the other does not.
//!
//! The fixture is a line-for-line consistent identifier rename with one
//! literal preserved, so the pair carries a rename anchor and must be
//! admitted. Both files are the same shape at the same depth: any
//! honest view of them covers the same authored declaration in both.

use anyhow::Result;
use serde_json::Value;

use crate::common::go_scope::*;
use crate::common::*;

/// Fixture whose two modules are a line-for-line identifier rename.
const EXTENT_FIXTURE: &str = "python-cluster-extent-alignment";

/// The renamed pair, which any honest report must span together.
const BILLING: &str = "billing_totals.py";
/// The other half of the renamed pair.
const INVOICE: &str = "invoice_totals.py";

/// `--min-nodes` low enough that both the body and the whole function
/// clear the floor, which is what puts two competing views in each file.
const EXTENT_MIN_NODES: u32 = 8;

/// The Python declaration keyword. An occurrence either opens the
/// authored declaration or sits inside it; the cluster may not mix the
/// two.
const DECLARATION_KEYWORD: &str = "def ";

/// Both files carry the same authored declaration, so an aligned report
/// covers the same number of lines in each.
const EXPECTED_OCCURRENCES: usize = 2;

/// Returns the 1-indexed line span each occurrence covers, in report
/// order, so a failure names the physical rows rather than byte offsets.
fn occurrence_line_spans(cluster: &Value) -> Vec<(u64, u64)> {
    cluster_line_spans(cluster)
}

/// Returns how many lines each occurrence covers, inclusive of both
/// endpoints.
fn occurrence_line_counts(cluster: &Value) -> Vec<u64> {
    occurrence_line_spans(cluster)
        .into_iter()
        .map(|(start, end)| end.saturating_sub(start).saturating_add(1))
        .collect()
}

/// [RANK-MASS-SUM] A canonical node count that describes only one
/// occurrence prices the whole cluster wrong: mass is canonical nodes x
/// additional visible occurrences.
fn assert_mass_prices_visible_occurrences(cluster: &Value, visible: usize) {
    let nodes = field(cluster, "canonical_node_count")
        .as_u64()
        .unwrap_or_default();
    let visible = u64::try_from(visible).unwrap_or_default();
    assert_eq!(
        field(cluster, "mass").as_u64().unwrap_or_default(),
        nodes.saturating_mul(visible.saturating_sub(1)),
        "[RANK-MASS-SUM] mass is canonical nodes x additional visible \
         occurrences: {cluster:#}"
    );
}

#[test]
fn a_renamed_pair_is_reported_at_the_same_authored_view_in_both_files() -> Result<()> {
    let scan_root = fixture(EXTENT_FIXTURE);
    let report = run_report(&scan_root, EXTENT_MIN_NODES)?;
    let cluster = expect_cluster_spanning(&report, &[BILLING, INVOICE])?;

    let texts = occurrence_texts(&scan_root, cluster)?;
    assert_eq!(
        texts.len(),
        EXPECTED_OCCURRENCES,
        "the renamed pair is one clone in each file: {cluster:#}"
    );

    // The defect in one assertion: one occurrence opens the declaration
    // and the other starts inside the body, so the cluster describes two
    // different subtrees under one canonical extent.
    let opens_declaration = texts
        .iter()
        .filter(|text| text.trim_start().starts_with(DECLARATION_KEYWORD))
        .count();
    assert!(
        opens_declaration == 0 || opens_declaration == texts.len(),
        "[PIPELINE-CLUSTER-EXACT-SCOPE] every occurrence must describe the \
         same authored view: {opens_declaration} of {} open the \
         declaration, so the cluster mixes a body window with a whole \
         function: {texts:#?}",
        texts.len()
    );

    // The same defect measured on the rendered rows, because the metrics
    // project line sets rather than bytes ([METRICS-REPO]).
    let line_counts = occurrence_line_counts(cluster);
    let widest = line_counts.iter().copied().max().unwrap_or_default();
    let narrowest = line_counts.iter().copied().min().unwrap_or_default();
    assert_eq!(
        widest,
        narrowest,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] a line-for-line rename must cover \
         the same row count in both files; spans={:?} texts={texts:#?}",
        occurrence_line_spans(cluster)
    );

    assert_mass_prices_visible_occurrences(cluster, texts.len());
    Ok(())
}

/// Go fixture whose two files share exactly one authored run of two
/// tree-writing functions. Everything else in each file is unique to it:
/// `json_report.go` carries a package prologue, three imports its
/// counterpart does not have, and a lone type declaration above the
/// pair; `text_report.go` carries an unrelated function above it.
const GO_EXTENT_FIXTURE: &str = "go-cluster-extent-alignment";

/// The file whose prologue, imports and type declaration have no
/// counterpart in the other occurrence.
const JSON_REPORT: &str = "json_report.go";
/// The file that carries an extra unrelated function before the pair.
const TEXT_REPORT: &str = "text_report.go";

/// `--min-nodes` far above the pair's floor. The defect reproduces at 8,
/// 12, 20 and 30 alike, so the assertion does not hinge on the threshold.
const GO_EXTENT_MIN_NODES: u32 = 12;

/// Every `--min-nodes` the authored window must hold at.
const GO_EXTENT_THRESHOLDS: [u32; 4] = [8, GO_EXTENT_MIN_NODES, 20, 30];

/// The type declared in `json_report.go` alone. Nothing in
/// `text_report.go` names it, so no occurrence of a shared cluster may.
const JSON_ONLY_TYPE: &str = "settingSpec";

/// A declaration's rows: first and last, both inclusive.
type RowWindow = (u64, u64);

/// `GenJSONTree`, the plain writer, and its counterpart `GenTextTree`.
const JSON_PLAIN_WRITER: RowWindow = (23, 27);
/// The plain writer's counterpart in `text_report.go`.
const TEXT_PLAIN_WRITER: RowWindow = (38, 42);
/// `GenJSONTreeCustom`, the custom writer.
const JSON_CUSTOM_WRITER: RowWindow = (30, 55);
/// The custom writer's counterpart, `GenTextTreeCustom`.
const TEXT_CUSTOM_WRITER: RowWindow = (46, 71);

/// The shared declarations of each file in authored order. An index into
/// one array names the counterpart at the same index in the other.
const JSON_SHARED: [RowWindow; 2] = [JSON_PLAIN_WRITER, JSON_CUSTOM_WRITER];
/// The counterparts of [`JSON_SHARED`], in the same order.
const TEXT_SHARED: [RowWindow; 2] = [TEXT_PLAIN_WRITER, TEXT_CUSTOM_WRITER];

/// The run of both shared writers: the widest honest window in each
/// file. Two adjacent duplicated siblings may be published as one run,
/// and the text run carries one comment row more than the json run.
const JSON_SHARED_RUN: RowWindow = (JSON_PLAIN_WRITER.0, JSON_CUSTOM_WRITER.1);
/// The text counterpart of [`JSON_SHARED_RUN`].
const TEXT_SHARED_RUN: RowWindow = (TEXT_PLAIN_WRITER.0, TEXT_CUSTOM_WRITER.1);

/// `writeSettings` repeats one block twice inside `text_report.go`. Those
/// rows are duplicated within that file and nowhere else.
const TEXT_SETTINGS_PAIR: [RowWindow; 2] = [(19, 25), (27, 33)];

/// Rows a window covers, both endpoints included.
const fn window_rows(window: RowWindow) -> u64 {
    window.1.saturating_sub(window.0).saturating_add(1)
}

/// [METRICS-REPO] The most rows an honest report of the fixture can count
/// as duplicated: both shared runs plus the settings pair. Every row
/// beyond it is one nobody copied.
const HONEST_DUPLICATED_ROW_CEILING: u64 = window_rows(JSON_SHARED_RUN)
    .saturating_add(window_rows(TEXT_SHARED_RUN))
    .saturating_add(window_rows(TEXT_SETTINGS_PAIR[0]))
    .saturating_add(window_rows(TEXT_SETTINGS_PAIR[1]));

/// True when `row` lies inside `window`.
fn within(window: RowWindow, row: u64) -> bool {
    (window.0..=window.1).contains(&row)
}

/// Which shared declaration holds `row`, or `None` when the row sits
/// outside every shared declaration: in the prologue, the type, the
/// unrelated function, or the comments between the two writers.
fn shared_declaration_holding(shared: &[RowWindow; 2], row: u64) -> Option<usize> {
    shared.iter().position(|window| within(*window, row))
}

/// The occurrences of `cluster` that point into `file`.
fn occurrences_in<'a>(cluster: &'a Value, file: &str) -> Vec<&'a Value> {
    occurrences(cluster)
        .iter()
        .filter(|occurrence| occurrence_path(occurrence).is_ok_and(|path| path.ends_with(file)))
        .collect()
}

/// For each occurrence in `file`, the shared declaration its first and
/// last row fall in, sorted so the two files compare as sets.
fn declaration_footprint(
    cluster: &Value,
    file: &str,
    shared: &[RowWindow; 2],
) -> Vec<(Option<usize>, Option<usize>)> {
    let mut footprint: Vec<_> = occurrences_in(cluster, file)
        .into_iter()
        .map(|occurrence| {
            let (start, end) = occurrence_line_span(occurrence);
            (
                shared_declaration_holding(shared, start),
                shared_declaration_holding(shared, end),
            )
        })
        .collect();
    footprint.sort_unstable();
    footprint
}

/// [PIPELINE-CLUSTER-EXACT-SCOPE] A cluster spanning both files describes
/// the same shared writers in each: every occurrence starts and ends
/// inside a shared writer, and the json footprint is the text footprint.
/// A pair held inside one writer is line-for-line, so it also covers the
/// same row count on both sides.
fn assert_shared_writers_correspond(cluster: &Value) {
    let json = declaration_footprint(cluster, JSON_REPORT, &JSON_SHARED);
    let text = declaration_footprint(cluster, TEXT_REPORT, &TEXT_SHARED);
    let spans = go_spans(cluster);
    assert!(
        json.iter()
            .chain(&text)
            .all(|footprint| footprint.0.is_some() && footprint.1.is_some()),
        "[PIPELINE-CLUSTER-EXACT-SCOPE] an occurrence reaches outside the \
         shared writers, into the prologue, `{JSON_ONLY_TYPE}` or \
         `writeSettings`: json={json:?} text={text:?} spans={spans:?}"
    );
    assert_eq!(
        json, text,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] the two halves describe different \
         declarations: spans={spans:?}"
    );
    if json.iter().all(|(start, end)| start == end) {
        assert_symmetric_rows(cluster, GO_EXTENT_FIXTURE);
    }
}

/// The rows of `file` an honest report may count as duplicated.
fn honest_windows(file: &str) -> Result<Vec<RowWindow>> {
    if file.ends_with(JSON_REPORT) {
        return Ok(vec![JSON_SHARED_RUN]);
    }
    if file.ends_with(TEXT_REPORT) {
        return Ok(vec![
            TEXT_SHARED_RUN,
            TEXT_SETTINGS_PAIR[0],
            TEXT_SETTINGS_PAIR[1],
        ]);
    }
    Err(anyhow::anyhow!(
        "{file} is not part of the {GO_EXTENT_FIXTURE} fixture"
    ))
}

/// [METRICS-REPO] Every row the report counts as duplicated lies inside
/// an honest window of its file, so the headline figure cannot be
/// inflated by rows nobody copied.
fn assert_only_authored_rows_are_duplicated(report: &Value, label: &str) -> Result<()> {
    for (file, rows) in visible_duplicated_lines(report) {
        let windows = honest_windows(&file)?;
        let strays: Vec<u64> = rows
            .iter()
            .copied()
            .filter(|row| !windows.iter().any(|window| within(*window, *row)))
            .collect();
        assert!(
            strays.is_empty(),
            "[METRICS-REPO] {label}: {file} counts rows {strays:?} as \
             duplicated, but only rows {windows:?} were ever copied: {:?}",
            visible_cluster_lines(report)
        );
    }
    Ok(())
}

/// [METRICS-REPO] The headline `duplicated_loc` is the rows the visible
/// clusters cover, and it cannot exceed the rows the fixture duplicates.
fn assert_duplicated_loc_within_honest_ceiling(report: &Value, label: &str) {
    let reported = metric_field(report, "duplicated_loc")
        .as_u64()
        .unwrap_or_default();
    assert_eq!(
        reported,
        visible_duplicated_loc(report),
        "[METRICS-REPO] {label}: duplicated_loc must be the rows the visible \
         clusters cover: {report:#}"
    );
    assert!(
        reported <= HONEST_DUPLICATED_ROW_CEILING,
        "[METRICS-REPO] {label}: duplicated_loc {reported} exceeds the \
         {HONEST_DUPLICATED_ROW_CEILING} rows the fixture ever duplicates, so \
         the headline percentage is inflated by rows nobody copied: {:?}",
        visible_cluster_lines(report)
    );
}

/// [METRICS-REPO] `json_report.go` duplicates its two writers and nothing
/// else, so its per-file `duplicated_loc` is bounded by that run.
fn assert_json_file_metric_is_bounded(report: &Value, label: &str) -> Result<()> {
    let duplicated = per_file_metrics(report)
        .iter()
        .find(|row| {
            field(row, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(JSON_REPORT))
        })
        .map(|row| field(row, "duplicated_loc").as_u64().unwrap_or_default())
        .ok_or_else(|| anyhow::anyhow!("{label}: {JSON_REPORT} has no per-file metric row"))?;
    assert!(
        duplicated <= window_rows(JSON_SHARED_RUN),
        "[METRICS-REPO] {label}: {JSON_REPORT} reports {duplicated} duplicated \
         rows, but only its {} shared rows {JSON_SHARED_RUN:?} were copied: {:?}",
        window_rows(JSON_SHARED_RUN),
        visible_cluster_lines(report)
    );
    Ok(())
}

/// Every report-wide rule of the fixture: the authored window in every
/// cluster, and metrics that count only rows someone copied.
fn assert_go_extent_report(scan_root: &std::path::Path, report: &Value, label: &str) -> Result<()> {
    assert_go_authored_scope(scan_root, report, label)?;
    assert_only_authored_rows_are_duplicated(report, label)?;
    assert_duplicated_loc_within_honest_ceiling(report, label);
    assert_json_file_metric_is_bounded(report, label)
}

#[test]
fn a_go_pair_is_not_padded_with_the_prologue_and_types_of_one_file() -> Result<()> {
    let scan_root = fixture(GO_EXTENT_FIXTURE);
    let report = run_report(&scan_root, GO_EXTENT_MIN_NODES)?;
    let cluster = expect_cluster_spanning(&report, &[JSON_REPORT, TEXT_REPORT])?;
    assert_eq!(
        occurrences(cluster).len(),
        EXPECTED_OCCURRENCES,
        "the shared pair is one clone in each file: {cluster:#}"
    );

    // The defect: one occurrence starts at the authored declaration, the
    // other reaches back to the top of its file and collects the package
    // clause, the import block and a type its counterpart never had.
    assert_cluster_scope(&scan_root, cluster, GO_EXTENT_FIXTURE)?;
    assert_no_unshared_symbol(&scan_root, cluster, JSON_ONLY_TYPE, GO_EXTENT_FIXTURE)?;
    assert_shared_writers_correspond(cluster);
    assert_mass_prices_visible_occurrences(cluster, EXPECTED_OCCURRENCES);
    Ok(())
}

#[test]
fn every_go_cluster_touching_json_report_describes_the_same_shared_writers() -> Result<()> {
    let scan_root = fixture(GO_EXTENT_FIXTURE);
    let report = run_report(&scan_root, GO_EXTENT_MIN_NODES)?;
    assert_go_authored_scope(&scan_root, &report, GO_EXTENT_FIXTURE)?;

    let touching: Vec<&Value> = clusters(&report)
        .iter()
        .filter(|cluster| !occurrences_in(cluster, JSON_REPORT).is_empty())
        .collect();
    assert!(
        !touching.is_empty(),
        "the shared writers must surface at --min-nodes {GO_EXTENT_MIN_NODES}: {report:#}"
    );
    for cluster in touching {
        // `json_report.go` duplicates nothing within itself: every row it
        // shares is mirrored in `text_report.go`, so a cluster touching it
        // reaches the other file and describes the same writers there.
        assert!(
            !occurrences_in(cluster, TEXT_REPORT).is_empty(),
            "[PIPELINE-CLUSTER-EXACT-SCOPE] {JSON_REPORT} has no duplication \
             of its own, yet a cluster stays inside it: {:?}",
            go_spans(cluster)
        );
        assert_shared_writers_correspond(cluster);
    }
    Ok(())
}

#[test]
fn the_go_authored_window_and_its_metrics_hold_at_every_threshold() -> Result<()> {
    let scan_root = fixture(GO_EXTENT_FIXTURE);
    for min_nodes in GO_EXTENT_THRESHOLDS {
        let label = format!("{GO_EXTENT_FIXTURE} --min-nodes {min_nodes}");
        let report = run_report(&scan_root, min_nodes)?;
        assert_go_extent_report(&scan_root, &report, &label)?;
    }
    Ok(())
}
