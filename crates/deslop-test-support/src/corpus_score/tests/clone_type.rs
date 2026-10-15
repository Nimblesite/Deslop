//! [CORPUS-REGISTER-TYPE] A CLEARLY IN pinned to a clone type is found only by
//! a cluster whose kind reports that type; a blank type accepts any clone kind.

use anyhow::Result;
use serde_json::{json, Value};

use super::{
    card::card, first_entry, pinned_sha, register, score_repo, scorecard, the_pair, CLEARLY_IN,
    CLEARLY_OUT, FIRST_RANGE, REPO, SECOND_RANGE,
};

/// The register field a judge pins the type in.
const TYPE_FIELD: &str = "type";
/// The four research types, spelled as the judging protocol spells them.
const TYPE_ONE: &str = "Type I";
const TYPE_TWO: &str = "Type II";
const TYPE_THREE: &str = "Type III";
const TYPE_FOUR: &str = "Type IV";
/// Wire spellings of every cluster kind a report can publish.
const IDENTICAL: &str = "identical";
const NEARLY_IDENTICAL: &str = "nearly_identical";
const LOOSELY_SIMILAR: &str = "loosely_similar";
const SAME_BEHAVIOR: &str = "same_behavior";
const STRUCTURAL_ONLY: &str = "structural_only";
const EVERY_KIND: [&str; 5] = [
    IDENTICAL,
    NEARLY_IDENTICAL,
    LOOSELY_SIMILAR,
    SAME_BEHAVIOR,
    STRUCTURAL_ONLY,
];
/// Which kinds answer which type, per [CLONE-KIND-LABELS]: the engine splits
/// Type III between nearly identical and similar code.
const ANSWERS: [(&str, &[&str]); 4] = [
    (TYPE_ONE, &[IDENTICAL]),
    (TYPE_TWO, &[NEARLY_IDENTICAL]),
    (TYPE_THREE, &[NEARLY_IDENTICAL, LOOSELY_SIMILAR]),
    (TYPE_FOUR, &[SAME_BEHAVIOR]),
];
/// Every kind that is a clone at all; a blank type accepts each of them.
const CLONE_KINDS: [&str; 4] = [IDENTICAL, NEARLY_IDENTICAL, LOOSELY_SIMILAR, SAME_BEHAVIOR];
/// Cluster ids for a report that publishes the same pair under two kinds.
const EXACT_ID: &str = "exact";
const RENAMED_ID: &str = "renamed";
/// Labels a judge might type that are not one of the four.
const MALFORMED_TYPES: [&str; 4] = ["Type 2", "type ii", "Type-1", "Type V"];
/// What a typed miss adds to its defect line, so a reader can see why a pair
/// the engine did report still counts against it.
const MISTYPED_DEFECT: &str =
    "`src/one.rs:10-20` + `src/two.rs:30-40` (Type I; reported as nearly_identical)";
/// A typed pair nothing reported at all names only the type it was held to,
/// then how it stands against the judged pair record.
const UNREPORTED_DEFECT: &str = "`src/one.rs:10-20` + `src/two.rs:30-40` (Type I) — failing, and \
                                 not on record as failing\n";

/// One judged pair under `verdict`, pinned to `clone_type`.
fn typed(verdict: &str, clone_type: &Value) -> Value {
    json!({
        "sha": pinned_sha(),
        verdict: [ {
            "why": "judged",
            TYPE_FIELD: clone_type,
            "occurrences": [FIRST_RANGE, SECOND_RANGE],
        } ],
    })
}

/// A report that publishes the judged pair once, as one cluster of `kind`.
fn published_as(kind: &str) -> Value {
    json!({ "clusters": [ { "id": kind, "kind": kind, "occurrences": the_pair() } ] })
}

/// A report that publishes the judged pair twice: identical, then renamed.
fn published_twice() -> Value {
    json!({ "clusters": [
        { "id": EXACT_ID, "kind": IDENTICAL, "occurrences": the_pair() },
        { "id": RENAMED_ID, "kind": NEARLY_IDENTICAL, "occurrences": the_pair() },
    ] })
}

/// Asserts whether `judged` is found in a report publishing the pair as `kind`.
fn assert_found(judged: &Value, kind: &str, found: bool, label: &str) -> Result<()> {
    let score = score_repo(REPO, judged, &published_as(kind))?;
    let entry = first_entry(&score)?;
    assert_eq!(
        score.clearly_in_total, 1,
        "{label} vs {kind}: one judged pair"
    );
    assert_eq!(
        score.clearly_in_found,
        usize::from(found),
        "{label} vs {kind}"
    );
    assert_eq!(
        score.false_negatives,
        usize::from(!found),
        "{label} vs {kind}"
    );
    assert_eq!(entry.matched, found, "{label} vs {kind}: matched");
    assert_eq!(entry.correct, found, "{label} vs {kind}: correct");
    let expected_cluster = found.then_some(kind);
    assert_eq!(
        entry.cluster.as_deref(),
        expected_cluster,
        "{label} vs {kind}"
    );
    Ok(())
}

