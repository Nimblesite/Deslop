//! [FUSED-CONTENT-GATE-CALL-TARGET] A corroborated method rename is a
//! rename, not a changed operation ([CLONE-BUCKETS-NORTH-STAR],
//! [FUSED-CONTENT-GATE-RENAME], [TECH-PMATCH-BAKER]).
//!
//! [CLONE-BUCKETS-NORTH-STAR] is unconditional: "Changing names or values
//! does not, by itself, make code shape-only. A systematic rename or
//! parameter substitution is still a copy." [FUSED-CONTENT-GATE-RENAME]
//! says the same from the other side: "A consistent rename is admitted
//! whatever its literals do."
//!
//! [FUSED-CONTENT-GATE-CALL-TARGET] carves out one exception, and it is a
//! real one: "Calls such as `toHaveCount` and `toContainText` perform
//! different operations." Swapping one assertion for another inside
//! otherwise-matching scaffolding is not a copy, and treating it as one
//! floods a report with false positives.
//!
//! But that exception is about an operation that *changed*, not a name
//! that was *renamed*, and the spec tells them apart by the receiver the
//! call is made on. A changed method is excused when its receiver is
//! itself a corroborated rename: the same handle mapped the same way more
//! than once, inside a mapping nothing contradicts ([TECH-PMATCH-BAKER]).
//! `ledger.postEntry` against `journal.writeRecord`, in a function whose
//! receiver, parameters, locals and types all rename together, is one
//! function written twice. `client.startJob` against `client.cancelJob`
//! renames nothing else and is a changed operation.
//!
//! This suite pins that rule with a fixture and its own controls. Every
//! corpus holds the same two functions and a byte-identical `ledger.ts`.
//! The `preserved` sides keep the two method names; the `renamed` sides
//! substitute them consistently at every call. The `near-miss` corpora
//! insert one statement into `journal.ts`, so the two endpoints no longer
//! align position for position ([FUSED-SHARED-SUBTREE]). Each control is
//! the causation proof: whatever the renamed corpus reports, the method
//! names are the only variable that could explain it.

use std::path::Path;

use serde_json::Value;

use crate::common::{signals::*, *};

/// Node floor matching the sibling rename suites, so each whole function
/// subtree qualifies as a candidate.
const MIN_NODES: u32 = 12;

/// The fixture root holding both corpora.
const FIXTURE: &str = "type2-rename-method-calls";

/// The corpus whose two method selectors are renamed with their callers.
const RENAMED_CORPUS: &str = "renamed";

/// The same pair with those selectors left alone — the control.
const PRESERVED_CORPUS: &str = "preserved";

/// The renamed pair with one statement inserted on the journal side, so
/// the endpoints no longer align position for position.
const NEAR_MISS_RENAMED_CORPUS: &str = "near-miss-renamed";

/// The near-miss pair with its selectors left alone — its control.
const NEAR_MISS_PRESERVED_CORPUS: &str = "near-miss-preserved";

/// The rename contract a rendered cluster must satisfy for one corpus.
type RenameContract = fn(&Path, &Value, &str) -> Result<()>;

/// The two sides of the copy, in both corpora.
const SIDES: [&str; 2] = ["ledger.ts", "journal.ts"];

/// The copy spans two files, so the cluster holds two occurrences.
const EXPECTED_OCCURRENCES: u64 = 2;

/// Both corpora hold nothing but the copy, so every analysed line is
/// duplicated and the repo metric is total ([METRICS-REPO]).
const TOTAL_DUPLICATION: f64 = 100.0;

/// One copy, written twice, is one clone group.
const EXPECTED_CLUSTERS: usize = 1;

/// Renaming names may not move a duplication figure by even one line.
const NO_TOLERANCE: u64 = 0;

/// Scans one corpus of the fixture on its own, so the renamed pair can
/// never be rescued by matching the control sitting beside it.
fn report_for(corpus: &str) -> Result<Value> {
    run_report(&fixture(FIXTURE).join(corpus), MIN_NODES)
}

/// The one cluster spanning both sides of the copy.
fn copy_cluster<'a>(report: &'a Value, corpus: &str) -> Result<&'a Value> {
    cluster_spanning_sides(
        report,
        &SIDES,
        &format!(
            "{corpus}: no rendered cluster spans both sides of the copy. One \
             function was copied and its names substituted one for one; a \
             clone that reaches no visible cluster is a false negative"
        ),
    )
}

/// Asserts a corpus reports the copy exactly once, with both occurrences,
/// and that the cluster satisfies the given rename contract.
fn assert_reports_the_copy(report: &Value, corpus: &str, contract: RenameContract) -> Result<()> {
    assert_eq!(
        cluster_count(report),
        EXPECTED_CLUSTERS,
        "{corpus}: one copy written twice is one clone group: {report:#}"
    );
    let cluster = copy_cluster(report, corpus)?;
    assert_eq!(
        cluster_size(cluster),
        EXPECTED_OCCURRENCES,
        "{corpus}: the copy has exactly two occurrences — {}",
        signal_dump(cluster)
    );
    contract(&fixture(FIXTURE).join(corpus), cluster, corpus)
}

