//! [PIPELINE-CLUSTER-EXACT-SCOPE] The Go authored-window contract.
//!
//! A cluster is one duplication, so every occurrence it publishes must
//! describe the same authored view. In Go that view is always a top-level
//! declaration — `func`, `type`, `const` or `var` — or a node inside one.
//! It is never the file itself.
//!
//! A Go file opens with a `package` clause and an `import` block that every
//! file in the package repeats verbatim. Those rows are not duplication:
//! they are what the language requires. An occurrence that reaches back to
//! line 1 to collect them is claiming the compiler's boilerplate as a copy,
//! and it drags whatever sits between the prologue and the real clone —
//! type declarations, constants — in with it.
//!
//! The damage is not cosmetic. `duplicated_loc` and `duplication_percent`
//! project these line sets ([METRICS-REPO]), so one padded occurrence
//! inflates the headline figure for the whole repository, and `mass`
//! prices the cluster from a `canonical_node_count` that describes only
//! the padded half ([RANK-MASS-SUM]).
//!
//! Every Go suite calls into this module, so the contract is asserted once
//! and enforced everywhere rather than restated per fixture.

use std::path::Path;

use serde_json::Value;

use super::{
    clusters, occurrence_line_span, occurrence_path, occurrence_text, occurrences, Result,
};

/// The file extension whose scope contract this module enforces.
pub(crate) const GO_EXTENSION: &str = ".go";

/// The clause every Go file opens with. No occurrence may contain it.
pub(crate) const GO_PACKAGE_CLAUSE: &str = "package ";

/// The import block every file in a package repeats. Not duplication.
pub(crate) const GO_IMPORT_CLAUSE: &str = "import ";

/// The top-level declaration keywords a Go occurrence may open with.
pub(crate) const GO_DECLARATION_KEYWORDS: [&str; 4] = ["func ", "type ", "const ", "var "];

/// The first physical row of a file. An occurrence that starts here has
/// taken the whole file rather than an authored declaration.
pub(crate) const FIRST_LINE_OF_FILE: u64 = 1;

/// Returns the occurrences of `cluster` that point into a Go file.
pub(crate) fn go_occurrences(cluster: &Value) -> Vec<&Value> {
    occurrences(cluster)
        .iter()
        .filter(|occurrence| {
            occurrence_path(occurrence)
                .map(|path| path.ends_with(GO_EXTENSION))
                .unwrap_or(false)
        })
        .collect()
}

/// How many rows an occurrence covers, both endpoints included.
pub(crate) fn occurrence_row_count(occurrence: &Value) -> u64 {
    let (start, end) = occurrence_line_span(occurrence);
    end.saturating_sub(start).saturating_add(1)
}

/// Renders every Go occurrence of `cluster` as `path L<start>-<end>` so a
/// failure names the physical rows a reader would be sent to.
pub(crate) fn go_spans(cluster: &Value) -> Vec<String> {
    go_occurrences(cluster)
        .iter()
        .map(|occurrence| {
            let (start, end) = occurrence_line_span(occurrence);
            format!(
                "{} L{start}-{end}",
                occurrence_path(occurrence).unwrap_or("?")
            )
        })
        .collect()
}

/// True when `text` opens one of Go's top-level declaration keywords.
pub(crate) fn opens_authored_declaration(text: &str) -> bool {
    let head = text.trim_start();
    GO_DECLARATION_KEYWORDS
        .iter()
        .any(|keyword| head.starts_with(keyword))
}

/// True when any row of `text` begins with `clause` at column zero, which
/// is where Go puts the constructs a file may not share as duplication.
pub(crate) fn holds_top_level_clause(text: &str, clause: &str) -> bool {
    text.lines().any(|line| line.starts_with(clause))
}

/// No Go occurrence may reach back to row 1 and take the whole file.
pub(crate) fn assert_no_occurrence_takes_the_file(cluster: &Value, label: &str) {
    for occurrence in go_occurrences(cluster) {
        let (start, _end) = occurrence_line_span(occurrence);
        assert_ne!(
            start,
            FIRST_LINE_OF_FILE,
            "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: an occurrence opens at \
             row {FIRST_LINE_OF_FILE}, so it has taken the whole file rather \
             than an authored declaration: {:?}",
            go_spans(cluster)
        );
    }
}

/// No Go occurrence may carry the package clause its counterpart repeats
/// only because the language demands it.
pub(crate) fn assert_no_package_clause(scan_root: &Path, cluster: &Value, label: &str) -> Result<()> {
    for occurrence in go_occurrences(cluster) {
        let text = occurrence_text(scan_root, occurrence)?;
        assert!(
            !holds_top_level_clause(&text, GO_PACKAGE_CLAUSE),
            "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: an occurrence swallows \
             the `{GO_PACKAGE_CLAUSE}` clause, which every file in the \
             package repeats and none of them copied: {:?}",
            go_spans(cluster)
        );
    }
    Ok(())
}

