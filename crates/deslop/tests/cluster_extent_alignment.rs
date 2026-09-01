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
    occurrences(cluster)
        .iter()
        .map(|occurrence| {
            (
                field(occurrence, "start_line").as_u64().unwrap_or_default(),
                field(occurrence, "end_line").as_u64().unwrap_or_default(),
            )
        })
        .collect()
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
