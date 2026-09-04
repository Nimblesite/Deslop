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
fn occurrence_line_spans(cluster: &serde_json::Value) -> Vec<(u64, u64)> {
    cluster_line_spans(cluster)
}

/// Returns how many lines each occurrence covers, inclusive of both
/// endpoints.
fn occurrence_line_counts(cluster: &serde_json::Value) -> Vec<u64> {
    occurrence_line_spans(cluster)
        .into_iter()
        .map(|(start, end)| end.saturating_sub(start).saturating_add(1))
        .collect()
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

    // A canonical node count that describes only one occurrence prices
    // the whole cluster wrong ([RANK-MASS-SUM]).
    let nodes = field(cluster, "canonical_node_count")
        .as_u64()
        .unwrap_or_default();
    let visible = u64::try_from(texts.len()).unwrap_or_default();
    assert_eq!(
        field(cluster, "mass").as_u64().unwrap_or_default(),
        nodes.saturating_mul(visible.saturating_sub(1)),
        "[RANK-MASS-SUM] mass is canonical nodes x additional visible \
         occurrences: {cluster:#}"
    );
    Ok(())
}

/// Go fixture whose two files share exactly one authored pair of
/// tree-writing functions. Everything else in each file is unique to it:
/// one carries an extra unrelated function before the pair, the other a
/// package prologue, three imports its counterpart does not have, and a
/// lone type declaration.
const GO_EXTENT_FIXTURE: &str = "go-cluster-extent-alignment";

/// The file whose prologue, imports and type declaration have no
/// counterpart in the other occurrence.
const JSON_REPORT: &str = "json_report.go";
/// The file that carries an extra unrelated function before the pair.
const TEXT_REPORT: &str = "text_report.go";

/// The Go declaration keyword. Both halves of the shared pair are
/// functions, so every honest occurrence of this cluster opens one.
const GO_DECLARATION_KEYWORD: &str = "func ";
/// The Go file prologue. An occurrence that swallows it is claiming the
/// package clause and import block are part of the duplication.
const GO_PACKAGE_KEYWORD: &str = "package ";
/// The type declaration that exists in one file only.
const GO_TYPE_KEYWORD: &str = "type ";

/// `--min-nodes` far above the pair's floor. The defect reproduces at 8,
/// 12, 20 and 30 alike, so the assertion does not hinge on the threshold.
const GO_EXTENT_MIN_NODES: u32 = 12;

/// Counts the occurrences whose text opens with `keyword`.
fn occurrences_opening_with(texts: &[String], keyword: &str) -> usize {
    texts
        .iter()
        .filter(|text| text.trim_start().starts_with(keyword))
        .count()
}

/// Counts the occurrences whose text contains `keyword` at the start of
/// any line, which is where Go puts its top-level constructs.
fn occurrences_containing_top_level(texts: &[String], keyword: &str) -> usize {
    texts
        .iter()
        .filter(|text| text.lines().any(|line| line.starts_with(keyword)))
        .count()
}

#[test]
fn a_go_pair_is_not_padded_with_the_prologue_and_types_of_one_file() -> Result<()> {
    let scan_root = fixture(GO_EXTENT_FIXTURE);
    let report = run_report(&scan_root, GO_EXTENT_MIN_NODES)?;
    let cluster = expect_cluster_spanning(&report, &[JSON_REPORT, TEXT_REPORT])?;

    let texts = occurrence_texts(&scan_root, cluster)?;
    assert_eq!(
        texts.len(),
        EXPECTED_OCCURRENCES,
        "the shared pair is one clone in each file: {cluster:#}"
    );

    // The defect: one occurrence starts at the authored declaration, the
    // other reaches back to the top of its file.
    let opens_declaration = occurrences_opening_with(&texts, GO_DECLARATION_KEYWORD);
    assert_eq!(
        opens_declaration,
        texts.len(),
        "[PIPELINE-CLUSTER-EXACT-SCOPE] every occurrence must open the \
         authored declaration; only {opens_declaration} of {} do, so the \
         cluster mixes a declaration window with a whole-file window: \
         spans={:?}",
        texts.len(),
        occurrence_line_spans(cluster)
    );

    // A package clause and import block are not duplication. Either both
    // occurrences carry a prologue or neither may.
    let carries_prologue = occurrences_containing_top_level(&texts, GO_PACKAGE_KEYWORD);
    assert_eq!(
        carries_prologue,
        0,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] {carries_prologue} occurrence(s) \
         swallow the package clause and import block, which the other \
         occurrence has no counterpart for: spans={:?}",
        occurrence_line_spans(cluster)
    );

    // Same for the type declaration that exists in one file only.
    let carries_type = occurrences_containing_top_level(&texts, GO_TYPE_KEYWORD);
    assert_eq!(
        carries_type,
        0,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] {carries_type} occurrence(s) claim \
         a type declaration the counterpart does not contain: spans={:?}",
        occurrence_line_spans(cluster)
    );

    // The metric consequence: `duplicated_loc` projects these line sets
    // ([METRICS-REPO]), so an over-wide occurrence inflates the headline
    // percentage for the whole repository.
    let line_counts = occurrence_line_counts(cluster);
    let widest = line_counts.iter().copied().max().unwrap_or_default();
    let narrowest = line_counts.iter().copied().min().unwrap_or_default();
    assert_eq!(
        widest,
        narrowest,
        "[PIPELINE-CLUSTER-EXACT-SCOPE] both halves of the pair are the \
         same authored shape and must cover the same row count; spans={:?} \
         texts={texts:#?}",
        occurrence_line_spans(cluster)
    );

    // A canonical node count that describes only one occurrence prices
    // the whole cluster wrong ([RANK-MASS-SUM]).
    let nodes = field(cluster, "canonical_node_count")
        .as_u64()
        .unwrap_or_default();
    let visible = u64::try_from(texts.len()).unwrap_or_default();
    assert_eq!(
        field(cluster, "mass").as_u64().unwrap_or_default(),
        nodes.saturating_mul(visible.saturating_sub(1)),
        "[RANK-MASS-SUM] mass is canonical nodes x additional visible \
         occurrences: {cluster:#}"
    );
    Ok(())
}
