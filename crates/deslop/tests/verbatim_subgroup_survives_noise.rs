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

use crate::common::{signals::*, *};

/// Node floor low enough that a run of four sibling constant
/// declarations, a run of four calls, or one collection cell qualifies
/// as a candidate window — the geometry all three issues report.
const MIN_NODES: u32 = 8;

/// The node floor the collection-cell family needs: one cell of a list
/// literal is a smaller window than a four-statement run.
const CELL_MIN_NODES: u32 = 4;

/// The two byte-identical constant tables, and the unrelated table that
/// shares their normalised shape.
const CONST_COPY: [&str; 2] = ["retry_defaults.py", "retry_defaults_copy.py"];
/// The stranger whose only relation to [`CONST_COPY`] is its shape.
const CONST_STRANGER: &str = "theme_tokens.py";

/// The two byte-identical call runs, and the run whose literals vary.
const CALL_COPY: [&str; 2] = ["invoice_emitter.py", "invoice_emitter_copy.py"];
/// The stranger whose only relation to [`CALL_COPY`] is its shape.
const CALL_STRANGER: &str = "refund_emitter.py";

/// The single file holding one list literal whose first two cells are
/// byte-identical and whose third is not.
const CELL_FILE: &str = "ledger_rows.py";
/// The 1-based lines of the two identical cells, in file order.
const CELL_COPY_LINES: [(u64, u64); 2] = [(2, 2), (3, 3)];
/// The 1-based line of the cell that differs.
const CELL_STRANGER_LINE: u64 = 4;

/// Lines each copy of the constant table covers.
const CONST_LOC_PER_FILE: u64 = 4;
/// Lines each copy of the call run covers — the whole five-line `emit`
/// function, not only the four `persist` calls inside it.
///
/// `invoice_emitter.py` and `invoice_emitter_copy.py` are byte-identical
/// files, so `def emit():` is as duplicated as the calls under it, and
/// the published `identical` cluster spans L1-5 of both. Four was what
/// the same-file overlap collapse elected while it ranked an
/// overlapping run by cross-file edge strength: the four-call window
/// scored higher than the function enclosing it purely by carrying less
/// code, and the `def` line went uncounted
/// ([PIPELINE-CLUSTER-EXACT-SCOPE], gh #408). The undercount was the
/// artifact; five is what the two files actually share.
const CALL_LOC_PER_FILE: u64 = 5;

/// Renders one `verbatim-subgroup` case.
fn render(case: &str, min_nodes: u32) -> Result<Value> {
    run_report(&fixture("verbatim-subgroup").join(case), min_nodes)
}

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

/// Per-file duplicated LOC as the report renders it, `0` when the file
/// carries no row at all.
fn duplicated_loc_for(report: &Value, file: &str) -> u64 {
    per_file_metrics(report)
        .iter()
        .find(|metric| {
            field(metric, "path")
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .map_or(0, |metric| {
            field(metric, "duplicated_loc").as_u64().unwrap_or_default()
        })
}

/// Every visible cluster as `id [bucket] files` — the smallest dump
/// that diagnoses a failure without re-running the scan.
fn published(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} [{bucket}] {files:?}",
                id = cluster_id(cluster),
                bucket = cluster_bucket(cluster),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

/// Asserts the copy spanning `copy` survives as one saturated,
/// `identical`, size-2 cluster that the stranger is not part of.
fn assert_copy_survives_alone(
    report: &Value,
    label: &str,
    copy: &[&str; 2],
    stranger: &str,
) -> Result<()> {
    let cluster = expect_cluster_spanning(report, copy)?;
    let dump = signal_dump(cluster);
    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "{label}: the pair is copied byte for byte, so it is `identical` \
         whatever else joined its cluster — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "{label}: exactly the two copies are shown — {dump}"
    );
    assert!(
        approx(signal(cluster, "structural"), 1.0)
            && approx(signal(cluster, "token_jaccard"), 1.0)
            && approx(signal(cluster, "fused"), 1.0),
        "{label}: byte-proven duplication saturates every axis it was \
         measured on — {dump}"
    );
    assert_eq!(
        cluster_file_set(cluster),
        copy.iter().map(|name| (*name).to_owned()).collect(),
        "{label}: the copy's cluster spans exactly its own two files"
    );
    assert!(
        !occurrence_files(cluster)
            .iter()
            .any(|file| file == stranger),
        "{label}: {stranger} is not a copy of anything — it shares only the \
         shape normalisation leaves behind, so it must not be an occurrence \
         of the copy's cluster: {files:?}",
        files = occurrence_files(cluster),
    );
    Ok(())
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
    let report = render("literal-calls", MIN_NODES)?;
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