/// No Go occurrence may carry the import block.
pub(crate) fn assert_no_import_block(scan_root: &Path, cluster: &Value, label: &str) -> Result<()> {
    for occurrence in go_occurrences(cluster) {
        let text = occurrence_text(scan_root, occurrence)?;
        assert!(
            !holds_top_level_clause(&text, GO_IMPORT_CLAUSE),
            "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: an occurrence swallows \
             the `{GO_IMPORT_CLAUSE}` block, so the report claims a file's \
             dependency list is a copy of its counterpart's: {:?}",
            go_spans(cluster)
        );
    }
    Ok(())
}

/// Every Go occurrence opens an authored declaration, or none does. A
/// cluster that mixes the two describes two different subtrees under one
/// canonical extent.
pub(crate) fn assert_declaration_alignment(
    scan_root: &Path,
    cluster: &Value,
    label: &str,
) -> Result<()> {
    let occurrences = go_occurrences(cluster);
    let mut opening = 0usize;
    for occurrence in &occurrences {
        if opens_authored_declaration(&occurrence_text(scan_root, occurrence)?) {
            opening += 1;
        }
    }
    assert!(
        opening == 0 || opening == occurrences.len(),
        "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: {opening} of {} occurrences \
         open an authored declaration, so the cluster mixes a declaration \
         window with a window that starts somewhere else: {:?}",
        occurrences.len(),
        go_spans(cluster)
    );
    Ok(())
}

/// Applies every Go scope rule to one cluster.
pub(crate) fn assert_cluster_scope(scan_root: &Path, cluster: &Value, label: &str) -> Result<()> {
    assert_no_occurrence_takes_the_file(cluster, label);
    assert_no_package_clause(scan_root, cluster, label)?;
    assert_no_import_block(scan_root, cluster, label)?;
    assert_declaration_alignment(scan_root, cluster, label)
}

/// Applies every Go scope rule to every cluster of `report`. This is the
/// blanket contract: any Go suite that produces a report calls it, so a
/// padded window cannot survive anywhere in the corpus of fixtures.
pub(crate) fn assert_go_authored_scope(scan_root: &Path, report: &Value, label: &str) -> Result<()> {
    for cluster in clusters(report) {
        assert_cluster_scope(scan_root, cluster, label)?;
    }
    Ok(())
}

/// Both halves of a same-shape Go pair cover the same number of rows. A
/// line-for-line counterpart that is reported twice as wide in one file is
/// not the same authored view ([PIPELINE-CLUSTER-EXACT-SCOPE]).
pub(crate) fn assert_symmetric_rows(cluster: &Value, label: &str) {
    let counts: Vec<u64> = go_occurrences(cluster)
        .iter()
        .map(|occurrence| occurrence_row_count(occurrence))
        .collect();
    let widest = counts.iter().copied().max().unwrap_or_default();
    let narrowest = counts.iter().copied().min().unwrap_or_default();
    assert_eq!(
        widest, narrowest,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: the halves of one pair cover \
         {narrowest} and {widest} rows, so the cluster prices one view and \
         renders another: {:?}",
        go_spans(cluster)
    );
}

/// No Go occurrence may contain `symbol`, a name that exists in one file of
/// the pair and has no counterpart in the other. Its presence proves the
/// window was padded past the shared declaration.
pub(crate) fn assert_no_unshared_symbol(
    scan_root: &Path,
    cluster: &Value,
    symbol: &str,
    label: &str,
) -> Result<()> {
    for occurrence in go_occurrences(cluster) {
        let text = occurrence_text(scan_root, occurrence)?;
        assert!(
            !text.contains(symbol),
            "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: an occurrence claims \
             `{symbol}`, which its counterpart does not contain, so the two \
             halves are not copies of one another: {:?}",
            go_spans(cluster)
        );
    }
    Ok(())
}

/// Applies the same-shape row-symmetry rule to every cluster of `report`.
/// Only same-shape fixtures call this: a genuine Type-3 near-miss may
/// legitimately cover a different row count on each side.
pub(crate) fn assert_symmetric_rows_everywhere(report: &Value, label: &str) {
    for cluster in clusters(report) {
        assert_symmetric_rows(cluster, label);
    }
}

/// Every Go occurrence in `report` opens an authored declaration. Stronger
/// than [`assert_declaration_alignment`], which permits a cluster in which
/// *no* occurrence opens one: fixtures whose clones are whole functions
/// assert this instead.
pub(crate) fn assert_every_occurrence_opens_a_declaration(
    scan_root: &Path,
    report: &Value,
    label: &str,
) -> Result<()> {
    for cluster in clusters(report) {
        for occurrence in go_occurrences(cluster) {
            let text = occurrence_text(scan_root, occurrence)?;
            assert!(
                opens_authored_declaration(&text),
                "[PIPELINE-CLUSTER-EXACT-SCOPE] {label}: an occurrence opens \
                 with something other than a Go declaration keyword \
                 {GO_DECLARATION_KEYWORDS:?}, so the published window is not \
                 an authored view: {:?}",
                go_spans(cluster)
            );
        }
    }
    Ok(())
}
