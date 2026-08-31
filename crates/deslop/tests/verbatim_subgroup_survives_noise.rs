//! [CLONE-NOISE-VERBATIM-SUBGROUP] — a noise filter must never erase a
//! byte-identical subgroup because an unrelated member joined its
//! cluster.
//!
//! Every suppression in [`cluster_filters`] guards itself with a
//! verbatim escape hatch, and every one of them documents the same
//! intent: *"a byte-identical repeated entry is a real copy and still
//! surfaces"*, *"a verbatim copy survives"*. The shipped predicate said
//! something strictly weaker — **at least two members differ** — which
//! is satisfied the moment one unrelated member joins. A cluster of a
//! proven copy `A`/`A` plus a shape-compatible stranger `C` therefore
//! took the suppression whole, and the copy the tool exists to find
//! disappeared from the report.
//!
//! This is the worst defect class the accuracy contract names: a
//! duplicate that is never reported is never found. It is also
//! *reachable by accident* — nothing about `C` is unusual; it only has
//! to normalise to the same shape, which is exactly what the noise
//! families are made of.
//!
//! Three families are pinned here because the three erase by three
//! different routes:
//!
//! - [CLONE-NOISE-CONSTANT-TABLE] — two files of unrelated constants;
//! - [CLONE-NOISE-LITERAL-VARIATION-CALLS] — a call run whose literals
//!   vary;
//! - [CLONE-NOISE-PY-COLLECTION-SIBLING-CELLS] — sibling cells of one
//!   collection literal, where the stranger sits *in the same file* as
//!   the copy, so the erasure cannot even be blamed on file geometry.
//!
//! # Why this fixture cannot pass by going blind
//!
//! The copy is the assertion. Each case demands the exact pair — its
//! two paths, its two line ranges, its `identical` bucket, saturated
//! signals, and its own duplicated lines in the metric. A detector that
//! stopped finding anything fails every one of them, and a fix that
//! merely stops suppressing fails the other half: the stranger must not
//! be an occurrence of the copy's cluster, and must not contribute a
//! duplicated line of its own.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::common::{signals::*, verbatim_subgroup::*, *};

/// The node floor the collection-cell family needs: one cell of a list
/// literal is a smaller window than a four-statement run.
const CELL_MIN_NODES: u32 = 4;

/// The two byte-identical constant tables, and the unrelated table that
/// shares their normalised shape.
const CONST_COPY: [&str; 2] = ["retry_defaults.py", "retry_defaults_copy.py"];

/// Anchor positions of the constant-table pair's elected measurement: the
/// four assignments hold four literals and four identifiers, so the rename
/// proof scales by `8 / (8 + 4) = 0.6667` — below the ten-anchor
/// certification point, which is exactly what the assertion pins
/// ([FUSED-CONTENT-GATE]).
const CONST_TABLE_ANCHORS: u32 = 8;
/// The stranger whose only relation to [`CONST_COPY`] is its shape.
const CONST_STRANGER: &str = "theme_tokens.py";

/// The corpus holding one list literal whose first two cells are
/// byte-identical and whose third is not. It nests one level down so
/// the control-clone pin can scan the directory above it and read the
/// very same file — a second copy of these bytes is what turned two
/// pins over one literal into two pins asserting opposite things
/// (gh #462).
const CELL_CASE: &str = "collection-cells/cells";
/// The single file holding that literal.
const CELL_FILE: &str = "ledger_rows.py";
/// The 1-based lines of the two identical cells, in file order.
const CELL_COPY_LINES: [(u64, u64); 2] = [(2, 2), (3, 3)];
/// The 1-based line of the cell that differs.
const CELL_STRANGER_LINE: u64 = 4;

/// Lines each copy of the constant table covers.
const CONST_LOC_PER_FILE: u64 = 4;
/// `(start_line, end_line)` for every occurrence of `cluster`, sorted.
fn occurrence_ranges(cluster: &Value) -> Vec<(u64, u64)> {
    let mut ranges: Vec<(u64, u64)> = occurrences(cluster)
        .iter()
        .map(|occurrence| {
            (
                field(occurrence, "start_line").as_u64().unwrap_or(0),
                field(occurrence, "end_line").as_u64().unwrap_or(0),
            )
        })
        .collect();
    ranges.sort_unstable();
    ranges
}