#[test]
fn a_pinned_type_is_found_only_by_the_kinds_that_report_it() -> Result<()> {
    for (clone_type, answered_by) in ANSWERS {
        let judged = typed(CLEARLY_IN, &json!(clone_type));
        for kind in EVERY_KIND {
            assert_found(&judged, kind, answered_by.contains(&kind), clone_type)?;
        }
        let score = score_repo(REPO, &judged, &published_as(IDENTICAL))?;
        assert_eq!(
            first_entry(&score)?.clone_type.as_deref(),
            Some(clone_type),
            "the scored entry carries the type it was held to"
        );
    }
    Ok(())
}

#[test]
fn a_blank_or_absent_type_is_found_by_any_clone_kind() -> Result<()> {
    let blank = typed(CLEARLY_IN, &json!(""));
    let absent = register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]);
    for (judged, label) in [(&blank, "blank"), (&absent, "absent")] {
        for kind in CLONE_KINDS {
            assert_found(judged, kind, true, label)?;
        }
        assert_found(judged, STRUCTURAL_ONLY, false, label)?;
        let score = score_repo(REPO, judged, &published_as(IDENTICAL))?;
        assert_eq!(
            first_entry(&score)?.clone_type,
            None,
            "{label} pins nothing"
        );
    }
    Ok(())
}

#[test]
fn a_typed_pair_is_answered_by_the_cluster_of_its_own_type() -> Result<()> {
    let report = published_twice();
    for (clone_type, cluster) in [(TYPE_ONE, EXACT_ID), (TYPE_TWO, RENAMED_ID)] {
        let score = score_repo(REPO, &typed(CLEARLY_IN, &json!(clone_type)), &report)?;
        assert_eq!(score.clearly_in_found, 1, "{clone_type} is published");
        assert_eq!(first_entry(&score)?.cluster.as_deref(), Some(cluster));
    }
    let untyped = register(CLEARLY_IN, &[FIRST_RANGE, SECOND_RANGE]);
    let score = score_repo(REPO, &untyped, &report)?;
    assert_eq!(score.clearly_in_found, 1);
    assert_eq!(
        first_entry(&score)?.cluster.as_deref(),
        Some(EXACT_ID),
        "an untyped pair keeps the first of two equally covering clusters"
    );
    let unpublished = score_repo(REPO, &typed(CLEARLY_IN, &json!(TYPE_FOUR)), &report)?;
    assert_eq!(unpublished.false_negatives, 1, "no cluster reports Type IV");
    Ok(())
}

#[test]
fn a_type_outside_the_four_fails_the_score_rather_than_matching_anything() {
    for malformed in MALFORMED_TYPES {
        let scored = score_repo(
            REPO,
            &typed(CLEARLY_IN, &json!(malformed)),
            &published_twice(),
        );
        assert!(scored.is_err(), "`{malformed}` is not a clone type");
    }
    let numeric = score_repo(REPO, &typed(CLEARLY_IN, &json!(2)), &published_twice());
    assert!(numeric.is_err(), "a number is not a clone type");
}

#[test]
fn a_type_on_a_clearly_out_fails_the_score_because_it_names_no_clone() -> Result<()> {
    let pinned = score_repo(
        REPO,
        &typed(CLEARLY_OUT, &json!(TYPE_ONE)),
        &published_twice(),
    );
    assert!(
        pinned.is_err(),
        "a CLEARLY OUT is not a clone, so it has no type"
    );
    let blank = score_repo(REPO, &typed(CLEARLY_OUT, &json!("")), &published_twice())?;
    assert_eq!(blank.false_positives, 1, "a blank type pins nothing");
    assert_eq!(first_entry(&blank)?.cluster.as_deref(), Some(EXACT_ID));
    Ok(())
}

#[test]
fn a_typed_miss_names_its_type_and_the_kind_that_did_report_it() -> Result<()> {
    let judged = typed(CLEARLY_IN, &json!(TYPE_ONE));
    let found = score_repo(REPO, &judged, &published_as(IDENTICAL))?;
    let mistyped = score_repo(REPO, &judged, &published_as(NEARLY_IDENTICAL))?;
    let unreported = score_repo(REPO, &judged, &json!({ "clusters": [] }))?;
    assert_eq!(
        mistyped.false_negatives, 1,
        "renamed code is not a Type I answer"
    );
    assert_eq!(
        first_entry(&found)?.reported_as,
        None,
        "a found pair needs no excuse"
    );
    assert_eq!(
        first_entry(&mistyped)?.reported_as.as_deref(),
        Some(NEARLY_IDENTICAL),
        "the kind the engine did report the pair under is recorded"
    );
    assert_eq!(
        first_entry(&mistyped)?.cluster,
        None,
        "no cluster answered the type"
    );
    assert_eq!(
        first_entry(&unreported)?.reported_as,
        None,
        "nothing reported it at all"
    );
    let rendered = scorecard(&card(&found, &mistyped));
    assert!(
        rendered.contains("**FALSE NEGATIVE** fixture"),
        "the typed miss is a false negative: {rendered}"
    );
    assert!(
        rendered.contains(MISTYPED_DEFECT),
        "the defect line names the type and what the engine called it: {rendered}"
    );
    let silent = scorecard(&card(&found, &unreported));
    assert!(
        silent.contains(UNREPORTED_DEFECT),
        "an unreported typed pair names only its type: {silent}"
    );
    Ok(())
}
