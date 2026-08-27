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

use crate::common::{
    negative_pin::{
        assert_control_is_the_only_published_cluster, assert_family_hidden_with_control,
        assert_only_the_control_files_carry_duplicated_lines,
    },
    signals::*,
    verbatim_subgroup::*,
    *,
};

/// The node floor the collection-cell family needs: one cell of a list
/// literal is a smaller window than a four-statement run.
const CELL_MIN_NODES: u32 = 4;

/// The two byte-identical constant tables, and the unrelated table that
/// shares their normalised shape.
const CONST_COPY: [&str; 2] = ["retry_defaults.py", "retry_defaults_copy.py"];
/// The stranger whose only relation to [`CONST_COPY`] is its shape.
const CONST_STRANGER: &str = "theme_tokens.py";

/// The single file holding one list literal whose first two cells are
/// byte-identical and whose third is not.
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
    assert_copy_survives_alone(&report, "constant table", &CONST_COPY, CONST_STRANGER)?;
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
    assert_copy_survives_alone(&report, "literal calls", &CALL_COPY, CALL_STRANGER)?;
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
    let report = render("collection-cells", CELL_MIN_NODES)?;
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
        visible_duplicated_lines(&report)
            .values()
            .flatten()
            .copied()
            .collect::<BTreeSet<u64>>(),
        CELL_COPY_LINES
            .iter()
            .map(|(start, _)| *start)
            .collect::<BTreeSet<u64>>(),
        "exactly the two copied cell lines are duplicated: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}

/// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] The one file holding a
/// genuinely byte-identical pair of collection cells inside a component
/// the sibling-cell filter suppresses.
const PRICE_FILE: [&str; 1] = ["ledger_rows.py"];
/// The cross-file byte-identical copy staged in the same run. Without
/// it, "the pair stayed hidden" is a bar a blind detector clears.
const PRICE_CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];
/// Components suppressed here: the one collection-cell component.
const PRICE_HIDDEN: u64 = 1;
/// Duplicated lines the control accounts for: eight lines, twice.
const PRICE_CONTROL_LOC: u64 = 16;
/// The cell file and both control files.
const PRICE_FILES_ANALYSED: u64 = 3;
const PRICE_CASE: &str = "collection-cells-price";
const PRICE_LABEL: &str = "[CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] intra-file verbatim pair";

// [CLONE-NOISE-VERBATIM-SUBGROUP-CROSS-FILE] The price the arbitration
// accepts, stated as a contract instead of an absence.
//
// `noise.md` says what this pass deliberately gives up: "a genuine
// intra-file byte-identical copy sitting inside a component the filters
// suppressed stays hidden; that is the price of the idiom proof, paid
// once, visibly, in the pins". Nowhere was it visible. The four gh #434
// noise pins cannot pay it — every member of their families varies in
// its literals by construction, so none of them stages a byte-identical
// pair at all, and "the whole family was suppressed" would hold
// identically if the price did not exist.
//
// `ledger_rows.py` does stage one: cells 2 and 3 are byte-identical and
// cell 4 is not. Byte-identity across files is proof of copying —
// independently authored code does not coincide byte for byte.
// Byte-identity inside one file is proof of the *idiom* the filter just
// recognised, so the pair takes the suppression with its component. This
// fails if the hatch ever re-opens for an intra-file family, and it
// fails just as hard if the cross-file copy in the same run stops being
// reported.
#[test]
fn an_intra_file_verbatim_pair_inside_a_suppressed_component_stays_hidden() -> Result<()> {
    let report = render(PRICE_CASE, CELL_MIN_NODES)?;
    assert_family_hidden_with_control(
        &report,
        PRICE_LABEL,
        &PRICE_FILE,
        &PRICE_CONTROL,
        PRICE_HIDDEN,
    )?;
    assert_control_is_the_only_published_cluster(
        &report,
        PRICE_LABEL,
        &PRICE_CONTROL,
        PRICE_CONTROL_LOC,
    )?;
    assert_only_the_control_files_carry_duplicated_lines(&report, PRICE_LABEL, &PRICE_CONTROL);
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(PRICE_FILES_ANALYSED),
        "the intra-file pair was analysed and decided *against*, not skipped — a \
         file the scan never opened proves nothing about the arbitration: {report:#}"
    );
    for file in PRICE_FILE {
        assert_eq!(
            duplicated_loc_for(&report, file),
            0,
            "{file}: the price is that this pair earns nothing — a copy the report \
             will not show may not reach the duplication gate either: {lines:#?}",
            lines = visible_cluster_lines(&report),
        );
    }
    Ok(())
}