// [CLONE-NOISE-CONSTANT-TABLE] Two byte-identical constant tables stay
// reported when an unrelated third table joins their cluster, and the
// third table still earns nothing.
#[test]
fn a_copied_constant_table_survives_an_unrelated_table_in_its_cluster() -> Result<()> {
    let report = render("constant-table", MIN_NODES)?;
    assert_copy_survives_alone(
        &report,
        "constant table",
        &CONST_COPY,
        CONST_STRANGER,
        rename_consistency_for(CONST_TABLE_ANCHORS),
    )?;
    for file in CONST_COPY {
        assert_eq!(
            duplicated_loc_for(&report, file),
            CONST_LOC_PER_FILE,
            "{file}: every line of the copy is duplicated and must keep \
             counting — erasing the cluster also zeroed the metric that \
             feeds the CI duplication gate: {lines:#?}",
            lines = visible_cluster_lines(&report),
        );
    }
    assert_eq!(
        duplicated_loc_for(&report, CONST_STRANGER),
        0,
        "{CONST_STRANGER} holds no duplicated line — surfacing the copy must \
         not smuggle the stranger into the metric: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    assert_eq!(
        visible_duplicated_loc(&report),
        CONST_LOC_PER_FILE.saturating_mul(2),
        "the corpus contains exactly one duplication, of \
         {CONST_LOC_PER_FILE} lines, in two files: {published:#?}",
        published = published(&report),
    );
    Ok(())
}