/// Asserts a corpus is wholly duplicated: it holds nothing but the copy,
/// so reporting any of it as clean is a false negative ([METRICS-REPO]).
fn assert_wholly_duplicated(report: &Value, corpus: &str) {
    assert_eq!(
        metric_field(report, "duplication_percent").as_f64(),
        Some(TOTAL_DUPLICATION),
        "{corpus}: the corpus is nothing but the copy: {report:#}"
    );
    for side in SIDES {
        assert!(
            duplicated_loc_for(report, side) > NO_TOLERANCE,
            "{corpus}: {side} carries duplicated lines: {report:#}"
        );
    }
}

/// Asserts a rename moved no figure: the renamed corpus reports the same
/// duplicated lines as its control, in total and on each side.
fn assert_figures_match(renamed: &Value, control: &Value) {
    assert_eq!(
        visible_duplicated_loc(renamed),
        visible_duplicated_loc(control),
        "the two corpora hold the same copy of the same length; only two \
         method names differ, and each is substituted the same way at \
         every call. Substituting a name cannot delete a duplicated line: \
         {renamed:#}"
    );
    for side in SIDES {
        assert_eq!(
            duplicated_loc_for(renamed, side),
            duplicated_loc_for(control, side),
            "{side} loses duplicated lines to the rename alone: {renamed:#}"
        );
    }
}

// The control. Nothing here is in doubt: it fixes what the corpus is
// worth when the two selectors are left alone, so the suite below can
// attribute any difference to the rename and to nothing else.
#[test]
fn the_same_copy_with_its_method_names_left_alone_is_reported() -> Result<()> {
    let report = report_for(PRESERVED_CORPUS)?;
    assert_reports_the_copy(&report, PRESERVED_CORPUS, assert_proven_rename_contract)?;
    assert_wholly_duplicated(&report, PRESERVED_CORPUS);
    Ok(())
}

// [FUSED-CONTENT-GATE-CALL-TARGET] [CLONE-BUCKETS-NORTH-STAR] Renaming a
// method consistently, with every one of its callers, is a systematic
// rename — the Type-2 definition. It may not delete the copy.
#[test]
fn a_corroborated_method_rename_is_still_a_type2_clone() -> Result<()> {
    let report = report_for(RENAMED_CORPUS)?;
    assert_reports_the_copy(&report, RENAMED_CORPUS, assert_proven_rename_contract)
}

// The figures are the product this tool sells. A rename that empties
// them reports a copied codebase as clean ([METRICS-REPO]).
#[test]
fn a_method_rename_may_not_move_a_duplication_figure() -> Result<()> {
    let renamed = report_for(RENAMED_CORPUS)?;
    let control = report_for(PRESERVED_CORPUS)?;
    assert_figures_match(&renamed, &control);
    assert_wholly_duplicated(&renamed, RENAMED_CORPUS);
    Ok(())
}

// The near-miss control: one statement inserted, method names left alone.
// It fixes what the near-miss copy is worth before its methods are renamed.
#[test]
fn the_near_miss_copy_with_its_method_names_left_alone_is_reported() -> Result<()> {
    let report = report_for(NEAR_MISS_PRESERVED_CORPUS)?;
    assert_reports_the_copy(
        &report,
        NEAR_MISS_PRESERVED_CORPUS,
        assert_near_miss_rename_contract,
    )?;
    assert_wholly_duplicated(&report, NEAR_MISS_PRESERVED_CORPUS);
    assert_eq!(
        cluster_kind(copy_cluster(&report, NEAR_MISS_PRESERVED_CORPUS)?),
        LOOSELY_SIMILAR_KIND,
        "an inserted statement makes the copy a near miss: {report:#}"
    );
    Ok(())
}

// [FUSED-CONTENT-GATE-CALL-TARGET] [CLONE-BUCKETS-NORTH-STAR] The receiver
// rule does not stop applying because a statement was inserted. The renamed
// near miss is its control with two method names substituted on a receiver
// that renames throughout, and the spec excuses every one of them.
#[test]
fn a_corroborated_method_rename_is_still_a_type3_clone() -> Result<()> {
    let renamed = report_for(NEAR_MISS_RENAMED_CORPUS)?;
    let control = report_for(NEAR_MISS_PRESERVED_CORPUS)?;
    assert_reports_the_copy(
        &renamed,
        NEAR_MISS_RENAMED_CORPUS,
        assert_near_miss_rename_contract,
    )?;
    assert_wholly_duplicated(&renamed, NEAR_MISS_RENAMED_CORPUS);
    assert_figures_match(&renamed, &control);
    assert_eq!(
        cluster_kind(copy_cluster(&renamed, NEAR_MISS_RENAMED_CORPUS)?),
        cluster_kind(copy_cluster(&control, NEAR_MISS_PRESERVED_CORPUS)?),
        "renaming names may not move the copy between buckets: {renamed:#}"
    );
    Ok(())
}
