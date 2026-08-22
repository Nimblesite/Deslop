//! gh #362 — two files of unrelated constant declarations are not
//! duplication ([CLONE-NOISE-CONSTANT-TABLE], [RANK-STRUCTURAL-ONLY]).
//!
//! Normalisation collapses every string literal to `__literal__` and
//! every name to `__ident__`, so a run of sibling `const NAME: &str =
//! r"…";` declarations has exactly the same normalised shape as any
//! other such run. Two test-data files holding entirely different
//! payloads — one a set of language snippets, the other generated DTOs,
//! a migration schema and HTTP routes — therefore fuse at `structural =
//! 1.00`. `token_jaccard = 0.00` states outright that they share no
//! content: **there is no extraction available**, which is the
//! definition of scaffolding.
//!
//! The shipped defect had two halves, and demoting the cluster only
//! fixes the first:
//!
//! 1. it was reported, and on the repository it was found in it was the
//!    single largest ranked finding at 344 LOC; and
//! 2. **a demoted cluster is still counted in `duplicated_loc`**, which
//!    is what feeds the CI duplication gate — so the false positive
//!    moves the repository's own budget whether or not a reader ever
//!    sees it ranked.
//!
//! Neither the ≥3-file `[CLONE-NOISE-SCAFFOLDING]` hide nor the
//! single-file `[RANK-STRUCTURAL-ONLY]` declaration-family hide covers a
//! **two**-file spread, so this geometry fell straight through the gap.
//!
//! # Why this fixture cannot pass by going blind
//!
//! `alpha_report.rs` / `beta_report.rs` hold a byte-identical
//! `apply_discount_schedule`. Any suppression wide enough to eat that
//! pair fails this suite: the real clone must stay visible, stay
//! `identical`, rank first, and keep counting its own eleven lines in
//! the metric. Widening a suppression is exactly how a real clone gets
//! erased, so the control is asserted in the same run as the
//! suppression.

use serde_json::Value;

use crate::common::{signals::*, *};

/// Node floor low enough that a run of three or four sibling constant
/// declarations qualifies as a candidate window — the geometry the
/// issue reports.
const MIN_NODES: u32 = 8;

/// The two files whose constant runs share nothing but their shape.
const CONST_TABLES: [&str; 2] = ["boilerplate_cases.rs", "defaults_cases.rs"];

/// The two files holding the authored byte-identical clone.
const REAL_CLONE: [&str; 2] = ["alpha_report.rs", "beta_report.rs"];

/// Lines of `apply_discount_schedule`, counted in the fixture: the whole
/// of the duplication this corpus actually contains.
const CLONE_LOC_PER_FILE: u64 = 11;

/// Renders the fixture.
fn render() -> Result<Value> {
    run_report(&fixture("two-file-const-tables").join("src"), MIN_NODES)
}

/// Per-file duplicated LOC as the report renders it.
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

// [CLONE-NOISE-CONSTANT-TABLE] The suppression itself: two unrelated
// constant tables produce no visible cluster, and — the half a demotion
// cannot deliver — contribute nothing to the duplication metric.
#[test]
fn unrelated_constant_tables_are_neither_reported_nor_counted() -> Result<()> {
    let report = render()?;
    assert!(
        cluster_spanning(&report, &CONST_TABLES).is_none(),
        "two runs of unrelated constant declarations share only the shape \
         normalisation leaves behind — there is no extraction available, so no \
         cluster may span {CONST_TABLES:?} (gh #362): {found:#?}",
        found = clusters(&report)
            .iter()
            .map(|cluster| (
                cluster_id(cluster),
                cluster_bucket(cluster),
                occurrence_files(cluster)
            ))
            .collect::<Vec<_>>(),
    );
    for file in CONST_TABLES {
        assert_eq!(
            duplicated_loc_for(&report, file),
            0,
            "{file} holds no duplicated line, so it must contribute none to \
             `duplicated_loc` — a demoted cluster still feeds the CI duplication \
             gate, which is the half of gh #362 that ranking cannot fix"
        );
    }
    Ok(())
}

// [RANK-STRUCTURAL-ONLY] The false-negative control, asserted in the
// same run: the authored byte-identical clone survives the suppression
// intact, at the head of the report.
#[test]
fn the_authored_clone_survives_the_suppression_and_ranks_first() -> Result<()> {
    let report = render()?;
    let cluster = expect_cluster_spanning(&report, &REAL_CLONE)?;
    let dump = signal_dump(cluster);

    assert_eq!(
        cluster_bucket(cluster),
        "identical",
        "`apply_discount_schedule` is copied byte for byte — {dump}"
    );
    assert_eq!(
        cluster_size(cluster),
        2,
        "both copies must be shown; a suppression that eats one is a false \
         negative — {dump}"
    );
    assert!(
        approx(signal(cluster, "fused"), 1.0),
        "byte-proven duplication saturates the confidence — {dump}"
    );
    assert_eq!(
        cluster_id(clusters(&report).first().unwrap_or(&Value::Null)),
        cluster_id(cluster),
        "the one real duplicate in this corpus must be the report's first \
         finding; the shipped defect ranked a 344-LOC constant table above \
         every genuine clone (gh #362): {ranked:#?}",
        ranked = clusters(&report)
            .iter()
            .map(|entry| (
                cluster_id(entry),
                cluster_bucket(entry),
                signal(entry, "weight")
            ))
            .collect::<Vec<_>>(),
    );
    for file in REAL_CLONE {
        assert_eq!(
            duplicated_loc_for(&report, file),
            CLONE_LOC_PER_FILE,
            "{file}: the real clone's own lines must keep counting — a fix that \
             zeroes the metric everywhere has not distinguished anything"
        );
    }
    Ok(())
}

// [METRICS-REPO] The metric counts exactly the list the report shows.
// Asserted here because gh #362's damage was to the number, not to the
// list: a cluster hidden from the reader while still inflating
// `duplicated_loc` would satisfy both tests above and still ship the
// defect.
#[test]
fn the_duplication_metric_counts_only_what_the_report_shows() -> Result<()> {
    let report = render()?;
    assert_eq!(
        metric_field(&report, "duplicated_loc")
            .as_u64()
            .unwrap_or_default(),
        visible_duplicated_loc(&report),
        "`duplicated_loc` must equal the lines the visible clusters cover: \
         {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    assert_eq!(
        visible_duplicated_loc(&report),
        CLONE_LOC_PER_FILE.saturating_mul(2),
        "the corpus contains exactly one duplication, of {CLONE_LOC_PER_FILE} \
         lines, in two files: {lines:#?}",
        lines = visible_cluster_lines(&report),
    );
    Ok(())
}