// [CLONE-NOISE-LITERAL-VARIATION-CALLS] The literal-variation filter is
// correct about the stranger and wrong about the copy; only the copy is
// reported.
#[test]
fn a_copied_call_run_survives_a_literal_varying_run_in_its_cluster() -> Result<()> {
    let report = render(CALL_CASE, MIN_NODES)?;
    assert_copy_survives_alone(
        &report,
        "literal calls",
        &CALL_COPY,
        CALL_STRANGER,
        rename_consistency_for(CALL_PAIR_ANCHORS),
    )?;
    for file in CALL_COPY {
        assert_eq!(
            duplicated_loc_for(&report, file),
            CALL_LOC_PER_FILE,
            "{file}: the copied call run's own lines must keep counting: \
             {lines:#?}",
            lines = visible_cluster_lines(&report),
        );
    }
    assert_eq!(
        duplicated_loc_for(&report, CALL_STRANGER),
        0,
        "{CALL_STRANGER} varies its literals — it is the family the filter \
         exists to suppress and must contribute nothing: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}

// [CLONE-NOISE-PY-COLLECTION-SIBLING-CELLS] The stranger sits in the
// same collection literal as the copy, so the erasure cannot be blamed
// on file geometry: the two identical cells must still be reported, by
// their exact lines, and the differing cell must not join them.
#[test]
fn two_identical_collection_cells_survive_a_differing_sibling_cell() -> Result<()> {
    let report = render(CELL_CASE, CELL_MIN_NODES)?;
    let cluster = expect_cluster_spanning(&report, &[CELL_FILE])?;
    let dump = signal_dump(cluster);
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "the first two cells are byte-identical — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "exactly the two identical cells are shown — {dump}"
    );
    assert_eq!(
        occurrence_ranges(cluster),
        CELL_COPY_LINES.to_vec(),
        "the copy is the two cells on lines {CELL_COPY_LINES:?}; a cluster \
         that reports different lines has found something else: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    assert!(
        !occurrence_ranges(cluster)
            .iter()
            .any(|(start, _)| *start == CELL_STRANGER_LINE),
        "line {CELL_STRANGER_LINE} is a different record's cell — it is the \
         family the filter exists to suppress and must not be an occurrence \
         of the copy: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    assert_eq!(
        duplicated_line_numbers(&report),
        start_lines(&CELL_COPY_LINES),
        "exactly the two copied cell lines are duplicated: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}

/// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL] The same
/// literal with its differing cell removed, so every cell is the copy.
/// No member differs, the sibling-cell filter never fires, and the copy
/// is reported — the baseline the contested corpus must not fall below.
const MONOTONIC_CASE: &str = "collection-cells-monotonic";
/// The 1-based lines of the three identical cells, in file order.
const MONOTONIC_COPY_LINES: [(u64, u64); 3] = [(2, 2), (3, 3), (4, 4)];
/// Nothing is suppressed in either collection-cell corpus.
const NOTHING_HIDDEN: u64 = 0;

/// The corpus holding the copied cells *and* a cross-file control
/// clone. It is the `cells/` directory the survival pin scans, read one
/// level up — one `ledger_rows.py` on disk, never a second copy of it.
const CONTROL_CASE: &str = "collection-cells";
/// The cross-file byte-identical copy staged beside the cells.
const CONTROL_COPY: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];
/// Lines of `settle_ledger` in each of the two control files.
const CONTROL_LOC_PER_FILE: u64 = 8;
/// Duplicated lines the copied cells contribute: one line each.
const CELL_LOC: u64 = 2;
/// The cell file and both control files.
const CONTROL_FILES_ANALYSED: u64 = 3;
/// The control clone and the copied cells — and nothing else.
const CONTROL_CLUSTERS: usize = 2;
const CONTROL_LABEL: &str =
    "[CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL] cells beside the control";

/// Every 1-based line the visible clusters mark duplicated, in every
/// file, as one set.
fn duplicated_line_numbers(report: &Value) -> BTreeSet<u64> {
    visible_duplicated_lines(report)
        .values()
        .flatten()
        .copied()
        .collect()
}

/// The 1-based start line of each range in `ranges`.
fn start_lines(ranges: &[(u64, u64)]) -> BTreeSet<u64> {
    ranges.iter().map(|(start, _)| *start).collect()
}

// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL] The sharpest
// statement of gh #462, as an A/B over one collection literal.
//
// A: every cell holds the same bytes. No member differs, so the
//    sibling-cell filter's own verbatim hatch keeps it quiet and the
//    copy is reported — three occurrences, `identical`, nothing hidden.
// B: one *differing* cell is added. That cell is the only change, and
//    it is precisely the member the filter exists to suppress.
//
// Adding it must not delete the copy A reported. The cross-file
// arbitration did exactly that: B published nothing at all, so a corpus
// lost a finding by gaining a line that was never part of it. Detection
// has to be monotone in the noise around a copy — a report that is a
// function of a duplicate's neighbours rather than of the duplicate
// cannot be read, because nothing tells the reader which one they got.
#[test]
fn adding_a_differing_sibling_never_deletes_a_visible_copy() -> Result<()> {
    let alone = render(MONOTONIC_CASE, CELL_MIN_NODES)?;
    let uncontested = expect_cluster_spanning(&alone, &[CELL_FILE])?;
    assert_eq!(
        cluster_bucket(uncontested),
        IDENTICAL_BUCKET,
        "every cell is the same bytes — {dump}",
        dump = signal_dump(uncontested)
    );
    assert_eq!(
        occurrence_ranges(uncontested),
        MONOTONIC_COPY_LINES.to_vec(),
        "with no stranger present the copy is all three cells: {lines:#?}",
        lines = visible_cluster_lines(&alone),
    );
    assert_eq!(
        clusters_hidden(&alone),
        NOTHING_HIDDEN,
        "no member differs, so the sibling-cell filter cannot fire and \
         nothing is suppressed: {alone:#}"
    );

    let joined = render(CELL_CASE, CELL_MIN_NODES)?;
    let contested = expect_cluster_spanning(&joined, &[CELL_FILE])?;
    assert_eq!(
        cluster_bucket(contested),
        IDENTICAL_BUCKET,
        "the copy is still copied byte for byte once the stranger joins \
         — {dump}",
        dump = signal_dump(contested)
    );
    assert_eq!(
        occurrence_ranges(contested),
        CELL_COPY_LINES.to_vec(),
        "the stranger joined the literal, not the copy: {lines:#?}",
        lines = visible_cluster_lines(&joined),
    );
    assert_eq!(
        clusters_hidden(&joined),
        NOTHING_HIDDEN,
        "the filter fired on the literal and the copy still escaped it, \
         so no component is left suppressed: {joined:#}"
    );

    let before = duplicated_line_numbers(&alone);
    let after = duplicated_line_numbers(&joined);
    for line in start_lines(&CELL_COPY_LINES) {
        assert!(
            before.contains(&line),
            "line {line} carries the copy and is duplicated with no \
             stranger present: {before:?}"
        );
        assert!(
            after.contains(&line),
            "line {line} carries the same copy and must stay duplicated \
             after the stranger joined — a duplicate deleted by the \
             arrival of a line that is not part of it (gh #462): {after:?}"
        );
    }
    assert!(
        !after.contains(&CELL_STRANGER_LINE),
        "the stranger is still not a duplicate of anything: {after:?}"
    );
    Ok(())
}

// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE-SAME-LITERAL] What this pin
// used to assert, and why it now asserts the opposite.
//
// It was `an_intra_file_verbatim_pair_inside_a_suppressed_component_
// stays_hidden`, and it paid the price [CLONE-NOISE-VERBATIM-SUBGROUP-
// CROSS-FILE] named: an intra-file byte-identical family inside a
// suppressed component stays hidden. On the sibling-cell route that was
// not a price but a false negative — the same literal with its
// differing cell removed published the copy happily, so the copy was
// being deleted by the arrival of a member that was never part of it
// (gh #462, `adding_a_differing_sibling_never_deletes_a_visible_copy`).
// The price is still owed, and still paid, on the routes whose families
// really can span files: `verbatim_subgroup_idiom_price.rs`.
//
// This pin's other half survives untouched and is why it stays here. A
// pin that only counts absences passes just as well when the detector
// has gone blind, so the copied cells are asserted *beside* a cross-file
// clone that must stay visible, stay `identical`, and stay ranked first.
#[test]
fn the_copied_cells_publish_beside_the_cross_file_control() -> Result<()> {
    let report = render(CONTROL_CASE, CELL_MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(CONTROL_FILES_ANALYSED),
        "{CONTROL_LABEL}: the cells and both controls were all analysed — \
         a file the scan never opened proves nothing: {report:#}"
    );
    assert_eq!(
        clusters_hidden(&report),
        NOTHING_HIDDEN,
        "{CONTROL_LABEL}: the sibling-cell filter fired and the copy \
         escaped it, so no component is left suppressed: {report:#}"
    );
    assert_eq!(
        clusters(&report).len(),
        CONTROL_CLUSTERS,
        "{CONTROL_LABEL}: exactly the control clone and the copied cells: \
         {published:#?}",
        published = published(&report),
    );

    let control = expect_cluster_spanning(&report, &CONTROL_COPY)?;
    assert_eq!(
        cluster_bucket(control),
        IDENTICAL_BUCKET,
        "{CONTROL_LABEL}: the control is a byte-for-byte paste — {dump}",
        dump = signal_dump(control)
    );
    assert_eq!(
        clusters(&report).first().map(cluster_id),
        Some(cluster_id(control)),
        "{CONTROL_LABEL}: the control is sixteen lines of copied logic and \
         the cells are two — the control ranks first ([RANK-SCORE]): \
         {published:#?}",
        published = published(&report),
    );

    let cells = expect_cluster_spanning(&report, &[CELL_FILE])?;
    assert_eq!(
        cluster_bucket(cells),
        IDENTICAL_BUCKET,
        "{CONTROL_LABEL}: the two cells are byte-identical — {dump}",
        dump = signal_dump(cells)
    );
    assert_eq!(
        occurrence_ranges(cells),
        CELL_COPY_LINES.to_vec(),
        "{CONTROL_LABEL}: the copy is the two cells on lines \
         {CELL_COPY_LINES:?}: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );

    for file in CONTROL_COPY {
        assert_eq!(
            duplicated_loc_for(&report, file),
            CONTROL_LOC_PER_FILE,
            "{CONTROL_LABEL}: {file} keeps every one of its copied lines: \
             {lines:#?}",
            lines = visible_cluster_lines(&report),
        );
    }
    assert_eq!(
        duplicated_loc_for(&report, CELL_FILE),
        CELL_LOC,
        "{CONTROL_LABEL}: a copy the report shows also reaches the \
         duplication gate — the two cell lines count: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    assert_eq!(
        visible_duplicated_loc(&report),
        CONTROL_LOC_PER_FILE
            .saturating_mul(2)
            .saturating_add(CELL_LOC),
        "{CONTROL_LABEL}: the corpus duplicates the control twice over and \
         the cell once: {published:#?}",
        published = published(&report),
    );
    Ok(())
}
